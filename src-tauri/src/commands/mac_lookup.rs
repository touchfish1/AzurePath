use crate::core::mac_lookup::MacLookup;
use std::sync::OnceLock;

static MAC_LOOKUP: OnceLock<MacLookup> = OnceLock::new();

fn get_lookup() -> &'static MacLookup {
    MAC_LOOKUP.get_or_init(|| MacLookup::new())
}

#[tauri::command]
pub async fn mac_lookup(mac: String) -> Result<Option<String>, String> {
    let lookup = get_lookup();
    Ok(lookup.lookup(&mac))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mac_lookup_known_vendor() {
        let result = futures::executor::block_on(mac_lookup("00:00:0C:12:34:56".into()));
        assert_eq!(result.unwrap(), Some("Cisco".to_string()));
    }

    #[test]
    fn test_mac_lookup_unknown_vendor() {
        let result = futures::executor::block_on(mac_lookup("AA:BB:CC:DD:EE:FF".into()));
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn test_mac_lookup_invalid_mac() {
        let result = futures::executor::block_on(mac_lookup("invalid".into()));
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn test_mac_lookup_empty_string() {
        let result = futures::executor::block_on(mac_lookup("".into()));
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn test_mac_lookup_dash_format() {
        let result = futures::executor::block_on(mac_lookup("00-14-22-AB-CD-EF".into()));
        assert_eq!(result.unwrap(), Some("Dell".to_string()));
    }

    #[test]
    fn test_mac_lookup_lowercase() {
        let result = futures::executor::block_on(mac_lookup("b8:27:eb:12:34:56".into()));
        assert_eq!(result.unwrap(), Some("Raspberry Pi".to_string()));
    }
}
