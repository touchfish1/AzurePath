use std::net::{IpAddr, Ipv6Addr};

use crate::types::subnet::{IpClassification, SubnetResult, SubnetSplitResult};

/// Parse an IPv4 CIDR notation string into (ip_as_u32, prefix_len).
///
/// Supports "192.168.1.0/24", single IP (treated as /32), /31 (RFC 3021), /16-/30.
/// Rejects ranges larger than /16 (65534 hosts max).
pub fn parse_cidr_v4(input: &str) -> Result<(u32, u8), String> {
    let (ip_str, prefix_len) = match input.split_once('/') {
        Some((ip, len)) => {
            let len: u8 = len
                .parse()
                .map_err(|_| "Invalid CIDR prefix".to_string())?;
            if len > 32 {
                return Err(format!(
                    "Invalid CIDR prefix length {} (must be 0-32)",
                    len
                ));
            }
            (ip, len)
        }
        None => (input, 32),
    };

    let ip: IpAddr = ip_str
        .parse()
        .map_err(|e| format!("Invalid IP: {}", e))?;
    let ip_u32 = match ip {
        IpAddr::V4(v4) => u32::from(v4),
        IpAddr::V6(_) => return Err("Only IPv4 addresses are supported".to_string()),
    };

    // Reject ranges larger than /16
    if prefix_len < 16 {
        return Err("CIDR range too large (max /16)".to_string());
    }

    Ok((ip_u32, prefix_len))
}

/// Calculate full IPv4 subnet information.
pub fn calculate_v4(ip: &str, cidr: u8) -> Result<SubnetResult, String> {
    if cidr > 32 {
        return Err(format!(
            "Invalid CIDR prefix length {} (IPv4 max is 32)",
            cidr
        ));
    }

    let ip: IpAddr = ip
        .parse()
        .map_err(|e| format!("Invalid IP address: {}", e))?;
    let ip_u32 = match ip {
        IpAddr::V4(v4) => u32::from(v4),
        IpAddr::V6(_) => return Err("Only IPv4 addresses are supported".to_string()),
    };

    let host_bits = 32 - cidr;
    let mask = if cidr == 0 {
        0u32
    } else {
        (!0u32) << host_bits
    };
    let network = ip_u32 & mask;
    let broadcast = if cidr == 32 {
        ip_u32
    } else {
        network | !mask
    };

    // For /31 (RFC 3021): both addresses are usable, no network/broadcast deduction
    // For /32: single host
    let (usable_hosts, first_host, last_host) = if cidr >= 31 {
        if cidr == 32 {
            (1u64, network, network)
        } else {
            (2u64, network, broadcast)
        }
    } else {
        let hosts = (1u64 << host_bits) - 2;
        (hosts, network + 1, broadcast - 1)
    };

    let classification = classify_ipv4(ip_u32);

    Ok(SubnetResult {
        network_address: ipv4_to_string(network),
        broadcast_address: if cidr >= 31 {
            String::new()
        } else {
            ipv4_to_string(broadcast)
        },
        subnet_mask: ipv4_to_string(mask),
        wildcard_mask: ipv4_to_string(!mask),
        usable_hosts,
        ip_range: format!(
            "{} \u{2014} {}",
            ipv4_to_string(first_host),
            ipv4_to_string(last_host)
        ),
        cidr,
        ip_version: "IPv4".to_string(),
        classification,
    })
}

fn ipv4_to_string(ip: u32) -> String {
    format!(
        "{}.{}.{}.{}",
        (ip >> 24) & 0xff,
        (ip >> 16) & 0xff,
        (ip >> 8) & 0xff,
        ip & 0xff
    )
}

/// Parse an IPv6 CIDR notation string into (ip_as_u128, prefix_len).
///
/// Supports "2001:db8::/32" or "fe80::1/64".
pub fn parse_cidr_v6(input: &str) -> Result<(u128, u8), String> {
    let (ip_str, prefix_len) = match input.split_once('/') {
        Some((ip, len)) => {
            let len: u8 = len
                .parse()
                .map_err(|_| "Invalid CIDR prefix".to_string())?;
            if len > 128 {
                return Err(format!(
                    "Invalid CIDR prefix length {} (IPv6 max is 128)",
                    len
                ));
            }
            (ip, len)
        }
        None => (input, 128),
    };

    let ip: IpAddr = ip_str
        .parse()
        .map_err(|e| format!("Invalid IP address: {}", e))?;
    let ip_u128 = match ip {
        IpAddr::V6(v6) => u128::from(v6),
        IpAddr::V4(_) => return Err("Only IPv6 addresses are supported".to_string()),
    };

    Ok((ip_u128, prefix_len))
}

/// Calculate full IPv6 subnet information.
pub fn calculate_v6(ip: &str, cidr: u8) -> Result<SubnetResult, String> {
    if cidr > 128 {
        return Err(format!(
            "Invalid CIDR prefix length {} (IPv6 max is 128)",
            cidr
        ));
    }

    let ip: IpAddr = ip
        .parse()
        .map_err(|e| format!("Invalid IP address: {}", e))?;
    let ip_u128 = match ip {
        IpAddr::V6(v6) => u128::from(v6),
        IpAddr::V4(_) => return Err("Only IPv6 addresses are supported".to_string()),
    };

    let host_bits = 128 - cidr;
    let mask = if cidr == 0 {
        0u128
    } else {
        (!0u128) << host_bits
    };
    let network = ip_u128 & mask;

    // Subnet mask and wildcard mask in standard IPv6 notation
    let subnet_mask = Ipv6Addr::from(mask).to_string();
    let wildcard_mask = Ipv6Addr::from(!mask).to_string();

    // IPv6 has no broadcast address
    let broadcast_address = String::new();

    // Calculate usable hosts; cap at u64::MAX for very large ranges
    let usable_hosts = if cidr <= 64 {
        u64::MAX
    } else {
        1u64 << (128 - cidr)
    };

    // IP range: network address to last address in subnet
    let last_addr = network | !mask;
    let ip_range = format!(
        "{} \u{2014} {}",
        Ipv6Addr::from(network),
        Ipv6Addr::from(last_addr)
    );

    let classification = classify_ipv6(ip_u128);

    Ok(SubnetResult {
        network_address: Ipv6Addr::from(network).to_string(),
        broadcast_address,
        subnet_mask,
        wildcard_mask,
        usable_hosts,
        ip_range,
        cidr,
        ip_version: "IPv6".to_string(),
        classification,
    })
}

/// Classify an IPv4 address according to RFC standards.
pub fn classify_ipv4(ip: u32) -> IpClassification {
    // 10.0.0.0/8 — private (RFC 1918)
    if ip & 0xFF00_0000 == 0x0A00_0000 {
        return IpClassification {
            is_private: true,
            is_loopback: false,
            is_link_local: false,
            is_multicast: false,
            is_public: false,
            description: "私有地址 (RFC 1918)".to_string(),
        };
    }

    // 172.16.0.0/12 — private (RFC 1918)
    if ip & 0xFFF0_0000 == 0xAC10_0000 {
        return IpClassification {
            is_private: true,
            is_loopback: false,
            is_link_local: false,
            is_multicast: false,
            is_public: false,
            description: "私有地址 (RFC 1918)".to_string(),
        };
    }

    // 192.168.0.0/16 — private (RFC 1918)
    if ip & 0xFFFF_0000 == 0xC0A8_0000 {
        return IpClassification {
            is_private: true,
            is_loopback: false,
            is_link_local: false,
            is_multicast: false,
            is_public: false,
            description: "私有地址 (RFC 1918)".to_string(),
        };
    }

    // 127.0.0.0/8 — loopback
    if ip & 0xFF00_0000 == 0x7F00_0000 {
        return IpClassification {
            is_private: false,
            is_loopback: true,
            is_link_local: false,
            is_multicast: false,
            is_public: false,
            description: "环回地址".to_string(),
        };
    }

    // 169.254.0.0/16 — link-local
    if ip & 0xFFFF_0000 == 0xA9FE_0000 {
        return IpClassification {
            is_private: false,
            is_loopback: false,
            is_link_local: true,
            is_multicast: false,
            is_public: false,
            description: "链路本地地址".to_string(),
        };
    }

    // 224.0.0.0/4 — multicast
    if ip & 0xF000_0000 == 0xE000_0000 {
        return IpClassification {
            is_private: false,
            is_loopback: false,
            is_link_local: false,
            is_multicast: true,
            is_public: false,
            description: "多播地址".to_string(),
        };
    }

    // 240.0.0.0/4 — reserved
    if ip & 0xF000_0000 == 0xF000_0000 {
        return IpClassification {
            is_private: false,
            is_loopback: false,
            is_link_local: false,
            is_multicast: false,
            is_public: false,
            description: "保留地址".to_string(),
        };
    }

    // Public address
    IpClassification {
        is_private: false,
        is_loopback: false,
        is_link_local: false,
        is_multicast: false,
        is_public: true,
        description: "公网地址".to_string(),
    }
}

/// Classify an IPv6 address according to RFC standards.
pub fn classify_ipv6(ip: u128) -> IpClassification {
    // ::1 — loopback
    if ip == 1 {
        return IpClassification {
            is_private: false,
            is_loopback: true,
            is_link_local: false,
            is_multicast: false,
            is_public: false,
            description: "环回地址".to_string(),
        };
    }

    // fe80::/10 — link-local
    if ip & 0xFFC0_0000_0000_0000_0000_0000_0000_0000
        == 0xFE80_0000_0000_0000_0000_0000_0000_0000
    {
        return IpClassification {
            is_private: false,
            is_loopback: false,
            is_link_local: true,
            is_multicast: false,
            is_public: false,
            description: "链路本地地址".to_string(),
        };
    }

    // fc00::/7 — ULA (唯一本地地址)
    if ip & 0xFE00_0000_0000_0000_0000_0000_0000_0000
        == 0xFC00_0000_0000_0000_0000_0000_0000_0000
    {
        return IpClassification {
            is_private: true,
            is_loopback: false,
            is_link_local: false,
            is_multicast: false,
            is_public: false,
            description: "唯一本地地址 (ULA)".to_string(),
        };
    }

    // ff00::/8 — multicast
    if ip & 0xFF00_0000_0000_0000_0000_0000_0000_0000
        == 0xFF00_0000_0000_0000_0000_0000_0000_0000
    {
        return IpClassification {
            is_private: false,
            is_loopback: false,
            is_link_local: false,
            is_multicast: true,
            is_public: false,
            description: "多播地址".to_string(),
        };
    }

    // 2000::/3 — global unicast (公网)
    if ip & 0xE000_0000_0000_0000_0000_0000_0000_0000
        == 0x2000_0000_0000_0000_0000_0000_0000_0000
    {
        return IpClassification {
            is_private: false,
            is_loopback: false,
            is_link_local: false,
            is_multicast: false,
            is_public: true,
            description: "公网地址 (全局单播)".to_string(),
        };
    }

    // Other
    IpClassification {
        is_private: false,
        is_loopback: false,
        is_link_local: false,
        is_multicast: false,
        is_public: false,
        description: "其他".to_string(),
    }
}

/// Auto-detect IPv4/IPv6 and calculate subnet information.
pub fn calculate_subnet(address: &str, cidr: u8) -> Result<SubnetResult, String> {
    if address.contains(':') {
        calculate_v6(address, cidr)
    } else {
        calculate_v4(address, cidr)
    }
}

/// Split a network into smaller subnets of the given target prefix size.
///
/// Supports both IPv4 and IPv6. The target prefix must be larger than the input prefix.
pub fn split_subnet(network: &str, target_prefix: u8) -> Result<SubnetSplitResult, String> {
    if network.contains(':') {
        split_subnet_v6(network, target_prefix)
    } else {
        split_subnet_v4(network, target_prefix)
    }
}

fn split_subnet_v4(network: &str, target_prefix: u8) -> Result<SubnetSplitResult, String> {
    let (ip, prefix) = parse_cidr_v4(network)?;

    if target_prefix <= prefix {
        return Err(format!(
            "Target prefix ({}) must be greater than current prefix ({})",
            target_prefix, prefix
        ));
    }
    if target_prefix > 32 {
        return Err("Target prefix cannot exceed 32 for IPv4".to_string());
    }

    let step = 1u32 << (32 - target_prefix);
    let base_mask = if prefix == 0 {
        0u32
    } else {
        (!0u32) << (32 - prefix)
    };
    let base_network = ip & base_mask;
    let count = 1u32 << (target_prefix - prefix);

    // Sanity check: limit number of subnets to 4096
    if count > 4096 {
        return Err(format!(
            "Too many subnets to generate ({}). Maximum is 4096.",
            count
        ));
    }

    let mut subnets = Vec::with_capacity(count as usize);
    let mut total_usable = 0u64;

    for i in 0..count {
        let subnet_network = base_network + i * step;
        let cidr_string = format!("{}/{}", ipv4_to_string(subnet_network), target_prefix);
        let subnet_result = calculate_v4(&cidr_string, target_prefix)?;
        total_usable = total_usable.saturating_add(subnet_result.usable_hosts);
        subnets.push(subnet_result);
    }

    Ok(SubnetSplitResult {
        subnets,
        total_usable,
    })
}

fn split_subnet_v6(network: &str, target_prefix: u8) -> Result<SubnetSplitResult, String> {
    let (ip, prefix) = parse_cidr_v6(network)?;

    if target_prefix <= prefix {
        return Err(format!(
            "Target prefix ({}) must be greater than current prefix ({})",
            target_prefix, prefix
        ));
    }
    if target_prefix > 128 {
        return Err("Target prefix cannot exceed 128 for IPv6".to_string());
    }

    let step = 1u128 << (128 - target_prefix);
    let base_mask = if prefix == 0 {
        0u128
    } else {
        (!0u128) << (128 - prefix)
    };
    let base_network = ip & base_mask;
    let count = 1u128 << (target_prefix - prefix);

    // Sanity check: limit number of subnets
    if count > 4096 {
        return Err(format!(
            "Too many subnets to generate ({}). Maximum is 4096.",
            count
        ));
    }

    let mut subnets = Vec::with_capacity(count as usize);
    let mut total_usable = 0u64;

    for i in 0..count {
        let subnet_network = base_network + i * step;
        let cidr_string = format!(
            "{}/{}",
            Ipv6Addr::from(subnet_network),
            target_prefix
        );
        let subnet_result = calculate_v6(&cidr_string, target_prefix)?;
        total_usable = total_usable.saturating_add(subnet_result.usable_hosts);
        subnets.push(subnet_result);
    }

    Ok(SubnetSplitResult {
        subnets,
        total_usable,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── parse_cidr_v4 ───────────────────────────────────────

    #[test]
    fn test_parse_cidr_v4_slash24() {
        let (ip, prefix) = parse_cidr_v4("192.168.1.0/24").unwrap();
        assert_eq!(ip, 0xC0A8_0100);
        assert_eq!(prefix, 24);
    }

    #[test]
    fn test_parse_cidr_v4_slash32() {
        let (ip, prefix) = parse_cidr_v4("192.168.1.42/32").unwrap();
        assert_eq!(ip, 0xC0A8_012A);
        assert_eq!(prefix, 32);
    }

    #[test]
    fn test_parse_cidr_v4_slash31() {
        let (ip, prefix) = parse_cidr_v4("10.0.0.0/31").unwrap();
        assert_eq!(ip, 0x0A00_0000);
        assert_eq!(prefix, 31);
    }

    #[test]
    fn test_parse_cidr_v4_slash16() {
        let (ip, prefix) = parse_cidr_v4("192.168.0.0/16").unwrap();
        assert_eq!(ip, 0xC0A8_0000);
        assert_eq!(prefix, 16);
    }

    #[test]
    fn test_parse_cidr_v4_single_ip() {
        let (ip, prefix) = parse_cidr_v4("192.168.1.1").unwrap();
        assert_eq!(ip, 0xC0A8_0101);
        assert_eq!(prefix, 32);
    }

    #[test]
    fn test_parse_cidr_v4_slash0_rejected() {
        let result = parse_cidr_v4("0.0.0.0/0");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too large"));
    }

    #[test]
    fn test_parse_cidr_v4_invalid_ip() {
        let result = parse_cidr_v4("not_an_ip/24");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_cidr_v4_prefix_too_large() {
        let result = parse_cidr_v4("192.168.1.0/33");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_cidr_v4_slash15_rejected() {
        let result = parse_cidr_v4("10.0.0.0/15");
        assert!(result.is_err());
    }

    // ─── parse_cidr_v6 ───────────────────────────────────────

    #[test]
    fn test_parse_cidr_v6_slash64() {
        let (ip, prefix) = parse_cidr_v6("2001:db8::/64").unwrap();
        assert!(ip > 0);
        assert_eq!(prefix, 64);
    }

    #[test]
    fn test_parse_cidr_v6_slash32() {
        let (ip, prefix) = parse_cidr_v6("2001:db8::/32").unwrap();
        assert!(ip > 0);
        assert_eq!(prefix, 32);
    }

    #[test]
    fn test_parse_cidr_v6_slash128() {
        let (ip, prefix) = parse_cidr_v6("::1/128").unwrap();
        assert_eq!(ip, 1);
        assert_eq!(prefix, 128);
    }

    #[test]
    fn test_parse_cidr_v6_loopback() {
        let (ip, prefix) = parse_cidr_v6("::1").unwrap();
        assert_eq!(ip, 1);
        assert_eq!(prefix, 128);
    }

    #[test]
    fn test_parse_cidr_v6_link_local() {
        let (ip, prefix) = parse_cidr_v6("fe80::1/64").unwrap();
        assert!(ip > 0);
        assert_eq!(prefix, 64);
    }

    #[test]
    fn test_parse_cidr_v6_prefix_too_large() {
        let result = parse_cidr_v6("::1/129");
        assert!(result.is_err());
    }

    // ─── classify_ipv4 ───────────────────────────────────────

    #[test]
    fn test_classify_ipv4_private_10() {
        let c = classify_ipv4(0x0A00_0001);
        assert!(c.is_private);
        assert!(!c.is_loopback);
        assert!(!c.is_public);
        assert!(!c.is_multicast);
        assert!(c.description.contains("私有"));
    }

    #[test]
    fn test_classify_ipv4_private_172() {
        let c = classify_ipv4(0xAC10_0001);
        assert!(c.is_private);
    }

    #[test]
    fn test_classify_ipv4_private_192() {
        let c = classify_ipv4(0xC0A8_0001);
        assert!(c.is_private);
    }

    #[test]
    fn test_classify_ipv4_loopback() {
        let c = classify_ipv4(0x7F00_0001);
        assert!(c.is_loopback);
        assert!(!c.is_private);
        assert!(c.description.contains("环回"));
    }

    #[test]
    fn test_classify_ipv4_public() {
        let c = classify_ipv4(0x0808_0808);
        assert!(c.is_public);
        assert!(!c.is_private);
        assert!(!c.is_loopback);
        assert!(c.description.contains("公网"));
    }

    #[test]
    fn test_classify_ipv4_multicast() {
        let c = classify_ipv4(0xE000_0001);
        assert!(c.is_multicast);
    }

    #[test]
    fn test_classify_ipv4_link_local() {
        let c = classify_ipv4(0xA9FE_0001);
        assert!(c.is_link_local);
    }

    // ─── classify_ipv6 ───────────────────────────────────────

    #[test]
    fn test_classify_ipv6_ula() {
        let ip = u128::from_str_radix("fc001234567890000000000000000000", 16).unwrap();
        let c = classify_ipv6(ip);
        assert!(c.is_private);
        assert!(c.description.contains("ULA"));
    }

    #[test]
    fn test_classify_ipv6_link_local() {
        let ip = u128::from_str_radix("fe800000000000000000000000000000", 16).unwrap();
        let c = classify_ipv6(ip);
        assert!(c.is_link_local);
        assert!(c.description.contains("链路本地"));
    }

    #[test]
    fn test_classify_ipv6_multicast() {
        let ip = u128::from_str_radix("ff000000000000000000000000000000", 16).unwrap();
        let c = classify_ipv6(ip);
        assert!(c.is_multicast);
        assert!(c.description.contains("多播"));
    }

    #[test]
    fn test_classify_ipv6_loopback() {
        let c = classify_ipv6(1);
        assert!(c.is_loopback);
        assert!(c.description.contains("环回"));
    }

    #[test]
    fn test_classify_ipv6_global_unicast() {
        let ip = u128::from_str_radix("20010000000000000000000000000000", 16).unwrap();
        let c = classify_ipv6(ip);
        assert!(c.is_public);
        assert!(c.description.contains("公网"));
    }

    // ─── calculate_v4 ────────────────────────────────────────

    #[test]
    fn test_calculate_v4_24() {
        let r = calculate_v4("192.168.1.0", 24).unwrap();
        assert_eq!(r.network_address, "192.168.1.0");
        assert_eq!(r.broadcast_address, "192.168.1.255");
        assert_eq!(r.subnet_mask, "255.255.255.0");
        assert_eq!(r.wildcard_mask, "0.0.0.255");
        assert_eq!(r.usable_hosts, 254);
        assert!(r.ip_range.contains("192.168.1.1"));
        assert!(r.ip_range.contains("192.168.1.254"));
        assert_eq!(r.ip_version, "IPv4");
        assert!(r.classification.is_private);
    }

    #[test]
    fn test_calculate_v4_16() {
        let r = calculate_v4("192.168.0.0", 16).unwrap();
        assert_eq!(r.network_address, "192.168.0.0");
        assert_eq!(r.broadcast_address, "192.168.255.255");
        assert_eq!(r.usable_hosts, 65534);
    }

    #[test]
    fn test_calculate_v4_32() {
        let r = calculate_v4("192.168.1.42", 32).unwrap();
        assert_eq!(r.network_address, "192.168.1.42");
        assert_eq!(r.broadcast_address, ""); // no broadcast for /32
        assert_eq!(r.usable_hosts, 1);
    }

    #[test]
    fn test_calculate_v4_31() {
        let r = calculate_v4("10.0.0.0", 31).unwrap();
        assert_eq!(r.network_address, "10.0.0.0");
        assert_eq!(r.broadcast_address, ""); // no broadcast for /31
        assert_eq!(r.usable_hosts, 2);
    }

    #[test]
    fn test_calculate_v4_public() {
        let r = calculate_v4("8.8.8.8", 24).unwrap();
        assert!(r.classification.is_public);
    }

    // ─── calculate_v6 ────────────────────────────────────────

    #[test]
    fn test_calculate_v6_basic() {
        let r = calculate_v6("2001:db8::", 32).unwrap();
        assert_eq!(r.network_address, "2001:db8::");
        assert_eq!(r.broadcast_address, ""); // IPv6 has no broadcast
        assert_eq!(r.usable_hosts, u64::MAX); // /32 is huge, capped
        assert_eq!(r.ip_version, "IPv6");
        assert!(r.classification.is_public);
    }

    #[test]
    fn test_calculate_v6_64() {
        let r = calculate_v6("2001:db8::", 64).unwrap();
        assert_eq!(r.usable_hosts, u64::MAX); // /64 capped
    }

    #[test]
    fn test_calculate_v6_120() {
        let r = calculate_v6("2001:db8::", 120).unwrap();
        assert_eq!(r.usable_hosts, 256); // 2^8 = 256
    }

    #[test]
    fn test_calculate_v6_link_local() {
        let r = calculate_v6("fe80::1", 64).unwrap();
        assert!(r.classification.is_link_local);
        assert!(r.classification.description.contains("链路本地"));
    }

    // ─── split_subnet ────────────────────────────────────────

    #[test]
    fn test_split_subnet_v4_24_to_26() {
        let r = split_subnet("192.168.1.0/24", 26).unwrap();
        assert_eq!(r.subnets.len(), 4);
        assert_eq!(r.total_usable, 248); // 4 * 62

        // Verify non-overlap
        let ranges: Vec<(u32, u32)> = r
            .subnets
            .iter()
            .map(|s| {
                let network = parse_ipv4(&s.network_address);
                let host_bits = 32 - s.cidr;
                let last = network | (!0u32 << host_bits);
                (network, last)
            })
            .collect();

        for i in 0..ranges.len() {
            for j in (i + 1)..ranges.len() {
                assert!(
                    ranges[i].1 < ranges[j].0 || ranges[j].1 < ranges[i].0,
                    "Subnets {} and {} overlap",
                    r.subnets[i].network_address,
                    r.subnets[j].network_address
                );
            }
        }

        // Verify first subnet
        assert_eq!(r.subnets[0].network_address, "192.168.1.0");
        assert_eq!(r.subnets[0].usable_hosts, 62);
        assert_eq!(r.subnets[1].network_address, "192.168.1.64");
        assert_eq!(r.subnets[2].network_address, "192.168.1.128");
        assert_eq!(r.subnets[3].network_address, "192.168.1.192");
    }

    #[test]
    fn test_split_subnet_v4_target_less_than_prefix() {
        let result = split_subnet("192.168.1.0/24", 16);
        assert!(result.is_err());
    }

    #[test]
    fn test_split_subnet_v4_invalid_target() {
        let result = split_subnet("192.168.1.0/24", 33);
        assert!(result.is_err());
    }

    #[test]
    fn test_split_subnet_v6_basic() {
        let r = split_subnet("2001:db8::/32", 36).unwrap();
        assert_eq!(r.subnets.len(), 16); // 2^(36-32) = 16
    }

    fn parse_ipv4(s: &str) -> u32 {
        let parts: Vec<u8> = s
            .split('.')
            .map(|p| p.parse().unwrap())
            .collect();
        u32::from_be_bytes([parts[0], parts[1], parts[2], parts[3]])
    }
}
