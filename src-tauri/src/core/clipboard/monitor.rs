use crate::core::clipboard::ClipboardStore;
use crate::types::clipboard::ClipboardEntry;
use crate::types::clipboard::MAX_CLIPBOARD_BYTES;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tauri::{AppHandle, Emitter};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tokio::sync::Mutex;
use uuid::Uuid;

pub const DEFAULT_INTERVAL_MS: u64 = 1000;

enum ClipboardContent {
    Text {
        text: String,
        hash: u64,
    },
    Image {
        rgba: Vec<u8>,
        width: u32,
        height: u32,
        hash: u64,
    },
}

pub struct ClipboardMonitor {
    store: Arc<ClipboardStore>,
    running: Arc<AtomicBool>,
    interval_ms: Arc<AtomicU64>,
    last_hash: Arc<Mutex<String>>,
    app: Arc<Mutex<Option<AppHandle>>>,
}

impl ClipboardMonitor {
    pub fn new(store: Arc<ClipboardStore>) -> Self {
        Self {
            store,
            running: Arc::new(AtomicBool::new(false)),
            interval_ms: Arc::new(AtomicU64::new(DEFAULT_INTERVAL_MS)),
            last_hash: Arc::new(Mutex::new(String::new())),
            app: Arc::new(Mutex::new(None)),
        }
    }

    /// Load the persisted interval from settings, or use default.
    pub async fn load_interval(&self) {
        let saved = self.store.get_setting("clipboard_interval_ms").ok().flatten();
        if let Some(val) = saved {
            if let Ok(ms) = val.parse::<u64>() {
                self.interval_ms.store(ms, Ordering::Release);
            }
        }
    }

    pub fn get_interval_ms(&self) -> u64 {
        self.interval_ms.load(Ordering::Acquire)
    }

    /// Update interval at runtime and persist to settings.
    pub fn set_interval_ms(&self, ms: u64) {
        self.interval_ms.store(ms, Ordering::Release);
        let _ = self.store.set_setting("clipboard_interval_ms", &ms.to_string());
    }

    pub async fn set_app(&self, app: AppHandle) {
        *self.app.lock().await = Some(app);
    }

    /// Seed last_hash from the most recent stored entry to avoid
    /// re-recording the same content on restart.
    pub async fn seed_last_hash(&self) {
        if let Ok(entries) = self.store.list(None, 1) {
            if let Some(entry) = entries.first() {
                let mut last = self.last_hash.lock().await;
                *last = entry.content_hash.clone();
            }
        }
    }

    pub async fn start(&self) {
        if self.running.swap(true, Ordering::SeqCst) {
            return;
        }

        let running = self.running.clone();
        let interval_ms = self.interval_ms.clone();
        let last_hash = self.last_hash.clone();
        let store = self.store.clone();
        let app_lock = self.app.clone();

        tokio::spawn(async move {
            while running.load(Ordering::SeqCst) {
                let ms = interval_ms.load(Ordering::Acquire);
                tokio::time::sleep(tokio::time::Duration::from_millis(ms)).await;

                let app_handle = {
                    let guard = app_lock.lock().await;
                    match guard.as_ref() {
                        Some(a) => a.clone(),
                        None => continue,
                    }
                };

                // Phase 1: detect clipboard content (no disk writes yet)
                let (content, mut entry) = match Self::detect(&app_handle).await {
                    Some(result) => result,
                    None => continue,
                };

                // Phase 2: dedup check
                {
                    let mut last = last_hash.lock().await;
                    if *last == entry.content_hash {
                        continue;
                    }
                    *last = entry.content_hash.clone();
                }

                // Phase 3: persist images to disk (only for new content)
                if let ClipboardContent::Image { rgba, width, height, .. } = &content {
                    let home = std::env::var("USERPROFILE")
                        .or_else(|_| std::env::var("HOME"))
                        .unwrap_or_else(|_| ".".to_string());
                    let img_dir = std::path::PathBuf::from(&home).join("AzurePath/clipboard/images");
                    let _ = std::fs::create_dir_all(&img_dir);

                    let img_path = img_dir.join(format!("{}.rgba", entry.id));
                    if let Err(e) = std::fs::write(&img_path, rgba) {
                        eprintln!("[clipboard] Failed to save image: {}", e);
                    }

                    let meta_path = img_dir.join(format!("{}.json", entry.id));
                    if let Ok(meta) = serde_json::to_string(&serde_json::json!({
                        "width": width,
                        "height": height,
                        "format": "rgba"
                    })) {
                        let _ = std::fs::write(&meta_path, &meta);
                    }

                    entry.image_path = Some(img_path.to_string_lossy().to_string());
                }

                // Phase 4: persist to DB
                if let Err(e) = store.insert(&entry) {
                    eprintln!("[clipboard] Insert error: {}", e);
                    continue;
                }

                // Phase 5: notify frontend
                let _ = app_handle.emit("clipboard:new", &entry);
            }
        });
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Detect clipboard content and compute hash without writing files.
    /// Returns (ClipboardContent, partial ClipboardEntry).
    async fn detect(app: &AppHandle) -> Option<(ClipboardContent, ClipboardEntry)> {
        let now = chrono::Utc::now().to_rfc3339();

        // Try image first (rich content takes priority)
        if let Ok(image) = app.clipboard().read_image() {
            let rgba = image.rgba().to_vec();
            // Skip images larger than 20 MB (width × height × 4 bytes)
            if rgba.len() as u64 > MAX_CLIPBOARD_BYTES {
                return None;
            }
            let mut hasher = std::hash::DefaultHasher::new();
            let width = image.width();
            let height = image.height();
            rgba.hash(&mut hasher);
            width.hash(&mut hasher);
            height.hash(&mut hasher);
            let hash_val = hasher.finish();
            let hash_str = hash_val.to_string();
            let id = Uuid::new_v4().to_string();

            return Some((
                ClipboardContent::Image { rgba, width, height, hash: hash_val },
                ClipboardEntry {
                    id,
                    content_type: "image".to_string(),
                    text_content: None,
                    image_path: None,
                    file_paths: None,
                    content_hash: hash_str,
                    is_favorite: false,
                    created_at: now,
                },
            ));
        }

        // Try text
        if let Ok(text) = app.clipboard().read_text() {
            if text.is_empty() || text.len() as u64 > MAX_CLIPBOARD_BYTES {
                return None;
            }
            let mut hasher = std::hash::DefaultHasher::new();
            text.hash(&mut hasher);
            let hash_val = hasher.finish();
            let id = Uuid::new_v4().to_string();

            return Some((
                ClipboardContent::Text { text: text.clone(), hash: hash_val },
                ClipboardEntry {
                    id,
                    content_type: "text".to_string(),
                    text_content: Some(text),
                    image_path: None,
                    file_paths: None,
                    content_hash: hash_val.to_string(),
                    is_favorite: false,
                    created_at: now,
                },
            ));
        }

        None
    }
}
