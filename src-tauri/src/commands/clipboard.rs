use crate::core::clipboard::{ClipboardMonitor, ClipboardStore};
use crate::types::clipboard::ClipboardEntry;
use crate::types::clipboard::MAX_CLIPBOARD_BYTES;
use std::sync::Arc;
use std::sync::OnceLock;
use tauri::AppHandle;
use tauri::Emitter;
use tauri::image::Image;
use tauri_plugin_clipboard_manager::ClipboardExt;

static CLIPBOARD_STORE: OnceLock<Arc<ClipboardStore>> = OnceLock::new();
static CLIPBOARD_MONITOR: OnceLock<ClipboardMonitor> = OnceLock::new();

pub fn clipboard_store() -> Option<&'static Arc<ClipboardStore>> {
    CLIPBOARD_STORE.get()
}

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

    println!("[clipboard] Monitor started");
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
pub async fn clipboard_delete(id: String) -> Result<(), String> {
    let store = CLIPBOARD_STORE.get().ok_or("Clipboard not initialized")?;
    store.delete(&id)
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
            eprintln!("[clipboard] write_files not supported by clipboard plugin; skipping");
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
    println!("[clipboard] Interval set to {}ms", ms);
    Ok(())
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
                eprintln!("[clipboard] Skipping synced entry exceeding size limit");
                continue;
            }
            if let Err(e) = store.insert(entry) {
                eprintln!("[clipboard] Failed to save synced entry: {}", e);
            }
        }
        let _ = app.emit("clipboard:synced", entries);
    }
}
