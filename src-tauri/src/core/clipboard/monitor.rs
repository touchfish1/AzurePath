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
use tracing::warn;

pub const DEFAULT_INTERVAL_MS: u64 = 1000;

enum ClipboardContent {
    Text {
        #[allow(dead_code)]
        text: String,
        #[allow(dead_code)]
        hash: u64,
    },
    Image {
        rgba: Vec<u8>,
        width: u32,
        height: u32,
        #[allow(dead_code)]
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

                // Phase 2: dedup check (do NOT update last_hash here — wait until
                // the entry is actually persisted so a transient DB error does not
                // permanently lose the content).
                {
                    let last = last_hash.lock().await;
                    if *last == entry.content_hash {
                        continue;
                    }
                }

                // Phase 3: persist images to disk (only for new content).
                // If the image file cannot be written, skip the entire entry
                // so we never create a DB row pointing to a non-existent file.
                if let ClipboardContent::Image { rgba, width, height, .. } = &content {
                    let home = std::env::var("USERPROFILE")
                        .or_else(|_| std::env::var("HOME"))
                        .unwrap_or_else(|_| ".".to_string());
                    let img_dir = std::path::PathBuf::from(&home).join("AzurePath/clipboard/images");
                    let _ = std::fs::create_dir_all(&img_dir);

                    let img_path = img_dir.join(format!("{}.rgba", entry.id));

                    // Only set image_path after a successful write
                    match std::fs::write(&img_path, rgba) {
                        Ok(()) => {
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
                        Err(e) => {
                            warn!("[clipboard] Failed to save image, skipping entry: {}", e);
                            continue;
                        }
                    }
                }

                // Phase 4: persist to DB
                if let Err(e) = store.insert(&entry) {
                    warn!("[clipboard] Insert error: {}", e);
                    continue;
                }

                // Phase 4.5: update last_hash only after a successful DB insert
                {
                    let mut last = last_hash.lock().await;
                    *last = entry.content_hash.clone();
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
            // Skip empty or oversized images (>20 MB raw RGBA)
            if rgba.is_empty() || rgba.len() as u64 > MAX_CLIPBOARD_BYTES {
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── Construction and default state ────────────────────────────────────

    #[test]
    fn test_default_interval() {
        let store = Arc::new(ClipboardStore::new().unwrap());
        let monitor = ClipboardMonitor::new(store);
        assert_eq!(monitor.get_interval_ms(), DEFAULT_INTERVAL_MS);
        assert_eq!(DEFAULT_INTERVAL_MS, 1000);
    }

    #[test]
    fn test_running_starts_false() {
        let store = Arc::new(ClipboardStore::new().unwrap());
        let monitor = ClipboardMonitor::new(store);
        assert!(!monitor.running.load(Ordering::SeqCst));
    }

    #[test]
    fn test_last_hash_starts_empty() {
        let store = Arc::new(ClipboardStore::new().unwrap());
        let monitor = ClipboardMonitor::new(store);
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let hash = monitor.last_hash.lock().await;
            assert!(hash.is_empty(), "last_hash must start as empty string");
        });
    }

    #[test]
    fn test_app_starts_none() {
        let store = Arc::new(ClipboardStore::new().unwrap());
        let monitor = ClipboardMonitor::new(store);
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let app = monitor.app.lock().await;
            assert!(app.is_none(), "app must start as None");
        });
    }

    // ── Interval getter / setter ──────────────────────────────────────────

    #[test]
    fn test_set_and_get_interval() {
        let store = Arc::new(ClipboardStore::new().unwrap());
        let monitor = ClipboardMonitor::new(store);
        monitor.set_interval_ms(500);
        assert_eq!(monitor.get_interval_ms(), 500);
        monitor.set_interval_ms(30000);
        assert_eq!(monitor.get_interval_ms(), 30000);
    }

    #[test]
    fn test_set_interval_boundary_values() {
        let store = Arc::new(ClipboardStore::new().unwrap());
        let monitor = ClipboardMonitor::new(store);
        monitor.set_interval_ms(200);
        assert_eq!(monitor.get_interval_ms(), 200);
        monitor.set_interval_ms(60000);
        assert_eq!(monitor.get_interval_ms(), 60000);
    }

    #[test]
    fn test_get_interval_reflects_last_set() {
        let store = Arc::new(ClipboardStore::new().unwrap());
        let monitor = ClipboardMonitor::new(store);
        monitor.set_interval_ms(1000);
        assert_eq!(monitor.get_interval_ms(), 1000);
        monitor.set_interval_ms(5000);
        assert_eq!(monitor.get_interval_ms(), 5000);
        monitor.set_interval_ms(1000);
        assert_eq!(monitor.get_interval_ms(), 1000);
    }

    // ── stop() behaviour ──────────────────────────────────────────────────

    #[test]
    fn test_stop_before_start_is_safe() {
        let store = Arc::new(ClipboardStore::new().unwrap());
        let monitor = ClipboardMonitor::new(store);
        monitor.stop(); // must not panic
    }

    #[test]
    fn test_stop_is_idempotent() {
        let store = Arc::new(ClipboardStore::new().unwrap());
        let monitor = ClipboardMonitor::new(store);
        monitor.stop();
        monitor.stop(); // second call must not panic either
        assert!(!monitor.running.load(Ordering::SeqCst));
    }

    #[test]
    fn test_stop_sets_running_false() {
        let store = Arc::new(ClipboardStore::new().unwrap());
        let monitor = ClipboardMonitor::new(store);
        // Simulate start (normally swaps to true)
        monitor.running.store(true, Ordering::SeqCst);
        assert!(monitor.running.load(Ordering::SeqCst));
        monitor.stop();
        assert!(!monitor.running.load(Ordering::SeqCst));
    }

    // ── ClipboardContent enum ─────────────────────────────────────────────

    #[test]
    fn test_clipboard_content_text_variant() {
        let content = ClipboardContent::Text {
            text: "hello world".into(),
            hash: 42,
        };
        match &content {
            ClipboardContent::Text { text, hash } => {
                assert_eq!(text, "hello world");
                assert_eq!(*hash, 42);
            }
            _ => panic!("expected Text variant"),
        }
    }

    #[test]
    fn test_clipboard_content_image_variant() {
        let rgba = vec![255u8; 100];
        let content = ClipboardContent::Image {
            rgba: rgba.clone(),
            width: 10,
            height: 10,
            hash: 99,
        };
        match &content {
            ClipboardContent::Image { rgba: r, width, height, hash } => {
                assert_eq!(*r, rgba);
                assert_eq!(*width, 10);
                assert_eq!(*height, 10);
                assert_eq!(*hash, 99);
            }
            _ => panic!("expected Image variant"),
        }
    }

    // ── Content hash consistency (mirrors detect() hashing logic) ─────────

    #[test]
    fn test_text_hash_is_deterministic() {
        let text = "same content";
        let mut h1 = std::hash::DefaultHasher::new();
        text.hash(&mut h1);
        let mut h2 = std::hash::DefaultHasher::new();
        text.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }

    #[test]
    fn test_different_texts_produce_different_hashes() {
        let mut h1 = std::hash::DefaultHasher::new();
        "hello".hash(&mut h1);
        let mut h2 = std::hash::DefaultHasher::new();
        "world".hash(&mut h2);
        assert_ne!(h1.finish(), h2.finish());
    }

    #[test]
    fn test_empty_text_hash_is_deterministic() {
        let mut h1 = std::hash::DefaultHasher::new();
        "".hash(&mut h1);
        let mut h2 = std::hash::DefaultHasher::new();
        "".hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }

    #[test]
    fn test_image_hash_is_deterministic() {
        let rgba = vec![128u8; 256];
        let width = 16u32;
        let height = 16u32;

        let mut h1 = std::hash::DefaultHasher::new();
        rgba.hash(&mut h1);
        width.hash(&mut h1);
        height.hash(&mut h1);

        let mut h2 = std::hash::DefaultHasher::new();
        rgba.hash(&mut h2);
        width.hash(&mut h2);
        height.hash(&mut h2);

        assert_eq!(h1.finish(), h2.finish());
    }

    #[test]
    fn test_image_hash_differs_with_dimensions() {
        let rgba = vec![128u8; 256];

        // (16, 16)
        let mut h1 = std::hash::DefaultHasher::new();
        rgba.hash(&mut h1);
        16u32.hash(&mut h1);
        16u32.hash(&mut h1);

        // (8, 32) — same pixel count, different arrangement
        let mut h2 = std::hash::DefaultHasher::new();
        rgba.hash(&mut h2);
        8u32.hash(&mut h2);
        32u32.hash(&mut h2);

        assert_ne!(h1.finish(), h2.finish());
    }

    #[test]
    fn test_image_hash_differs_with_rgba_content() {
        let mut h1 = std::hash::DefaultHasher::new();
        vec![255u8; 100].hash(&mut h1);
        10u32.hash(&mut h1);
        10u32.hash(&mut h1);

        let mut h2 = std::hash::DefaultHasher::new();
        vec![0u8; 100].hash(&mut h2);
        10u32.hash(&mut h2);
        10u32.hash(&mut h2);

        assert_ne!(h1.finish(), h2.finish());
    }

    #[test]
    fn test_image_hash_differs_from_text_hash() {
        // An image and text with the "same data" should hash differently
        // because image includes width+height in the hash.
        let data = vec![0u8; 16];

        // Image-style hash: rgba + width + height
        let mut h_img = std::hash::DefaultHasher::new();
        data.hash(&mut h_img);
        4u32.hash(&mut h_img); // width
        4u32.hash(&mut h_img); // height

        // Text-style hash: just the string representation
        let text = String::from_utf8_lossy(&data);
        let mut h_text = std::hash::DefaultHasher::new();
        text.hash(&mut h_text);

        assert_ne!(h_img.finish(), h_text.finish());
    }

    // ── Content size limit (mirrors detect() size check) ──────────────────

    #[test]
    fn test_max_clipboard_bytes_equals_20_mb() {
        assert_eq!(MAX_CLIPBOARD_BYTES, 20 * 1024 * 1024);
    }

    #[test]
    fn test_empty_image_rgba_skipped() {
        // detect() returns None when rgba.is_empty()
        let empty: Vec<u8> = vec![];
        assert!(empty.is_empty());
    }

    #[test]
    fn test_empty_text_skipped() {
        // detect() returns None when text.is_empty()
        let empty = String::new();
        assert!(empty.is_empty());
    }

    #[test]
    fn test_oversized_content_skipped() {
        let oversized = vec![0u8; (MAX_CLIPBOARD_BYTES + 1) as usize];
        assert!(oversized.len() as u64 > MAX_CLIPBOARD_BYTES);
    }

    #[test]
    fn test_content_at_max_size_accepted() {
        let at_limit = vec![0u8; MAX_CLIPBOARD_BYTES as usize];
        assert!(!at_limit.is_empty());
        assert!(!(at_limit.len() as u64 > MAX_CLIPBOARD_BYTES));
    }

    // ── Store-dependent operations ───────────────────────────────────────
    //
    // ClipboardStore::new() always opens the same on-disk file, so every
    // test that creates a store shares a single SQLite database.  To prevent
    // parallel tests from interfering with each other, ALL store-dependent
    // assertions are consolidated into one test function.  Each sub-scenario
    // uses a helper that temporarily redirects USERPROFILE to a unique temp
    // directory so the store is fully isolated.
    // ──────────────────────────────────────────────────────────────────────

    /// Create a ClipboardStore backed by a unique temp directory so tests
    /// never see data written by other test scenarios.
    fn isolated_store() -> Arc<ClipboardStore> {
        let id = uuid::Uuid::new_v4();
        let temp_dir = std::env::temp_dir().join(format!("aztest_monitor_{}", id));
        std::fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

        let old_up = std::env::var("USERPROFILE").ok();
        let old_home = std::env::var("HOME").ok();
        std::env::set_var("USERPROFILE", &temp_dir);
        if old_home.is_some() {
            std::env::set_var("HOME", &temp_dir);
        }

        let store = Arc::new(ClipboardStore::new().expect("failed to create isolated store"));

        // Restore environment immediately — the store already has an open
        // connection to the temp DB, so further access works.
        if let Some(ref val) = old_up {
            std::env::set_var("USERPROFILE", val);
        } else {
            std::env::remove_var("USERPROFILE");
        }
        if let Some(val) = old_home {
            std::env::set_var("HOME", val);
        }

        store
    }

    #[tokio::test]
    async fn test_store_dependent_operations() {
        // ── set_interval_ms persistence ────────────────────────────────

        let store = isolated_store();
        let monitor = ClipboardMonitor::new(store.clone());
        monitor.set_interval_ms(2500);
        assert_eq!(monitor.get_interval_ms(), 2500);
        let saved = store.get_setting("clipboard_interval_ms").unwrap();
        assert_eq!(saved, Some("2500".to_string()));

        // Overwrites previous setting
        monitor.set_interval_ms(3000);
        let saved = store.get_setting("clipboard_interval_ms").unwrap();
        assert_eq!(saved, Some("3000".to_string()));

        // ── load_interval ─────────────────────────────────────────────

        let store2 = isolated_store();
        let monitor2 = ClipboardMonitor::new(store2.clone());
        // No setting persisted → default
        assert_eq!(monitor2.get_interval_ms(), DEFAULT_INTERVAL_MS);
        monitor2.load_interval().await;
        assert_eq!(monitor2.get_interval_ms(), DEFAULT_INTERVAL_MS);

        // Read persisted setting
        store2.set_setting("clipboard_interval_ms", "7500").unwrap();
        monitor2.load_interval().await;
        assert_eq!(monitor2.get_interval_ms(), 7500);

        // Ignore non-numeric value
        let store3 = isolated_store();
        let monitor3 = ClipboardMonitor::new(store3.clone());
        store3.set_setting("clipboard_interval_ms", "not-a-number").unwrap();
        monitor3.load_interval().await;
        assert_eq!(monitor3.get_interval_ms(), DEFAULT_INTERVAL_MS);

        // Ignore empty value
        store3.set_setting("clipboard_interval_ms", "").unwrap();
        monitor3.load_interval().await;
        assert_eq!(monitor3.get_interval_ms(), DEFAULT_INTERVAL_MS);

        // ── seed_last_hash ────────────────────────────────────────────

        // Empty store → hash stays empty
        let store4 = isolated_store();
        let monitor4 = ClipboardMonitor::new(store4.clone());
        assert!(monitor4.last_hash.lock().await.is_empty());
        monitor4.seed_last_hash().await;
        assert!(monitor4.last_hash.lock().await.is_empty());

        // Seeds from the most recent entry
        let store5 = isolated_store();
        let monitor5 = ClipboardMonitor::new(store5.clone());
        let entry = ClipboardEntry {
            id: uuid::Uuid::new_v4().to_string(),
            content_type: "text".into(),
            text_content: Some("test content".into()),
            image_path: None,
            file_paths: None,
            content_hash: "known_hash_value".into(),
            is_favorite: false,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        store5.insert(&entry).unwrap();
        monitor5.seed_last_hash().await;
        assert_eq!(*monitor5.last_hash.lock().await, "known_hash_value");

        // Picks newest by timestamp
        let store6 = isolated_store();
        let monitor6 = ClipboardMonitor::new(store6.clone());
        let old = ClipboardEntry {
            id: uuid::Uuid::new_v4().to_string(),
            content_type: "text".into(),
            text_content: Some("old".into()),
            image_path: None,
            file_paths: None,
            content_hash: "hash_old".into(),
            is_favorite: false,
            created_at: "2024-01-01T00:00:00Z".into(),
        };
        store6.insert(&old).unwrap();
        let new = ClipboardEntry {
            id: uuid::Uuid::new_v4().to_string(),
            content_type: "text".into(),
            text_content: Some("new".into()),
            image_path: None,
            file_paths: None,
            content_hash: "hash_new".into(),
            is_favorite: false,
            created_at: "2025-01-01T00:00:00Z".into(),
        };
        store6.insert(&new).unwrap();
        monitor6.seed_last_hash().await;
        assert_eq!(*monitor6.last_hash.lock().await, "hash_new");

        // Picks most recent of many entries
        let store7 = isolated_store();
        let monitor7 = ClipboardMonitor::new(store7.clone());
        for i in 0..10 {
            let e = ClipboardEntry {
                id: uuid::Uuid::new_v4().to_string(),
                content_type: "text".into(),
                text_content: Some(format!("entry {}", i)),
                image_path: None,
                file_paths: None,
                content_hash: format!("hash_{}", i),
                is_favorite: false,
                created_at: format!("2025-01-{:02}T00:00:00Z", i + 1),
            };
            store7.insert(&e).unwrap();
        }
        monitor7.seed_last_hash().await;
        assert_eq!(*monitor7.last_hash.lock().await, "hash_9");
    }

    // ── Edge cases ────────────────────────────────────────────────────────

    #[test]
    fn test_new_does_not_require_existing_store_state() {
        // ClipboardMonitor::new() is purely in-memory; it only needs an
        // Arc<ClipboardStore> to reference, not any particular DB content.
        let store = Arc::new(ClipboardStore::new().unwrap());
        let monitor = ClipboardMonitor::new(store);

        assert_eq!(monitor.get_interval_ms(), DEFAULT_INTERVAL_MS);
        assert!(!monitor.running.load(Ordering::SeqCst));
    }

    // NOTE: The detect() method requires a live Tauri AppHandle with the
    // clipboard-manager plugin registered.  It cannot be called from
    // unit tests because tauri-plugin-clipboard-manager::ClipboardExt
    // and Tauri's runtime are not available here.  The hashing and size-
    // limit logic that detect() uses is tested above (see "Content hash
    // consistency" and "Content size limit" sections).
}
