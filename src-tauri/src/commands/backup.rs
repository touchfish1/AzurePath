use std::path::PathBuf;
use tauri::AppHandle;
use tauri::Manager;

fn azurepath_data_dir() -> PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join("AzurePath")
}

#[tauri::command]
pub fn backup_all_data(app: AppHandle) -> Result<String, String> {
    use std::io::Write;

    let data_dir = azurepath_data_dir();
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let backup_dir = data_dir.join("backups");
    std::fs::create_dir_all(&backup_dir)
        .map_err(|e| format!("Failed to create backup dir: {}", e))?;

    let backup_path = backup_dir.join(format!("azurepath_backup_{}.json", timestamp));

    let mut backup = serde_json::Map::new();

    // Settings
    let settings_path = data_dir.join("settings.json");
    if let Ok(content) = std::fs::read_to_string(&settings_path) {
        backup.insert("settings".into(), serde_json::Value::String(content));
    }

    // Clipboard entries (stored in azurepath.db)
    if let Ok(store) = crate::core::clipboard::ClipboardStore::new() {
        if let Ok(entries) = store.list(None, 10000) {
            if let Ok(json) = serde_json::to_value(&entries) {
                backup.insert("clipboard".into(), json);
            }
        }
    }

    // Monitor targets & history (stored in monitor.db)
    if let Ok(store) = crate::core::monitor::MonitorStore::new() {
        if let Ok(targets) = store.list_targets() {
            if let Ok(json) = serde_json::to_value(&targets) {
                backup.insert("monitor_targets".into(), json);
            }
        }
        if let Ok(history) = store.get_all_recent_history(365) {
            if let Ok(json) = serde_json::to_value(&history) {
                backup.insert("monitor_history".into(), json);
            }
        }
    }

    // Chat history (stored in azurepath.db)
    if let Ok(store) = crate::core::chat::ChatStore::new() {
        if let Ok(messages) = store.get_messages(None, 10000) {
            if let Ok(json) = serde_json::to_value(&messages) {
                backup.insert("chat_history".into(), json);
            }
        }
    }

    // WOL devices (stored in Tauri app data dir)
    if let Ok(app_data) = app.path().app_data_dir() {
        if let Ok(devices) = crate::core::wol::load_records(&app_data) {
            if let Ok(json) = serde_json::to_value(&devices) {
                backup.insert("wol_devices".into(), json);
            }
        }
    }

    // Activity history (stored in azurepath.db)
    if let Ok(store) = crate::core::history::ActivityStore::new() {
        if let Ok(activities) = store.list(10000, 0, None) {
            if let Ok(json) = serde_json::to_value(&activities) {
                backup.insert("activity_history".into(), json);
            }
        }
    }

    let json =
        serde_json::to_string_pretty(&backup).map_err(|e| format!("Failed to serialize: {}", e))?;

    let mut file = std::fs::File::create(&backup_path)
        .map_err(|e| format!("Failed to create backup file: {}", e))?;
    file.write_all(json.as_bytes())
        .map_err(|e| format!("Failed to write backup: {}", e))?;

    Ok(backup_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn list_backups() -> Result<Vec<serde_json::Value>, String> {
    let backup_dir = azurepath_data_dir().join("backups");
    if !backup_dir.exists() {
        return Ok(Vec::new());
    }

    let mut backups = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(&backup_dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "json"))
        .collect();

    entries.sort_by_key(|e| {
        std::cmp::Reverse(
            e.metadata()
                .and_then(|m| m.created().or_else(|_| m.modified()))
                .ok(),
        )
    });

    for entry in entries.into_iter().take(50) {
        let path = entry.path();
        let metadata = entry.metadata().ok();
        backups.push(serde_json::json!({
            "name": path.file_name().and_then(|n| n.to_str()).unwrap_or(""),
            "size": metadata.as_ref().map(|m| m.len()).unwrap_or(0),
            "created": metadata.and_then(|m| m.created().ok())
                .map(|t| {
                    let dt: chrono::DateTime<chrono::Local> = t.into();
                    dt.to_rfc3339()
                }),
            "path": path.to_string_lossy(),
        }));
    }

    Ok(backups)
}

#[tauri::command]
pub fn restore_backup(path: String) -> Result<String, String> {
    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read backup file: {}", e))?;
    let data: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("Invalid backup file: {}", e))?;

    let mut restored = Vec::new();

    if let Some(settings) = data.get("settings").and_then(|v| v.as_str()) {
        let settings_path = azurepath_data_dir().join("settings.json");
        std::fs::write(&settings_path, settings)
            .map_err(|e| format!("Failed to restore settings: {}", e))?;
        restored.push("settings");
    }

    // For other data types, restoration would need more complex logic.
    // This is a baseline that restores what's easily restorable.

    Ok(format!("Restored: {}", restored.join(", ")))
}

#[tauri::command]
pub fn delete_backup(path: String) -> Result<(), String> {
    std::fs::remove_file(&path).map_err(|e| format!("Failed to delete backup: {}", e))
}
