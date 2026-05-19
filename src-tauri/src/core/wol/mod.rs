use std::path::PathBuf;
use uuid::Uuid;

use chrono::Utc;

use crate::types::wol::WolRecord;

/// Parse a MAC address into 6 bytes.
/// Supports formats: 00:11:22:33:44:55, 00-11-22-33-44-55, 001122334455
fn parse_mac(mac: &str) -> Result<[u8; 6], String> {
    let cleaned: String = mac.chars().filter(|c| *c != ':' && *c != '-' && *c != ' ').collect();

    if cleaned.len() != 12 {
        return Err(format!(
            "Invalid MAC length: expected 12 hex digits, got {}",
            cleaned.len()
        ));
    }

    let bytes: Result<Vec<u8>, _> = (0..12)
        .step_by(2)
        .map(|i| u8::from_str_radix(&cleaned[i..i + 2], 16))
        .collect();

    match bytes {
        Ok(b) => {
            if b.len() != 6 {
                return Err("Failed to parse MAC: expected 6 bytes".to_string());
            }
            let mut arr = [0u8; 6];
            arr.copy_from_slice(&b);
            Ok(arr)
        }
        Err(e) => Err(format!("Invalid MAC hex digit: {}", e)),
    }
}

/// Send a Wake-on-LAN magic packet.
pub async fn send_magic_packet(mac: &str, broadcast_ip: &str, port: u16) -> Result<String, String> {
    let mac_bytes = parse_mac(mac)?;

    // Build magic packet: 6 bytes of 0xFF + MAC repeated 16 times
    let mut packet = Vec::with_capacity(102);
    packet.extend_from_slice(&[0xFF; 6]);
    for _ in 0..16 {
        packet.extend_from_slice(&mac_bytes);
    }

    let addr = format!("{}:{}", broadcast_ip, port);

    let socket = tokio::net::UdpSocket::bind("0.0.0.0:0")
        .await
        .map_err(|e| format!("Failed to bind UDP socket: {}", e))?;

    socket
        .set_broadcast(true)
        .map_err(|e| format!("Failed to set broadcast: {}", e))?;

    let sent = socket
        .send_to(&packet, &addr)
        .await
        .map_err(|e| format!("Failed to send magic packet: {}", e))?;

    if sent != packet.len() {
        return Err(format!(
            "Sent only {} bytes of {}",
            sent,
            packet.len()
        ));
    }

    Ok(format!("Magic packet sent to {} via {}", mac, addr))
}

// ─── Persistence ───────────────────────────────────────────────

fn records_path(app_data_dir: &PathBuf) -> PathBuf {
    app_data_dir.join("wol_records.json")
}

fn load_all(app_data_dir: &PathBuf) -> Vec<WolRecord> {
    let path = records_path(app_data_dir);
    if !path.exists() {
        return Vec::new();
    }
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

fn save_all(app_data_dir: &PathBuf, records: &[WolRecord]) -> Result<(), String> {
    let path = records_path(app_data_dir);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create dir: {}", e))?;
    }

    let content = serde_json::to_string_pretty(records)
        .map_err(|e| format!("Failed to serialize records: {}", e))?;
    std::fs::write(&path, content).map_err(|e| format!("Failed to write records: {}", e))?;
    Ok(())
}

/// Save a new WOL record.
pub fn save_record(
    mac: String,
    broadcast_ip: String,
    label: String,
    app_data_dir: &PathBuf,
) -> Result<WolRecord, String> {
    let mut records = load_all(app_data_dir);

    let record = WolRecord {
        id: Uuid::new_v4().to_string(),
        mac,
        broadcast_ip,
        port: 9,
        label,
        last_used: Utc::now().to_rfc3339(),
    };

    records.push(record.clone());
    save_all(app_data_dir, &records)?;

    Ok(record)
}

/// Load all saved WOL records.
pub fn load_records(app_data_dir: &PathBuf) -> Result<Vec<WolRecord>, String> {
    Ok(load_all(app_data_dir))
}

/// Delete a WOL record by ID.
pub fn delete_record(id: String, app_data_dir: &PathBuf) -> Result<(), String> {
    let mut records = load_all(app_data_dir);
    let len_before = records.len();
    records.retain(|r| r.id != id);

    if records.len() == len_before {
        return Err(format!("Record with id '{}' not found", id));
    }

    save_all(app_data_dir, &records)
}

/// Update the last_used timestamp for a record.
pub fn touch_record(id: &str, app_data_dir: &PathBuf) {
    let mut records = load_all(app_data_dir);
    if let Some(record) = records.iter_mut().find(|r| r.id == id) {
        record.last_used = Utc::now().to_rfc3339();
        let _ = save_all(app_data_dir, &records);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().to_path_buf();
        (dir, path)
    }

    #[test]
    fn test_parse_mac_colon() {
        let mac = parse_mac("00:11:22:33:44:55").unwrap();
        assert_eq!(mac, [0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    }

    #[test]
    fn test_parse_mac_hyphen() {
        let mac = parse_mac("00-11-22-33-44-55").unwrap();
        assert_eq!(mac, [0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    }

    #[test]
    fn test_parse_mac_compact() {
        let mac = parse_mac("001122334455").unwrap();
        assert_eq!(mac, [0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    }

    #[test]
    fn test_parse_mac_invalid() {
        assert!(parse_mac("not-a-mac").is_err());
        assert!(parse_mac("").is_err());
        assert!(parse_mac("00:11:22:33:44:GG").is_err());
        assert!(parse_mac("00:11:22:33:44").is_err());
    }

    #[test]
    fn test_parse_mac_broadcast() {
        let mac = parse_mac("FF:FF:FF:FF:FF:FF").unwrap();
        assert_eq!(mac, [0xFF; 6]);
    }

    #[test]
    fn test_save_and_load_records() {
        let (_tmp, dir) = setup();

        let record = save_record(
            "00:11:22:33:44:55".to_string(),
            "192.168.1.255".to_string(),
            "Test PC".to_string(),
            &dir,
        )
        .unwrap();

        assert_eq!(record.mac, "00:11:22:33:44:55");
        assert_eq!(record.broadcast_ip, "192.168.1.255");
        assert_eq!(record.label, "Test PC");
        assert_eq!(record.port, 9);

        let records = load_records(&dir).unwrap();
        assert_eq!(records.len(), 1);
    }

    #[test]
    fn test_delete_record() {
        let (_tmp, dir) = setup();

        let record = save_record(
            "00:11:22:33:44:55".to_string(),
            "192.168.1.255".to_string(),
            "Test PC".to_string(),
            &dir,
        )
        .unwrap();

        assert_eq!(load_records(&dir).unwrap().len(), 1);

        delete_record(record.id.clone(), &dir).unwrap();
        assert_eq!(load_records(&dir).unwrap().len(), 0);
    }

    #[test]
    fn test_delete_nonexistent() {
        let (_tmp, dir) = setup();
        let result = delete_record("nonexistent".to_string(), &dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_load() {
        let (_tmp, dir) = setup();
        let records = load_records(&dir).unwrap();
        assert!(records.is_empty());
    }

    #[tokio::test]
    async fn test_send_magic_packet_invalid_mac() {
        let result = send_magic_packet("invalid", "192.168.1.255", 9).await;
        assert!(result.is_err());
    }
}
