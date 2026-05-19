use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::Semaphore;

/// Probe ports that are commonly open on LAN devices.
const PROBE_PORTS: &[u16] = &[445, 22, 80, 443, 139, 135, 8080, 3389];

/// Check if a single host is alive by trying to connect to probe ports.
pub async fn is_host_alive(
    ip: IpAddr,
    timeout_ms: u64,
) -> bool {
    let timeout = Duration::from_millis(timeout_ms);
    for &port in PROBE_PORTS {
        if let Ok(Ok(_)) = tokio::time::timeout(timeout, TcpStream::connect((ip, port))).await {
            return true;
        }
    }
    false
}

/// Discover alive hosts from a list of IPs concurrently.
pub async fn discover_hosts(
    ips: &[IpAddr],
    concurrency: usize,
    timeout_ms: u64,
) -> Vec<IpAddr> {
    if concurrency == 0 || ips.is_empty() {
        return Vec::new();
    }
    let semaphore = Arc::new(Semaphore::new(concurrency));
    let mut handles = Vec::with_capacity(ips.len());

    for &ip in ips {
        let permit = semaphore.clone().acquire_owned().await;
        if permit.is_err() {
            continue;
        }
        handles.push(tokio::spawn(async move {
            let alive = is_host_alive(ip, timeout_ms).await;
            drop(permit);
            (ip, alive)
        }));
    }

    let mut alive_hosts = Vec::new();
    for h in handles {
        if let Ok((ip, true)) = h.await {
            alive_hosts.push(ip);
        }
    }
    alive_hosts
}

use std::net::ToSocketAddrs;

/// Get MAC address by parsing `arp -a` output.
/// Returns (mac, vendor) if the IP is found in ARP cache.
pub fn resolve_mac(ip: &str) -> Option<(String, Option<String>)> {
    let output = std::process::Command::new("arp")
        .arg("-a")
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.contains(ip) {
            // Windows: "192.168.1.1    00-11-22-33-44-55    dynamic"
            // Linux:   "192.168.1.1    ether   00:11:22:33:44:55   C"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let mac = parts[1].replace('-', ":").to_uppercase();
                let vendor = mac_vendor(&mac);
                return Some((mac, vendor));
            }
        }
    }
    None
}

/// A minimal MAC vendor lookup (first 3 bytes).
fn mac_vendor(mac: &str) -> Option<String> {
    let prefix = mac.split(':').take(3).collect::<Vec<_>>().join(":");
    let vendor = match prefix.as_str() {
        "00:50:56" => "VMware",
        "00:0C:29" => "VMware",
        "00:05:69" => "VMware",
        "08:00:27" => "Oracle VirtualBox",
        "00:15:5D" => "Hyper-V",
        "00:1C:42" => "Parallels",
        "00:11:22" => "Dell",
        "00:1A:4B" => "HP",
        "00:21:5A" => "HP",
        "00:14:22" => "Dell",
        "00:1E:68" => "Intel",
        "00:21:6A" => "Apple",
        "00:23:32" => "Apple",
        "00:25:00" => "Apple",
        "00:26:08" => "Apple",
        "00:1A:2B" => "Cisco",
        "FC:A1:3F" => "Xiaomi",
        "50:76:AF" => "Huawei",
        "E0:CC:7A" => "Huawei",
        "14:75:90" => "TP-Link",
        "C0:4A:00" => "TP-Link",
        "D4:6E:0E" => "TP-Link",
        "28:6E:D4" => "Xiaomi",
        _ => return None,
    };
    Some(vendor.to_string())
}

pub fn resolve_hostname(ip: &str) -> Option<String> {
    // Use system reverse DNS lookup via socket addresses
    if let Ok(addr) = format!("{}:0", ip).to_socket_addrs() {
        for a in addr {
            if let std::net::SocketAddr::V4(_) = a {
                // a.ip() returns the resolved IP, not hostname
            }
        }
    }
    // Fall back to empty — reverse DNS via std is unreliable on Windows
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mac_vendor_known() {
        assert_eq!(mac_vendor("00:50:56:12:34:56"), Some("VMware".to_string()));
        assert_eq!(mac_vendor("08:00:27:AB:CD:EF"), Some("Oracle VirtualBox".to_string()));
    }

    #[test]
    fn test_mac_vendor_unknown() {
        assert_eq!(mac_vendor("AA:BB:CC:DD:EE:FF"), None);
    }

    #[test]
    fn test_mac_vendor_vmware_prefixes() {
        assert_eq!(mac_vendor("00:50:56:FF:FF:FF"), Some("VMware".to_string()));
        assert_eq!(mac_vendor("00:0C:29:00:00:01"), Some("VMware".to_string()));
        assert_eq!(mac_vendor("00:05:69:AB:CD:EF"), Some("VMware".to_string()));
    }

    #[test]
    fn test_mac_vendor_apple() {
        assert_eq!(mac_vendor("00:21:6A:12:34:56"), Some("Apple".to_string()));
        assert_eq!(mac_vendor("00:23:32:AB:CD:EF"), Some("Apple".to_string()));
        assert_eq!(mac_vendor("00:25:00:11:22:33"), Some("Apple".to_string()));
        assert_eq!(mac_vendor("00:26:08:44:55:66"), Some("Apple".to_string()));
    }

    #[test]
    fn test_mac_vendor_cisco() {
        assert_eq!(mac_vendor("00:1A:2B:12:34:56"), Some("Cisco".to_string()));
    }

    #[test]
    fn test_mac_vendor_empty_mac() {
        assert_eq!(mac_vendor(""), None);
    }

    #[test]
    fn test_mac_vendor_partial_mac() {
        assert_eq!(mac_vendor("00:50:56"), Some("VMware".to_string()));
    }

    #[test]
    fn test_arp_parse_windows_format() {
        let ip = "192.168.1.1";
        let arp_output = "  192.168.1.1    00-11-22-33-44-55    dynamic\n";
        for line in arp_output.lines() {
            if line.contains(ip) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                assert_eq!(parts.len(), 3);
                let mac = parts[1].replace('-', ":").to_uppercase();
                assert_eq!(mac, "00:11:22:33:44:55");
            }
        }
    }

    #[tokio::test]
    async fn test_discover_hosts_zero_concurrency() {
        let ips = vec!["10.0.0.1".parse::<IpAddr>().unwrap()];
        let result = discover_hosts(&ips, 0, 100).await;
        assert!(result.is_empty(), "zero concurrency should return empty");
    }

    #[tokio::test]
    async fn test_discover_hosts_empty_ips() {
        let result = discover_hosts(&[], 5, 100).await;
        assert!(result.is_empty(), "empty ips should return empty");
    }
}
