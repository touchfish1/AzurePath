use crate::types::network_sniffer::DeviceResult;
use std::net::IpAddr;

/// Analyze TTL value to guess OS.
/// Common TTLs: Windows=128, Linux=64, macOS=64, Solaris=255, Network devices=255
#[cfg_attr(not(test), allow(dead_code))]
pub fn guess_os_by_ttl(ttl: u8) -> Option<&'static str> {
    match ttl {
        0..=64 => Some("Linux/Unix"),
        65..=128 => Some("Windows"),
        129..=255 => Some("Network Device"),
    }
}

/// Refine OS guess based on open port profile.
/// Certain port combinations strongly suggest specific OS families.
pub fn refine_os_by_ports(ports: &[u16]) -> Option<String> {
    let has_windows_ports = ports.iter().any(|p| matches!(p, 135 | 139 | 445 | 3389));
    let has_linux_ports = ports.iter().any(|p| matches!(p, 22 | 111 | 2049));
    let has_router_ports = ports.iter().any(|p| matches!(p, 53 | 67 | 68 | 1900 | 5000));

    let os = match (has_windows_ports, has_linux_ports, has_router_ports) {
        (true, false, _) => "Windows",
        (false, true, _) => "Linux/Unix",
        (_, _, true) => "Network Device",
        (true, true, _) => "Unknown (mixed ports)",
        (false, false, false) => "Unknown",
    };
    Some(os.to_string())
}

/// Build a DeviceResult from scan data.
pub fn assemble_device(
    ip: IpAddr,
    hostname: Option<String>,
    mac: Option<String>,
    mac_vendor: Option<String>,
    open_ports: Vec<crate::types::network_sniffer::PortResult>,
    scan_mode: &str,
) -> DeviceResult {
    let ports: Vec<u16> = open_ports.iter().map(|p| p.port).collect();
    let os = refine_os_by_ports(&ports);

    DeviceResult {
        ip: ip.to_string(),
        hostname: hostname.filter(|h| !h.is_empty()),
        mac,
        vendor: mac_vendor,
        os,
        open_ports,
        is_alive: true,
        scan_mode: scan_mode.to_string(),
        scan_completed: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guess_os_ttl() {
        assert_eq!(guess_os_by_ttl(64), Some("Linux/Unix"));
        assert_eq!(guess_os_by_ttl(128), Some("Windows"));
        assert_eq!(guess_os_by_ttl(255), Some("Network Device"));
    }

    #[test]
    fn test_os_by_ports_windows() {
        let os = refine_os_by_ports(&[135, 139, 445]);
        assert_eq!(os, Some("Windows".to_string()));
    }

    #[test]
    fn test_os_by_ports_linux() {
        let os = refine_os_by_ports(&[22, 111, 2049]);
        assert_eq!(os, Some("Linux/Unix".to_string()));
    }

    #[test]
    fn test_os_by_ports_router() {
        let os = refine_os_by_ports(&[53, 67, 1900]);
        assert_eq!(os, Some("Network Device".to_string()));
    }

    #[test]
    fn test_os_by_ports_mixed() {
        let os = refine_os_by_ports(&[22, 80, 443, 3389]);
        assert!(os.is_some());
    }

    #[test]
    fn test_assemble_device() {
        let ip: IpAddr = "192.168.1.1".parse().unwrap();
        let ports = vec![
            crate::types::network_sniffer::PortResult {
                port: 80, protocol: "tcp".to_string(), state: "open".to_string(),
                service: Some("HTTP".to_string()), version: None, banner: None,
                confidence: 50, probe_method: "tcp_connect".to_string(),
            },
        ];
        let device = assemble_device(
            ip,
            Some("router.local".to_string()),
            Some("00:11:22:33:44:55".to_string()),
            Some("Dell".to_string()),
            ports,
            "fast",
        );
        assert_eq!(device.ip, "192.168.1.1");
        assert_eq!(device.hostname, Some("router.local".to_string()));
        assert_eq!(device.is_alive, true);
        assert_eq!(device.scan_mode, "fast");
    }
}
