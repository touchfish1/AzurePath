use crate::types::network_sniffer::PortResult;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::Semaphore;

/// Scan ports on a single target, returning open port results.
pub async fn scan_ports(
    target: &str,
    ports: &[u16],
    concurrency: usize,
    timeout_ms: u64,
    cancel: Arc<AtomicBool>,
) -> Vec<PortResult> {
    if concurrency == 0 || ports.is_empty() {
        return Vec::new();
    }
    let timeout = Duration::from_millis(timeout_ms);
    let semaphore = Arc::new(Semaphore::new(concurrency));
    let mut results = Vec::new();

    for chunk in ports.chunks(concurrency * 2) {
        if cancel.load(Ordering::SeqCst) {
            break;
        }

        let mut handles = Vec::with_capacity(chunk.len());
        for &port in chunk {
            if cancel.load(Ordering::SeqCst) {
                break;
            }

            let target = target.to_string();
            let sem = semaphore.clone();
            let cancel = cancel.clone();

            handles.push(tokio::spawn(async move {
                let _permit = match sem.acquire_owned().await {
                    Ok(p) => p,
                    Err(_) => return None,
                };
                if cancel.load(Ordering::SeqCst) {
                    return None;
                }
                match tokio::time::timeout(timeout, TcpStream::connect((target.as_str(), port)))
                .await
                {
                    Ok(Ok(_)) => Some(PortResult {
                        port,
                        protocol: "tcp".to_string(),
                        state: "open".to_string(),
                        service: None,
                        version: None,
                        banner: None,
                        confidence: 0,
                        probe_method: "tcp_connect".to_string(),
                    }),
                    _ => None,
                }
            }));
        }

        for h in handles {
            if let Some(port_result) = h.await.ok().flatten() {
                results.push(port_result);
            }
        }
    }

    results.sort_by_key(|r| r.port);
    results
}

/// Return top ports for quick mode.
pub fn top_ports() -> Vec<u16> {
    vec![
        7, 9, 13, 21, 22, 23, 25, 26, 37, 53, 79, 80, 81, 88, 106,
        110, 111, 113, 119, 135, 139, 143, 144, 179, 199, 389, 427,
        443, 444, 445, 465, 513, 514, 515, 543, 544, 548, 554, 587,
        631, 646, 873, 990, 993, 995, 1025, 1026, 1027, 1028, 1029,
        1110, 1433, 1720, 1723, 1755, 1900, 2000, 2001, 2049, 2121,
        2717, 3000, 3128, 3306, 3389, 3986, 4899, 5000, 5009, 5051,
        5060, 5101, 5190, 5357, 5432, 5631, 5666, 5800, 5900, 6000,
        6001, 6646, 7070, 8000, 8008, 8009, 8080, 8443, 8888, 9000,
        9001, 9090, 9100, 9999, 10000, 32768, 49152, 49153, 49154,
        49155, 49156,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn test_top_ports_non_empty() {
        let ports = top_ports();
        assert!(!ports.is_empty());
        assert!(ports.contains(&80));
        assert!(ports.contains(&443));
        assert!(ports.contains(&22));
    }

    #[tokio::test]
    async fn test_cancel_stops_scan() {
        let cancel = Arc::new(AtomicBool::new(true));
        let ports = vec![80, 443, 22, 3306];
        let results = scan_ports("127.0.0.1", &ports, 5, 500, cancel).await;
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_scan_ports_zero_concurrency() {
        // concurrency=0 must not panic or deadlock
        let cancel = Arc::new(AtomicBool::new(false));
        let ports = vec![80, 443];
        let results = scan_ports("127.0.0.1", &ports, 0, 500, cancel).await;
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_scan_ports_empty_ports() {
        // empty port list must return immediately
        let cancel = Arc::new(AtomicBool::new(false));
        let results = scan_ports("127.0.0.1", &[], 5, 500, cancel).await;
        assert!(results.is_empty());
    }
}
