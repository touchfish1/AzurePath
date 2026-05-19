use crate::core::settings::AppSettings;

#[tauri::command]
pub async fn get_settings() -> Result<AppSettings, String> {
    AppSettings::load()
}

#[tauri::command]
pub async fn save_settings(settings: AppSettings) -> Result<(), String> {
    settings.save()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_roundtrip() {
        let settings = AppSettings {
            theme: "dark".to_string(),
            clipboard_interval: 2000,
            clipboard_max_entries: 100,
            ping_default_count: 5,
            ping_default_timeout: 5000,
            download_dir: "/tmp/test".to_string(),
            retention_days: 60,
            notify_file_transfer: false,
            notify_chat_message: false,
            notify_scan_complete: true,
        };

        let json = serde_json::to_string(&settings).unwrap();
        let deserialized: AppSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.theme, "dark");
        assert_eq!(deserialized.clipboard_interval, 2000);
        assert_eq!(deserialized.notify_scan_complete, true);
    }

    #[test]
    fn test_settings_camel_case_serde() {
        let json = r#"{
            "theme": "light",
            "clipboardInterval": 3000,
            "clipboardMaxEntries": 200,
            "pingDefaultCount": 3,
            "pingDefaultTimeout": 2000,
            "downloadDir": "/tmp/dl",
            "retentionDays": 90,
            "notifyFileTransfer": true,
            "notifyChatMessage": false,
            "notifyScanComplete": true
        }"#;
        let s: AppSettings = serde_json::from_str(json).unwrap();
        assert_eq!(s.theme, "light");
        assert_eq!(s.clipboard_interval, 3000);
        assert_eq!(s.clipboard_max_entries, 200);
        assert_eq!(s.download_dir, "/tmp/dl");
        assert_eq!(s.retention_days, 90);
    }
}
