/// MAC address vendor lookup using a built-in OUI database.
pub struct MacLookup;

const OUI_DB: &[(&str, &str)] = &[
    ("00:00:0C", "Cisco"),
    ("00:14:22", "Dell"),
    ("00:1A:A0", "Broadcom"),
    ("3C:5A:B4", "Intel"),
    ("B8:27:EB", "Raspberry Pi"),
    ("DC:A6:32", "Intel"),
    ("00:50:56", "VMware"),
    ("00:0C:29", "VMware"),
    ("00:05:69", "VMware"),
    ("08:00:27", "Oracle VirtualBox"),
    ("00:15:5D", "Microsoft"),
    ("00:03:FF", "Microsoft"),
    ("A8:5E:45", "Realtek"),
    ("00:E0:4C", "Realtek"),
    ("00:23:8E", "Samsung"),
    ("00:25:00", "Apple"),
    ("00:26:08", "Apple"),
    ("F0:18:98", "Apple"),
    ("00:17:34", "Google"),
    ("E0:AC:CB", "Huawei"),
    ("00:25:9C", "Huawei"),
    ("3C:61:05", "TP-Link"),
    ("54:A6:0B", "TP-Link"),
    ("30:39:F2", "Xiaomi"),
    ("F4:6D:04", "Xiaomi"),
    ("28:16:2E", "Xiaomi"),
    ("00:0A:EB", "Juniper"),
    ("00:1B:17", "HP"),
    ("00:14:5E", "Hewlett Packard"),
    ("00:26:55", "Netgear"),
    ("00:90:7F", "Linksys"),
    ("00:1A:70", "D-Link"),
    ("00:80:C8", "Asus"),
    ("00:0A:F5", "Sony"),
    ("10:68:3F", "Nokia"),
    ("00:17:88", "Roku"),
    ("A0:CE:C8", "Nest Labs"),
    ("B0:4E:26", "Amazon Tech"),
    ("AC:63:BE", "Samsung"),
    ("00:1D:41", "ZTE"),
];

impl MacLookup {
    pub fn new() -> Self {
        Self
    }

    /// Look up the vendor name for a given MAC address.
    /// Returns `None` if the OUI is not found in the database.
    ///
    /// The MAC address can be in any common format:
    /// - `XX:XX:XX:XX:XX:XX` (colons)
    /// - `XX-XX-XX-XX-XX-XX` (dashes)
    /// - `XXXXXXXXXXXX` (no separators)
    /// - `XXXX.XXXX.XXXX` (dots - Cisco style)
    pub fn lookup(&self, mac: &str) -> Option<String> {
        let normalized = normalize_mac(mac)?;
        let prefix = &normalized[..8]; // First 8 chars = "XX:XX:XX"

        for (oui, vendor) in OUI_DB {
            if *oui == prefix {
                return Some(vendor.to_string());
            }
        }
        None
    }
}

/// Normalize a MAC address to `XX:XX:XX:XX:XX:XX` format.
fn normalize_mac(mac: &str) -> Option<String> {
    let cleaned: String = mac
        .chars()
        .filter(|c| c.is_ascii_hexdigit() || *c == ':' || *c == '-' || *c == '.')
        .collect::<String>()
        .to_uppercase();

    let hex_part: String = cleaned.chars().filter(|c| c.is_ascii_hexdigit()).collect();

    if hex_part.len() != 12 {
        return None;
    }

    // Format as XX:XX:XX:XX:XX:XX
    let bytes: Vec<String> = hex_part
        .as_bytes()
        .chunks(2)
        .map(|chunk| std::str::from_utf8(chunk).unwrap_or("00").to_string())
        .collect();

    Some(bytes.join(":"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_cisco() {
        let lookup = MacLookup::new();
        assert_eq!(
            lookup.lookup("00:00:0C:12:34:56"),
            Some("Cisco".to_string())
        );
    }

    #[test]
    fn test_lookup_vmware() {
        let lookup = MacLookup::new();
        assert_eq!(
            lookup.lookup("00:50:56:AB:CD:EF"),
            Some("VMware".to_string())
        );
    }

    #[test]
    fn test_lookup_raspberry_pi() {
        let lookup = MacLookup::new();
        assert_eq!(
            lookup.lookup("B8:27:EB:12:34:56"),
            Some("Raspberry Pi".to_string())
        );
    }

    #[test]
    fn test_lookup_dash_format() {
        let lookup = MacLookup::new();
        assert_eq!(
            lookup.lookup("00-14-22-AB-CD-EF"),
            Some("Dell".to_string())
        );
    }

    #[test]
    fn test_lookup_no_separator_format() {
        let lookup = MacLookup::new();
        assert_eq!(
            lookup.lookup("3C5AB4123456"),
            Some("Intel".to_string())
        );
    }

    #[test]
    fn test_lookup_cisco_dot_format() {
        let lookup = MacLookup::new();
        assert_eq!(
            lookup.lookup("0000.0C12.3456"),
            Some("Cisco".to_string())
        );
    }

    #[test]
    fn test_lookup_not_found() {
        let lookup = MacLookup::new();
        assert_eq!(lookup.lookup("AA:BB:CC:DD:EE:FF"), None);
    }

    #[test]
    fn test_lookup_invalid_mac() {
        let lookup = MacLookup::new();
        assert_eq!(lookup.lookup("invalid"), None);
    }

    #[test]
    fn test_lookup_empty_string() {
        let lookup = MacLookup::new();
        assert_eq!(lookup.lookup(""), None);
    }

    #[test]
    fn test_lookup_short_mac() {
        let lookup = MacLookup::new();
        assert_eq!(lookup.lookup("00:11:22"), None);
    }

    #[test]
    fn test_lookup_lowercase() {
        let lookup = MacLookup::new();
        assert_eq!(
            lookup.lookup("b8:27:eb:12:34:56"),
            Some("Raspberry Pi".to_string())
        );
    }

    #[test]
    fn test_normalize_mac_colons() {
        assert_eq!(
            normalize_mac("00:50:56:AB:CD:EF"),
            Some("00:50:56:AB:CD:EF".to_string())
        );
    }

    #[test]
    fn test_normalize_mac_dashes() {
        assert_eq!(
            normalize_mac("00-50-56-AB-CD-EF"),
            Some("00:50:56:AB:CD:EF".to_string())
        );
    }

    #[test]
    fn test_normalize_mac_no_separator() {
        assert_eq!(
            normalize_mac("005056ABCDEF"),
            Some("00:50:56:AB:CD:EF".to_string())
        );
    }

    #[test]
    fn test_normalize_mac_dot_format() {
        assert_eq!(
            normalize_mac("0050.56AB.CDEF"),
            Some("00:50:56:AB:CD:EF".to_string())
        );
    }

    #[test]
    fn test_normalize_mac_invalid_chars() {
        assert_eq!(normalize_mac("00:50:56:AB:CD:GG"), None);
    }

    #[test]
    fn test_normalize_mac_too_short() {
        assert_eq!(normalize_mac("00:11:22:33"), None);
    }

    #[test]
    fn test_normalize_mac_too_long() {
        assert_eq!(normalize_mac("00:11:22:33:44:55:66"), None);
    }

    #[test]
    fn test_new_returns_instance() {
        let lookup = MacLookup::new();
        assert!(lookup.lookup("00:00:0C:00:00:00").is_some());
    }
}
