use std::net::IpAddr;
use std::str::FromStr;
use tokio::net::TcpStream;
use tokio::time::timeout;

/// Check if a specific port on a target is open by attempting a TCP connection.
///
/// Returns `true` if the connection succeeds within the timeout, `false` otherwise.
pub async fn check_port(target: &str, port: u16, timeout_ms: u64) -> bool {
    let addr = format!("{}:{}", target, port);

    // Resolve the target if it's a hostname
    let resolved = if let Ok(ip) = IpAddr::from_str(target) {
        format!("{}:{}", ip, port)
    } else {
        // Try to resolve via DNS using tokio
        match tokio::net::lookup_host((target, port)).await {
            Ok(mut addrs) => {
                if let Some(addr) = addrs.next() {
                    format!("{}:{}", addr.ip(), port)
                } else {
                    addr // fallback to original string
                }
            }
            Err(_) => addr,
        }
    };

    let duration = std::time::Duration::from_millis(timeout_ms);
    match timeout(duration, TcpStream::connect(&resolved)).await {
        Ok(Ok(_stream)) => {
            // Connection succeeded - port is open
            true
        }
        _ => false,
    }
}
