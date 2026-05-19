use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Read initial banner from an open TCP connection.
/// Sends optional probe bytes, then reads up to 4096 bytes of response.
pub async fn grab_banner(
    target: &str,
    port: u16,
    timeout_ms: u64,
) -> Option<String> {
    let timeout = std::time::Duration::from_millis(timeout_ms);
    let addr = format!("{}:{}", target, port);

    let mut stream = tokio::time::timeout(timeout, TcpStream::connect(&addr))
        .await
        .ok()?
        .ok()?;

    // Send probe based on port
    let probe = probe_for_port(port);
    if !probe.is_empty() {
        let _ = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            stream.write_all(probe.as_bytes()),
        )
        .await;
    }

    // Read response
    let mut buf = vec![0u8; 4096];
    let n = tokio::time::timeout(timeout, stream.read(&mut buf))
        .await
        .ok()?
        .ok()?;

    if n == 0 {
        return None;
    }

    let banner = String::from_utf8_lossy(&buf[..n.min(4096)]).to_string();
    let banner = banner.trim_matches('\0').trim().to_string();
    if banner.is_empty() { None } else { Some(banner) }
}

/// Return a probe string appropriate for the given port.
fn probe_for_port(port: u16) -> &'static str {
    match port {
        21 => "",
        22 => "",
        25 | 587 => "EHLO probe\r\n",
        80 | 8080 | 8000 | 8888 => "GET / HTTP/1.0\r\nHost: localhost\r\n\r\n",
        110 => "",
        143 => "",
        3306 => "",
        5432 => "",
        6379 => "PING\r\n",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probe_for_port() {
        assert_eq!(probe_for_port(80), "GET / HTTP/1.0\r\nHost: localhost\r\n\r\n");
        assert_eq!(probe_for_port(22), "");
        assert_eq!(probe_for_port(6379), "PING\r\n");
        assert_eq!(probe_for_port(9999), "");
    }
}
