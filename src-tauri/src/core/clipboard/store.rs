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

        let (sql, _) = if let Some(_keyword) = search {
            (
                format!(
                    "SELECT id, content_type, text_content, image_path, file_paths, content_hash, is_favorite, created_at
                     FROM clipboard_entries
                     WHERE text_content LIKE ?1
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
