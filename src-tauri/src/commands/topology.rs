use std::collections::HashSet;
use std::net::Ipv4Addr;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use tauri::AppHandle;

use crate::core::ping;
use crate::core::utils::emit_or_warn;

// ============================================================
// Data types
// ============================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredNode {
    pub ip: String,
    pub hostname: Option<String>,
    pub latency_ms: Option<f64>,
    pub is_gateway: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredLink {
    pub source: String,
    pub target: String,
    pub hop_count: u32,
    pub latency_ms: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverProgress {
    pub phase: String,
    pub progress: f64,
    pub current_ip: String,
    pub nodes_found: u32,
    pub message: String,
}

// ============================================================
// Cancellation
// ============================================================

static CANCEL_FLAG: LazyLock<Mutex<Option<Arc<AtomicBool>>>> =
    LazyLock::new(|| Mutex::new(None));

fn set_cancel_flag(value: bool) {
    if let Ok(guard) = CANCEL_FLAG.lock() {
        if let Some(flag) = guard.as_ref() {
            flag.store(value, Ordering::Relaxed);
        }
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn is_cancelled() -> bool {
    if let Ok(guard) = CANCEL_FLAG.lock() {
        guard.as_ref().map_or(false, |f| f.load(Ordering::Relaxed))
    } else {
        false
    }
}

// ============================================================
// Subnet helpers
// ============================================================

fn parse_subnet(s: &str) -> Result<(Ipv4Addr, u32), String> {
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() != 2 {
        return Err("无效的子网格式，请使用 CIDR 格式（例如 192.168.1.0/24）".to_string());
    }
    let ip: Ipv4Addr = parts[0]
        .parse()
        .map_err(|_| "无效的 IP 地址".to_string())?;
    let prefix: u32 = parts[1]
        .parse()
        .map_err(|_| "无效的子网前缀长度".to_string())?;
    if prefix > 32 {
        return Err("子网前缀长度不能超过 32".to_string());
    }
    Ok((ip, prefix))
}

fn ip_from_base(base: Ipv4Addr, offset: u32) -> Ipv4Addr {
    let octets = base.octets();
    let raw = ((octets[0] as u32) << 24)
        | ((octets[1] as u32) << 16)
        | ((octets[2] as u32) << 8)
        | (octets[3] as u32);
    let new_raw = raw + offset;
    Ipv4Addr::new(
        (new_raw >> 24) as u8,
        (new_raw >> 16) as u8,
        (new_raw >> 8) as u8,
        new_raw as u8,
    )
}

fn network_address(base: Ipv4Addr, prefix: u32) -> Ipv4Addr {
    let octets = base.octets();
    let raw = ((octets[0] as u32) << 24)
        | ((octets[1] as u32) << 16)
        | ((octets[2] as u32) << 8)
        | (octets[3] as u32);
    let mask = if prefix == 0 {
        0u32
    } else {
        !0u32 << (32 - prefix)
    };
    let net_raw = raw & mask;
    Ipv4Addr::new(
        (net_raw >> 24) as u8,
        (net_raw >> 16) as u8,
        (net_raw >> 8) as u8,
        net_raw as u8,
    )
}

// ============================================================
// Command: discover_topology
// ============================================================

#[tauri::command]
pub async fn discover_topology(app: AppHandle, subnet: Option<String>) -> Result<(), String> {
    // Set up cancel flag
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut guard = CANCEL_FLAG.lock().map_err(|e| e.to_string())?;
        *guard = Some(cancel.clone());
    }

    let app_clone = app.clone();

    tauri::async_runtime::spawn(async move {
        let result = run_discovery(&app_clone, subnet, cancel).await;
        if let Err(e) = result {
            emit_or_warn(&app_clone, "topology:error", &serde_json::json!({ "error": e }));
        }

        // Clean up cancel flag
        let _ = CANCEL_FLAG.lock().map(|mut g| *g = None);
    });

    Ok(())
}

async fn run_discovery(
    app: &AppHandle,
    subnet: Option<String>,
    cancel: Arc<AtomicBool>,
) -> Result<(), String> {
    // Determine subnet to scan
    let subnet_str = subnet.unwrap_or_else(|| "192.168.1.0/24".to_string());
    let (base_ip, prefix_len) = parse_subnet(&subnet_str)?;
    let net_addr = network_address(base_ip, prefix_len);
    let net_octets = net_addr.octets();
    let broadcast_addr = ip_from_base(net_addr, (1u32 << (32 - prefix_len)).saturating_sub(1));
    let host_count = (1u32 << (32 - prefix_len).max(0).min(32)).saturating_sub(2); // Exclude network and broadcast
    let total = host_count.min(254); // Safety cap

    // ============================================================
    // Phase 1: Concurrent ping sweep (Semaphore = 50)
    // ============================================================
    let semaphore = Arc::new(tokio::sync::Semaphore::new(50));
    let completed = Arc::new(AtomicU32::new(0));
    let mut handles = Vec::new();

    for i in 0..total {
        if cancel.load(Ordering::Relaxed) {
            return Err("发现已取消".to_string());
        }

        let ip = ip_from_base(net_addr, i + 1); // Start from .1
        let octets = ip.octets();

        // Skip network and broadcast addresses
        if ip == net_addr || ip == broadcast_addr {
            continue;
        }
        // Skip .0 and .255
        if octets[3] == 0 || octets[3] == 255 {
            continue;
        }

        let ip_str = ip.to_string();
        let sem = semaphore.clone();
        let cancel = cancel.clone();
        let app_clone = app.clone();
        let completed = completed.clone();

        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.expect("Semaphore closed");

            if cancel.load(Ordering::Relaxed) {
                return None;
            }

            let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
            emit_or_warn(&app_clone, "topology:progress", &DiscoverProgress {
                progress: (done as f64 / total as f64) * 50.0,
                phase: "scan".to_string(),
                current_ip: ip_str.clone(),
                nodes_found: 0,
                message: format!("扫描 {} ({}/{})", ip_str, done, total),
            });

            // Quick ping with 1 packet, 2s timeout
            if let Ok(output) = ping::execute_ping(&ip_str, 1, 2000).await {
                let results = ping::parse_ping_output(&output);
                if results.iter().any(|r| r.status == "success") {
                    return Some(ip_str);
                }
            }
            None
        }));
    }

    let mut alive_hosts: HashSet<String> = HashSet::new();
    for handle in handles {
        if let Some(ip) = handle.await.unwrap_or(None) {
            alive_hosts.insert(ip);
        }
    }

    // ============================================================
    // Phase 2: Collect node info (gateway detection + latency)
    // ============================================================
    let mut nodes: Vec<DiscoveredNode> = Vec::new();
    let mut links: Vec<DiscoveredLink> = Vec::new();
    let mut link_set: HashSet<(String, String)> = HashSet::new();

    // Identify gateway candidates
    let gw_ip = if prefix_len >= 24 {
        format!("{}.{}.{}.1", net_octets[0], net_octets[1], net_octets[2])
    } else {
        format!("{}.1", base_ip)
    };

    // Check if gateway is alive
    let mut gateway_found = false;
    if alive_hosts.contains(&gw_ip) {
        if let Ok(output) = ping::execute_ping(&gw_ip, 3, 3000).await {
            let results = ping::parse_ping_output(&output);
            let stats = ping::compute_stats(&results);
            nodes.push(DiscoveredNode {
                ip: gw_ip.clone(),
                hostname: None,
                latency_ms: Some(stats.avg_ms),
                is_gateway: true,
            });
            gateway_found = true;
        }
    }

    // Also try .254 as gateway
    let gw_ip_254 = if prefix_len >= 24 {
        format!(
            "{}.{}.{}.254",
            net_octets[0], net_octets[1], net_octets[2]
        )
    } else {
        format!("{}.254", base_ip)
    };

    if !gateway_found && alive_hosts.contains(&gw_ip_254) {
        if let Ok(output) = ping::execute_ping(&gw_ip_254, 3, 3000).await {
            let results = ping::parse_ping_output(&output);
            let stats = ping::compute_stats(&results);
            nodes.push(DiscoveredNode {
                ip: gw_ip_254.clone(),
                hostname: None,
                latency_ms: Some(stats.avg_ms),
                is_gateway: true,
            });
        }
    }

    // Process remaining hosts
    let known_ips: HashSet<String> = nodes.iter().map(|n| n.ip.clone()).collect();

    let total_trace = alive_hosts.len().max(1);
    let inner_semaphore = Arc::new(tokio::sync::Semaphore::new(50));

    for (idx, host) in alive_hosts.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err("发现已取消".to_string());
        }

        emit_or_warn(app, "topology:progress", &DiscoverProgress {
            progress: 50.0 + (idx as f64 / total_trace as f64) * 45.0,
            phase: "trace".to_string(),
            current_ip: host.clone(),
            nodes_found: alive_hosts.len() as u32,
            message: format!("测量 {} ({}/{})", host, idx + 1, alive_hosts.len()),
        });

        // Measure latency to this host (3 pings for accuracy)
        let latency = match ping::execute_ping(host, 3, 3000).await {
            Ok(output) => {
                let results = ping::parse_ping_output(&output);
                let stats = ping::compute_stats(&results);
                if stats.received > 0 {
                    Some(stats.avg_ms)
                } else {
                    None
                }
            }
            Err(_) => None,
        };

        if !known_ips.contains(host) {
            nodes.push(DiscoveredNode {
                ip: host.clone(),
                hostname: None,
                latency_ms: latency,
                is_gateway: false,
            });
        }

        // Measure pairwise latency to already-processed nodes (concurrent)
        let current_nodes: Vec<DiscoveredNode> = nodes.clone();
        let mut ping_handles = Vec::new();

        for other_node in &current_nodes {
            if other_node.ip == *host {
                continue;
            }

            // O(1) lookup via HashSet instead of O(n) Vec scan
            let key1 = (host.clone(), other_node.ip.clone());
            let key2 = (other_node.ip.clone(), host.clone());
            if link_set.contains(&key1) || link_set.contains(&key2) {
                continue;
            }

            let sem = inner_semaphore.clone();
            let cancel = cancel.clone();
            let target_ip = other_node.ip.clone();
            let source_ip = host.clone();

            ping_handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.expect("Semaphore closed");

                if cancel.load(Ordering::Relaxed) {
                    return None;
                }

                // Quick 1-ping to check connectivity
                if let Ok(output) = ping::execute_ping(&target_ip, 1, 1000).await {
                    let results = ping::parse_ping_output(&output);
                    if let Some(r) = results.first() {
                        if r.status == "success" {
                            return Some(DiscoveredLink {
                                source: source_ip,
                                target: target_ip,
                                hop_count: 1,
                                latency_ms: Some(r.latency_ms),
                            });
                        }
                    }
                }
                None
            }));
        }

        // Collect results from concurrent pairwise pings
        for ping_handle in ping_handles {
            if let Some(link) = ping_handle.await.unwrap_or(None) {
                link_set.insert((link.source.clone(), link.target.clone()));
                links.push(link);
            }
        }
    }

    // ============================================================
    // Phase 3: Emit final results
    // ============================================================
    emit_or_warn(app, "topology:progress", &DiscoverProgress {
        progress: 100.0,
        phase: "complete".to_string(),
        current_ip: String::new(),
        nodes_found: nodes.len() as u32,
        message: format!("发现 {} 个节点, {} 条连接", nodes.len(), links.len()),
    });

    emit_or_warn(app, "topology:result", &serde_json::json!({
        "nodes": nodes,
        "links": links,
    }));

    Ok(())
}

// ============================================================
// Command: cancel_topology_discovery
// ============================================================

#[tauri::command]
pub fn cancel_topology_discovery() -> Result<(), String> {
    set_cancel_flag(true);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_subnet_valid() {
        let (ip, prefix) = parse_subnet("192.168.1.0/24").unwrap();
        assert_eq!(ip, Ipv4Addr::new(192, 168, 1, 0));
        assert_eq!(prefix, 24);
    }

    #[test]
    fn test_parse_subnet_no_slash() {
        assert!(parse_subnet("192.168.1.0").is_err());
    }

    #[test]
    fn test_parse_subnet_invalid_ip() {
        assert!(parse_subnet("not-an-ip/24").is_err());
    }

    #[test]
    fn test_parse_subnet_invalid_prefix() {
        assert!(parse_subnet("192.168.1.0/abc").is_err());
    }

    #[test]
    fn test_parse_subnet_prefix_too_large() {
        assert!(parse_subnet("192.168.1.0/33").is_err());
    }

    #[test]
    fn test_ip_from_base() {
        let base = Ipv4Addr::new(192, 168, 1, 0);
        assert_eq!(ip_from_base(base, 1), Ipv4Addr::new(192, 168, 1, 1));
        assert_eq!(ip_from_base(base, 255), Ipv4Addr::new(192, 168, 1, 255));
    }

    #[test]
    fn test_ip_from_base_cross_octet() {
        let base = Ipv4Addr::new(192, 168, 1, 0);
        assert_eq!(ip_from_base(base, 256), Ipv4Addr::new(192, 168, 2, 0));
    }

    #[test]
    fn test_network_address() {
        let ip = Ipv4Addr::new(192, 168, 1, 100);
        assert_eq!(network_address(ip, 24), Ipv4Addr::new(192, 168, 1, 0));
        assert_eq!(network_address(ip, 16), Ipv4Addr::new(192, 168, 0, 0));
    }

    #[test]
    fn test_network_address_exact() {
        let ip = Ipv4Addr::new(10, 20, 30, 40);
        assert_eq!(network_address(ip, 24), Ipv4Addr::new(10, 20, 30, 0));
    }

    #[test]
    fn test_cancel_flag_lifecycle() {
        let flag = Arc::new(AtomicBool::new(false));
        {
            let mut guard = CANCEL_FLAG.lock().unwrap();
            *guard = Some(flag.clone());
        }

        assert!(!is_cancelled());

        set_cancel_flag(true);
        assert!(is_cancelled());

        set_cancel_flag(false);
        assert!(!is_cancelled());

        let _ = CANCEL_FLAG.lock().map(|mut g| *g = None);
        assert!(!is_cancelled());
    }

    #[test]
    fn test_discovered_node_serialization() {
        let node = DiscoveredNode {
            ip: "192.168.1.1".to_string(),
            hostname: Some("router".to_string()),
            latency_ms: Some(5.2),
            is_gateway: true,
        };
        let json = serde_json::to_value(&node).unwrap();
        assert_eq!(json["ip"], "192.168.1.1");
        assert_eq!(json["hostname"], "router");
        assert_eq!(json["latencyMs"], 5.2);
        assert!(json["isGateway"].as_bool().unwrap());
    }

    #[test]
    fn test_discovered_link_serialization() {
        let link = DiscoveredLink {
            source: "192.168.1.1".to_string(),
            target: "192.168.1.100".to_string(),
            hop_count: 1,
            latency_ms: Some(2.3),
        };
        let json = serde_json::to_value(&link).unwrap();
        assert_eq!(json["source"], "192.168.1.1");
        assert_eq!(json["target"], "192.168.1.100");
        assert_eq!(json["hopCount"], 1);
        assert_eq!(json["latencyMs"], 2.3);
    }

    #[test]
    fn test_discover_progress_serialization() {
        let progress = DiscoverProgress {
            phase: "scan".to_string(),
            progress: 42.5,
            current_ip: "192.168.1.1".to_string(),
            nodes_found: 5,
            message: "Scanning...".to_string(),
        };
        let json = serde_json::to_value(&progress).unwrap();
        assert_eq!(json["phase"], "scan");
        assert!((json["progress"].as_f64().unwrap() - 42.5).abs() < 0.001);
        assert_eq!(json["currentIp"], "192.168.1.1");
        assert_eq!(json["nodesFound"], 5);
        assert_eq!(json["message"], "Scanning...");
    }
}
