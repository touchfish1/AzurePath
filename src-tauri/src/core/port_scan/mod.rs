pub mod tcp_connect;

use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::types::port_scan::{OpenPort, PortRange, ScanOptions};

/// Scan a range of ports on a target using TCP Connect scanning.
///
/// Uses `tokio::sync::Semaphore` to limit concurrency.
///
/// # Errors
///
/// Returns an error if:
/// - `options.concurrency` is 0 (would deadlock the semaphore)
/// - DNS resolution of `target` fails and `target` is not an IP address
pub async fn scan_ports(
    target: &str,
    port_range: &PortRange,
    options: &ScanOptions,
    cancel: Option<Arc<std::sync::atomic::AtomicBool>>,
) -> Result<Vec<OpenPort>, String> {
    if options.concurrency == 0 {
        return Err("Concurrency must be greater than 0".to_string());
    }

    // Pre-resolve target to an IP string once so check_port never re-resolves
    let resolved_target = tcp_connect::resolve_target(target).await?;

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

        let target_str = resolved_target.clone();
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
    use crate::types::port_scan::ScanOptions;

    #[test]
    fn test_guess_service_known_ports() {
        let cases = [
            (20, "FTP"),
            (21, "FTP"),
            (22, "SSH"),
            (23, "Telnet"),
            (25, "SMTP"),
            (53, "DNS"),
            (80, "HTTP"),
            (110, "POP3"),
            (143, "IMAP"),
            (443, "HTTPS"),
            (465, "SMTPS"),
            (587, "SMTP Submission"),
            (993, "IMAPS"),
            (995, "POP3S"),
            (1433, "MSSQL"),
            (1521, "Oracle DB"),
            (3306, "MySQL"),
            (3389, "RDP"),
            (5432, "PostgreSQL"),
            (5900, "VNC"),
            (6379, "Redis"),
            (8080, "HTTP-Alt"),
            (8443, "HTTPS-Alt"),
            (27017, "MongoDB"),
        ];
        for (port, expected) in &cases {
            assert_eq!(guess_service(*port), Some(expected.to_string()));
        }
    }

    #[test]
    fn test_guess_service_unknown_ports() {
        // Boundary: port 0 (valid u16)
        assert_eq!(guess_service(0), None);
        // Entirely unassigned
        assert_eq!(guess_service(9999), None);
        // Upper bound
        assert_eq!(guess_service(65535), None);
    }

    #[test]
    fn test_scan_ports_rejects_zero_concurrency() {
        let range = PortRange { start: 1, end: 10 };
        let opts = ScanOptions {
            concurrency: 0,
            timeout_ms: 100,
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(scan_ports("127.0.0.1", &range, &opts, None));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Concurrency"));
    }

    #[test]
    fn test_scan_ports_resolves_target_ip() {
        // When target is already an IP, resolution succeeds synchronously.
        let range = PortRange { start: 1, end: 3 };
        let opts = ScanOptions {
            concurrency: 10,
            timeout_ms: 100,
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        // These ports are unlikely to be open on localhost,
        // but the function should not panic or fail from DNS resolution.
        let result = rt.block_on(scan_ports("127.0.0.1", &range, &opts, None));
        // The scan itself returns Ok even if no ports are found
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolve_target_ip_is_ok() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(tcp_connect::resolve_target("192.168.0.1"));
        assert_eq!(result.unwrap(), "192.168.0.1");
    }
}
