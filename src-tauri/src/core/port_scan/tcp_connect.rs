use std::net::IpAddr;
use std::str::FromStr;
use tokio::net::TcpStream;
use tokio::time::timeout;

/// Check if a specific port on a target is open by attempting a TCP connection.
///
/// Returns `true` if the connection succeeds within the timeout, `false` otherwise.
///
/// When `target` is an IP address string, no DNS lookup is performed.
/// When `target` is a hostname, `tokio::net::lookup_host` is called
/// to resolve it once per call.
///
/// For bulk scanning, pre-resolve with [`resolve_target`] and pass the
/// returned IP string to avoid per-port DNS lookups.
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

/// Resolve a target hostname to an IP address string.
///
/// If `target` is already a valid IP address string, returns it unchanged
/// without performing any DNS lookup. This is efficient for bulk port scans
/// where the same target is probed across many ports.
///
/// If DNS resolution fails, an error is returned so callers can fail fast
/// rather than retrying resolution inside every port probe.
pub async fn resolve_target(target: &str) -> Result<String, String> {
    if let Ok(ip) = IpAddr::from_str(target) {
        return Ok(ip.to_string());
    }
    match tokio::net::lookup_host((target, 0)).await {
        Ok(mut addrs) => addrs
            .next()
            .map(|addr| addr.ip().to_string())
            .ok_or_else(|| format!("DNS resolved but no addresses found for: {}", target)),
        Err(e) => Err(format!("DNS resolution failed for '{}': {}", target, e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resolve_target_ipv4() {
        let result = resolve_target("127.0.0.1").await;
        assert_eq!(result.unwrap(), "127.0.0.1");
    }

    #[tokio::test]
    async fn test_resolve_target_ipv6() {
        let result = resolve_target("::1").await;
        assert_eq!(result.unwrap(), "::1");
    }

    #[tokio::test]
    #[cfg(not(target_os = "windows"))]
    async fn test_resolve_target_empty_fails() {
        // On Unix-like systems, an empty hostname should fail DNS resolution.
        // On Windows, lookup_host("") may resolve to localhost instead,
        // so this test is disabled there.
        let result = resolve_target("").await;
        assert!(result.is_err());
    }
}
