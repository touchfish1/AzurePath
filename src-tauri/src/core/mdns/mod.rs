use std::collections::HashMap;
use std::time::Duration;

use tokio::net::UdpSocket;

use crate::types::mdns::MdnsService;

const MDNS_ADDR: &str = "224.0.0.251:5353";
const COLLECT_TIMEOUT: Duration = Duration::from_secs(3);

/// Common mDNS service types that we are interested in.
const SERVICE_TYPES: &[&str] = &[
    "_http._tcp",
    "_https._tcp",
    "_smb._tcp",
    "_ssh._tcp",
    "_ftp._tcp",
    "_afpovertcp._tcp",
    "_rdp._tcp",
    "_teamviewer._tcp",
    "_vnc._tcp",
];

// --- DNS record type constants ---
const TYPE_A: u16 = 1;
const TYPE_AAAA: u16 = 28;
const TYPE_PTR: u16 = 12;
const TYPE_SRV: u16 = 33;
const TYPE_TXT: u16 = 16;

/// Discover mDNS services on the local network.
///
/// 1. Sends a PTR query for `_services._dns-sd._udp.local` to enumerate
///    all advertised service types.
/// 2. Filters the discovered types against our known list, then sends a
///    PTR query for each matched service type.
/// 3. Parses PTR / SRV / A / AAAA / TXT records from all collected responses.
pub async fn discover() -> Result<Vec<MdnsService>, String> {
    let socket = bind_mdns_socket().await?;

    // --- Step 1: Enumerate all service types ---
    let enum_query = build_mdns_ptrs_query("_services._dns-sd._udp.local")?;
    socket
        .send_to(&enum_query, MDNS_ADDR)
        .await
        .map_err(|e| format!("Failed to send mDNS enumeration query: {}", e))?;

    let enum_responses = collect_responses(&socket, Duration::from_secs(2)).await?;
    let discovered_types = extract_service_types(&enum_responses);

    // Keep only the service types that match our known list.
    let target_types: Vec<&str> = SERVICE_TYPES
        .iter()
        .filter(|st| discovered_types.iter().any(|d| d.as_str() == **st))
        .copied()
        .collect();

    // Also explicitly query the known types even if they weren't in the
    // enumeration response — some devices don't implement DNS-SD enumeration
    // but still respond to direct PTR queries.
    let mut query_types: Vec<&str> = SERVICE_TYPES.to_vec();
    for tt in &target_types {
        if !query_types.contains(tt) {
            query_types.push(tt);
        }
    }

    // --- Step 2: Query each service type ---
    let mut all_responses = Vec::new();
    for service_type in &query_types {
        let domain = format!("{}.local", service_type);
        let query = build_mdns_ptrs_query(&domain)?;
        let _ = socket.send_to(&query, MDNS_ADDR).await;
    }

    // Collect additional responses for the remaining time.
    let elapsed = Duration::from_secs(2); // We already waited 2s for enumeration.
    let remaining = COLLECT_TIMEOUT.saturating_sub(elapsed);
    let service_responses = collect_responses(&socket, remaining).await?;
    all_responses.extend(enum_responses);
    all_responses.extend(service_responses);

    // --- Step 3: Parse all responses ---
    let services = parse_all_services(&all_responses);

    Ok(services)
}

// ---------------------------------------------------------------------------
// Socket helpers
// ---------------------------------------------------------------------------

/// Bind a UDP socket suitable for mDNS queries.
async fn bind_mdns_socket() -> Result<UdpSocket, String> {
    // First, try binding to a random high port.
    let socket = UdpSocket::bind("0.0.0.0:0")
        .await
        .map_err(|e| format!("Failed to bind UDP socket: {}", e))?;

    // Set multicast TTL so packets leave the host.
    socket
        .set_multicast_ttl_v4(4)
        .map_err(|e| format!("Failed to set multicast TTL: {}", e))?;

    // Allow the socket to be reused.
    let std_socket = socket
        .into_std()
        .map_err(|e| format!("Failed to convert socket: {}", e))?;

    // Re-wrap into tokio socket.
    let socket = UdpSocket::from_std(std_socket)
        .map_err(|e| format!("Failed to re-wrap socket: {}", e))?;

    Ok(socket)
}

/// Collect all UDP responses received within the given timeout.
async fn collect_responses(socket: &UdpSocket, timeout: Duration) -> Result<Vec<Vec<u8>>, String> {
    let mut responses: Vec<Vec<u8>> = Vec::new();
    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }

        let mut buf = vec![0u8; 2048];
        let result = tokio::time::timeout(remaining, socket.recv(&mut buf)).await;

        match result {
            Ok(Ok(len)) => {
                buf.truncate(len);
                responses.push(buf);
            }
            Ok(Err(_)) => break,
            Err(_) => break, // timeout elapsed
        }
    }

    Ok(responses)
}

// ---------------------------------------------------------------------------
// DNS message building (mDNS-specific)
// ---------------------------------------------------------------------------

/// Build a DNS PTR query with the unicast-response bit set (0x8001).
fn build_mdns_ptrs_query(target: &str) -> Result<Vec<u8>, String> {
    let mut buf = Vec::with_capacity(512);

    // Header (12 bytes) — ID = 0 for mDNS
    buf.extend_from_slice(&[0x00, 0x00]); // ID = 0
    buf.extend_from_slice(&[0x00, 0x00]); // Flags = standard query
    buf.extend_from_slice(&[0x00, 0x01]); // QDCOUNT = 1
    buf.extend_from_slice(&[0x00, 0x00]); // ANCOUNT = 0
    buf.extend_from_slice(&[0x00, 0x00]); // NSCOUNT = 0
    buf.extend_from_slice(&[0x00, 0x00]); // ARCOUNT = 0

    // Question
    encode_dns_name(target, &mut buf)?;
    buf.extend_from_slice(&TYPE_PTR.to_be_bytes()); // QTYPE = PTR
    buf.extend_from_slice(&[0x80, 0x01]); // QCLASS = IN (0x0001) + unicast-response bit (0x8000) = 0x8001

    Ok(buf)
}

/// Encode a domain name into DNS wire format.
fn encode_dns_name(name: &str, buf: &mut Vec<u8>) -> Result<(), String> {
    if name.is_empty() {
        return Err("Domain name cannot be empty".to_string());
    }

    for label in name.split('.') {
        if label.is_empty() {
            continue;
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

// ---------------------------------------------------------------------------
// DNS message parsing
// ---------------------------------------------------------------------------

/// Decode a DNS name at the given offset, returning the name and new offset.
fn decode_dns_name(data: &[u8], mut offset: usize) -> Option<(String, usize)> {
    let mut labels: Vec<String> = Vec::new();
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
        let label = std::str::from_utf8(&data[offset + 1..offset + 1 + len as usize])
            .ok()?
            .to_string();
        labels.push(label);
        offset += 1 + len as usize;
        if !jumped {
            end_offset = offset;
        }
    }

    Some((labels.join("."), end_offset))
}

/// Skip over a DNS name, returning the new offset.
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
            return Ok(offset + 2);
        }
        offset += 1 + len as usize;
    }
}

// ---------------------------------------------------------------------------
// Response parsing
// ---------------------------------------------------------------------------

/// Extract service type names from PTR response packets.
///
/// Parses the answer section of each DNS response for PTR records whose
/// owner name is `_services._dns-sd._udp.local`. The RDATA of each such
/// record is a service type like `_http._tcp.local`.
fn extract_service_types(responses: &[Vec<u8>]) -> Vec<String> {
    let mut types: Vec<String> = Vec::new();

    for data in responses {
        if data.len() < 12 {
            continue;
        }

        // Parse header
        let ancount = u16::from_be_bytes([data[6], data[7]]);
        // Skip question section
        let mut offset = match skip_question_section(data, 12) {
            Ok(o) => o,
            Err(_) => continue,
        };

        for _ in 0..ancount {
            if offset >= data.len() {
                break;
            }
            let (_owner_name, new_offset) = match decode_dns_name(data, offset) {
                Some(n) => n,
                None => break,
            };
            offset = new_offset;

            if offset + 10 > data.len() {
                break;
            }

            let rtype = u16::from_be_bytes([data[offset], data[offset + 1]]);
            offset += 2;
            let _rclass = u16::from_be_bytes([data[offset], data[offset + 1]]);
            offset += 2;
            let _ttl = u32::from_be_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            offset += 4;
            let rdlength = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
            offset += 2;

            if offset + rdlength > data.len() {
                break;
            }

            // For PTR records in the services enumeration response, the
            // owner name should be "_services._dns-sd._udp.local" and
            // the RDATA is a service type domain name.
            if rtype == TYPE_PTR {
                // The owner should be _services._dns-sd._udp.local
                // but we accept any PTR record here.
                if let Some((service_name, _)) = decode_dns_name(data, offset) {
                    // service_name is like "_http._tcp.local"
                    // Extract just the service type part: "_http._tcp"
                    let parts: Vec<&str> = service_name.splitn(3, '.').collect();
                    if parts.len() >= 2 {
                        let st = format!("{}.{}", parts[0], parts[1]);
                        if st.starts_with('_') && !types.contains(&st) {
                            types.push(st);
                        }
                    }
                }
            }

            offset += rdlength;
        }
    }

    types
}

/// Skip the question section of a DNS message.
fn skip_question_section(data: &[u8], mut offset: usize) -> Result<usize, String> {
    let qdcount = u16::from_be_bytes([data[4], data[5]]);
    for _ in 0..qdcount {
        offset = skip_dns_name(data, offset)?;
        offset += 4; // QTYPE (2) + QCLASS (2)
    }
    Ok(offset)
}

/// Parse all responses and build MdnsService entries from PTR, SRV, A/AAAA, TXT records.
fn parse_all_services(responses: &[Vec<u8>]) -> Vec<MdnsService> {
    // Collections for each record type
    let mut ptr_entries: Vec<(String, String)> = Vec::new(); // (service_domain, instance_domain)
    let mut srv_entries: Vec<(String, String, u16)> = Vec::new(); // (instance_domain, hostname, port)
    let mut a_entries: Vec<(String, String)> = Vec::new(); // (hostname, ip)
    let mut txt_entries: Vec<(String, HashMap<String, String>)> = Vec::new(); // (instance_domain, kv_map)

    for data in responses {
        if data.len() < 12 {
            continue;
        }

        let ancount = u16::from_be_bytes([data[6], data[7]]);
        let nscount = u16::from_be_bytes([data[8], data[9]]);
        let arcount = u16::from_be_bytes([data[10], data[11]]);

        let offset = match skip_question_section(data, 12) {
            Ok(o) => o,
            Err(_) => continue,
        };

        // Parse all three sections
        let offset = parse_section(data, offset, ancount, &mut ptr_entries, &mut srv_entries, &mut a_entries, &mut txt_entries);
        let offset = parse_section(data, offset, nscount, &mut ptr_entries, &mut srv_entries, &mut a_entries, &mut txt_entries);
        let _ = parse_section(data, offset, arcount, &mut ptr_entries, &mut srv_entries, &mut a_entries, &mut txt_entries);
    }

    // Join records into services
    build_services(&ptr_entries, &srv_entries, &a_entries, &txt_entries)
}

fn parse_section(
    data: &[u8],
    mut offset: usize,
    count: u16,
    ptr_entries: &mut Vec<(String, String)>,
    srv_entries: &mut Vec<(String, String, u16)>,
    a_entries: &mut Vec<(String, String)>,
    txt_entries: &mut Vec<(String, HashMap<String, String>)>,
) -> usize {
    for _ in 0..count {
        if offset >= data.len() {
            break;
        }

        let (name, new_offset) = match decode_dns_name(data, offset) {
            Some(n) => n,
            None => break,
        };
        offset = new_offset;

        if offset + 10 > data.len() {
            break;
        }

        let rtype = u16::from_be_bytes([data[offset], data[offset + 1]]);
        offset += 2;
        let _rclass = u16::from_be_bytes([data[offset], data[offset + 1]]);
        offset += 2;
        let _ttl = u32::from_be_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]);
        offset += 4;
        let rdlength = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
        offset += 2;

        if offset + rdlength > data.len() {
            break;
        }

        match rtype {
            TYPE_PTR => {
                if let Some((instance, _)) = decode_dns_name(data, offset) {
                    ptr_entries.push((name, instance));
                }
            }
            TYPE_SRV => {
                if rdlength >= 7 {
                    let _priority = u16::from_be_bytes([data[offset], data[offset + 1]]);
                    let _weight = u16::from_be_bytes([data[offset + 2], data[offset + 3]]);
                    let port = u16::from_be_bytes([data[offset + 4], data[offset + 5]]);
                    if let Some((target, _)) = decode_dns_name(data, offset + 6) {
                        srv_entries.push((name, target, port));
                    }
                }
            }
            TYPE_A => {
                if rdlength == 4 {
                    let ip = format!(
                        "{}.{}.{}.{}",
                        data[offset],
                        data[offset + 1],
                        data[offset + 2],
                        data[offset + 3]
                    );
                    a_entries.push((name, ip));
                }
            }
            TYPE_AAAA => {
                if rdlength == 16 {
                    let segments: Vec<String> = data[offset..offset + 16]
                        .chunks(2)
                        .map(|c| format!("{:02x}{:02x}", c[0], c[1]))
                        .collect();
                    let ip = segments.join(":");
                    // Collapse zeros with "::"
                    let ip = compress_ipv6(&ip);
                    a_entries.push((name, ip));
                }
            }
            TYPE_TXT => {
                let mut kv = HashMap::new();
                let mut txt_offset = offset;
                let end = offset + rdlength;
                while txt_offset < end {
                    if txt_offset >= data.len() {
                        break;
                    }
                    let txt_len = data[txt_offset] as usize;
                    txt_offset += 1;
                    if txt_offset + txt_len <= end && txt_offset + txt_len <= data.len() {
                        let txt =
                            String::from_utf8_lossy(&data[txt_offset..txt_offset + txt_len])
                                .to_string();
                        // Parse key=value
                        if let Some(eq_pos) = txt.find('=') {
                            let key = txt[..eq_pos].to_string();
                            let value = txt[eq_pos + 1..].to_string();
                            kv.insert(key, value);
                        } else {
                            kv.insert(txt, String::new());
                        }
                        txt_offset += txt_len;
                    } else {
                        break;
                    }
                }
                if !kv.is_empty() {
                    txt_entries.push((name, kv));
                }
            }
            _ => {}
        }

        offset += rdlength;
    }

    offset
}

/// Build MdnsService entries by correlating PTR, SRV, A, and TXT records.
fn build_services(
    ptr_entries: &[(String, String)],
    srv_entries: &[(String, String, u16)],
    a_entries: &[(String, String)],
    txt_entries: &[(String, HashMap<String, String>)],
) -> Vec<MdnsService> {
    let mut services: Vec<MdnsService> = Vec::new();

    // Build a map: hostname -> IP (first match wins)
    let ip_map: HashMap<&str, &str> = a_entries.iter().map(|(h, ip)| (h.as_str(), ip.as_str())).collect();

    for (service_domain, instance_domain) in ptr_entries {
        // Extract service type from the PTR owner name
        // e.g. "_http._tcp.local" or from instance "_http._tcp.local"
        let service_type = extract_service_type(service_domain);

        // Find matching SRV record
        let (hostname, port, _host_for_lookup) = srv_entries
            .iter()
            .find(|(inst, _, _)| inst == instance_domain || *inst == *instance_domain)
            .map(|(_, host, port)| (host.clone(), *port, host.clone()))
            .unwrap_or_else(|| {
                // No SRV record — use instance name as hostname and default port
                let host = instance_domain.trim_end_matches(".local").to_string();
                (host.clone(), 0, instance_domain.clone())
            });

        // Find IP for the hostname (try exact match, then hostname.local)
        let ip = ip_map
            .get(hostname.as_str())
            .or_else(|| {
                let with_local = format!("{}.local", hostname.trim_end_matches(".local"));
                ip_map.get(with_local.as_str())
            })
            .or_else(|| {
                // Try instance name as hostname
                let inst_host = format!("{}.local", extract_hostname_from_instance(instance_domain));
                ip_map.get(inst_host.as_str())
            })
            .map(|s| s.to_string())
            .unwrap_or_default();

        // Find matching TXT record
        let txt = txt_entries
            .iter()
            .find(|(inst, _)| *inst == *instance_domain)
            .map(|(_, kv)| kv.clone())
            .unwrap_or_default();

        // Extract hostname (without .local suffix)
        let friendly_hostname = hostname
            .trim_end_matches(".local")
            .to_string();

        services.push(MdnsService {
            service_type,
            hostname: friendly_hostname,
            ip,
            port,
            txt,
        });
    }

    // Deduplicate by (ip, port, hostname)
    services.dedup_by_key(|s| (s.ip.clone(), s.port, s.hostname.clone()));

    services
}

/// Extract service type from a DNS name like "_http._tcp.local" or similar.
fn extract_service_type(name: &str) -> String {
    // Try to find the service type prefix like "_http._tcp"
    let parts: Vec<&str> = name.split('.').collect();
    if parts.len() >= 2 && parts[0].starts_with('_') && parts[1].starts_with('_') {
        return format!("{}.{}", parts[0], parts[1]);
    }
    name.to_string()
}

/// Extract hostname part from an instance domain like "My Printer._http._tcp.local"
fn extract_hostname_from_instance(instance: &str) -> &str {
    instance.split('.').next().unwrap_or(instance)
}

// ---------------------------------------------------------------------------
// IPv6 compression helper
// ---------------------------------------------------------------------------

fn compress_ipv6(ip: &str) -> String {
    let segments: Vec<&str> = ip.split(':').collect();
    if segments.len() != 8 {
        return ip.to_string();
    }

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
        return segments
            .iter()
            .map(|s| s.trim_start_matches('0'))
            .map(|s| if s.is_empty() { "0" } else { s })
            .collect::<Vec<_>>()
            .join(":");
    }

    if best_start == 0 && best_len == 8 {
        return "::".to_string();
    }

    let mut parts: Vec<String> = Vec::new();

    for i in 0..best_start {
        let mut s = segments[i].trim_start_matches('0').to_string();
        if s.is_empty() {
            s = "0".to_string();
        }
        parts.push(s);
    }

    parts.push(String::new());

    for i in (best_start + best_len)..8 {
        let s = segments[i].trim_start_matches('0').to_string();
        parts.push(if s.is_empty() { "0".to_string() } else { s });
    }

    let joined = parts.join(":").replace(":::", "::");

    if joined.starts_with(':') && !joined.starts_with("::") {
        return format!(":{}", joined);
    }
    if joined.ends_with(':') && !joined.ends_with("::") {
        return format!("{}:", joined);
    }

    joined
}
