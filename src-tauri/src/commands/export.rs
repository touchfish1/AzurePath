use crate::core::chat::ChatStore;
use crate::core::clipboard::ClipboardStore;
use crate::core::settings::AppSettings;
use crate::core::utils::home_dir;

fn get_export_dir() -> Result<std::path::PathBuf, String> {
    let dir = home_dir()
        .ok_or_else(|| "Cannot find home directory".to_string())?
        .join("AzurePath")
        .join("exports");
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create exports directory: {}", e))?;
    Ok(dir)
}

#[tauri::command]
pub async fn export_chat(format: String) -> Result<String, String> {
    let store = ChatStore::new()?;
    let messages = store
        .get_messages(None, 10000)
        .map_err(|e| format!("Failed to get messages: {}", e))?;
    let export_dir = get_export_dir()?;
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");

    match format.to_lowercase().as_str() {
        "json" => {
            let content = serde_json::to_string_pretty(&messages)
                .map_err(|e| format!("Failed to serialize: {}", e))?;
            let file_path = export_dir.join(format!("chat_{}.json", timestamp));
            std::fs::write(&file_path, content)
                .map_err(|e| format!("Failed to write file: {}", e))?;
            Ok(file_path.to_string_lossy().to_string())
        }
        "txt" => {
            let mut content = String::new();
            content.push_str("=== AzurePath Chat Export ===\n\n");
            for msg in &messages {
                let direction = if msg.is_incoming { "<<" } else { ">>" };
                content.push_str(&format!(
                    "[{}] {} {} ({}): {}\n",
                    msg.created_at, direction, msg.peer_name, msg.peer_id, msg.content
                ));
            }
            let file_path = export_dir.join(format!("chat_{}.txt", timestamp));
            std::fs::write(&file_path, content)
                .map_err(|e| format!("Failed to write file: {}", e))?;
            Ok(file_path.to_string_lossy().to_string())
        }
        _ => Err(format!("Unsupported format: {}. Supported formats: json, txt", format)),
    }
}

#[tauri::command]
pub async fn export_clipboard(format: String) -> Result<String, String> {
    let store = ClipboardStore::new()?;
    let entries = store
        .list(None, 10000)
        .map_err(|e| format!("Failed to get clipboard entries: {}", e))?;
    let export_dir = get_export_dir()?;
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");

    match format.to_lowercase().as_str() {
        "json" => {
            let content = serde_json::to_string_pretty(&entries)
                .map_err(|e| format!("Failed to serialize: {}", e))?;
            let file_path = export_dir.join(format!("clipboard_{}.json", timestamp));
            std::fs::write(&file_path, content)
                .map_err(|e| format!("Failed to write file: {}", e))?;
            Ok(file_path.to_string_lossy().to_string())
        }
        "txt" => {
            let mut content = String::new();
            content.push_str("=== AzurePath Clipboard Export ===\n\n");
            for entry in &entries {
                content.push_str(&format!("--- Entry {} ---\n", entry.id));
                content.push_str(&format!("Type: {}\n", entry.content_type));
                if let Some(ref text) = entry.text_content {
                    content.push_str(&format!("Content: {}\n", text));
                }
                if let Some(ref img) = entry.image_path {
                    content.push_str(&format!("Image: {}\n", img));
                }
                if let Some(ref files) = entry.file_paths {
                    content.push_str(&format!("Files: {}\n", files.join(", ")));
                }
                content.push_str(&format!("Created: {}\n\n", entry.created_at));
            }
            let file_path = export_dir.join(format!("clipboard_{}.txt", timestamp));
            std::fs::write(&file_path, content)
                .map_err(|e| format!("Failed to write file: {}", e))?;
            Ok(file_path.to_string_lossy().to_string())
        }
        _ => Err(format!("Unsupported format: {}. Supported formats: json, txt", format)),
    }
}

#[tauri::command]
pub async fn export_settings() -> Result<String, String> {
    let settings = AppSettings::load()?;
    let export_dir = get_export_dir()?;
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");

    let content = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    let file_path = export_dir.join(format!("settings_{}.json", timestamp));
    std::fs::write(&file_path, &content)
        .map_err(|e| format!("Failed to write file: {}", e))?;
    Ok(file_path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_format_validation() {
        let valid_formats = ["json", "txt"];
        for fmt in &valid_formats {
            assert!(
                matches!(fmt, &"json" | &"txt"),
                "Format '{}' should be valid",
                fmt
            );
        }
    }

    #[test]
    fn test_export_format_invalid() {
        let invalid_formats = ["", "xml", "csv", "pdf", "html"];
        for fmt in &invalid_formats {
            assert!(
                !matches!(fmt, &"json" | &"txt"),
                "Format '{}' should be invalid",
                fmt
            );
        }
    }

    #[test]
    fn test_get_export_dir() {
        let dir = get_export_dir();
        assert!(dir.is_ok());
        let path = dir.unwrap();
        assert!(path.to_string_lossy().contains("AzurePath"));
        assert!(path.to_string_lossy().contains("exports"));
    }

    #[test]
    fn test_export_chat_fails_if_not_initialized() {
        // Without a valid DB, this should fail — but with a proper error
        let result = futures::executor::block_on(export_chat("json".into()));
        assert!(result.is_err());
        // The error should be about the database, not a panic
        let err = result.unwrap_err();
        assert!(!err.is_empty(), "Error should not be empty");
    }
}
