pub mod tcp_connect;

use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::types::port_scan::{OpenPort, PortRange, ScanOptions};

/// Scan a range of ports on a target using TCP Connect scanning.
///
/// Uses `tokio::sync::Semaphore` to limit concurrency.
pub async fn scan_ports(
    target: &str,
    port_range: &PortRange,
    options: &ScanOptions,
    cancel: Option<Arc<std::sync::atomic::AtomicBool>>,
) -> Result<Vec<OpenPort>, String> {
    let semaphore = Arc::new(Semaphore::new(options.concurrency as usize));
    let mut handles = Vec::new();

    let start = port_range.start;
    let end = port_range.end;
    let timeout_ms = options.timeout_ms;

    for port in start..=end {
        // Check cancellation before spawning
        if let Some(ref cancel_flag) = cancel {
            if cancel_flag.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }
        }

        let permit = semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| format!("Semaphore error: {}", e))?;

        let target_str = target.to_string();
        let cancel_flag = cancel.clone();

        let handle = tokio::spawn(async move {
            let _permit = permit;
            if let Some(ref flag) = cancel_flag {
                if flag.load(std::sync::atomic::Ordering::SeqCst) {
                    return None;
                }
            }

            let is_open = tcp_connect::check_port(&target_str, port, timeout_ms).await;
            if is_open {
                let service = guess_service(port);
                Some(OpenPort {
                    port,
                    service,
                })
            } else {
                None
            }
        });

        handles.push(handle);
    }

    let mut open_ports = Vec::new();
    for handle in handles {
        match handle.await {
            Ok(Some(port)) => open_ports.push(port),
            Ok(None) => {}
            Err(e) => return Err(format!("Task join error: {}", e)),
        }
    }

    open_ports.sort_by_key(|p| p.port);
    Ok(open_ports)
}

/// Guess the service name for a well-known port.
pub fn guess_service(port: u16) -> Option<String> {
    let name = match port {
        20 | 21 => "FTP",
        22 => "SSH",
        23 => "Telnet",
        25 => "SMTP",
        53 => "DNS",
        80 => "HTTP",
        110 => "POP3",
        143 => "IMAP",
        443 => "HTTPS",
        465 => "SMTPS",
        587 => "SMTP Submission",
        993 => "IMAPS",
        995 => "POP3S",
        1433 => "MSSQL",
        1521 => "Oracle DB",
        3306 => "MySQL",
        3389 => "RDP",
        5432 => "PostgreSQL",
        5900 => "VNC",
        6379 => "Redis",
        8080 => "HTTP-Alt",
        8443 => "HTTPS-Alt",
        27017 => "MongoDB",
        _ => return None,
    };
    Some(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guess_service() {
        assert_eq!(guess_service(22), Some("SSH".to_string()));
        assert_eq!(guess_service(80), Some("HTTP".to_string()));
        assert_eq!(guess_service(443), Some("HTTPS".to_string()));
        assert_eq!(guess_service(9999), None);
    }
}
