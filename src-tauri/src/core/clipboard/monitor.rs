use crate::core::clipboard::ClipboardStore;
use crate::types::clipboard::ClipboardEntry;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct ClipboardMonitor {
    store: Arc<ClipboardStore>,
    running: Arc<AtomicBool>,
    last_hash: Arc<Mutex<String>>,
    app: Arc<Mutex<Option<AppHandle>>>,
}

impl ClipboardMonitor {
    pub fn new(store: Arc<ClipboardStore>) -> Self {
        Self {
            store,
            running: Arc::new(AtomicBool::new(false)),
            last_hash: Arc::new(Mutex::new(String::new())),
            app: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn set_app(&self, app: AppHandle) {
        *self.app.lock().await = Some(app);
    }

    pub async fn start(&self) {
        if self.running.swap(true, Ordering::SeqCst) {
            return;
        }

        let running = self.running.clone();
        let last_hash = self.last_hash.clone();
        let store = self.store.clone();
        let app_lock = self.app.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(1500));
            while running.load(Ordering::SeqCst) {
                interval.tick().await;

                let app_handle = {
                    let guard = app_lock.lock().await;
                    match guard.as_ref() {
                        Some(a) => a.clone(),
                        None => continue,
                    }
                };

                let entry = match Self::read_clipboard(&app_handle).await {
                    Some(e) => e,
                    None => continue,
                };

                let mut last = last_hash.lock().await;
                if *last == entry.content_hash {
                    continue;
                }
                *last = entry.content_hash.clone();
                drop(last);

                if let Err(e) = store.insert(&entry) {
                    eprintln!("[clipboard] Insert error: {}", e);
                    continue;
                }

                let _ = app_handle.emit("clipboard:new", &entry);
            }
        });
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    async fn read_clipboard(app: &AppHandle) -> Option<ClipboardEntry> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut hasher = std::hash::DefaultHasher::new();

        // Try image first (rich content takes priority)
        if let Ok(image) = app.clipboard().read_image() {
            let id = Uuid::new_v4().to_string();
            let home = std::env::var("USERPROFILE")
                .or_else(|_| std::env::var("HOME"))
                .unwrap_or_else(|_| ".".to_string());
            let img_dir = std::path::PathBuf::from(&home).join("AzurePath/clipboard/images");
            let _ = std::fs::create_dir_all(&img_dir);

            // Save raw RGBA data to disk
            let rgba_data = image.rgba();
            let img_path = img_dir.join(format!("{}.rgba", id));
            if let Err(e) = std::fs::write(&img_path, rgba_data) {
                eprintln!("[clipboard] Failed to save image: {}", e);
            }

            // Write metadata sidecar (width, height)
            let meta_path = img_dir.join(format!("{}.json", id));
            if let Ok(meta) = serde_json::to_string(&serde_json::json!({
                "width": image.width(),
                "height": image.height(),
                "format": "rgba"
            })) {
                let _ = std::fs::write(&meta_path, &meta);
            }

            // Compute hash from the raw pixel data
            rgba_data.hash(&mut hasher);
            image.width().hash(&mut hasher);
            image.height().hash(&mut hasher);

            return Some(ClipboardEntry {
                id,
                content_type: "image".to_string(),
                text_content: None,
                image_path: Some(img_path.to_string_lossy().to_string()),
                file_paths: None,
                content_hash: hasher.finish().to_string(),
                is_favorite: false,
                created_at: now,
            });
        }

        // Try text
        if let Ok(text) = app.clipboard().read_text() {
            if text.is_empty() {
                return None;
            }
            let id = Uuid::new_v4().to_string();
            text.hash(&mut hasher);
            return Some(ClipboardEntry {
                id,
                content_type: "text".to_string(),
                text_content: Some(text),
                image_path: None,
                file_paths: None,
                content_hash: hasher.finish().to_string(),
                is_favorite: false,
                created_at: now,
            });
        }

        None
    }
}
