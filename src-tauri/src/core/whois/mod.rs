use crate::types::whois::WhoisResult;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

/// Map a domain to its appropriate WHOIS server based on TLD.
fn get_whois_server(domain: &str) -> &str {
    let lower = domain.to_lowercase();
    if lower.ends_with(".com") || lower.ends_with(".net") {
        "whois.verisign-grs.com"
    } else if lower.ends_with(".org") {
        "whois.pir.org"
    } else if lower.ends_with(".cn") {
        "whois.cnnic.cn"
    } else if lower.ends_with(".edu") {
        "whois.educause.edu"
    } else if lower.ends_with(".gov") {
        "whois.dotgov.gov"
    } else if lower.ends_with(".info") {
        "whois.afilias.net"
    } else if lower.ends_with(".biz") {
        "whois.neulevel.biz"
    } else if lower.ends_with(".io") {
        "whois.nic.io"
    } else if lower.ends_with(".me") {
        "whois.nic.me"
    } else {
        "whois.verisign-grs.com"
    }
}

/// Parse a key: value line from WHOIS response.
fn parse_field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let lower = line.to_lowercase();
    if lower.starts_with(&key.to_lowercase()) {
        let val = line.split(':').nth(1)?.trim();
        if !val.is_empty() {
            return Some(val);
        }
    }
    None
}

/// Collect all values for a given key (for multi-valued fields like name servers).
fn parse_fields<'a>(response: &'a str, key: &str) -> Vec<&'a str> {
    response
        .lines()
        .filter_map(|line| parse_field(line, key))
        .collect()
}

pub async fn whois_lookup(query: &str, server: Option<&str>) -> Result<WhoisResult, String> {
    let domain = query.trim().trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_start_matches("www.")
        .split('/')
        .next()
        .unwrap_or(query);

    let whois_server = server.unwrap_or_else(|| get_whois_server(domain));

    let addr = format!("{}:43", whois_server);
    let stream = TcpStream::connect(&addr)
        .await
        .map_err(|e| format!("Failed to connect to WHOIS server {}: {}", whois_server, e))?;

    let mut stream = stream;
    let query_line = format!("{}\r\n", domain);
    stream
        .write_all(query_line.as_bytes())
        .await
        .map_err(|e| format!("Failed to send query: {}", e))?;
    stream
        .flush()
        .await
        .map_err(|e| format!("Failed to flush: {}", e))?;

    let mut buf = vec![0u8; 4096];
    let read_result = timeout(Duration::from_secs(5), stream.read(&mut buf)).await;

    let n = match read_result {
        Ok(Ok(n)) => n,
        Ok(Err(e)) => return Err(format!("Failed to read response: {}", e)),
        Err(_) => return Err("WHOIS query timed out after 5 seconds".to_string()),
    };

    buf.truncate(n);
    let raw = String::from_utf8_lossy(&buf).to_string();

    // Helper: get first value from multiple field names
    let first_of = |keys: &[&str]| -> Option<String> {
        for key in keys {
            let vals = parse_fields(&raw, key);
            if let Some(v) = vals.first() {
                return Some(v.to_string());
            }
        }
        None
    };

    // Parse fields from response
    let registrar = parse_fields(&raw, "Registrar:").first().map(|s| s.to_string());
    let creation_date = first_of(&["Creation Date:", "Created:", "created:"]);
    let expiration_date = first_of(&["Registry Expiry Date:", "Expiration Date:", "Expiry Date:"]);
    let name_servers: Vec<String> = parse_fields(&raw, "Name Server:")
        .iter()
        .map(|s| s.to_string())
        .collect();

    Ok(WhoisResult {
        raw,
        domain: Some(domain.to_string()),
        registrar,
        creation_date,
        expiration_date,
        name_servers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_whois_server() {
        assert_eq!(get_whois_server("example.com"), "whois.verisign-grs.com");
        assert_eq!(get_whois_server("example.net"), "whois.verisign-grs.com");
        assert_eq!(get_whois_server("example.org"), "whois.pir.org");
        assert_eq!(get_whois_server("example.cn"), "whois.cnnic.cn");
        assert_eq!(get_whois_server("example.edu"), "whois.educause.edu");
    }

    #[test]
    fn test_get_whois_server_case_insensitive() {
        assert_eq!(get_whois_server("EXAMPLE.COM"), "whois.verisign-grs.com");
        assert_eq!(get_whois_server("Example.Org"), "whois.pir.org");
    }

    #[test]
    fn test_get_whois_server_unknown_tld() {
        assert_eq!(get_whois_server("example.xyz"), "whois.verisign-grs.com");
        assert_eq!(get_whois_server("example.dev"), "whois.verisign-grs.com");
    }

    #[test]
    fn test_get_whois_server_no_tld() {
        assert_eq!(get_whois_server("localhost"), "whois.verisign-grs.com");
    }

    #[test]
    fn test_parse_field() {
        assert_eq!(parse_field("Registrar: GoDaddy", "Registrar:"), Some("GoDaddy"));
        assert_eq!(parse_field("  Name Server: NS1.EXAMPLE.COM", "Name Server:"), Some("NS1.EXAMPLE.COM"));
    }

    #[test]
    fn test_parse_field_not_found() {
        assert_eq!(parse_field("Some random line", "Registrar:"), None);
    }

    #[test]
    fn test_parse_field_empty_value() {
        assert_eq!(parse_field("Registrar: ", "Registrar:"), None);
    }

    #[test]
    fn test_parse_fields_multi_value() {
        let response = "Name Server: NS1.EXAMPLE.COM\nName Server: NS2.EXAMPLE.COM\nOther: stuff\n";
        let servers = parse_fields(response, "Name Server:");
        assert_eq!(servers.len(), 2);
        assert_eq!(servers[0], "NS1.EXAMPLE.COM");
        assert_eq!(servers[1], "NS2.EXAMPLE.COM");
    }

    #[test]
    fn test_parse_fields_none_found() {
        let response = "Some other data\nMore lines\n";
        let servers = parse_fields(response, "Name Server:");
        assert!(servers.is_empty());
    }

    #[tokio::test]
    async fn test_whois_lookup_invalid_server() {
        let result = whois_lookup("example.com", Some("nonexistent.invalid:43")).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_domain_url_stripping() {
        // These tests just verify the domain parsing logic inline
        let cases = [
            ("https://www.example.com/path", "example.com"),
            ("http://example.com", "example.com"),
            ("www.example.com", "example.com"),
            ("example.com", "example.com"),
        ];
        for (input, expected) in &cases {
            let result = input.trim().trim_start_matches("http://")
                .trim_start_matches("https://")
                .trim_start_matches("www.")
                .split('/')
                .next()
                .unwrap_or(input);
            assert_eq!(result, *expected);
        }
    }
}
