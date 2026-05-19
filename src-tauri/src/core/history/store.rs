use crate::types::activity::ActivityEntry;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug)]
pub struct ActivityStore {
    conn: Mutex<Connection>,
}

impl ActivityStore {
    pub fn new() -> Result<Self, String> {
        let db_path = Self::db_path()?;
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create db directory: {}", e))?;
        }
        let conn = Connection::open(&db_path)
            .map_err(|e| format!("Failed to open database: {}", e))?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.init_tables()?;
        Ok(store)
    }

    #[cfg(test)]
    pub(crate) fn new_test() -> Result<Self, String> {
        let conn = Connection::open_in_memory()
            .map_err(|e| format!("Failed to create in-memory db: {}", e))?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.init_tables()?;
        Ok(store)
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
            CREATE TABLE IF NOT EXISTS activity_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                action_type TEXT NOT NULL,
                description TEXT NOT NULL,
                metadata TEXT,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_activity_type ON activity_log(action_type);
            CREATE INDEX IF NOT EXISTS idx_activity_created ON activity_log(created_at);
            ",
        )
        .map_err(|e| format!("Failed to init activity tables: {}", e))?;
        Ok(())
    }

    pub fn log(&self, action_type: &str, description: &str, metadata: Option<&str>) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO activity_log (action_type, description, metadata, created_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![action_type, description, metadata, now],
        )
        .map_err(|e| format!("Failed to insert activity log: {}", e))?;
        Ok(())
    }

    pub fn list(
        &self,
        limit: usize,
        offset: usize,
        action_type: Option<&str>,
    ) -> Result<Vec<ActivityEntry>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        let rows: Vec<ActivityEntry> = if let Some(atype) = action_type {
            let mut stmt = conn
                .prepare(
                    "SELECT id, action_type, description, metadata, created_at
                     FROM activity_log
                     WHERE action_type = ?1
                     ORDER BY created_at DESC
                     LIMIT ?2 OFFSET ?3",
                )
                .map_err(|e| format!("Failed to prepare: {}", e))?;
            let mapped = stmt
                .query_map(rusqlite::params![atype, limit, offset], Self::map_row)
                .map_err(|e| format!("Failed to query: {}", e))?;
            mapped
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("Failed to read row: {}", e))?
        } else {
            let mut stmt = conn
                .prepare(
                    "SELECT id, action_type, description, metadata, created_at
                     FROM activity_log
                     ORDER BY created_at DESC
                     LIMIT ?1 OFFSET ?2",
                )
                .map_err(|e| format!("Failed to prepare: {}", e))?;
            let mapped = stmt
                .query_map(rusqlite::params![limit, offset], Self::map_row)
                .map_err(|e| format!("Failed to query: {}", e))?;
            mapped
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("Failed to read row: {}", e))?
        };

        Ok(rows)
    }

    pub fn search(&self, keyword: &str) -> Result<Vec<ActivityEntry>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let pattern = format!("%{}%", keyword.replace("\\", "\\\\").replace("%", "\\%").replace("_", "\\_"));
        let mut stmt = conn
            .prepare(
                "SELECT id, action_type, description, metadata, created_at
                 FROM activity_log
                 WHERE description LIKE ?1 ESCAPE '\\'
                    OR action_type LIKE ?1 ESCAPE '\\'
                 ORDER BY created_at DESC
                 LIMIT 200",
            )
            .map_err(|e| format!("Failed to prepare: {}", e))?;
        let rows = stmt
            .query_map(rusqlite::params![pattern], Self::map_row)
            .map_err(|e| format!("Failed to query: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to read row: {}", e))?;
        Ok(rows)
    }

    pub fn delete(&self, ids: &[i64]) -> Result<(), String> {
        if ids.is_empty() {
            return Ok(());
        }
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let placeholders: Vec<String> = ids.iter().map(|_| "?".to_string()).collect();
        let sql = format!(
            "DELETE FROM activity_log WHERE id IN ({})",
            placeholders.join(", ")
        );
        let params: Vec<&dyn rusqlite::types::ToSql> =
            ids.iter().map(|id| id as &dyn rusqlite::types::ToSql).collect();
        conn.execute(&sql, params.as_slice())
            .map_err(|e| format!("Failed to delete: {}", e))?;
        Ok(())
    }

    pub fn clear(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute_batch("DELETE FROM activity_log")
            .map_err(|e| format!("Failed to clear activity log: {}", e))?;
        Ok(())
    }

    fn map_row(row: &rusqlite::Row) -> rusqlite::Result<ActivityEntry> {
        Ok(ActivityEntry {
            id: row.get(0)?,
            action_type: row.get(1)?,
            description: row.get(2)?,
            metadata: row.get(3)?,
            created_at: row.get(4)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_store() -> ActivityStore {
        ActivityStore::new_test().unwrap()
    }

    #[test]
    fn test_log_and_list() {
        let store = make_store();
        store.log("ping", "Ping test completed", None).unwrap();
        store.log("scan", "Port scan done", Some(r#"{"ports":3}"#)).unwrap();

        let entries = store.list(10, 0, None).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].action_type, "scan");
        assert_eq!(entries[1].description, "Ping test completed");
    }

    #[test]
    fn test_list_with_action_filter() {
        let store = make_store();
        store.log("ping", "ping1", None).unwrap();
        store.log("scan", "scan1", None).unwrap();
        store.log("ping", "ping2", None).unwrap();

        let pings = store.list(10, 0, Some("ping")).unwrap();
        assert_eq!(pings.len(), 2);
        let scans = store.list(10, 0, Some("scan")).unwrap();
        assert_eq!(scans.len(), 1);
    }

    #[test]
    fn test_list_limit_offset() {
        let store = make_store();
        for i in 0..10 {
            store.log("test", &format!("entry {}", i), None).unwrap();
        }

        let page = store.list(3, 5, None).unwrap();
        assert_eq!(page.len(), 3);
        // Since order is DESC, entry 9 is first, entry 5 is at offset 5
        assert_eq!(page[0].description, "entry 4");
        assert_eq!(page[2].description, "entry 2");
    }

    #[test]
    fn test_search() {
        let store = make_store();
        store.log("ping", "Ping 192.168.1.1 success", None).unwrap();
        store.log("dns", "DNS lookup example.com", None).unwrap();
        store.log("scan", "Port scan 192.168.1.1", None).unwrap();

        let results = store.search("192.168").unwrap();
        assert_eq!(results.len(), 2);

        let results = store.search("DNS").unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_delete_by_ids() {
        let store = make_store();
        store.log("a", "first", None).unwrap();
        store.log("b", "second", None).unwrap();
        store.log("c", "third", None).unwrap();

        let entries = store.list(10, 0, None).unwrap();
        let ids: Vec<i64> = entries.iter().take(2).map(|e| e.id).collect();
        store.delete(&ids).unwrap();

        let remaining = store.list(10, 0, None).unwrap();
        assert_eq!(remaining.len(), 1);
    }

    #[test]
    fn test_delete_empty_ids() {
        let store = make_store();
        store.log("a", "test", None).unwrap();
        store.delete(&[]).unwrap();
        assert_eq!(store.list(10, 0, None).unwrap().len(), 1);
    }

    #[test]
    fn test_clear() {
        let store = make_store();
        store.log("a", "test", None).unwrap();
        store.clear().unwrap();
        assert!(store.list(10, 0, None).unwrap().is_empty());
    }

    #[test]
    fn test_log_with_metadata() {
        let store = make_store();
        store.log("transfer", "File sent", Some(r#"{"file":"test.txt","size":1024}"#)).unwrap();
        let entries = store.list(10, 0, None).unwrap();
        assert_eq!(entries[0].metadata.as_deref(), Some(r#"{"file":"test.txt","size":1024}"#));
    }

    #[test]
    fn test_log_empty_description() {
        let store = make_store();
        store.log("test", "", None).unwrap();
        let entries = store.list(10, 0, None).unwrap();
        assert_eq!(entries[0].description, "");
    }

    #[test]
    fn test_list_empty() {
        let store = make_store();
        assert!(store.list(10, 0, None).unwrap().is_empty());
    }

    #[test]
    fn test_search_empty_keyword_returns_all() {
        let store = make_store();
        store.log("a", "hello", None).unwrap();
        store.log("b", "world", None).unwrap();
        let results = store.search("").unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_list_past_end() {
        let store = make_store();
        store.log("a", "test", None).unwrap();
        let entries = store.list(10, 100, None).unwrap();
        assert!(entries.is_empty());
    }
}
