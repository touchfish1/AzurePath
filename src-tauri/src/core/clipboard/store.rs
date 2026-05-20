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

    #[cfg(test)]
    pub(crate) fn test_store() -> Self {
        let conn = Connection::open_in_memory().unwrap();
        let store = Self { conn: Mutex::new(conn) };
        store.init_tables().unwrap();
        store
    }

    fn db_path() -> Result<PathBuf, String> {
        let home = crate::core::utils::home_dir()
            .ok_or_else(|| "Cannot find home directory".to_string())?;
        Ok(home.join("AzurePath").join("azurepath.db"))
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
                content_hash TEXT NOT NULL UNIQUE,
                is_favorite  INTEGER DEFAULT 0,
                created_at   TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_clipboard_created ON clipboard_entries(created_at);
            CREATE INDEX IF NOT EXISTS idx_clipboard_favorite ON clipboard_entries(is_favorite);
            CREATE INDEX IF NOT EXISTS idx_clipboard_hash ON clipboard_entries(content_hash);
            CREATE TABLE IF NOT EXISTS clipboard_settings (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            ",
        )
        .map_err(|e| format!("Failed to init clipboard tables: {}", e))?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT value FROM clipboard_settings WHERE key = ?1")
            .map_err(|e| format!("Failed to prepare: {}", e))?;
        let mut rows = stmt
            .query_map(params![key], |row| row.get::<_, String>(0))
            .map_err(|e| format!("Failed to query: {}", e))?;
        match rows.next() {
            Some(Ok(val)) => Ok(Some(val)),
            _ => Ok(None),
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO clipboard_settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )
        .map_err(|e| format!("Failed to set setting: {}", e))?;
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

        let sql = if search.is_some() {
                format!(
                    "SELECT id, content_type, text_content, image_path, file_paths, content_hash, is_favorite, created_at
                     FROM clipboard_entries
                     WHERE text_content LIKE ?1 ESCAPE '\\'
                     ORDER BY is_favorite DESC, created_at DESC LIMIT ?2"
                )
        } else {
                "SELECT id, content_type, text_content, image_path, file_paths, content_hash, is_favorite, created_at
                 FROM clipboard_entries
                 ORDER BY is_favorite DESC, created_at DESC LIMIT ?1"
                    .to_string()
        };

        let mut stmt = conn.prepare(&sql).map_err(|e| format!("Failed to prepare: {}", e))?;

        let rows = if let Some(keyword) = search {
            let escaped = keyword.replace("\\", "\\\\").replace("%", "\\%").replace("_", "\\_");
            let pattern = format!("%{}%", escaped);
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

    pub fn delete_entries(&self, ids: &[String]) -> Result<(), String> {
        if ids.is_empty() {
            return Ok(());
        }
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let placeholders: Vec<String> = ids.iter().map(|_| "?".to_string()).collect();
        let sql = format!(
            "DELETE FROM clipboard_entries WHERE id IN ({})",
            placeholders.join(", ")
        );
        let params: Vec<&dyn rusqlite::types::ToSql> =
            ids.iter().map(|id| id as &dyn rusqlite::types::ToSql).collect();
        conn.execute(&sql, params.as_slice())
            .map_err(|e| format!("Failed to delete entries: {}", e))?;
        Ok(())
    }

    pub fn export_entries(&self, ids: &[String]) -> Result<Vec<ClipboardEntry>, String> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let placeholders: Vec<String> = ids.iter().map(|_| "?".to_string()).collect();
        let sql = format!(
            "SELECT id, content_type, text_content, image_path, file_paths, content_hash, is_favorite, created_at
             FROM clipboard_entries WHERE id IN ({})
             ORDER BY created_at DESC",
            placeholders.join(", ")
        );
        let params: Vec<&dyn rusqlite::types::ToSql> =
            ids.iter().map(|id| id as &dyn rusqlite::types::ToSql).collect();
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| format!("Failed to prepare export: {}", e))?;
        let rows = stmt
            .query_map(params.as_slice(), Self::map_row)
            .map_err(|e| format!("Failed to query export: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read row: {}", e))?;
        Ok(rows)
    }

    pub fn get_unique_sources(&self) -> Result<Vec<String>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT DISTINCT content_type FROM clipboard_entries ORDER BY content_type")
            .map_err(|e| format!("Failed to prepare: {}", e))?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| format!("Failed to query sources: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read row: {}", e))?;
        Ok(rows)
    }

    pub fn set_max_entries(&self, limit: usize) -> Result<(), String> {
        self.set_setting("max_entries", &limit.to_string())
    }

    pub fn get_max_entries(&self) -> Result<usize, String> {
        match self.get_setting("max_entries")? {
            Some(val) => val.parse::<usize>().or(Ok(MAX_ENTRIES as usize)),
            None => Ok(MAX_ENTRIES as usize),
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
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

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn count(&self) -> Result<u32, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let count: u32 = conn
            .query_row("SELECT COUNT(*) FROM clipboard_entries", [], |row| row.get(0))
            .map_err(|e| format!("Failed to count: {}", e))?;
        Ok(count)
    }

    fn evict_old(&self) -> Result<(), String> {
        let max = self.get_max_entries()? as u32;
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "DELETE FROM clipboard_entries WHERE id IN (
                SELECT id FROM clipboard_entries
                WHERE is_favorite = 0
                ORDER BY created_at ASC
                LIMIT MAX(0, (SELECT COUNT(*) - ?1 FROM clipboard_entries WHERE is_favorite = 0))
            )",
            params![max],
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

#[cfg(test)]
mod tests {
    use super::*;

    // Helper — creates an in-memory store so tests never touch the real DB.
    fn test_store() -> ClipboardStore {
        let conn = Connection::open_in_memory().unwrap();
        let store = ClipboardStore { conn: Mutex::new(conn) };
        store.init_tables().unwrap();
        store
    }

    fn make_entry(
        content_type: &str,
        text: Option<&str>,
        hash: &str,
        favorite: bool,
    ) -> ClipboardEntry {
        ClipboardEntry {
            id: uuid::Uuid::new_v4().to_string(),
            content_type: content_type.to_string(),
            text_content: text.map(|s| s.to_string()),
            image_path: None,
            file_paths: None,
            content_hash: hash.to_string(),
            is_favorite: favorite,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    // ── Basic CRUD ────────────────────────────────────────────────────────

    #[test]
    fn test_insert_and_list() {
        let store = test_store();
        let e = make_entry("text", Some("hello world"), "h1", false);
        store.insert(&e).unwrap();

        let entries = store.list(None, 100).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text_content.as_deref(), Some("hello world"));
        assert_eq!(entries[0].content_hash, "h1");
        assert!(!entries[0].is_favorite);
    }

    #[test]
    fn test_delete() {
        let store = test_store();
        let e = make_entry("text", Some("delete me"), "h_del", false);
        store.insert(&e).unwrap();
        assert_eq!(store.list(None, 100).unwrap().len(), 1);

        store.delete(&e.id).unwrap();
        assert_eq!(store.list(None, 100).unwrap().len(), 0);
    }

    #[test]
    fn test_delete_nonexistent() {
        let store = test_store();
        store.delete("no-such-id").unwrap();
    }

    #[test]
    fn test_get_by_id() {
        let store = test_store();
        let e = make_entry("text", Some("found"), "h_find", false);
        store.insert(&e).unwrap();

        let found = store.get_by_id(&e.id).unwrap().expect("should exist");
        assert_eq!(found.text_content.as_deref(), Some("found"));
    }

    #[test]
    fn test_get_by_id_nonexistent() {
        let store = test_store();
        assert!(store.get_by_id("no-such-id").unwrap().is_none());
    }

    #[test]
    fn test_clear() {
        let store = test_store();
        store.insert(&make_entry("text", Some("a"), "h_a", false)).unwrap();
        store.insert(&make_entry("text", Some("b"), "h_b", false)).unwrap();
        assert_eq!(store.count().unwrap(), 2);

        store.clear().unwrap();
        assert_eq!(store.count().unwrap(), 0);
    }

    #[test]
    fn test_count() {
        let store = test_store();
        assert_eq!(store.count().unwrap(), 0);

        store.insert(&make_entry("text", Some("a"), "h_c1", false)).unwrap();
        assert_eq!(store.count().unwrap(), 1);

        store.insert(&make_entry("text", Some("b"), "h_c2", false)).unwrap();
        assert_eq!(store.count().unwrap(), 2);
    }

    #[test]
    fn test_list_limit() {
        let store = test_store();
        for i in 0..10 {
            let e = make_entry("text", Some(&format!("e{i}")), &format!("h_lim{i}"), false);
            store.insert(&e).unwrap();
        }
        assert_eq!(store.list(None, 3).unwrap().len(), 3);
        assert_eq!(store.list(None, 20).unwrap().len(), 10);
    }

    // ── Hash dedup ────────────────────────────────────────────────────────

    #[test]
    fn test_insert_duplicate_hash_is_ignored() {
        let store = test_store();
        let e1 = make_entry("text", Some("hello"), "dup_hash", false);
        let e2 = make_entry("text", Some("hello"), "dup_hash", false);
        store.insert(&e1).unwrap();
        store.insert(&e2).unwrap();

        // UNIQUE constraint on content_hash should cause INSERT OR IGNORE
        // to skip the second row.
        assert_eq!(store.count().unwrap(), 1);
    }

    // ── Toggle favorite ───────────────────────────────────────────────────

    #[test]
    fn test_toggle_favorite() {
        let store = test_store();
        let e = make_entry("text", Some("fav"), "h_fav", false);
        store.insert(&e).unwrap();

        let state = store.toggle_favorite(&e.id).unwrap();
        assert!(state);

        let entries = store.list(None, 100).unwrap();
        assert!(entries[0].is_favorite);

        let state2 = store.toggle_favorite(&e.id).unwrap();
        assert!(!state2);
    }

    #[test]
    fn test_toggle_favorite_nonexistent_is_error() {
        let store = test_store();
        assert!(store.toggle_favorite("no-such-id").is_err());
    }

    // ── Search and special-character handling ─────────────────────────────

    #[test]
    fn test_list_search() {
        let store = test_store();
        store.insert(&make_entry("text", Some("apple banana"), "h_s1", false)).unwrap();
        store.insert(&make_entry("text", Some("banana cherry"), "h_s2", false)).unwrap();
        store.insert(&make_entry("text", Some("date"), "h_s3", false)).unwrap();

        let entries = store.list(Some("banana"), 100).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_list_search_with_special_like_chars() {
        let store = test_store();
        // These strings contain characters that have special meaning
        // in SQL LIKE patterns: % and _.  The store must escape them
        // correctly so they are treated as literals.
        store.insert(&make_entry("text", Some("100% complete"), "h_sp1", false)).unwrap();
        store.insert(&make_entry("text", Some("test_underscore"), "h_sp2", false)).unwrap();
        store.insert(&make_entry("text", Some(r"back\slash"), "h_sp3", false)).unwrap();

        let r1 = store.list(Some("100%"), 100).unwrap();
        assert_eq!(r1.len(), 1, "LIKE escape of %% failed");

        let r2 = store.list(Some("test_"), 100).unwrap();
        assert_eq!(r2.len(), 1, "LIKE escape of _ failed");

        let r3 = store.list(Some(r"back\"), 100).unwrap();
        assert_eq!(r3.len(), 1, "LIKE escape of backslash failed");
    }

    #[test]
    fn test_list_search_empty_query_returns_all() {
        let store = test_store();
        store.insert(&make_entry("text", Some("a"), "h_eq1", false)).unwrap();
        store.insert(&make_entry("text", Some("b"), "h_eq2", false)).unwrap();
        let r = store.list(Some(""), 100).unwrap();
        assert_eq!(r.len(), 2);
    }

    // ── Settings ──────────────────────────────────────────────────────────

    #[test]
    fn test_get_set_setting() {
        let store = test_store();
        assert_eq!(store.get_setting("foo").unwrap(), None);

        store.set_setting("foo", "bar").unwrap();
        assert_eq!(
            store.get_setting("foo").unwrap(),
            Some("bar".to_string())
        );

        // Overwrite
        store.set_setting("foo", "baz").unwrap();
        assert_eq!(
            store.get_setting("foo").unwrap(),
            Some("baz".to_string())
        );
    }

    // ── Entry types ───────────────────────────────────────────────────────

    #[test]
    fn test_image_entry() {
        let store = test_store();
        let e = make_entry("image", None, "h_img", false);
        store.insert(&e).unwrap();
        let entries = store.list(None, 100).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].content_type, "image");
    }

    #[test]
    fn test_file_entry_with_paths() {
        let store = test_store();
        let e = ClipboardEntry {
            id: "file-id-1".to_string(),
            content_type: "file".to_string(),
            text_content: None,
            image_path: None,
            file_paths: Some(vec!["C:\\Users\\test\\doc.pdf".to_string()]),
            content_hash: "h_file".to_string(),
            is_favorite: false,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        store.insert(&e).unwrap();

        let entries = store.list(None, 100).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].file_paths,
            Some(vec!["C:\\Users\\test\\doc.pdf".to_string()])
        );
    }

    // ── Eviction ──────────────────────────────────────────────────────────

    #[test]
    fn test_evict_old_basic() {
        let store = test_store();
        // Insert more entries than MAX_ENTRIES
        for i in 0..MAX_ENTRIES + 10 {
            let e = make_entry("text", Some(&format!("e{i}")), &format!("h_ev{i}"), false);
            store.insert(&e).unwrap();
        }
        let count = store.count().unwrap();
        assert!(
            count <= MAX_ENTRIES,
            "expected <= {MAX_ENTRIES} entries after eviction, got {count}"
        );
    }

    #[test]
    fn test_evict_old_preserves_favorites() {
        let store = test_store();
        // Fill with non-favorites up to the limit
        for i in 0..MAX_ENTRIES {
            let e = make_entry("text", Some(&format!("e{i}")), &format!("h_ef{i}"), false);
            store.insert(&e).unwrap();
        }
        // Insert a favorite with an old timestamp — it should never be evicted.
        let fav = ClipboardEntry {
            id: "fav-keep".to_string(),
            content_type: "text".to_string(),
            text_content: Some("keeper".to_string()),
            image_path: None,
            file_paths: None,
            content_hash: "fav_keep_hash".to_string(),
            is_favorite: true,
            created_at: "2020-01-01T00:00:00Z".to_string(),
        };
        store.insert(&fav).unwrap();

        // Push one more non-favorite over the limit to trigger eviction
        let extra = make_entry("text", Some("extra"), "h_extra", false);
        store.insert(&extra).unwrap();

        let kept = store.get_by_id("fav-keep").unwrap();
        assert!(kept.is_some(), "favorite entry was incorrectly evicted");
        assert!(kept.unwrap().is_favorite);
    }

    // ── SQL injection vectors ─────────────────────────────────────────────

    /// Verify that malicious text_content cannot break out of the LIKE
    /// clause — all user input is parameterised, never string-interpolated.
    #[test]
    fn test_sql_injection_like_escaping() {
        let store = test_store();
        let payload = make_entry(
            "text",
            Some("'; DROP TABLE clipboard_entries; --"),
            "h_inj1",
            false,
        );
        store.insert(&payload).unwrap();

        // The LIKE search should not crash or drop the table.
        let r = store.list(Some("'; DROP"), 100).unwrap();
        assert_eq!(r.len(), 1, "SQL injection payload should be found by LIKE");

        // Table still exists and the entry is there.
        assert_eq!(store.count().unwrap(), 1);
    }

    #[test]
    fn test_sql_injection_in_hash_and_id() {
        let store = test_store();
        let payload = ClipboardEntry {
            id: "'; DELETE FROM clipboard_entries; --".to_string(),
            content_type: "text".to_string(),
            text_content: Some("safe".to_string()),
            image_path: None,
            file_paths: None,
            content_hash: "'; DROP TABLE clipboard_settings; --".to_string(),
            is_favorite: false,
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        store.insert(&payload).unwrap();

        // All entries should still exist
        assert_eq!(store.count().unwrap(), 1);

        // Should be findable by id
        let found = store.get_by_id(&payload.id).unwrap();
        assert!(found.is_some());
    }

    // ── Empty and edge-case entries ───────────────────────────────────────

    #[test]
    fn test_empty_text_content() {
        let store = test_store();
        let e = make_entry("text", Some(""), "h_empty", false);
        store.insert(&e).unwrap();
        let entries = store.list(None, 100).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text_content.as_deref(), Some(""));
    }

    #[test]
    fn test_null_text_content() {
        let store = test_store();
        let e = make_entry("text", None, "h_nulltext", false);
        store.insert(&e).unwrap();
        let entries = store.list(None, 100).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].text_content.is_none());
    }
}
