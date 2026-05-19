use crate::types::http_check::HttpCheckResult;
use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

/// Parse the status line from an HTTP response (e.g. "HTTP/1.1 200 OK").
fn parse_status_line(line: &str) -> Option<(u16, String)> {
    let parts: Vec<&str> = line.splitn(3, ' ').collect();
    if parts.len() < 2 {
        return None;
    }
    let status_code = parts[1].parse::<u16>().ok()?;
    let status_text = parts.get(2).unwrap_or(&"").trim().to_string();
    Some((status_code, status_text))
}

/// Parse a single header line (e.g. "Content-Type: text/html").
fn parse_header(line: &str) -> Option<(String, String)> {
    let idx = line.find(':')?;
    let key = line[..idx].trim().to_string();
    let value = line[idx + 1..].trim().to_string();
    if key.is_empty() {
        return None;
    }
    Some((key, value))
}

pub async fn check_http(url: &str, timeout_secs: u64) -> Result<HttpCheckResult, String> {
    // Parse URL
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("URL must start with http:// or https://".to_string());
    }

    let is_tls = url.starts_with("https://");
    let rest = url
        .trim_start_matches("https://")
        .trim_start_matches("http://");

    let (host, port) = if let Some(idx) = rest.find(':') {
        let host_part = &rest[..idx];
        let after_colon = &rest[idx + 1..];
        let port_end = after_colon.find(|c: char| c == '/' || c == '?').unwrap_or(after_colon.len());
        let port_str = &after_colon[..port_end];
        let p: u16 = port_str.parse().map_err(|_| format!("Invalid port: {}", port_str))?;
        (host_part.to_string(), p)
    } else {
        let path_end = rest.find(|c: char| c == '/' || c == '?').unwrap_or(rest.len());
        let host_part = &rest[..path_end];
        (host_part.to_string(), if is_tls { 443 } else { 80 })
    };

    let path = if let Some(idx) = rest.find('/') {
        &rest[idx..]
    } else {
        "/"
    };

    let addr = format!("{}:{}", host, port);
    let start = std::time::Instant::now();

    let stream = TcpStream::connect(&addr)
        .await
        .map_err(|e| format!("Failed to connect to {}: {}", addr, e))?;

    // For HTTPS, we can't do TLS over raw TCP in this simple check.
    // We'll still attempt it but note the limitation.
    if is_tls {
        return Err("HTTPS check requires TLS support. Please use http:// or the SSL check feature.".to_string());
    }

    let mut stream = stream;
    let request = format!(
        "GET {} HTTP/1.0\r\nHost: {}\r\nUser-Agent: AzurePath/1.0\r\nAccept: */*\r\nConnection: close\r\n\r\n",
        path, host
    );

    stream
        .write_all(request.as_bytes())
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;
    stream
        .flush()
        .await
        .map_err(|e| format!("Failed to flush: {}", e))?;

    let mut buf = vec![0u8; 8192];
    let read_result = timeout(Duration::from_secs(timeout_secs), stream.read(&mut buf)).await;

    let n = match read_result {
        Ok(Ok(n)) => n,
        Ok(Err(e)) => return Err(format!("Failed to read response: {}", e)),
        Err(_) => return Err("HTTP check timed out".to_string()),
    };

    let elapsed = start.elapsed();
    buf.truncate(n);
    let raw = String::from_utf8_lossy(&buf);

    // Split headers from body
    let parts: Vec<&str> = raw.splitn(2, "\r\n\r\n").collect();
    let header_section = parts.first().unwrap_or(&"");

    let mut lines = header_section.lines();
    let status_line = lines.next().unwrap_or("");

    let (status_code, status_text) = parse_status_line(status_line)
        .unwrap_or((0, "Unknown".to_string()));

    let mut headers = HashMap::new();
    for line in lines {
        if let Some((key, value)) = parse_header(line) {
            headers.insert(key, value);
        }
    }

    Ok(HttpCheckResult {
        url: url.to_string(),
        status_code,
        status_text,
        headers,
        response_time_ms: elapsed.as_millis() as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_status_line() {
        let (code, text) = parse_status_line("HTTP/1.1 200 OK").unwrap();
        assert_eq!(code, 200);
        assert_eq!(text, "OK");
    }

    #[test]
    fn test_parse_status_line_not_found() {
        let (code, text) = parse_status_line("HTTP/1.1 404 Not Found").unwrap();
        assert_eq!(code, 404);
        assert_eq!(text, "Not Found");
    }

    #[test]
    fn test_parse_status_line_redirect() {
        let (code, text) = parse_status_line("HTTP/1.1 301 Moved Permanently").unwrap();
        assert_eq!(code, 301);
        assert_eq!(text, "Moved Permanently");
    }

    #[test]
    fn test_parse_status_line_invalid() {
        assert!(parse_status_line("Invalid line").is_none());
        assert!(parse_status_line("").is_none());
    }

    #[test]
    fn test_parse_status_line_minimal() {
        let (code, text) = parse_status_line("HTTP/1.0 200").unwrap();
        assert_eq!(code, 200);
        assert_eq!(text, "");
    }

    #[test]
    fn test_parse_header() {
        let (key, value) = parse_header("Content-Type: text/html").unwrap();
        assert_eq!(key, "Content-Type");
        assert_eq!(value, "text/html");
    }

    #[test]
    fn test_parse_header_multiple_colons() {
        let (key, value) = parse_header("Location: http://example.com/test").unwrap();
        assert_eq!(key, "Location");
        assert_eq!(value, "http://example.com/test");
    }

    #[test]
    fn test_parse_header_no_colon() {
        assert!(parse_header("NoColonLine").is_none());
    }

    #[test]
    fn test_parse_header_empty_value() {
        let (key, value) = parse_header("X-Empty: ").unwrap();
        assert_eq!(key, "X-Empty");
        assert_eq!(value, "");
    }

    #[test]
    fn test_parse_header_empty_key() {
        assert!(parse_header(": value").is_none());
    }

    #[test]
    fn test_check_http_invalid_url() {
        let result = futures::executor::block_on(check_http("ftp://example.com", 5));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("URL must start with"));
    }

    #[test]
    fn test_check_http_empty_url() {
        let result = futures::executor::block_on(check_http("", 5));
        assert!(result.is_err());
    }

    #[test]
    fn test_url_parsing_logic() {
        // Test the URL parsing logic used in check_http
        let url = "http://example.com:8080/path?query=1";
        let rest = url.trim_start_matches("https://").trim_start_matches("http://");
        assert_eq!(rest, "example.com:8080/path?query=1");

        let at_slash = rest.find('/').unwrap();
        let host_and_port = &rest[..at_slash];
        assert_eq!(host_and_port, "example.com:8080");

        let path = &rest[at_slash..];
        assert_eq!(path, "/path?query=1");
    }
}
