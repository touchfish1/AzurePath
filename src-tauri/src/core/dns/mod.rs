use crate::types::dns::{DnsRecord, RecordType};
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

/// Resolve DNS records for a target domain using a public DNS server.
pub async fn resolve(target: &str, record_type: &RecordType) -> Result<Vec<DnsRecord>, String> {
    let types = get_qtypes(record_type);
    let mut all_records = Vec::new();

    for qtype in types {
        let records = resolve_qtype(target, qtype, record_type).await?;
        all_records.extend(records);
    }

    Ok(all_records)
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
) -> Result<Vec<DnsRecord>, String> {
    // Build the DNS query
    let query = build_dns_query(target, qtype)?;

    // Send query via UDP
    let socket = UdpSocket::bind("0.0.0.0:0")
        .await
        .map_err(|e| format!("Failed to bind UDP socket: {}", e))?;

    socket
        .send_to(&query, DNS_SERVER)
        .await
        .map_err(|e| format!("Failed to send DNS query: {}", e))?;

    let mut buf = vec![0u8; 4096];
    let len = tokio::time::timeout(std::time::Duration::from_secs(5), socket.recv(&mut buf))
        .await
        .map_err(|_| "DNS query timed out".to_string())?
        .map_err(|e| format!("Failed to receive DNS response: {}", e))?;

    buf.truncate(len);

    // Parse the response
    parse_dns_response(&buf, target, qtype)
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
    encode_dns_name(target, &mut buf);

    // QTYPE
    buf.extend_from_slice(&qtype.to_be_bytes());
    // QCLASS: IN (1)
    buf.extend_from_slice(&1u16.to_be_bytes());

    Ok(buf)
}

/// Encode a domain name in DNS format (length-prefixed labels).
fn encode_dns_name(name: &str, buf: &mut Vec<u8>) {
    for label in name.split('.') {
        buf.push(label.len() as u8);
        buf.extend_from_slice(label.as_bytes());
    }
    buf.push(0); // root label
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
                let _minimum = u32::from_be_bytes(
                    [data[off2 + 16], data[off2 + 17], data[off2 + 18], data[off2 + 19]],
                );
                format!(
                    "{} {} {} {} {} {}",
                    mname, rname, serial, refresh, retry, expire
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

    let mut parts = Vec::new();
    for i in 0..best_start {
        parts.push(
            segments[i]
                .trim_start_matches('0')
                .to_string()
                .trim_end_matches('0')
                .to_string(),
        );
        if parts.last().map_or(true, |s: &String| s.is_empty()) {
            *parts.last_mut().unwrap() = "0".to_string();
        }
    }
    parts.push(String::new()); // "::"
    for i in (best_start + best_len)..8 {
        let s = segments[i]
            .trim_start_matches('0')
            .to_string()
            .trim_end_matches('0')
            .to_string();
        parts.push(if s.is_empty() { "0".to_string() } else { s });
    }

    parts.join(":").replace(":::", "::")
}
