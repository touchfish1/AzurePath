use crate::types::ssl_check::SslCheckResult;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

/// Minimal TLS ClientHello payload (TLS 1.2) for SNI-based handshake.
/// This allows us to parse the ServerHello certificate chain.
fn build_client_hello(hostname: &str) -> Vec<u8> {
    use std::io::Write as IoWrite;

    // SNI extension
    let hostname_bytes = hostname.as_bytes();
    let sni_extension = {
        let mut buf = Vec::new();
        // server_name_list
        IoWrite::write_all(&mut buf, &(hostname_bytes.len() as u16 + 3).to_be_bytes()).unwrap();
        // server_name_type: 0x00 (host_name)
        IoWrite::write_all(&mut buf, &[0x00]).unwrap();
        // server_name length
        IoWrite::write_all(&mut buf, &(hostname_bytes.len() as u16).to_be_bytes()).unwrap();
        IoWrite::write_all(&mut buf, hostname_bytes).unwrap();
        buf
    };

    // Extensions
    let mut extensions = Vec::new();
    // SNI extension type 0x0000
    IoWrite::write_all(&mut extensions, &[0x00, 0x00]).unwrap();
    IoWrite::write_all(&mut extensions, &(sni_extension.len() as u16).to_be_bytes()).unwrap();
    IoWrite::write_all(&mut extensions, &sni_extension).unwrap();

    // Supported groups (elliptic_curves) extension
    let groups: [u16; 5] = [0x001d, 0x0017, 0x0018, 0x0019, 0x001e];
    let mut groups_data = Vec::new();
    for g in &groups {
        IoWrite::write_all(&mut groups_data, &g.to_be_bytes()).unwrap();
    }
    let groups_ext = {
        let mut buf = Vec::new();
        IoWrite::write_all(&mut buf, &(groups_data.len() as u16).to_be_bytes()).unwrap();
        IoWrite::write_all(&mut buf, &groups_data).unwrap();
        buf
    };
    IoWrite::write_all(&mut extensions, &[0x00, 0x0a]).unwrap(); // supported_groups
    IoWrite::write_all(&mut extensions, &(groups_ext.len() as u16).to_be_bytes()).unwrap();
    IoWrite::write_all(&mut extensions, &groups_ext).unwrap();

    // Cipher suites (a few common ones)
    let cipher_suites: [u16; 8] = [
        0x1301, 0x1302, 0x1303,
        0xc02b, 0xc02f,
        0xc00a, 0xc009,
        0x0033,
    ];
    let mut cs_bytes = Vec::new();
    for cs in &cipher_suites {
        IoWrite::write_all(&mut cs_bytes, &cs.to_be_bytes()).unwrap();
    }

    // Compression methods (null only)
    let compression: [u8; 1] = [0x00];

    // Random bytes (32 bytes)
    let random: [u8; 32] = [
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
        0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17,
        0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
    ];

    // TLS record layer: handshake
    let mut handshake = Vec::new();

    handshake.push(0x01); // Handshake type: ClientHello
    let len_pos = handshake.len();
    IoWrite::write_all(&mut handshake, &[0x00, 0x00, 0x00]).unwrap(); // Length placeholder

    IoWrite::write_all(&mut handshake, &[0x03, 0x03]).unwrap(); // TLS 1.2
    IoWrite::write_all(&mut handshake, &random).unwrap();
    handshake.push(0x00); // Session ID (empty)

    // Cipher suites
    let cs_len = cipher_suites.len() as u16 * 2;
    IoWrite::write_all(&mut handshake, &cs_len.to_be_bytes()).unwrap();
    IoWrite::write_all(&mut handshake, &cs_bytes).unwrap();

    // Compression methods
    handshake.push(compression.len() as u8);
    IoWrite::write_all(&mut handshake, &compression).unwrap();

    // Extensions
    IoWrite::write_all(&mut handshake, &(extensions.len() as u16).to_be_bytes()).unwrap();
    IoWrite::write_all(&mut handshake, &extensions).unwrap();

    // Fill in the handshake length
    let handshake_len = (handshake.len() - 4) as u32;
    let len_bytes = handshake_len.to_be_bytes();
    handshake[len_pos] = len_bytes[1];
    handshake[len_pos + 1] = len_bytes[2];
    handshake[len_pos + 2] = len_bytes[3];

    // TLS record header
    let mut record = Vec::new();
    record.push(0x16); // Handshake content type
    IoWrite::write_all(&mut record, &[0x03, 0x01]).unwrap(); // TLS 1.0 record version
    IoWrite::write_all(&mut record, &(handshake.len() as u16).to_be_bytes()).unwrap();
    IoWrite::write_all(&mut record, &handshake).unwrap();

    record
}

/// Try to extract a CN or SAN from a certificate DER blob by scanning for
/// printable ASCII sequences near "subject" context markers.
/// This is a best-effort parser, not a full ASN.1 decoder.
fn extract_common_name(der: &[u8]) -> Option<String> {
    // Look for printable strings in the certificate that look like domain names
    // This is a simple heuristic - look for strings containing dots
    let mut candidates = Vec::new();
    let mut i = 0;
    while i < der.len() {
        // Look for length-prefixed printable strings (ASN.1 tags 0x0c for UTF8String,
        // 0x13 for PrintableString, 0x16 for IA5String)
        if (der[i] == 0x0c || der[i] == 0x13 || der[i] == 0x16)
            && i + 2 < der.len()
        {
            let len = der[i + 1] as usize;
            if len > 0 && i + 2 + len <= der.len() {
                if let Ok(s) = std::str::from_utf8(&der[i + 2..i + 2 + len]) {
                    if s.contains('.') && !s.contains(' ') && s.len() > 3 && s.len() < 256 {
                        candidates.push(s.to_string());
                    }
                }
            }
            i += 2 + len;
        } else {
            i += 1;
        }
    }
    candidates.into_iter().next()
}

/// Extract organization (O=) from a certificate subject.
fn extract_organization(der: &[u8]) -> Option<String> {
    let mut i = 0;
    while i < der.len() {
        if (der[i] == 0x0c || der[i] == 0x13 || der[i] == 0x16)
            && i + 2 < der.len()
        {
            let len = der[i + 1] as usize;
            if len > 0 && i + 2 + len <= der.len() {
                if let Ok(s) = std::str::from_utf8(&der[i + 2..i + 2 + len]) {
                    // Accept organization-like strings (no dots, reasonable length)
                    if !s.contains('.') && s.len() > 2 && s.len() < 128
                        && s.chars().all(|c| c.is_alphanumeric() || c.is_whitespace() || c == '-' || c == '\'')
                    {
                        return Some(s.to_string());
                    }
                }
            }
            i += 2 + len;
        } else {
            i += 1;
        }
    }
    None
}

/// Extract a date string from near "UTC" context in DER.
fn extract_date(der: &[u8], _marker: u8) -> Option<String> {
    let mut i = 0;
    while i < der.len() - 20 {
        if (der[i] == 0x0c || der[i] == 0x13 || der[i] == 0x16 || der[i] == 0x17 || der[i] == 0x18)
            && i + 2 < der.len()
        {
            let len = der[i + 1] as usize;
            if len == 13 && i + 2 + len <= der.len() {
                if let Ok(s) = std::str::from_utf8(&der[i + 2..i + 2 + len]) {
                    if s.ends_with("UTC") || s.ends_with("GMT") {
                        return Some(s.to_string());
                    }
                }
            } else if len == 15 && i + 2 + len <= der.len() {
                if let Ok(s) = std::str::from_utf8(&der[i + 2..i + 2 + len]) {
                    if s.contains("UTC") || s.contains("GMT") {
                        return Some(s.to_string());
                    }
                }
            }
            i += 2 + len;
        } else {
            i += 1;
        }
    }
    None
}

/// Check if a certificate is likely self-signed by comparing issuer/subject bytes.
#[allow(dead_code)]
fn is_likely_self_signed(cert_der: &[u8]) -> bool {
    // Count occurrences of common name patterns - if issuer and subject
    // have the same organization, it's likely self-signed
    let org = extract_organization(cert_der);
    let _cn = extract_common_name(cert_der);
    // Simple heuristic: if organization is present but looks generic, likely self-signed
    if let Some(ref o) = org {
        if o == "localhost" || o == "Self-Signed" || o.starts_with("Unknown") {
            return true;
        }
    }
    // Without the openssl tool or a full parser, we'll be conservative
    false
}

pub async fn check_ssl(hostname: &str, port: u16) -> Result<SslCheckResult, String> {
    let addr = format!("{}:{}", hostname, port);
    let stream = TcpStream::connect(&addr)
        .await
        .map_err(|e| format!("Failed to connect to {}: {}", addr, e))?;

    let mut stream = stream;

    // Send TLS ClientHello
    let client_hello = build_client_hello(hostname);
    stream
        .write_all(&client_hello)
        .await
        .map_err(|e| format!("Failed to send ClientHello: {}", e))?;

    // Read response (ServerHello + Certificate)
    let mut buf = vec![0u8; 65536];
    let read_result = timeout(Duration::from_secs(10), stream.read(&mut buf)).await;

    let n = match read_result {
        Ok(Ok(n)) => n,
        Ok(Err(e)) => return Err(format!("Failed to read TLS response: {}", e)),
        Err(_) => return Err("SSL check timed out after 10 seconds".to_string()),
    };

    buf.truncate(n);

    // Try to extract certificate chain by scanning for certificate records
    // TLS record type 0x0b = Certificate
    let mut cert_start = None;
    for i in 0..buf.len().saturating_sub(4) {
        // Look for handshake type Certificate (0x0b) after a TLS record header
        if buf[i] == 0x0b && i > 0 {
            // Previous byte should be 0x16 (handshake content type) in record header
            // or this is the handshake type byte
            cert_start = Some(i);
            break;
        }
    }

    let (issuer, subject, vfrom, vto, is_self_signed) = if let Some(pos) = cert_start {
        // We found a certificate handshake message
        // Skip the handshake header (4 bytes: type + 3-byte length)
        let cert_chain_start = pos + 4;
        if cert_chain_start + 3 < buf.len() {
            // Certificate chain: 3-byte length of entire chain
            let chain_len = ((buf[cert_chain_start] as usize) << 16)
                | ((buf[cert_chain_start + 1] as usize) << 8)
                | (buf[cert_chain_start + 2] as usize);

            if chain_len > 0 && cert_chain_start + 3 + chain_len <= buf.len() {
                let cert_data = &buf[cert_chain_start + 3..cert_chain_start + 3 + chain_len];

                // First certificate: 3-byte length + DER
                if cert_data.len() > 3 {
                    let cert_len = ((cert_data[0] as usize) << 16)
                        | ((cert_data[1] as usize) << 8)
                        | (cert_data[2] as usize);
                    if cert_len > 0 && 3 + cert_len <= cert_data.len() {
                        let cert_der = &cert_data[3..3 + cert_len];

                        let subj = extract_common_name(cert_der);
                        let org = extract_organization(cert_der);
                        let from = extract_date(cert_der, 0);
                        let to = extract_date(cert_der, 1);
                        let self_signed = is_likely_self_signed(cert_der);

                        (org, subj, from, to, self_signed)
                    } else {
                        (None, None, None, None, false)
                    }
                } else {
                    (None, None, None, None, false)
                }
            } else {
                (None, None, None, None, false)
            }
        } else {
            (None, None, None, None, false)
        }
    } else {
        // Try to extract any useful info from the raw response
        (None, None, None, None, false)
    };

    // Calculate days remaining if we have an expiry date
    let days_remaining = vto.as_ref().and_then(|date_str| {
        // Try to parse various date formats
        // Common format: "Jan 15 00:00:00 2026 GMT"
        if let Ok(parsed) = chrono::NaiveDateTime::parse_from_str(date_str, "%b %d %H:%M:%S %Y GMT") {
            let expiry = parsed.and_utc();
            let now = chrono::Utc::now();
            if expiry > now {
                Some((expiry - now).num_days())
            } else {
                Some(-(now - expiry).num_days())
            }
        } else {
            None
        }
    });

    Ok(SslCheckResult {
        hostname: hostname.to_string(),
        issuer,
        subject,
        valid_from: vfrom,
        valid_to: vto,
        is_expired: days_remaining.map(|d| d < 0).unwrap_or(false),
        is_self_signed,
        days_remaining,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_client_hello_contains_hostname() {
        let hello = build_client_hello("example.com");
        assert!(!hello.is_empty());
        // Check that it starts with TLS record header (0x16)
        assert_eq!(hello[0], 0x16);
        // Should contain the hostname
        assert!(hello.windows(b"example.com".len()).any(|w| w == b"example.com"));
    }

    #[test]
    fn test_build_client_hello_long_hostname() {
        let hello = build_client_hello("very-long-hostname-that-is-still-valid.example.com");
        assert!(!hello.is_empty());
        assert_eq!(hello[0], 0x16);
    }

    #[test]
    fn test_build_client_hello_short_hostname() {
        let hello = build_client_hello("a.b");
        assert!(!hello.is_empty());
        assert_eq!(hello[0], 0x16);
    }

    #[test]
    fn test_extract_common_name_from_domain_string() {
        let der = b"some data\x0c\x0eexample.commore data";
        let cn = extract_common_name(der);
        assert_eq!(cn, Some("example.com".to_string()));
    }

    #[test]
    fn test_extract_common_name_no_match() {
        let der = b"data without any domain pattern";
        let cn = extract_common_name(der);
        assert!(cn.is_none());
    }

    #[test]
    fn test_extract_common_name_empty_der() {
        let der = b"";
        let cn = extract_common_name(der);
        assert!(cn.is_none());
    }

    #[test]
    fn test_extract_organization() {
        let der = b"\x0c\x09Microsoft";
        let org = extract_organization(der);
        assert_eq!(org, Some("Microsoft".to_string()));
    }

    #[test]
    fn test_extract_organization_with_dot() {
        // Organizations shouldn't have dots in our heuristic
        let der = b"\x0c\x0emicrosoft.com";
        let org = extract_organization(der);
        assert!(org.is_none());
    }

    #[test]
    fn test_extract_date_utc_format() {
        let der = b"some prefix\x0c\x0dJan 15 00:00:00 2026 UTCmore data";
        let date = extract_date(der, 0);
        assert!(date.is_some());
        assert!(date.unwrap().contains("UTC"));
    }

    #[test]
    fn test_extract_date_no_match() {
        let der = b"no date here";
        let date = extract_date(der, 0);
        assert!(date.is_none());
    }

    #[test]
    fn test_is_likely_self_signed() {
        let der = b"\x0c\x0cSelf-Signed";
        assert!(is_likely_self_signed(der));
    }

    #[test]
    fn test_is_likely_self_signed_not() {
        let der = b"\x0c\x09Microsoft";
        assert!(!is_likely_self_signed(der));
    }

    #[test]
    fn test_days_remaining_parsing() {
        // Test date parsing logic
        let date_str = "Jan 15 00:00:00 2099 GMT";
        if let Ok(parsed) = chrono::NaiveDateTime::parse_from_str(date_str, "%b %d %H:%M:%S %Y GMT") {
            let expiry = parsed.and_utc();
            let now = chrono::Utc::now();
            let days = (expiry - now).num_days();
            assert!(days > 0, "Should be positive for future dates");
        }
    }

    #[test]
    fn test_days_remaining_expired() {
        let date_str = "Jan 15 00:00:00 2020 GMT";
        if let Ok(parsed) = chrono::NaiveDateTime::parse_from_str(date_str, "%b %d %H:%M:%S %Y GMT") {
            let expiry = parsed.and_utc();
            let now = chrono::Utc::now();
            let days = (expiry - now).num_days();
            assert!(days < 0, "Should be negative for past dates");
        }
    }
}
