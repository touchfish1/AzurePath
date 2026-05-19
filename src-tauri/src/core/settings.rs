use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub theme: String,
    pub clipboard_interval: u64,
    pub clipboard_max_entries: usize,
    pub ping_default_count: u32,
    pub ping_default_timeout: u64,
    pub download_dir: String,
    pub retention_days: u32,
    pub notify_file_transfer: bool,
    pub notify_chat_message: bool,
    pub notify_scan_complete: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "system".to_string(),
            clipboard_interval: 1000,
            clipboard_max_entries: 500,
            ping_default_count: 4,
            ping_default_timeout: 3000,
            download_dir: dirs_or_default(),
            retention_days: 30,
            notify_file_transfer: true,
            notify_chat_message: true,
            notify_scan_complete: false,
        }
    }
}

fn dirs_or_default() -> String {
    crate::core::utils::home_dir()
        .map(|p| p.join("Downloads").to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string())
}

use std::path::PathBuf;

impl AppSettings {
    fn settings_path() -> Result<PathBuf, String> {
        let home = crate::core::utils::home_dir()
            .ok_or_else(|| "Cannot find home directory".to_string())?;
        Ok(home.join("AzurePath").join("settings.json"))
    }

    pub fn load() -> Result<Self, String> {
        let path = Self::settings_path()?;
        if !path.exists() {
            let settings = Self::default();
            settings.save()?;
            return Ok(settings);
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read settings file: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse settings: {}", e))
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::settings_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create settings directory: {}", e))?;
        }
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;
        std::fs::write(&path, content)
            .map_err(|e| format!("Failed to write settings file: {}", e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let s = AppSettings::default();
        assert_eq!(s.theme, "system");
        assert_eq!(s.clipboard_interval, 1000);
        assert_eq!(s.clipboard_max_entries, 500);
        assert_eq!(s.ping_default_count, 4);
        assert_eq!(s.ping_default_timeout, 3000);
        assert!(s.notify_file_transfer);
        assert!(s.notify_chat_message);
        assert!(!s.notify_scan_complete);
        assert_eq!(s.retention_days, 30);
    }

    #[test]
    fn test_serialize_deserialize() {
        let s = AppSettings {
            theme: "dark".to_string(),
            clipboard_interval: 2000,
            clipboard_max_entries: 100,
            ping_default_count: 5,
            ping_default_timeout: 5000,
            download_dir: "/tmp/downloads".to_string(),
            retention_days: 60,
            notify_file_transfer: false,
            notify_chat_message: false,
            notify_scan_complete: true,
        };

        let json = serde_json::to_string_pretty(&s).unwrap();
        let deserialized: AppSettings = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.theme, "dark");
        assert_eq!(deserialized.clipboard_interval, 2000);
        assert_eq!(deserialized.clipboard_max_entries, 100);
        assert_eq!(deserialized.ping_default_count, 5);
        assert_eq!(deserialized.ping_default_timeout, 5000);
        assert_eq!(deserialized.download_dir, "/tmp/downloads");
        assert_eq!(deserialized.retention_days, 60);
        assert!(!deserialized.notify_file_transfer);
        assert!(!deserialized.notify_chat_message);
        assert!(deserialized.notify_scan_complete);
    }

    #[test]
    fn test_serde_field_names() {
        let json = r#"{
            "theme": "dark",
            "clipboardInterval": 2000,
            "clipboardMaxEntries": 100,
            "pingDefaultCount": 5,
            "pingDefaultTimeout": 5000,
            "downloadDir": "/tmp/dl",
            "retentionDays": 60,
            "notifyFileTransfer": false,
            "notifyChatMessage": true,
            "notifyScanComplete": false
        }"#;
        let s: AppSettings = serde_json::from_str(json).unwrap();
        assert_eq!(s.theme, "dark");
        assert_eq!(s.clipboard_interval, 2000);
        assert_eq!(s.clipboard_max_entries, 100);
        assert_eq!(s.download_dir, "/tmp/dl");
    }

    #[test]
    fn test_default_download_dir() {
        let s = AppSettings::default();
        assert!(!s.download_dir.is_empty());
    }
}
