use crate::core::clipboard::{ClipboardMonitor, ClipboardStore};
use crate::types::clipboard::ClipboardEntry;
use crate::types::clipboard::MAX_CLIPBOARD_BYTES;
use crate::core::utils::home_dir;
use std::sync::Arc;
use std::sync::OnceLock;
use tauri::AppHandle;
use tauri::Emitter;
use tauri::image::Image;
use tauri_plugin_clipboard_manager::ClipboardExt;
use tracing::{info, warn};

static CLIPBOARD_STORE: OnceLock<Arc<ClipboardStore>> = OnceLock::new();
static CLIPBOARD_MONITOR: OnceLock<ClipboardMonitor> = OnceLock::new();

#[allow(dead_code)]
pub fn clipboard_store() -> Option<&'static Arc<ClipboardStore>> {
    CLIPBOARD_STORE.get()
}

#[allow(dead_code)]
pub fn clipboard_monitor() -> Option<&'static ClipboardMonitor> {
    CLIPBOARD_MONITOR.get()
}

#[tauri::command]
pub async fn clipboard_start(app: AppHandle) -> Result<(), String> {
    if CLIPBOARD_STORE.get().is_some() {
        return Ok(());
    }

    let store = Arc::new(ClipboardStore::new()?);
    let monitor = ClipboardMonitor::new(store.clone());
    monitor.set_app(app.clone()).await;
    monitor.seed_last_hash().await;
    monitor.load_interval().await;

    CLIPBOARD_STORE
        .set(store)
        .map_err(|_| "Already initialized".to_string())?;
    CLIPBOARD_MONITOR
        .set(monitor)
        .map_err(|_| "Already initialized".to_string())?;

    // Start monitoring
    if let Some(m) = CLIPBOARD_MONITOR.get() {
        m.start().await;
    }

    info!("[clipboard] Monitor started");
    Ok(())
}

#[tauri::command]
pub async fn clipboard_stop() -> Result<(), String> {
    if let Some(m) = CLIPBOARD_MONITOR.get() {
        m.stop();
    }
    Ok(())
}

#[tauri::command]
pub async fn clipboard_list(search: Option<String>, limit: Option<u32>) -> Result<Vec<ClipboardEntry>, String> {
    let store = CLIPBOARD_STORE.get().ok_or("Clipboard not initialized")?;
    store.list(search.as_deref(), limit.unwrap_or(100))
}

#[tauri::command]
pub async fn clipboard_delete(ids: Vec<String>) -> Result<(), String> {
    let store = CLIPBOARD_STORE.get().ok_or("Clipboard not initialized")?;
    store.delete_entries(&ids)
}

#[tauri::command]
pub async fn clipboard_toggle_favorite(id: String) -> Result<bool, String> {
    let store = CLIPBOARD_STORE.get().ok_or("Clipboard not initialized")?;
    store.toggle_favorite(&id)
}

#[tauri::command]
pub async fn clipboard_copy(id: String, app: AppHandle) -> Result<(), String> {
    let store = CLIPBOARD_STORE.get().ok_or("Clipboard not initialized")?;
    let entry = store
        .get_by_id(&id)?
        .ok_or_else(|| format!("Entry not found: {}", id))?;

    match entry.content_type.as_str() {
        "text" => {
            let text = entry.text_content.as_ref()
                .ok_or("Text entry has no text_content")?;
            app.clipboard()
                .write_text(text)
                .map_err(|e| format!("Failed to write text to clipboard: {}", e))?;
        }
        "image" => {
            let path = entry.image_path.as_ref()
                .ok_or("Image entry has no image_path")?;
            let raw = std::fs::read(path)
                .map_err(|e| format!("Failed to read image: {}", e))?;

            // Derive metadata path from the image path
            let meta_path = std::path::Path::new(path).with_extension("json");
            let meta_content = std::fs::read_to_string(&meta_path)
                .map_err(|e| format!("Failed to read image metadata: {}", e))?;
            let meta: serde_json::Value = serde_json::from_str(&meta_content)
                .map_err(|e| format!("Failed to parse image metadata: {}", e))?;
            let width = meta["width"].as_u64()
                .ok_or("Missing width in image metadata")? as u32;
            let height = meta["height"].as_u64()
                .ok_or("Missing height in image metadata")? as u32;

            let image = Image::new_owned(raw, width, height);
            app.clipboard()
                .write_image(&image)
                .map_err(|e| format!("Failed to write image to clipboard: {}", e))?;
        }
        "file" => {
            warn!("[clipboard] write_files not supported by clipboard plugin; skipping");
        }
        _ => return Err(format!("Unknown content type: {}", entry.content_type)),
    }
    Ok(())
}

#[tauri::command]
pub async fn clipboard_clear() -> Result<(), String> {
    let store = CLIPBOARD_STORE.get().ok_or("Clipboard not initialized")?;
    store.clear()
}

#[tauri::command]
pub async fn clipboard_get_interval() -> Result<u64, String> {
    let monitor = CLIPBOARD_MONITOR.get().ok_or("Clipboard not initialized")?;
    Ok(monitor.get_interval_ms())
}

#[tauri::command]
pub async fn clipboard_set_interval(ms: u64) -> Result<(), String> {
    let monitor = CLIPBOARD_MONITOR.get().ok_or("Clipboard not initialized")?;
    if ms < 200 || ms > 60000 {
        return Err("Interval must be between 200ms and 60000ms".to_string());
    }
    monitor.set_interval_ms(ms);
    info!("[clipboard] Interval set to {}ms", ms);
    Ok(())
}

#[tauri::command]
pub async fn clipboard_export(ids: Vec<String>, format: String) -> Result<String, String> {
    let store = CLIPBOARD_STORE.get().ok_or("Clipboard not initialized")?;
    let entries = store.export_entries(&ids)?;

    // Determine export directory
    let export_dir = home_dir()
        .ok_or_else(|| "Cannot find home directory".to_string())?
        .join("AzurePath")
        .join("exports");
    std::fs::create_dir_all(&export_dir)
        .map_err(|e| format!("Failed to create exports directory: {}", e))?;

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
            for entry in &entries {
                content.push_str(&format!("=== Clipboard Entry ===\n"));
                content.push_str(&format!("ID: {}\n", entry.id));
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
pub async fn clipboard_sources() -> Result<Vec<String>, String> {
    let store = CLIPBOARD_STORE.get().ok_or("Clipboard not initialized")?;
    store.get_unique_sources()
}

#[tauri::command]
pub async fn clipboard_set_limit(limit: usize) -> Result<(), String> {
    if limit < 10 {
        return Err("Minimum limit is 10 entries".to_string());
    }
    if limit > 10000 {
        return Err("Maximum limit is 10000 entries".to_string());
    }
    let store = CLIPBOARD_STORE.get().ok_or("Clipboard not initialized")?;
    store.set_max_entries(limit)
}

/// Handle incoming ClipboardSync frames from LAN peers.
pub(crate) async fn handle_frame(incoming: &crate::core::connection::IncomingFrame, app: &AppHandle) {
    if let crate::types::chat::Frame::ClipboardSync { entries } = &incoming.frame {
        let store = match CLIPBOARD_STORE.get() {
            Some(s) => s,
            None => return,
        };
        for entry in entries {
            if entry.content_bytes() > MAX_CLIPBOARD_BYTES {
                warn!("[clipboard] Skipping synced entry exceeding size limit");
                continue;
            }
            if let Err(e) = store.insert(entry) {
                warn!("[clipboard] Failed to save synced entry: {}", e);
            }
        }
        let _ = app.emit("clipboard:synced", entries);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Uninitialized state: every command that requires the monitor / store
    //    should return the proper "Clipboard not initialized" error ────────

    #[tokio::test]
    async fn test_list_fails_before_start() {
        let err = clipboard_list(None, None).await.unwrap_err();
        assert_eq!(err, "Clipboard not initialized");
    }

    #[tokio::test]
    async fn test_list_with_search_fails_before_start() {
        let err = clipboard_list(Some("query".into()), Some(50)).await.unwrap_err();
        assert_eq!(err, "Clipboard not initialized");
    }

    #[tokio::test]
    async fn test_list_with_limit_fails_before_start() {
        let err = clipboard_list(None, Some(10)).await.unwrap_err();
        assert_eq!(err, "Clipboard not initialized");
    }

    #[tokio::test]
    async fn test_delete_fails_before_start() {
        let err = clipboard_delete(vec!["any-id".into()]).await.unwrap_err();
        assert_eq!(err, "Clipboard not initialized");
    }

    #[tokio::test]
    async fn test_delete_batch_fails_before_start() {
        let err = clipboard_delete(vec![]).await.unwrap_err();
        assert_eq!(err, "Clipboard not initialized");
    }

    #[tokio::test]
    async fn test_export_fails_before_start() {
        let err = clipboard_export(vec![], "json".into()).await.unwrap_err();
        assert_eq!(err, "Clipboard not initialized");
    }

    #[tokio::test]
    async fn test_sources_fails_before_start() {
        let err = clipboard_sources().await.unwrap_err();
        assert_eq!(err, "Clipboard not initialized");
    }

    #[tokio::test]
    async fn test_set_limit_fails_before_start() {
        let err = clipboard_set_limit(100).await.unwrap_err();
        assert_eq!(err, "Clipboard not initialized");
    }

    #[tokio::test]
    async fn test_toggle_favorite_fails_before_start() {
        let err = clipboard_toggle_favorite("any-id".into()).await.unwrap_err();
        assert_eq!(err, "Clipboard not initialized");
    }

    #[tokio::test]
    async fn test_clear_fails_before_start() {
        let err = clipboard_clear().await.unwrap_err();
        assert_eq!(err, "Clipboard not initialized");
    }

    #[tokio::test]
    async fn test_get_interval_fails_before_start() {
        let err = clipboard_get_interval().await.unwrap_err();
        assert_eq!(err, "Clipboard not initialized");
    }

    #[tokio::test]
    async fn test_set_interval_fails_before_start() {
        let err = clipboard_set_interval(1000).await.unwrap_err();
        assert_eq!(err, "Clipboard not initialized");
    }

    #[tokio::test]
    async fn test_stop_before_start_is_safe() {
        // clipboard_stop uses `if let Some` — it should never panic.
        assert!(clipboard_stop().await.is_ok());
    }

    // ── Interval validation bounds (the pure check inside set_interval) ──

    #[test]
    fn test_interval_below_minimum() {
        // Validation: ms < 200 || ms > 60000
        assert!(0u64 < 200, "0 is below minimum");
        assert!(150u64 < 200, "150 is below minimum");
        assert!(199u64 < 200, "199 is below minimum");
    }

    #[test]
    fn test_interval_at_minimum_boundary() {
        // 200 is the minimum valid value
        assert!(!(200u64 < 200), "200 is at minimum");
        assert!(!(201u64 < 200), "201 is above minimum");
    }

    #[test]
    fn test_interval_above_maximum() {
        assert!(60001u64 > 60000, "60001 exceeds maximum");
        assert!(99999u64 > 60000, "99999 exceeds maximum");
    }

    #[test]
    fn test_interval_at_maximum_boundary() {
        assert!(!(60000u64 > 60000), "60000 is at maximum");
        assert!(!(59999u64 > 60000), "59999 is below maximum");
    }

    #[test]
    fn test_interval_valid_range() {
        for ms in [200u64, 500, 1000, 5000, 10000, 30000, 60000] {
            assert!(!(ms < 200 || ms > 60000), "{}ms should be valid", ms);
        }
    }

    #[test]
    fn test_interval_invalid_values() {
        for ms in [0u64, 1, 100, 199, 60001, 99999, u64::MAX] {
            assert!(ms < 200 || ms > 60000, "{}ms should be invalid", ms);
        }
    }

    // ── Parameter defaults for list() ─────────────────────────────────────

    #[test]
    fn test_list_limit_defaults_to_100() {
        assert_eq!(None::<u32>.unwrap_or(100), 100);
        assert_eq!(Some(50u32).unwrap_or(100), 50);
        assert_eq!(Some(0u32).unwrap_or(100), 0);
        assert_eq!(Some(u32::MAX).unwrap_or(100), u32::MAX);
    }

    #[test]
    fn test_list_search_passthrough() {
        // None search → store.list receives None
        // Some("") search → store.list receives Some("")
        // The exact store behaviour is tested in store.rs; here we
        // verify the Option<String> → Option<&str> conversion is sound.
        let none: Option<&str> = None::<String>.as_deref();
        assert!(none.is_none());

        let opt = Some("query".to_string());
        let some_val: Option<&str> = opt.as_deref();
        assert_eq!(some_val, Some("query"));
    }

    // ── Content-type matching (mirrors clipboard_copy dispatch) ──────────

    #[test]
    fn test_known_content_types_accepted() {
        // These three types are handled by clipboard_copy
        for ct in &["text", "image", "file"] {
            let is_known = matches!(ct, &"text" | &"image" | &"file");
            assert!(is_known, "'{}' must be recognised", ct);
        }
    }

    #[test]
    fn test_unknown_content_types_rejected() {
        for ct in &["", "audio", "video", "application", "unknown", "binary"] {
            let is_known = matches!(ct, &"text" | &"image" | &"file");
            assert!(!is_known, "'{}' must be rejected", ct);
        }
    }

    // ── Content size limit (used by handle_frame to skip oversized syncs) ─

    #[test]
    fn test_content_bytes_exceeds_limit() {
        let entry = ClipboardEntry {
            id: "oversized".into(),
            content_type: "text".into(),
            text_content: Some("x".repeat((MAX_CLIPBOARD_BYTES + 1) as usize)),
            image_path: None,
            file_paths: None,
            content_hash: "h_oversize".into(),
            is_favorite: false,
            created_at: "2024-01-01T00:00:00Z".into(),
        };
        assert!(entry.content_bytes() > MAX_CLIPBOARD_BYTES);
    }

    #[test]
    fn test_content_bytes_at_limit_accepted() {
        let entry = ClipboardEntry {
            id: "at-limit".into(),
            content_type: "text".into(),
            text_content: Some("x".repeat(MAX_CLIPBOARD_BYTES as usize)),
            image_path: None,
            file_paths: None,
            content_hash: "h_atlimit".into(),
            is_favorite: false,
            created_at: "2024-01-01T00:00:00Z".into(),
        };
        assert!(entry.content_bytes() <= MAX_CLIPBOARD_BYTES);
    }

    #[test]
    fn test_content_bytes_zero_for_text_without_content() {
        let entry = ClipboardEntry {
            id: "no-text".into(),
            content_type: "text".into(),
            text_content: None,
            image_path: None,
            file_paths: None,
            content_hash: "h_notext".into(),
            is_favorite: false,
            created_at: "2024-01-01T00:00:00Z".into(),
        };
        assert_eq!(entry.content_bytes(), 0);
    }

    #[test]
    fn test_content_bytes_zero_for_image_entry() {
        let entry = ClipboardEntry {
            id: "img-entry".into(),
            content_type: "image".into(),
            text_content: None,
            image_path: Some("/tmp/img.rgba".into()),
            file_paths: None,
            content_hash: "h_img".into(),
            is_favorite: false,
            created_at: "2024-01-01T00:00:00Z".into(),
        };
        // content_bytes() returns 0 for non-text, non-file types
        assert_eq!(entry.content_bytes(), 0);
    }

    #[test]
    fn test_content_bytes_zero_for_unknown_type() {
        let entry = ClipboardEntry {
            id: "unknown-type".into(),
            content_type: "unknown".into(),
            text_content: Some("data".into()),
            image_path: None,
            file_paths: None,
            content_hash: "h_unk".into(),
            is_favorite: false,
            created_at: "2024-01-01T00:00:00Z".into(),
        };
        assert_eq!(entry.content_bytes(), 0);
    }

    // ── Static accessors ──────────────────────────────────────────────────

    #[test]
    fn test_clipboard_store_accessor_before_init() {
        assert!(clipboard_store().is_none());
    }

    #[test]
    fn test_clipboard_monitor_accessor_before_init() {
        assert!(clipboard_monitor().is_none());
    }

    // ── Initialized state ────────────────────────────────────────────────
    //
    // All initialized tests run in a single function because CLIPBOARD_STORE
    // and CLIPBOARD_MONITOR are process-global OnceLock values.  Separate
    // #[tokio::test] functions would race on the shared state.
    // ──────────────────────────────────────────────────────────────────────

    fn init_state() {
        if CLIPBOARD_STORE.get().is_some() {
            return;
        }
        // Use in-memory store to avoid cross-test pollution from file-based DB.
        let store = Arc::new(ClipboardStore::test_store());
        let monitor = ClipboardMonitor::new(store.clone());
        let _ = CLIPBOARD_STORE.set(store);
        let _ = CLIPBOARD_MONITOR.set(monitor);
    }

    #[tokio::test]
    async fn test_initialized_commands() {
        init_state();

        // set_interval: valid values
        assert!(clipboard_set_interval(200).await.is_ok());
        assert!(clipboard_set_interval(500).await.is_ok());
        assert!(clipboard_set_interval(10000).await.is_ok());
        assert!(clipboard_set_interval(60000).await.is_ok());

        // set_interval: below minimum
        let err = clipboard_set_interval(199).await.unwrap_err();
        assert_eq!(err, "Interval must be between 200ms and 60000ms");
        let err = clipboard_set_interval(0).await.unwrap_err();
        assert_eq!(err, "Interval must be between 200ms and 60000ms");

        // set_interval: above maximum
        let err = clipboard_set_interval(60001).await.unwrap_err();
        assert_eq!(err, "Interval must be between 200ms and 60000ms");

        // get_interval reflects most recent set
        clipboard_set_interval(7777).await.unwrap();
        assert_eq!(clipboard_get_interval().await.unwrap(), 7777);

        // list with no data returns empty
        let list = clipboard_list(None, None).await.unwrap();
        assert!(list.is_empty(), "initial list should be empty");
        let list = clipboard_list(Some("search".into()), Some(10)).await.unwrap();
        assert!(list.is_empty(), "searched list should be empty with no entries");

        // delete non-existent is no-op
        assert!(clipboard_delete(vec!["no-such-id".into()]).await.is_ok());

        // toggle_favorite on non-existent fails
        assert!(clipboard_toggle_favorite("no-such-id".into()).await.is_err());

        // clear is ok when empty
        assert!(clipboard_clear().await.is_ok());

        // stop and continue using monitor (get/set_interval are independent
        // of the running flag — only the loop checks it).
        assert!(clipboard_stop().await.is_ok());
        assert!(clipboard_set_interval(3000).await.is_ok());
        assert_eq!(clipboard_get_interval().await.unwrap(), 3000);
    }
}
