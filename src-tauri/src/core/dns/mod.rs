use crate::types::dns::{DnsRecord, RecordType};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::net::UdpSocket;

const DNS_SERVER: &str = "8.8.8.8:53";

/// DNS record type numbers.
const TYPE_A: u16 = 1;
const TYPE_AAAA: u16 = 28;
const TYPE_CNAME: u16 = 5;
const TYPE_MX: u16 = 15;
const TYPE_NS: u16 = 2;
const TYPE_SOA: u16 = 6;
const TYPE_TXT: u16 = 16;

/// Resolve DNS records for a target domain, optionally specifying a custom DNS server.
///
/// `dns_server` should be in `ip:port` format (e.g., `8.8.8.8:53`).
/// If the port is omitted, `:53` is appended automatically.
/// When `None`, the default server `8.8.8.8:53` is used.
pub async fn resolve(
    target: &str,
    record_type: &RecordType,
    dns_server: Option<&str>,
) -> Result<Vec<DnsRecord>, String> {
    let server = match dns_server {
        Some(s) => {
            let s = s.trim();
            if s.is_empty() {
                DNS_SERVER.to_string()
            } else if s.contains(':') {
                validate_dns_server(s)?;
                s.to_string()
            } else {
                let full = format!("{}:53", s);
                validate_dns_server(&full)?;
                full
            }
        }
        None => DNS_SERVER.to_string(),
    };

    let types = get_qtypes(record_type);
    let mut all_records = Vec::new();

    for qtype in types {
        let records = resolve_qtype(target, qtype, record_type, &server).await?;
        all_records.extend(records);
    }

    Ok(all_records)
}

/// Validate that a DNS server address is in valid `ip:port` format.
fn validate_dns_server(addr: &str) -> Result<(), String> {
    let parts: Vec<&str> = addr.rsplitn(2, ':').collect();
    // parts[0] = port, parts[1] = ip (when split with rsplitn(2))
    if parts.len() != 2 {
        return Err(
            "DNS 服务器地址格式无效，请使用 ip:port 格式 (例如 8.8.8.8:53)".to_string(),
        );
    }
    let port: u16 = parts[0].parse().map_err(|_| {
        format!(
            "DNS 服务器端口号无效: '{}'",
            parts[0]
        )
    })?;
    if port == 0 {
        return Err("DNS 服务器端口不能为 0".to_string());
    }
    Ok(())
}

/// Get the DNS query type numbers for a given RecordType.
fn get_qtypes(record_type: &RecordType) -> Vec<u16> {
    match record_type {
        RecordType::A => vec![TYPE_A],
        RecordType::Aaaa => vec![TYPE_AAAA],
        RecordType::Cname => vec![TYPE_CNAME],
        RecordType::Mx => vec![TYPE_MX],
        RecordType::Ns => vec![TYPE_NS],
        RecordType::Soa => vec![TYPE_SOA],
        RecordType::Txt => vec![TYPE_TXT],
        RecordType::All => vec![TYPE_A, TYPE_AAAA, TYPE_CNAME, TYPE_MX, TYPE_NS, TYPE_TXT],
    }
}

/// Resolve a single DNS query type.
async fn resolve_qtype(
    target: &str,
    qtype: u16,
    _record_type: &RecordType,
    server: &str,
) -> Result<Vec<DnsRecord>, String> {
    // Build the DNS query
    let query = build_dns_query(target, qtype)?;

    // Send query via UDP
    let socket = UdpSocket::bind("0.0.0.0:0")
        .await
        .map_err(|e| format!("Failed to bind UDP socket: {}", e))?;

    socket
        .send_to(&query, server)
        .await
        .map_err(|e| format!("Failed to send DNS query: {}", e))?;

    let mut buf = vec![0u8; 4096];
    let len = tokio::time::timeout(std::time::Duration::from_secs(5), socket.recv(&mut buf))
        .await
        .map_err(|_| "DNS query timed out".to_string())?
        .map_err(|e| format!("Failed to receive DNS response: {}", e))?;

    buf.truncate(len);

    // Check TC flag (Truncation) — bit 2 of flags field at bytes 2-3
    // If the response was truncated, retry the query via TCP (RFC 5966)
    if len >= 12 {
        let flags = u16::from_be_bytes([buf[2], buf[3]]);
        if flags & 0x0200 != 0 {
            let tcp_response = send_query_tcp(&query, server).await?;
            return parse_dns_response(&tcp_response, target, qtype);
        }
    }

    // Parse the response
    parse_dns_response(&buf, target, qtype)
}

/// Send a DNS query over TCP and return the raw response bytes.
///
/// DNS over TCP uses a 2-byte length prefix (network byte order) before
/// the query message, and the response also begins with a 2-byte length.
async fn send_query_tcp(query: &[u8], server: &str) -> Result<Vec<u8>, String> {
    let addr = server;
    let mut stream = TcpStream::connect(addr)
        .await
        .map_err(|e| format!("TCP connect to DNS server failed: {}", e))?;

    // Send: 2-byte length (big-endian) + query
    let len_bytes = (query.len() as u16).to_be_bytes();
    stream
        .write_all(&len_bytes)
        .await
        .map_err(|e| format!("TCP write length failed: {}", e))?;
    stream
        .write_all(query)
        .await
        .map_err(|e| format!("TCP write query failed: {}", e))?;

    // Read: 2-byte response length
    let mut len_buf = [0u8; 2];
    tokio::time::timeout(std::time::Duration::from_secs(5), stream.read_exact(&mut len_buf))
        .await
        .map_err(|_| "TCP DNS read timeout".to_string())?
        .map_err(|e| format!("TCP read length failed: {}", e))?;

    let resp_len = u16::from_be_bytes(len_buf) as usize;

    // Read the response body
    let mut resp = vec![0u8; resp_len];
    tokio::time::timeout(std::time::Duration::from_secs(5), stream.read_exact(&mut resp))
        .await
        .map_err(|_| "TCP DNS response read timeout".to_string())?
        .map_err(|e| format!("TCP read response failed: {}", e))?;

    Ok(resp)
}

/// Build a raw DNS query message.
fn build_dns_query(target: &str, qtype: u16) -> Result<Vec<u8>, String> {
    let mut buf = Vec::with_capacity(512);

    // Header (12 bytes)
    // ID: random 16-bit
    let id: u16 = rand::random();
    buf.extend_from_slice(&id.to_be_bytes());

    // Flags: standard query (0x0100 = recursion desired)
    buf.extend_from_slice(&0x0100u16.to_be_bytes());

    // QDCOUNT: 1 question
    buf.extend_from_slice(&1u16.to_be_bytes());
    // ANCOUNT: 0
    buf.extend_from_slice(&0u16.to_be_bytes());
    // NSCOUNT: 0
    buf.extend_from_slice(&0u16.to_be_bytes());
    // ARCOUNT: 0
    buf.extend_from_slice(&0u16.to_be_bytes());

    // Question section: encode domain name
    encode_dns_name(target, &mut buf)?;

    // QTYPE
    buf.extend_from_slice(&qtype.to_be_bytes());
    // QCLASS: IN (1)
    buf.extend_from_slice(&1u16.to_be_bytes());

    Ok(buf)
}

/// Encode a domain name in DNS format (length-prefixed labels).
///
/// Each label must be at most 63 bytes (DNS spec limit).
/// Empty labels (from trailing dots or consecutive dots) are skipped.
/// Returns an error if the name is empty or a label exceeds 63 bytes.
fn encode_dns_name(name: &str, buf: &mut Vec<u8>) -> Result<(), String> {
    if name.is_empty() {
        return Err("Domain name cannot be empty".to_string());
    }

    for label in name.split('.') {
        if label.is_empty() {
            continue; // Skip empty labels (trailing dot, consecutive dots)
        }
        if label.len() > 63 {
            return Err(format!(
                "Domain label '{}' exceeds maximum DNS label length of 63 characters",
                label
            ));
        }
        buf.push(label.len() as u8);
        buf.extend_from_slice(label.as_bytes());
    }
    buf.push(0); // root label

    Ok(())
}

/// Parse a DNS response message.
fn parse_dns_response(
    data: &[u8],
    target: &str,
    qtype: u16,
) -> Result<Vec<DnsRecord>, String> {
    if data.len() < 12 {
        return Err("DNS response too short".to_string());
    }

    // Check response flags
    let flags = u16::from_be_bytes([data[2], data[3]]);
    let rcode = flags & 0x000f;
    if rcode != 0 {
        let err = match rcode {
            1 => "Format error",
            2 => "Server failure",
            3 => "Name error (NXDOMAIN)",
            4 => "Not implemented",
            5 => "Refused",
            _ => "Unknown error",
        };
        return Err(format!("DNS error: {}", err));
    }

    let ancount = u16::from_be_bytes([data[6], data[7]]);
    let nscount = u16::from_be_bytes([data[8], data[9]]);
    let arcount = u16::from_be_bytes([data[10], data[11]]);

    let mut offset = 12;

    // Skip the question section
    offset = skip_dns_name(data, offset)?;
    offset += 4; // QTYPE + QCLASS

    let mut records = Vec::new();

    // Parse answer records
    for _ in 0..ancount {
        if let Some((rec, new_offset)) = parse_dns_record(data, offset, target) {
            // Filter by type if not "all"
            if rec.record_type == qtype_to_string(qtype) || qtype == 0 {
                records.push(rec);
            }
            offset = new_offset;
        } else {
            break;
        }
    }

    // Parse authority records
    for _ in 0..nscount {
        if let Some((rec, new_offset)) = parse_dns_record(data, offset, target) {
            records.push(rec);
            offset = new_offset;
        } else {
            break;
        }
    }

    // Parse additional records
    for _ in 0..arcount {
        if let Some((rec, new_offset)) = parse_dns_record(data, offset, target) {
            records.push(rec);
            offset = new_offset;
        } else {
            break;
        }
    }

    Ok(records)
}

/// Skip a DNS name at the given offset, returning the new offset.
fn skip_dns_name(data: &[u8], mut offset: usize) -> Result<usize, String> {
    loop {
        if offset >= data.len() {
            return Err("Unexpected end of DNS name".to_string());
        }
        let len = data[offset];
        if len == 0 {
            return Ok(offset + 1);
        }
        if len & 0xc0 == 0xc0 {
            // Compression pointer (2 bytes)
            return Ok(offset + 2);
        }
        offset += 1 + len as usize;
    }
}

/// Decode a DNS name at the given offset, returning the name and new offset.
fn decode_dns_name(data: &[u8], mut offset: usize) -> Option<(String, usize)> {
    let mut labels = Vec::new();
    let mut jumped = false;
    let mut end_offset = offset;

    loop {
        if offset >= data.len() {
            return None;
        }
        let len = data[offset];
        if len == 0 {
            if !jumped {
                end_offset = offset + 1;
            }
            break;
        }
        if len & 0xc0 == 0xc0 {
            // Compression pointer
            if offset + 1 >= data.len() {
                return None;
            }
            let pointer = ((len & 0x3f) as u16) << 8 | data[offset + 1] as u16;
            if !jumped {
                end_offset = offset + 2;
            }
            offset = pointer as usize;
            jumped = true;
            continue;
        }
        if offset + 1 + len as usize > data.len() {
            return None;
        }
        let label = std::str::from_utf8(&data[offset + 1..offset + 1 + len as usize]).ok()?;
        labels.push(label.to_string());
        offset += 1 + len as usize;
        if !jumped {
            end_offset = offset;
        }
    }

    Some((labels.join("."), end_offset))
}

/// Parse a single DNS resource record.
fn parse_dns_record(data: &[u8], offset: usize, _target: &str) -> Option<(DnsRecord, usize)> {
    let (name, mut offset) = decode_dns_name(data, offset)?;

    if offset + 10 > data.len() {
        return None;
    }

    let rtype = u16::from_be_bytes([data[offset], data[offset + 1]]);
    offset += 2;
    let _rclass = u16::from_be_bytes([data[offset], data[offset + 1]]);
    offset += 2;
    let ttl = u32::from_be_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]);
    offset += 4;
    let rdlength = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
    offset += 2;

    if offset + rdlength > data.len() {
        return None;
    }

    let value = match rtype {
        TYPE_A => {
            if rdlength == 4 {
                format!(
                    "{}.{}.{}.{}",
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3]
                )
            } else {
                format!("{:?}", &data[offset..offset + rdlength])
            }
        }
        TYPE_AAAA => {
            if rdlength == 16 {
                let segments: Vec<String> = data[offset..offset + 16]
                    .chunks(2)
                    .map(|c| format!("{:02x}{:02x}", c[0], c[1]))
                    .collect();
                // Collapse zeros with "::" - simple approach
                let ip = segments.join(":");
                compress_ipv6(&ip)
            } else {
                format!("{:?}", &data[offset..offset + rdlength])
            }
        }
        TYPE_CNAME | TYPE_NS => {
            let (decoded_name, _) = decode_dns_name(data, offset)?;
            decoded_name
        }
        TYPE_MX => {
            if rdlength >= 2 {
                let preference = u16::from_be_bytes([data[offset], data[offset + 1]]);
                let (exchange, _) = decode_dns_name(data, offset + 2)?;
                format!("{} {}", preference, exchange)
            } else {
                String::new()
            }
        }
        TYPE_SOA => {
            let (mname, off) = decode_dns_name(data, offset)?;
            let (rname, off2) = decode_dns_name(data, off)?;
            if off2 + 20 <= data.len() {
                let serial =
                    u32::from_be_bytes([data[off2], data[off2 + 1], data[off2 + 2], data[off2 + 3]]);
                let refresh = u32::from_be_bytes(
                    [data[off2 + 4], data[off2 + 5], data[off2 + 6], data[off2 + 7]],
                );
                let retry = u32::from_be_bytes(
                    [data[off2 + 8], data[off2 + 9], data[off2 + 10], data[off2 + 11]],
                );
                let expire = u32::from_be_bytes(
                    [data[off2 + 12], data[off2 + 13], data[off2 + 14], data[off2 + 15]],
                );
                let minimum = u32::from_be_bytes(
                    [data[off2 + 16], data[off2 + 17], data[off2 + 18], data[off2 + 19]],
                );
                format!(
                    "{} {} {} {} {} {} {}",
                    mname, rname, serial, refresh, retry, expire, minimum
                )
            } else {
                format!("{} {}", mname, rname)
            }
        }
        TYPE_TXT => {
            let mut txt_parts = Vec::new();
            let mut txt_offset = offset;
            let end = offset + rdlength;
            while txt_offset < end {
                if txt_offset >= data.len() {
                    break;
                }
                let txt_len = data[txt_offset] as usize;
                txt_offset += 1;
                if txt_offset + txt_len <= end && txt_offset + txt_len <= data.len() {
                    let txt = std::str::from_utf8(&data[txt_offset..txt_offset + txt_len])
                        .unwrap_or("")
                        .to_string();
                    txt_parts.push(txt);
                    txt_offset += txt_len;
                } else {
                    break;
                }
            }
            txt_parts.join("")
        }
        _ => {
            format!("{:?}", &data[offset..offset + rdlength])
        }
    };

    let record_type_name = qtype_to_string(rtype);

    let record = DnsRecord {
        name,
        record_type: record_type_name,
        value,
        ttl,
    };

    Some((record, offset + rdlength))
}

/// Convert DNS type number to string name.
fn qtype_to_string(qtype: u16) -> String {
    match qtype {
        TYPE_A => "A".to_string(),
        TYPE_AAAA => "AAAA".to_string(),
        TYPE_CNAME => "CNAME".to_string(),
        TYPE_MX => "MX".to_string(),
        TYPE_NS => "NS".to_string(),
        TYPE_SOA => "SOA".to_string(),
        TYPE_TXT => "TXT".to_string(),
        _ => format!("TYPE{}", qtype),
    }
}

/// Simple IPv6 compression: collapse the longest run of zero segments.
fn compress_ipv6(ip: &str) -> String {
    let segments: Vec<&str> = ip.split(':').collect();
    if segments.len() != 8 {
        return ip.to_string();
    }

    // Find the longest run of "0000" or "0"
    let mut best_start = 0;
    let mut best_len = 0;
    let mut cur_start = 0;
    let mut cur_len = 0;
    let mut in_zero_run = false;

    for (i, seg) in segments.iter().enumerate() {
        let is_zero = *seg == "0000" || *seg == "0";
        if is_zero && !in_zero_run {
            in_zero_run = true;
            cur_start = i;
            cur_len = 1;
        } else if is_zero && in_zero_run {
            cur_len += 1;
        } else if !is_zero && in_zero_run {
            if cur_len > best_len {
                best_len = cur_len;
                best_start = cur_start;
            }
            in_zero_run = false;
        }
    }
    if in_zero_run && cur_len > best_len {
        best_len = cur_len;
        best_start = cur_start;
    }

    if best_len < 2 {
        // No compressible run, just normalize
        return segments
            .iter()
            .map(|s| s.trim_start_matches('0'))
            .map(|s| if s.is_empty() { "0" } else { s })
            .collect::<Vec<_>>()
            .join(":");
    }

    // Entire address is a zero-run → "::"
    if best_start == 0 && best_len == 8 {
        return "::".to_string();
    }

    let mut parts: Vec<String> = Vec::new();

    // Segments before the zero-run
    for i in 0..best_start {
        let mut s = segments[i].trim_start_matches('0').to_string();
        if s.is_empty() {
            s = "0".to_string();
        }
        parts.push(s);
    }

    // "::" separator
    parts.push(String::new());

    // Segments after the zero-run
    for i in (best_start + best_len)..8 {
        let s = segments[i].trim_start_matches('0').to_string();
        parts.push(if s.is_empty() { "0".to_string() } else { s });
    }

    // Build result; handle edge cases where the zero-run
    // is at the very start or very end of the address.
    let joined = parts.join(":").replace(":::", "::");

    // Zero-run at start → result starts with a single ":" (e.g. ":1" → "::1")
    if joined.starts_with(':') && !joined.starts_with("::") {
        return format!(":{}", joined);
    }
    // Zero-run at end → result ends with a single ":" (e.g. "1:" → "1::")
    if joined.ends_with(':') && !joined.ends_with("::") {
        return format!("{}:", joined);
    }

    joined
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::dns::RecordType;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    // -----------------------------------------------------------------------
    // encode_dns_name
    // -----------------------------------------------------------------------

    #[test]
    fn test_encode_basic() {
        let mut buf = Vec::new();
        encode_dns_name("example.com", &mut buf).unwrap();
        let expected: Vec<u8> = vec![
            7, b'e', b'x', b'a', b'm', b'p', b'l', b'e', 3, b'c', b'o', b'm', 0,
        ];
        assert_eq!(buf, expected);
    }

    #[test]
    fn test_encode_trailing_dot_is_handled() {
        // "example.com." should produce the same wire format as "example.com"
        let mut buf1 = Vec::new();
        let mut buf2 = Vec::new();
        encode_dns_name("example.com", &mut buf1).unwrap();
        encode_dns_name("example.com.", &mut buf2).unwrap();
        assert_eq!(buf1, buf2, "trailing dot should not add an extra root label");
    }

    #[test]
    fn test_encode_empty_domain_rejected() {
        let mut buf = Vec::new();
        let result = encode_dns_name("", &mut buf);
        assert!(result.is_err(), "empty domain should be rejected");
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn test_encode_label_too_long_rejected() {
        let mut buf = Vec::new();
        let label = "a".repeat(64);
        let domain = format!("{}.com", label);
        let result = encode_dns_name(&domain, &mut buf);
        assert!(result.is_err(), "label over 63 chars should be rejected");
        assert!(result.unwrap_err().contains("63"));
    }

    #[test]
    fn test_encode_label_63_ok() {
        let mut buf = Vec::new();
        let label = "a".repeat(63);
        let domain = format!("{}.com", label);
        let result = encode_dns_name(&domain, &mut buf);
        assert!(result.is_ok(), "label of exactly 63 chars should be accepted");
    }

    #[test]
    fn test_encode_consecutive_dots_skipped() {
        // "example..com" has an empty label that should be skipped gracefully
        let mut buf1 = Vec::new();
        let mut buf2 = Vec::new();
        encode_dns_name("example..com", &mut buf1).unwrap();
        encode_dns_name("example.com", &mut buf2).unwrap();
        assert_eq!(buf1, buf2, "consecutive dots should be handled gracefully");
    }

    // -----------------------------------------------------------------------
    // qtype_to_string
    // -----------------------------------------------------------------------

    #[test]
    fn test_qtype_to_string_known() {
        assert_eq!(qtype_to_string(TYPE_A), "A");
        assert_eq!(qtype_to_string(TYPE_AAAA), "AAAA");
        assert_eq!(qtype_to_string(TYPE_CNAME), "CNAME");
        assert_eq!(qtype_to_string(TYPE_MX), "MX");
        assert_eq!(qtype_to_string(TYPE_NS), "NS");
        assert_eq!(qtype_to_string(TYPE_SOA), "SOA");
        assert_eq!(qtype_to_string(TYPE_TXT), "TXT");
    }

    #[test]
    fn test_qtype_to_string_unknown() {
        assert_eq!(qtype_to_string(99), "TYPE99");
        assert_eq!(qtype_to_string(0), "TYPE0");
        assert_eq!(qtype_to_string(65535), "TYPE65535");
    }

    // -----------------------------------------------------------------------
    // get_qtypes
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_qtypes_single() {
        assert_eq!(get_qtypes(&RecordType::A), vec![TYPE_A]);
        assert_eq!(get_qtypes(&RecordType::Aaaa), vec![TYPE_AAAA]);
        assert_eq!(get_qtypes(&RecordType::Cname), vec![TYPE_CNAME]);
        assert_eq!(get_qtypes(&RecordType::Mx), vec![TYPE_MX]);
        assert_eq!(get_qtypes(&RecordType::Ns), vec![TYPE_NS]);
        assert_eq!(get_qtypes(&RecordType::Soa), vec![TYPE_SOA]);
        assert_eq!(get_qtypes(&RecordType::Txt), vec![TYPE_TXT]);
    }

    #[test]
    fn test_get_qtypes_all() {
        let expected = vec![TYPE_A, TYPE_AAAA, TYPE_CNAME, TYPE_MX, TYPE_NS, TYPE_TXT];
        assert_eq!(get_qtypes(&RecordType::All), expected);
    }

    // -----------------------------------------------------------------------
    // compress_ipv6
    // -----------------------------------------------------------------------

    #[test]
    fn test_compress_ipv6_fe80() {
        // fe80:0:0:0:0:0:0:1  →  fe80::1
        let result = compress_ipv6("fe80:0000:0000:0000:0000:0000:0000:0001");
        assert_eq!(
            result, "fe80::1",
            "BUG: fe80 corrupted to '{}' (trim_end_matches bug)", result
        );
    }

    #[test]
    fn test_compress_ipv6_ff00_segment() {
        // 2001:db8:0:0:0:ff00:42:8329  →  2001:db8::ff00:42:8329  (NOT ff:42:8329)
        let result = compress_ipv6("2001:0db8:0000:0000:0000:ff00:0042:8329");
        assert!(
            result.contains("ff00"),
            "BUG: 'ff00' segment was corrupted to '{}' (should preserve trailing zeros)",
            result
        );
        assert_eq!(result, "2001:db8::ff00:42:8329");
    }

    #[test]
    fn test_compress_ipv6_loopback() {
        assert_eq!(compress_ipv6("0000:0000:0000:0000:0000:0000:0000:0001"), "::1");
    }

    #[test]
    fn test_compress_ipv6_all_zeros() {
        assert_eq!(compress_ipv6("0000:0000:0000:0000:0000:0000:0000:0000"), "::");
    }

    #[test]
    fn test_compress_ipv6_full() {
        assert_eq!(
            compress_ipv6("2001:0db8:0000:0000:0000:0000:0000:0001"),
            "2001:db8::1"
        );
    }

    #[test]
    fn test_compress_ipv6_no_compress_needed() {
        assert_eq!(
            compress_ipv6("2001:0db8:0001:0002:0003:0004:0005:0006"),
            "2001:db8:1:2:3:4:5:6"
        );
    }

    #[test]
    fn test_compress_ipv6_short_zero_run() {
        // Single zero should not trigger "::" compression
        assert_eq!(
            compress_ipv6("2001:0db8:0000:0001:0002:0003:0004:0005"),
            "2001:db8:0:1:2:3:4:5"
        );
    }

    #[test]
    fn test_compress_ipv6_multiple_zero_runs() {
        // Should pick the longest run
        // 2001:0:0:1:0:0:0:1  →  2001:0:0:1::1
        let result = compress_ipv6("2001:0000:0000:0001:0000:0000:0000:0001");
        assert_eq!(result, "2001:0:0:1::1");
    }

    // -----------------------------------------------------------------------
    // skip_dns_name
    // -----------------------------------------------------------------------

    #[test]
    fn test_skip_dns_name_simple() {
        let data = [
            7, b'e', b'x', b'a', b'm', b'p', b'l', b'e', 3, b'c', b'o', b'm', 0,
        ];
        let result = skip_dns_name(&data, 0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 13);
    }

    #[test]
    fn test_skip_dns_name_compressed() {
        // Two bytes: compression pointer to somewhere
        let data = [0xc0, 0x0c];
        let result = skip_dns_name(&data, 0);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);
    }

    #[test]
    fn test_skip_dns_name_truncated() {
        let data = [7, b'e', b'x']; // truncated label
        let result = skip_dns_name(&data, 0);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // decode_dns_name
    // -----------------------------------------------------------------------

    #[test]
    fn test_decode_dns_name_simple() {
        let data = [
            7, b'e', b'x', b'a', b'm', b'p', b'l', b'e', 3, b'c', b'o', b'm', 0,
        ];
        let result = decode_dns_name(&data, 0);
        assert!(result.is_some());
        let (name, offset) = result.unwrap();
        assert_eq!(name, "example.com");
        assert_eq!(offset, 13);
    }

    #[test]
    fn test_decode_dns_name_compressed() {
        // Build message: question "example.com" + compressed pointer to it
        let mut msg = vec![
            0, 0, // dummy header bytes (not parsed by decode_dns_name)
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            7, b'e', b'x', b'a', b'm', b'p', b'l', b'e', 3, b'c', b'o', b'm', 0, // name at offset 12
        ];
        // Compressed pointer at offset 25, pointing to offset 12
        msg.push(0xc0);
        msg.push(0x0c);

        let result = decode_dns_name(&msg, 25);
        assert!(result.is_some(), "compressed name should decode");
        let (name, offset) = result.unwrap();
        assert_eq!(name, "example.com");
        assert_eq!(offset, 27);
    }

    #[test]
    fn test_decode_dns_name_truncated() {
        let data = [7, b'e', b'x']; // incomplete
        let result = decode_dns_name(&data, 0);
        assert!(result.is_none());
    }

    // -----------------------------------------------------------------------
    // parse_dns_response — A record
    // -----------------------------------------------------------------------

    /// Build a minimal DNS response header with the given answer count.
    fn build_header(ancount: u16) -> Vec<u8> {
        let mut h = Vec::with_capacity(12);
        h.extend_from_slice(&[0x00, 0x01]); // ID
        h.extend_from_slice(&[0x81, 0x80]); // Flags (QR=1, RA=1, RD=1, RCODE=0)
        h.extend_from_slice(&[0x00, 0x01]); // QDCOUNT = 1
        h.extend_from_slice(&ancount.to_be_bytes()); // ANCOUNT
        h.extend_from_slice(&[0x00, 0x00]); // NSCOUNT
        h.extend_from_slice(&[0x00, 0x00]); // ARCOUNT
        h
    }

    fn question_bytes(name: &str, qtype: u16) -> Vec<u8> {
        let mut q = Vec::new();
        encode_dns_name(name, &mut q).unwrap();
        q.extend_from_slice(&qtype.to_be_bytes()); // QTYPE
        q.extend_from_slice(&[0x00, 0x01]); // QCLASS = IN
        q
    }

    #[test]
    fn test_parse_a_record() {
        let mut resp = build_header(1);
        resp.extend_from_slice(&question_bytes("example.com", TYPE_A));
        // Answer: compressed name, type A, class IN, TTL 60, rdata 4 bytes
        resp.extend_from_slice(&[0xc0, 0x0c]); // NAME pointer (offset 12 = question name)
        resp.extend_from_slice(&[0x00, 0x01]); // TYPE A
        resp.extend_from_slice(&[0x00, 0x01]); // CLASS IN
        resp.extend_from_slice(&[0x00, 0x00, 0x00, 0x3c]); // TTL 60
        resp.extend_from_slice(&[0x00, 0x04]); // RDLENGTH = 4
        resp.extend_from_slice(&[192, 168, 1, 1]); // 192.168.1.1

        let records = parse_dns_response(&resp, "example.com", TYPE_A).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].name, "example.com");
        assert_eq!(records[0].record_type, "A");
        assert_eq!(records[0].value, "192.168.1.1");
        assert_eq!(records[0].ttl, 60);
    }

    // -----------------------------------------------------------------------
    // parse_dns_response — AAAA record
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_aaaa_record() {
        let mut resp = build_header(1);
        resp.extend_from_slice(&question_bytes("example.com", TYPE_AAAA));
        // Answer
        resp.extend_from_slice(&[0xc0, 0x0c]);
        resp.extend_from_slice(&[0x00, 0x1c]); // TYPE AAAA
        resp.extend_from_slice(&[0x00, 0x01]); // CLASS IN
        resp.extend_from_slice(&[0x00, 0x00, 0x00, 0x3c]); // TTL 60
        resp.extend_from_slice(&[0x00, 0x10]); // RDLENGTH = 16
        // fe80::1 in wire format
        resp.extend_from_slice(&[
            0xfe, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
        ]);

        let records = parse_dns_response(&resp, "example.com", TYPE_AAAA).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].record_type, "AAAA");
        assert_eq!(
            records[0].value, "fe80::1",
            "BUG: expected 'fe80::1' but got '{}' (ipv6 compress bug with trailing zeros)",
            records[0].value
        );
    }

    // -----------------------------------------------------------------------
    // parse_dns_response — AAAA with ff00 segment
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_aaaa_ff00_segment() {
        let mut resp = build_header(1);
        resp.extend_from_slice(&question_bytes("example.com", TYPE_AAAA));
        resp.extend_from_slice(&[0xc0, 0x0c]);
        resp.extend_from_slice(&[0x00, 0x1c]); // TYPE AAAA
        resp.extend_from_slice(&[0x00, 0x01]);
        resp.extend_from_slice(&[0x00, 0x00, 0x00, 0x3c]); // TTL 60
        resp.extend_from_slice(&[0x00, 0x10]); // RDLENGTH = 16
        // 2001:db8::ff00:42:8329
        resp.extend_from_slice(&[
            0x20, 0x01, 0x0d, 0xb8, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0xff, 0x00, 0x00, 0x42, 0x83, 0x29,
        ]);

        let records = parse_dns_response(&resp, "example.com", TYPE_AAAA).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].record_type, "AAAA");
        assert!(
            records[0].value.contains("ff00"),
            "BUG: ff00 segment corrupted: value = '{}'",
            records[0].value
        );
        assert_eq!(records[0].value, "2001:db8::ff00:42:8329");
    }

    // -----------------------------------------------------------------------
    // parse_dns_response — SOA record (verifies minimum field)
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_soa_record_contains_minimum() {
        let mut resp = build_header(1);
        resp.extend_from_slice(&question_bytes("example.com", TYPE_SOA));

        // Answer: SOA record
        resp.extend_from_slice(&[0xc0, 0x0c]); // NAME pointer
        resp.extend_from_slice(&[0x00, 0x06]); // TYPE SOA
        resp.extend_from_slice(&[0x00, 0x01]); // CLASS IN
        resp.extend_from_slice(&[0x00, 0x00, 0x00, 0x3c]); // TTL 60

        // SOA RDATA
        // MNAME: "ns1.example.com"  →  3 ns1 7 example 3 com 0  =  17 bytes
        // RNAME: "host.example.com" →  4 host 7 example 3 com 0  =  18 bytes
        let mname: &[u8] = &[3, b'n', b's', b'1', 7, b'e', b'x', b'a', b'm', b'p', b'l', b'e', 3, b'c', b'o', b'm', 0];
        let rname: &[u8] = &[4, b'h', b'o', b's', b't', 7, b'e', b'x', b'a', b'm', b'p', b'l', b'e', 3, b'c', b'o', b'm', 0];
        let rdata_len: u16 = (mname.len() + rname.len() + 20) as u16; // + 5 × 4-byte integers

        resp.extend_from_slice(&rdata_len.to_be_bytes());
        resp.extend_from_slice(mname);
        resp.extend_from_slice(rname);
        // serial = 20240101
        resp.extend_from_slice(&20240101u32.to_be_bytes());
        // refresh = 3600
        resp.extend_from_slice(&3600u32.to_be_bytes());
        // retry = 600
        resp.extend_from_slice(&600u32.to_be_bytes());
        // expire = 86400
        resp.extend_from_slice(&86400u32.to_be_bytes());
        // minimum = 300  ←  THIS WAS PREVIOUSLY DROPPED
        resp.extend_from_slice(&300u32.to_be_bytes());

        let records = parse_dns_response(&resp, "example.com", TYPE_SOA).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].record_type, "SOA");

        let value = &records[0].value;
        assert!(value.contains("ns1.example.com"), "SOA missing mname: {}", value);
        assert!(value.contains("host.example.com"), "SOA missing rname: {}", value);
        assert!(value.contains("20240101"), "SOA missing serial: {}", value);
        assert!(value.contains("3600"), "SOA missing refresh: {}", value);
        assert!(value.contains("600"), "SOA missing retry: {}", value);
        assert!(value.contains("86400"), "SOA missing expire: {}", value);
        assert!(
            value.contains("300"),
            "BUG: SOA value '{}' is missing the 'minimum' TTL field (300)",
            value
        );
    }

    // -----------------------------------------------------------------------
    // parse_dns_response — CNAME record
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_cname_record() {
        let mut resp = build_header(1);
        resp.extend_from_slice(&question_bytes("www.example.com", TYPE_CNAME));

        // Answer: CNAME record
        resp.extend_from_slice(&[0xc0, 0x0c]); // NAME pointer
        resp.extend_from_slice(&[0x00, 0x05]); // TYPE CNAME
        resp.extend_from_slice(&[0x00, 0x01]); // CLASS IN
        resp.extend_from_slice(&[0x00, 0x00, 0x00, 0x3c]); // TTL 60

        // RDATA: "target.example.com" (compressed — pointer to "example.com" at offset 16)
        let target_name: &[u8] = &[6, b't', b'a', b'r', b'g', b'e', b't', 0xc0, 0x10];
        let rdata_len: u16 = target_name.len() as u16;
        resp.extend_from_slice(&rdata_len.to_be_bytes());
        resp.extend_from_slice(target_name);

        let records = parse_dns_response(&resp, "www.example.com", TYPE_CNAME).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].record_type, "CNAME");
        assert_eq!(records[0].value, "target.example.com");
    }

    // -----------------------------------------------------------------------
    // parse_dns_response — MX record
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_mx_record() {
        let mut resp = build_header(1);
        resp.extend_from_slice(&question_bytes("example.com", TYPE_MX));

        resp.extend_from_slice(&[0xc0, 0x0c]);
        resp.extend_from_slice(&[0x00, 0x0f]); // TYPE MX
        resp.extend_from_slice(&[0x00, 0x01]);
        resp.extend_from_slice(&[0x00, 0x00, 0x00, 0x3c]); // TTL 60

        // RDATA: preference 10, exchange "mail.example.com"
        let exchange: &[u8] = &[4, b'm', b'a', b'i', b'l', 0xc0, 0x0c];
        let rdata_len: u16 = (2 + exchange.len()) as u16;
        resp.extend_from_slice(&rdata_len.to_be_bytes());
        resp.extend_from_slice(&[0x00, 0x0a]); // preference = 10
        resp.extend_from_slice(exchange);

        let records = parse_dns_response(&resp, "example.com", TYPE_MX).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].record_type, "MX");
        assert_eq!(records[0].value, "10 mail.example.com");
    }

    // -----------------------------------------------------------------------
    // parse_dns_response — NS record
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_ns_record() {
        let mut resp = build_header(1);
        resp.extend_from_slice(&question_bytes("example.com", TYPE_NS));

        resp.extend_from_slice(&[0xc0, 0x0c]);
        resp.extend_from_slice(&[0x00, 0x02]); // TYPE NS
        resp.extend_from_slice(&[0x00, 0x01]);
        resp.extend_from_slice(&[0x00, 0x00, 0x00, 0x3c]); // TTL 60

        // RDATA: "ns1.example.com"
        let ns: &[u8] = &[3, b'n', b's', b'1', 0xc0, 0x0c];
        let rdata_len: u16 = ns.len() as u16;
        resp.extend_from_slice(&rdata_len.to_be_bytes());
        resp.extend_from_slice(ns);

        let records = parse_dns_response(&resp, "example.com", TYPE_NS).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].record_type, "NS");
        assert_eq!(records[0].value, "ns1.example.com");
    }

    // -----------------------------------------------------------------------
    // parse_dns_response — TXT record
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_txt_record() {
        let mut resp = build_header(1);
        resp.extend_from_slice(&question_bytes("example.com", TYPE_TXT));

        resp.extend_from_slice(&[0xc0, 0x0c]);
        resp.extend_from_slice(&[0x00, 0x10]); // TYPE TXT
        resp.extend_from_slice(&[0x00, 0x01]);
        resp.extend_from_slice(&[0x00, 0x00, 0x00, 0x3c]); // TTL 60

        // RDATA: TXT char-string "hello=world"
        let txt_data = b"hello=world";
        let rdata: Vec<u8> = {
            let mut v = Vec::new();
            v.push(txt_data.len() as u8);
            v.extend_from_slice(txt_data);
            v
        };
        let rdata_len: u16 = rdata.len() as u16;
        resp.extend_from_slice(&rdata_len.to_be_bytes());
        resp.extend_from_slice(&rdata);

        let records = parse_dns_response(&resp, "example.com", TYPE_TXT).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].record_type, "TXT");
        assert_eq!(records[0].value, "hello=world");
    }

    // -----------------------------------------------------------------------
    // parse_dns_response — error cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_response_too_short() {
        let result = parse_dns_response(&[0u8; 6], "example.com", TYPE_A);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too short"));
    }

    #[test]
    fn test_parse_response_nxdomain() {
        let mut resp = vec![0u8; 12];
        resp[2] = 0x81; // QR=1, RD=1
        resp[3] = 0x83; // RA=1, RCODE=3 (NXDOMAIN)
        let result = parse_dns_response(&resp, "example.com", TYPE_A);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("NXDOMAIN"));
    }

    // -----------------------------------------------------------------------
    // build_dns_query — validation
    // -----------------------------------------------------------------------

    #[test]
    fn test_build_query_empty_rejected() {
        let result = build_dns_query("", TYPE_A);
        assert!(result.is_err(), "empty target should be rejected");
    }

    #[test]
    fn test_build_query_long_label_rejected() {
        let label = "a".repeat(64);
        let target = format!("{}.com", label);
        let result = build_dns_query(&target, TYPE_A);
        assert!(result.is_err(), "label over 63 chars should be rejected");
    }

    // -----------------------------------------------------------------------
    // send_query_tcp
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_send_query_tcp_invalid_address() {
        // Attempting to connect to a non-listening port should fail gracefully.
        let result = send_query_tcp(b"test", "127.0.0.1:1").await;
        assert!(result.is_err(), "Expected error connecting to port 1");
    }

    #[tokio::test]
    async fn test_send_query_tcp_mock_server() {
        // Spin up a local TCP echo-like mock that responds with a minimal
        // valid DNS response (header only, no answer records).
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (mut stream, _) = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                listener.accept(),
            )
            .await
            .expect("Mock server accept timed out")
            .expect("Mock server accept error");

            // Read 2-byte TCP length prefix
            let mut len_buf = [0u8; 2];
            stream.read_exact(&mut len_buf).await.unwrap();
            let query_len = u16::from_be_bytes(len_buf) as usize;

            // Read the DNS query body
            let mut query = vec![0u8; query_len];
            stream.read_exact(&mut query).await.unwrap();

            // Build a minimal valid DNS response (12-byte header only).
            // Copy the ID from the query; set QR=1, RD=1, RA=1, RCODE=0;
            // QDCOUNT = 1; ANCOUNT, NSCOUNT, ARCOUNT = 0.
            let mut response = vec![0u8; 12];
            response[0..2].copy_from_slice(&query[0..2]); // ID
            response[2] = 0x81; // flags: QR + RD
            response[3] = 0x80; // flags: RA
            response[5] = 0x01; // QDCOUNT low byte = 1

            // Send 2-byte length prefix + response
            let resp_len = (response.len() as u16).to_be_bytes();
            stream.write_all(&resp_len).await.unwrap();
            stream.write_all(&response).await.unwrap();
        });

        // Build a real DNS query so the server can echo back the ID
        let query = build_dns_query("example.com", TYPE_A).unwrap();

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            send_query_tcp(&query, &addr.to_string()),
        )
        .await;

        match result {
            Ok(Ok(response)) => {
                assert!(response.len() >= 12, "Response too short");
                let flags = u16::from_be_bytes([response[2], response[3]]);
                assert!(
                    flags & 0x8000 != 0,
                    "QR flag should be set in response"
                );
                assert_eq!(flags & 0x000f, 0, "RCODE should be 0");
            }
            Ok(Err(e)) => panic!("send_query_tcp failed: {}", e),
            Err(_) => panic!("send_query_tcp timed out"),
        }
    }

    // -----------------------------------------------------------------------
    // qtype_to_string round-trip via get_qtypes
    // -----------------------------------------------------------------------

    #[test]
    fn test_qtype_roundtrip() {
        // Verify that every RecordType variant maps to a string
        // that matches what parse_dns_response produces.
        let cases = vec![
            (RecordType::A, "A"),
            (RecordType::Aaaa, "AAAA"),
            (RecordType::Cname, "CNAME"),
            (RecordType::Mx, "MX"),
            (RecordType::Ns, "NS"),
            (RecordType::Soa, "SOA"),
            (RecordType::Txt, "TXT"),
        ];
        for (rt, expected_name) in cases {
            let qtypes = get_qtypes(&rt);
            for qt in qtypes {
                assert_eq!(qtype_to_string(qt), expected_name);
            }
        }
    }
}
