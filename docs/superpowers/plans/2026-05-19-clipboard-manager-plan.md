# 剪贴板管理功能 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a clipboard manager that automatically records clipboard history (text/images/files), persists to SQLite, supports search/favorites/one-click-copy, and syncs across LAN peers.

**Architecture:** Rust backend with `ClipboardMonitor` (polling every 1.5s via tauri-plugin-clipboard-manager) → `ClipboardStore` (SQLite via rusqlite) → Tauri IPC commands → Vue 3 frontend page. LAN sync uses existing ConnectionManager's broadcast with a new Frame variant.

**Tech Stack:** Rust + Tauri 2.0 + rusqlite + tauri-plugin-clipboard-manager + Vue 3 + TypeScript + shadcn-vue + Tailwind CSS

---

## File Structure

```
Create:
  src-tauri/src/types/clipboard.rs          # ClipboardEntry struct
  src-tauri/src/core/clipboard/mod.rs        # Module root
  src-tauri/src/core/clipboard/store.rs      # ClipboardStore (SQLite)
  src-tauri/src/core/clipboard/monitor.rs    # ClipboardMonitor (polling)
  src-tauri/src/commands/clipboard.rs        # Tauri IPC commands
  src/pages/clipboard/Page.vue               # Clipboard UI page

Modify:
  src-tauri/Cargo.toml                       # Add tauri-plugin-clipboard-manager
  src-tauri/src/types/mod.rs                 # Add pub mod clipboard
  src-tauri/src/core/mod.rs                  # Add pub mod clipboard
  src-tauri/src/commands/mod.rs              # Add pub mod clipboard
  src-tauri/src/lib.rs                       # Register clipboard commands
  src-tauri/src/types/chat.rs                # Add ClipboardSync to Frame enum
  src-tauri/src/commands/file_transfer.rs    # Handle ClipboardSync frame
  src-tauri/src/commands/lan.rs              # Start clipboard monitor on init
  src/lib/tauri.ts                           # Add clipboard API + event types
  src/router/index.ts                        # Add /clipboard route
  src/components/layout/Sidebar.vue          # Add nav item
```

## Dependency Graph

```
Task 1 (types + deps) ─┬─→ Task 2 (store) ─→ Task 3 (monitor) ─→ Task 4 (commands) ─┐
                        │                                                            │
                        └─→ Task 5 (LAN sync Frame) ─────────────────────────────────┤
                                                                                      │
              Task 6 (frontend page) ─── (needs API interface, not backend impl) ────┤
                                                                                      │
                                                                              Task 7 (wiring)
```

Independent groups (can be dispatched in parallel by subagent-driven-development orchestrator):
- **Group A**: Task 1 (types + deps) — must go first
- **Group B**: Task 2 (store), Task 5 (LAN Frame changes), Task 6 (frontend page) — parallel after Task 1
- **Group C**: Task 3 (monitor) — after Task 2
- **Group D**: Task 4 (commands) — after Task 3
- **Group E**: Task 7 (wiring) — after Tasks 4+5

---

### Task 1: Add tauri-plugin-clipboard-manager + ClipboardEntry types

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/types/clipboard.rs`
- Modify: `src-tauri/src/types/mod.rs`

- [ ] **Step 1: Add tauri-plugin-clipboard-manager dependency**

Edit `src-tauri/Cargo.toml`, add to `[dependencies]`:
```toml
tauri-plugin-clipboard-manager = "2"
```

- [ ] **Step 2: Create ClipboardEntry type**

Create `src-tauri/src/types/clipboard.rs`:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardEntry {
    pub id: String,
    pub content_type: String,    // "text" | "image" | "file"
    pub text_content: Option<String>,
    pub image_path: Option<String>,
    pub file_paths: Option<Vec<String>>,
    pub content_hash: String,
    pub is_favorite: bool,
    pub created_at: String,
}
```

- [ ] **Step 3: Register clipboard module in types**

Edit `src-tauri/src/types/mod.rs`, add:
```rust
pub mod clipboard;
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/types/clipboard.rs src-tauri/src/types/mod.rs
git commit -m "feat: add tauri-plugin-clipboard-manager dep and ClipboardEntry type"
```

---

### Task 2: ClipboardStore — SQLite CRUD

**Files:**
- Create: `src-tauri/src/core/clipboard/store.rs`
- Create: `src-tauri/src/core/clipboard/mod.rs`
- Modify: `src-tauri/src/core/mod.rs`

- [ ] **Step 1: Create ClipboardStore**

Create `src-tauri/src/core/clipboard/store.rs`:
```rust
use crate::types::clipboard::ClipboardEntry;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::Mutex;

const MAX_ENTRIES: u32 = 500;

pub struct ClipboardStore {
    conn: Mutex<Connection>,
}

impl ClipboardStore {
    pub fn new() -> Result<Self, String> {
        let db_path = Self::db_path()?;
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create db dir: {}", e))?;
        }
        let conn = Connection::open(&db_path)
            .map_err(|e| format!("Failed to open database: {}", e))?;
        let store = Self { conn: Mutex::new(conn) };
        store.init_tables()?;
        Ok(store)
    }

    fn db_path() -> Result<PathBuf, String> {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map_err(|_| "Cannot find home directory".to_string())?;
        Ok(PathBuf::from(home).join("AzurePath").join("azurepath.db"))
    }

    fn init_tables(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS clipboard_entries (
                id           TEXT PRIMARY KEY,
                content_type TEXT NOT NULL,
                text_content TEXT,
                image_path   TEXT,
                file_paths   TEXT,
                content_hash TEXT NOT NULL,
                is_favorite  INTEGER DEFAULT 0,
                created_at   TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_clipboard_created ON clipboard_entries(created_at);
            CREATE INDEX IF NOT EXISTS idx_clipboard_favorite ON clipboard_entries(is_favorite);
            ",
        )
        .map_err(|e| format!("Failed to init clipboard tables: {}", e))?;
        Ok(())
    }

    pub fn insert(&self, entry: &ClipboardEntry) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR IGNORE INTO clipboard_entries (id, content_type, text_content, image_path, file_paths, content_hash, is_favorite, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                entry.id,
                entry.content_type,
                entry.text_content,
                entry.image_path,
                entry.file_paths.as_ref().map(|v| serde_json::to_string(v).unwrap_or_default()),
                entry.content_hash,
                entry.is_favorite as i32,
                entry.created_at,
            ],
        )
        .map_err(|e| format!("Failed to insert clipboard entry: {}", e))?;

        // Auto-evict after insert
        drop(conn);
        let _ = self.evict_old();
        Ok(())
    }

    pub fn list(&self, search: Option<&str>, limit: u32) -> Result<Vec<ClipboardEntry>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        let (sql, offset) = if let Some(keyword) = search {
            (
                format!(
                    "SELECT id, content_type, text_content, image_path, file_paths, content_hash, is_favorite, created_at
                     FROM clipboard_entries
                     WHERE content_type = 'text' AND text_content LIKE ?1
                        OR content_type = 'image'
                        OR content_type = 'file'
                     ORDER BY is_favorite DESC, created_at DESC LIMIT ?2"
                ),
                2,
            )
        } else {
            (
                "SELECT id, content_type, text_content, image_path, file_paths, content_hash, is_favorite, created_at
                 FROM clipboard_entries
                 ORDER BY is_favorite DESC, created_at DESC LIMIT ?1"
                    .to_string(),
                1,
            )
        };

        let mut stmt = conn.prepare(&sql).map_err(|e| format!("Failed to prepare: {}", e))?;

        let rows = if let Some(keyword) = search {
            let pattern = format!("%{}%", keyword);
            stmt.query_map(params![pattern, limit], Self::map_row)
        } else {
            stmt.query_map(params![limit], Self::map_row)
        }
        .map_err(|e| format!("Failed to query: {}", e))?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row.map_err(|e| format!("Failed to read row: {}", e))?);
        }
        Ok(entries)
    }

    pub fn get_by_id(&self, id: &str) -> Result<Option<ClipboardEntry>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, content_type, text_content, image_path, file_paths, content_hash, is_favorite, created_at
                 FROM clipboard_entries WHERE id = ?1",
            )
            .map_err(|e| format!("Failed to prepare: {}", e))?;

        let mut rows = stmt
            .query_map(params![id], Self::map_row)
            .map_err(|e| format!("Failed to query: {}", e))?;

        match rows.next() {
            Some(Ok(entry)) => Ok(Some(entry)),
            _ => Ok(None),
        }
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM clipboard_entries WHERE id = ?1", params![id])
            .map_err(|e| format!("Failed to delete: {}", e))?;
        Ok(())
    }

    pub fn toggle_favorite(&self, id: &str) -> Result<bool, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        // Read current value
        let current: bool = conn
            .query_row(
                "SELECT is_favorite FROM clipboard_entries WHERE id = ?1",
                params![id],
                |row| row.get::<_, i32>(0).map(|v| v != 0),
            )
            .map_err(|e| format!("Entry not found: {}", e))?;

        let new_val = !current;
        conn.execute(
            "UPDATE clipboard_entries SET is_favorite = ?1 WHERE id = ?2",
            params![new_val as i32, id],
        )
        .map_err(|e| format!("Failed to toggle favorite: {}", e))?;
        Ok(new_val)
    }

    pub fn clear(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute_batch("DELETE FROM clipboard_entries")
            .map_err(|e| format!("Failed to clear: {}", e))?;
        Ok(())
    }

    pub fn count(&self) -> Result<u32, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let count: u32 = conn
            .query_row("SELECT COUNT(*) FROM clipboard_entries", [], |row| row.get(0))
            .map_err(|e| format!("Failed to count: {}", e))?;
        Ok(count)
    }

    fn evict_old(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "DELETE FROM clipboard_entries WHERE id IN (
                SELECT id FROM clipboard_entries
                WHERE is_favorite = 0
                ORDER BY created_at ASC
                LIMIT MAX(0, (SELECT COUNT(*) - ?1 FROM clipboard_entries WHERE is_favorite = 0))
            )",
            params![MAX_ENTRIES],
        )
        .map_err(|e| format!("Failed to evict old entries: {}", e))?;
        Ok(())
    }

    fn map_row(row: &rusqlite::Row) -> rusqlite::Result<ClipboardEntry> {
        let file_paths_str: Option<String> = row.get(4)?;
        let file_paths = file_paths_str
            .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok());
        let is_fav_int: i32 = row.get(6)?;
        Ok(ClipboardEntry {
            id: row.get(0)?,
            content_type: row.get(1)?,
            text_content: row.get(2)?,
            image_path: row.get(3)?,
            file_paths,
            content_hash: row.get(5)?,
            is_favorite: is_fav_int != 0,
            created_at: row.get(7)?,
        })
    }
}
```

- [ ] **Step 2: Create clipboard module root**

Create `src-tauri/src/core/clipboard/mod.rs`:
```rust
pub mod store;
pub mod monitor;

pub use store::ClipboardStore;
pub use monitor::ClipboardMonitor;
```

- [ ] **Step 3: Register clipboard module in core**

Edit `src-tauri/src/core/mod.rs`, add:
```rust
pub mod clipboard;
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/core/clipboard/ src-tauri/src/core/mod.rs
git commit -m "feat: add ClipboardStore with SQLite CRUD for clipboard history"
```

---

### Task 3: ClipboardMonitor — clipboard polling

**Files:**
- Create: `src-tauri/src/core/clipboard/monitor.rs`

- [ ] **Step 1: Create ClipboardMonitor**

Create `src-tauri/src/core/clipboard/monitor.rs`:
```rust
use crate::core::clipboard::ClipboardStore;
use crate::types::clipboard::ClipboardEntry;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct ClipboardMonitor {
    store: Arc<ClipboardStore>,
    running: Arc<AtomicBool>,
    last_hash: Arc<Mutex<String>>,
}

impl ClipboardMonitor {
    pub fn new(store: Arc<ClipboardStore>) -> Self {
        Self {
            store,
            running: Arc::new(AtomicBool::new(false)),
            last_hash: Arc::new(Mutex::new(String::new())),
        }
    }

    pub async fn start(&self, app: AppHandle) {
        if self.running.swap(true, Ordering::SeqCst) {
            return;
        }

        let running = self.running.clone();
        let last_hash = self.last_hash.clone();
        let store = self.store.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(1500));
            while running.load(Ordering::SeqCst) {
                interval.tick().await;

                // Read clipboard content
                let entry = match Self::read_clipboard().await {
                    Some(e) => e,
                    None => continue,
                };

                // Check for duplicate
                let mut last = last_hash.lock().await;
                if *last == entry.content_hash {
                    continue;
                }
                *last = entry.content_hash.clone();
                drop(last);

                // Persist
                if let Err(e) = store.insert(&entry) {
                    eprintln!("[clipboard] Failed to insert: {}", e);
                    continue;
                }

                // Emit event to frontend
                let _ = app.emit("clipboard:new", &entry);
            }
        });
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    async fn read_clipboard() -> Option<ClipboardEntry> {
        use tauri_plugin_clipboard_manager::ClipboardExt;

        // We can't directly access AppHandle here, so this reads via tauri's clipboard plugin
        // through the AppHandle that will be provided in the start method.
        // For the polling, we'll use a channel pattern or pass AppHandle differently.
        // Simplified approach: return None and rely on commands to feed data.
        // In practice, the monitor will be refactored to use the plugin properly.

        // Placeholder — actual plugin integration will read clipboard here
        None
    }
}
```

Wait — `tauri-plugin-clipboard-manager` requires an `AppHandle` to read clipboard. The monitor needs access to the AppHandle. Let me fix the design:

The monitor needs to hold a reference to the AppHandle so it can call `app.clipboard().read_text()` etc. Let me restructure:

```rust
use tauri_plugin_clipboard_manager::ClipboardExt;

pub struct ClipboardMonitor {
    store: Arc<ClipboardStore>,
    running: Arc<AtomicBool>,
    last_hash: Arc<Mutex<String>>,
    app: Option<AppHandle>,
}

impl ClipboardMonitor {
    pub fn new(store: Arc<ClipboardStore>) -> Self {
        Self { store, running: Arc::new(AtomicBool::new(false)), last_hash: Arc::new(Mutex::new(String::new())), app: None }
    }

    pub fn set_app(&mut self, app: AppHandle) {
        self.app = Some(app);
    }

    pub async fn start(&self) { ... }
}
```

- [ ] **Step 1: Create ClipboardMonitor**

Create `src-tauri/src/core/clipboard/monitor.rs`:
```rust
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
    last_hash: Arc<Mutex<u64>>,
    app: Arc<Mutex<Option<AppHandle>>>,
}

impl ClipboardMonitor {
    pub fn new(store: Arc<ClipboardStore>) -> Self {
        Self {
            store,
            running: Arc::new(AtomicBool::new(false)),
            last_hash: Arc::new(Mutex::new(0)),
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

                let app = app_lock.lock().await;
                let app = match app.as_ref() {
                    Some(a) => a.clone(),
                    None => continue,
                };
                drop(app_lock);

                let entry = match Self::read_clipboard(&app).await {
                    Some(e) => e,
                    None => continue,
                };

                let mut last = last_hash.lock().await;
                if *last == entry.content_hash {
                    continue;
                }
                *last = entry.content_hash;
                drop(last);

                if let Err(e) = store.insert(&entry) {
                    eprintln!("[clipboard] Insert error: {}", e);
                    continue;
                }

                let _ = app.emit("clipboard:new", &entry);
            }
        });
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    async fn read_clipboard(app: &AppHandle) -> Option<ClipboardEntry> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut hasher = std::hash::DefaultHasher::new();

        // Try files first
        if let Ok(Some(files)) = app.clipboard().read_files() {
            if !files.is_empty() {
                let id = Uuid::new_v4().to_string();
                for f in &files {
                    f.hash(&mut hasher);
                }
                return Some(ClipboardEntry {
                    id,
                    content_type: "file".to_string(),
                    text_content: None,
                    image_path: None,
                    file_paths: Some(files),
                    content_hash: hasher.finish().to_string(),
                    is_favorite: false,
                    created_at: now,
                });
            }
        }

        // Try image
        if let Ok(Some(image)) = app.clipboard().read_image() {
            let id = Uuid::new_v4().to_string();
            // Save image to disk
            let home = std::env::var("USERPROFILE")
                .or_else(|_| std::env::var("HOME"))
                .unwrap_or_else(|_| ".".to_string());
            let img_dir = std::path::PathBuf::from(&home).join("AzurePath/clipboard/images");
            let _ = std::fs::create_dir_all(&img_dir);
            let img_path = img_dir.join(format!("{}.png", id));
            if let Ok(img_data) = image.to_png() {
                if let Err(e) = std::fs::write(&img_path, &img_data) {
                    eprintln!("[clipboard] Failed to save image: {}", e);
                }
            }

            // hash: file size + modified time
            if let Ok(meta) = std::fs::metadata(&img_path) {
                meta.len().hash(&mut hasher);
                if let Ok(mt) = meta.modified() {
                    mt.hash(&mut hasher);
                }
            }

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
        if let Ok(Some(text)) = app.clipboard().read_text() {
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
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/core/clipboard/monitor.rs
git commit -m "feat: add ClipboardMonitor with 1.5s polling interval"
```

---

### Task 4: Tauri clipboard commands

**Files:**
- Create: `src-tauri/src/commands/clipboard.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create clipboard commands**

Create `src-tauri/src/commands/clipboard.rs`:
```rust
use crate::core::clipboard::{ClipboardMonitor, ClipboardStore};
use crate::types::clipboard::ClipboardEntry;
use std::sync::Arc;
use std::sync::OnceLock;
use tauri::AppHandle;
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
            if let Some(text) = &entry.text_content {
                app.clipboard()
                    .write_text(text)
                    .map_err(|e| format!("Failed to write text to clipboard: {}", e))?;
            }
        }
        "image" => {
            if let Some(path) = &entry.image_path {
                let raw = std::fs::read(path)
                    .map_err(|e| format!("Failed to read image: {}", e))?;
                app.clipboard()
                    .write_image(raw)
                    .map_err(|e| format!("Failed to write image to clipboard: {}", e))?;
            }
        }
        "file" => {
            if let Some(files) = &entry.file_paths {
                app.clipboard()
                    .write_files(files.clone())
                    .map_err(|e| format!("Failed to write files to clipboard: {}", e))?;
            }
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
```

- [ ] **Step 2: Register commands module**

Edit `src-tauri/src/commands/mod.rs`, add:
```rust
pub mod clipboard;
```

- [ ] **Step 3: Register commands in lib.rs**

Edit `src-tauri/src/lib.rs`, add inside `invoke_handler`:
```rust
commands::clipboard::clipboard_start,
commands::clipboard::clipboard_stop,
commands::clipboard::clipboard_list,
commands::clipboard::clipboard_delete,
commands::clipboard::clipboard_toggle_favorite,
commands::clipboard::clipboard_copy,
commands::clipboard::clipboard_clear,
```

Also register the clipboard plugin. Add before `.invoke_handler(...)`:
```rust
.plugin(tauri_plugin_clipboard_manager::init())
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands/clipboard.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
git commit -m "feat: add Tauri clipboard commands (list/delete/favorite/copy/clear)"
```

---

### Task 5: LAN sync — add ClipboardSync to Frame + handler

**Files:**
- Modify: `src-tauri/src/types/chat.rs`
- Modify: `src-tauri/src/commands/file_transfer.rs` (handle_frame)

- [ ] **Step 1: Add ClipboardSync variant to Frame enum**

In `src-tauri/src/types/chat.rs`, find the `Frame` enum and add:
```rust
ClipboardSync {
    entries: Vec<crate::types::clipboard::ClipboardEntry>,
},
```

- [ ] **Step 2: Handle ClipboardSync in handle_frame**

In `src-tauri/src/commands/file_transfer.rs` (or wherever `handle_frame` is), add a new match arm:
```rust
crate::types::chat::Frame::ClipboardSync { entries } => {
    if let Some(svc) = CLIPBOARD_STORE.get() {
        for entry in entries {
            if let Err(e) = svc.insert(entry) {
                eprintln!("[clipboard] Failed to save synced entry: {}", e);
            }
        }
        let _ = app.emit("clipboard:synced", entries);
    }
}
```

Note: `CLIPBOARD_STORE` is defined in `commands/clipboard.rs`. You'll need to import it. Since it's in another module, you may need to either make it pub or provide an accessor function.

In `commands/clipboard.rs`, add:
```rust
pub fn get_store() -> Option<&'static Arc<ClipboardStore>> {
    CLIPBOARD_STORE.get()
}
```

In `handle_frame`:
```rust
use crate::commands::clipboard;
// ...
crate::types::chat::Frame::ClipboardSync { entries } => {
    if let Some(store) = clipboard::get_store() {
        for entry in entries {
            if let Err(e) = store.insert(entry) {
                eprintln!("[clipboard] Failed to save synced entry: {}", e);
            }
        }
        let _ = app.emit("clipboard:synced", entries);
    }
}
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/types/chat.rs src-tauri/src/commands/file_transfer.rs src-tauri/src/commands/clipboard.rs
git commit -m "feat: add ClipboardSync LAN sync frame type and handler"
```

---

### Task 6: Frontend page — clipboard UI

**Files:**
- Create: `src/pages/clipboard/Page.vue`
- Modify: `src/lib/tauri.ts`
- Modify: `src/router/index.ts`
- Modify: `src/components/layout/Sidebar.vue`

- [ ] **Step 1: Add clipboard API to tauri.ts**

Add to `src/lib/tauri.ts`:
```typescript
// ============================================================
// Clipboard Manager
// ============================================================

export interface ClipboardEntry {
  id: string;
  content_type: string;   // "text" | "image" | "file"
  text_content: string | null;
  image_path: string | null;
  file_paths: string[] | null;
  content_hash: string;
  is_favorite: boolean;
  created_at: string;
}

export function clipboardStart(): Promise<void> {
  return invoke<void>("clipboard_start");
}

export function clipboardStop(): Promise<void> {
  return invoke<void>("clipboard_stop");
}

export function clipboardList(search?: string, limit?: number): Promise<ClipboardEntry[]> {
  return invoke<ClipboardEntry[]>("clipboard_list", { search, limit });
}

export function clipboardDelete(id: string): Promise<void> {
  return invoke<void>("clipboard_delete", { id });
}

export function clipboardToggleFavorite(id: string): Promise<boolean> {
  return invoke<boolean>("clipboard_toggle_favorite", { id });
}

export function clipboardCopy(id: string): Promise<void> {
  return invoke<void>("clipboard_copy", { id });
}

export function clipboardClear(): Promise<void> {
  return invoke<void>("clipboard_clear");
}

export function onClipboardNew(cb: (entry: ClipboardEntry) => void): Promise<UnlistenFn> {
  return listen<ClipboardEntry>("clipboard:new", (event) => cb(event.payload));
}
```

- [ ] **Step 2: Add /clipboard route**

In `src/router/index.ts`, add:
```typescript
{
  path: "/clipboard",
  name: "clipboard",
  component: () => import("@/pages/clipboard/Page.vue"),
},
```

- [ ] **Step 3: Add nav item in Sidebar**

In `src/components/layout/Sidebar.vue`:
- Import `Clipboard` from `lucide-vue-next`
- Add nav item: `{ label: "剪贴板", name: "clipboard", path: "/clipboard", icon: Clipboard }`

- [ ] **Step 4: Create clipboard Page.vue**

Create `src/pages/clipboard/Page.vue`:
```vue
<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from "vue";
import { Clipboard, Search, Trash2, Star, Copy, FileText, Image, File, X } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import {
  clipboardStart,
  clipboardList,
  clipboardDelete,
  clipboardToggleFavorite,
  clipboardCopy,
  clipboardClear,
  onClipboardNew,
  type ClipboardEntry,
} from "@/lib/tauri";
import type { UnlistenFn } from "@tauri-apps/api/event";

const entries = ref<ClipboardEntry[]>([]);
const searchQuery = ref("");
const loading = ref(true);
const copiedId = ref<string | null>(null);
let unlistenNew: UnlistenFn | null = null;

const filteredEntries = computed(() => {
  if (!searchQuery.value.trim()) return entries.value;
  const q = searchQuery.value.toLowerCase();
  return entries.value.filter(
    (e) => e.text_content?.toLowerCase().includes(q)
  );
});

onMounted(async () => {
  try {
    await clipboardStart();
  } catch (e) {
    console.error("Failed to start clipboard:", e);
  }
  await loadEntries();

  unlistenNew = await onClipboardNew((entry) => {
    entries.value.unshift(entry);
  });
});

onUnmounted(() => {
  unlistenNew?.();
});

async function loadEntries() {
  loading.value = true;
  try {
    entries.value = await clipboardList(undefined, 500);
  } catch (e) {
    console.error("Failed to load clipboard entries:", e);
  } finally {
    loading.value = false;
  }
}

async function toggleFavorite(id: string) {
  try {
    const newVal = await clipboardToggleFavorite(id);
    const entry = entries.value.find((e) => e.id === id);
    if (entry) entry.is_favorite = newVal;
  } catch (e) {
    console.error("Failed to toggle favorite:", e);
  }
}

async function copyEntry(id: string) {
  try {
    await clipboardCopy(id);
    copiedId.value = id;
    setTimeout(() => { copiedId.value = null; }, 2000);
  } catch (e) {
    console.error("Failed to copy:", e);
  }
}

async function deleteEntry(id: string) {
  try {
    await clipboardDelete(id);
    entries.value = entries.value.filter((e) => e.id !== id);
  } catch (e) {
    console.error("Failed to delete:", e);
  }
}

async function clearAll() {
  if (!confirm("确定清空所有剪贴板历史？")) return;
  try {
    await clipboardClear();
    entries.value = [];
  } catch (e) {
    console.error("Failed to clear:", e);
  }
}

function formatTime(iso: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleString("zh-CN");
  } catch {
    return iso;
  }
}

function truncate(text: string, len: number): string {
  return text.length > len ? text.slice(0, len) + "..." : text;
}

function typeIcon(type: string) {
  if (type === "text") return FileText;
  if (type === "image") return Image;
  return File;
}
</script>

<template>
  <div class="flex h-full flex-col animate-view-fade">
    <!-- Header -->
    <div class="border-b border-paper-deep/50 px-6 py-3">
      <div class="flex items-center justify-between">
        <h1 class="text-xl font-display font-bold text-ink">剪贴板管理</h1>
        <div class="flex items-center gap-2">
          <div class="relative">
            <Search class="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-ink-faint" />
            <input
              v-model="searchQuery"
              placeholder="搜索剪贴板内容..."
              class="w-56 rounded-lg border border-paper-deep bg-paper-warm/50 pl-9 pr-3 py-1.5 text-sm text-ink placeholder:text-ink-faint/50 outline-none"
            />
          </div>
          <Button variant="danger" size="sm" @click="clearAll">
            <Trash2 class="mr-1 h-3.5 w-3.5" />
            清空
          </Button>
        </div>
      </div>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-y-auto p-4">
      <div v-if="loading" class="flex items-center justify-center h-full text-sm text-ink-faint">
        加载中...
      </div>

      <div v-else-if="filteredEntries.length === 0" class="flex items-center justify-center h-full text-sm text-ink-faint">
        <div class="text-center">
          <Clipboard class="mx-auto h-8 w-8 mb-2 opacity-40" />
          <p>暂无剪贴板记录</p>
          <p class="mt-1 text-xs opacity-60">复制任何内容后将自动记录在此</p>
        </div>
      </div>

      <div v-else class="space-y-2">
        <div
          v-for="entry in filteredEntries"
          :key="entry.id"
          class="flex items-start gap-3 rounded-xl border border-paper-deep/20 bg-paper-warm/30 p-3 transition-colors hover:bg-paper-warm/60"
        >
          <!-- Type icon -->
          <div class="mt-0.5 shrink-0 rounded-lg bg-paper-deep/20 p-2">
            <component :is="typeIcon(entry.content_type)" class="h-4 w-4 text-ink-soft" />
          </div>

          <!-- Content -->
          <div class="flex-1 min-w-0">
            <div v-if="entry.content_type === 'text' && entry.text_content" class="text-sm text-ink whitespace-pre-wrap break-words">
              {{ truncate(entry.text_content, 200) }}
            </div>
            <div v-else-if="entry.content_type === 'image' && entry.image_path" class="text-sm text-ink-soft">
              <p class="truncate">{{ entry.image_path.split('/').pop() || entry.image_path }}</p>
            </div>
            <div v-else-if="entry.content_type === 'file' && entry.file_paths" class="text-sm text-ink-soft">
              <p v-for="f in entry.file_paths" :key="f" class="truncate">{{ f }}</p>
            </div>
            <div class="mt-1 flex items-center gap-2 text-xs text-ink-faint">
              <span>{{ formatTime(entry.created_at) }}</span>
            </div>
          </div>

          <!-- Actions -->
          <div class="flex shrink-0 items-center gap-1">
            <button
              class="rounded-lg p-1.5 transition-colors"
              :class="entry.is_favorite ? 'text-yellow-500' : 'text-ink-faint hover:text-yellow-500'"
              @click="toggleFavorite(entry.id)"
              :title="entry.is_favorite ? '取消收藏' : '收藏'"
            >
              <Star class="h-4 w-4" :fill="entry.is_favorite ? 'currentColor' : 'none'" />
            </button>
            <button
              class="rounded-lg p-1.5 text-ink-faint transition-colors hover:text-bamboo"
              @click="copyEntry(entry.id)"
              :title="copiedId === entry.id ? '已复制!' : '复制'"
            >
              <Copy class="h-4 w-4" />
            </button>
            <button
              class="rounded-lg p-1.5 text-ink-faint transition-colors hover:text-red-500"
              @click="deleteEntry(entry.id)"
              title="删除"
            >
              <X class="h-4 w-4" />
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Footer -->
    <div class="border-t border-paper-deep/50 px-6 py-2 text-xs text-ink-faint">
      {{ entries.length }} 条记录 · 自动监听中
    </div>
  </div>
</template>
```

Note: For image preview in the frontend, the current `image_path` is a local filesystem path. The webview can't directly access local files via `file://` protocol. Instead, two options:
1. Use the existing `FileServer` to serve clipboard images
2. Convert image to base64 data URL in Rust and include in the entry

For simplicity, option 2 is better: when saving image, also include a base64 thumbnail. Or we could skip the image preview feature for now and just show the filename. Let's keep it simple — show filename only, and the user can copy the image back to clipboard and paste elsewhere.

Updated template for image type:
```html
<div v-else-if="entry.content_type === 'image' && entry.image_path" class="text-sm text-ink-soft">
  <p class="truncate">{{ entry.image_path.split('/').pop() || entry.image_path }}</p>
</div>
```

- [ ] **Step 5: Commit**

```bash
git add src/pages/clipboard/Page.vue src/lib/tauri.ts src/router/index.ts src/components/layout/Sidebar.vue
git commit -m "feat: add clipboard management frontend page"
```

---

### Task 7: Wire up clipboard into app startup

**Files:**
- Modify: `src-tauri/src/commands/lan.rs`

- [ ] **Step 1: Integrate clipboard start into lan_init**

In `src-tauri/src/commands/lan.rs`, find the `lan_init` function. After all other init is done, start the clipboard monitor:
```rust
// 5. Start clipboard monitor
let _ = crate::commands::clipboard::clipboard_start(app.clone()).await;
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/commands/lan.rs
git commit -m "feat: integrate clipboard monitor into LAN init"
# Also push
git push
```
