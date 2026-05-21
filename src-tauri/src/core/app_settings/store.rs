use crate::types::app_settings::SettingEntry;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::Mutex;

pub struct SettingsStore {
    conn: Mutex<Connection>,
}

impl SettingsStore {
    pub fn new() -> Result<Self, String> {
        let db_path = Self::db_path()?;
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create db dir: {}", e))?;
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
    pub fn test_store() -> Self {
        let conn = Connection::open_in_memory().unwrap();
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.init_tables().unwrap();
        store
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
            CREATE TABLE IF NOT EXISTS app_settings (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            ",
        )
        .map_err(|e| format!("Failed to init settings table: {}", e))?;
        Ok(())
    }

    pub fn get(&self, key: &str) -> Result<Option<String>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT value FROM app_settings WHERE key = ?1")
            .map_err(|e| format!("Failed to prepare: {}", e))?;
        let mut rows = stmt
            .query_map(params![key], |row| row.get::<_, String>(0))
            .map_err(|e| format!("Failed to query: {}", e))?;
        match rows.next() {
            Some(Ok(val)) => Ok(Some(val)),
            _ => Ok(None),
        }
    }

    pub fn set(&self, key: &str, value: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )
        .map_err(|e| format!("Failed to set setting: {}", e))?;
        Ok(())
    }

    pub fn delete(&self, key: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM app_settings WHERE key = ?1", params![key])
            .map_err(|e| format!("Failed to delete setting: {}", e))?;
        Ok(())
    }

    pub fn get_all(&self) -> Result<Vec<SettingEntry>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT key, value FROM app_settings ORDER BY key")
            .map_err(|e| format!("Failed to prepare: {}", e))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(SettingEntry {
                    key: row.get(0)?,
                    value: row.get(1)?,
                })
            })
            .map_err(|e| format!("Failed to query: {}", e))?;
        let mut entries = Vec::new();
        for row in rows {
            entries.push(row.map_err(|e| format!("Failed to read row: {}", e))?);
        }
        Ok(entries)
    }

    pub fn clear(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute_batch("DELETE FROM app_settings")
            .map_err(|e| format!("Failed to clear settings: {}", e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store() -> SettingsStore {
        let conn = Connection::open_in_memory().unwrap();
        let store = SettingsStore {
            conn: Mutex::new(conn),
        };
        store.init_tables().unwrap();
        store
    }

    #[test]
    fn test_get_set() {
        let store = test_store();
        assert_eq!(store.get("theme").unwrap(), None);

        store.set("theme", "dark").unwrap();
        assert_eq!(store.get("theme").unwrap(), Some("dark".to_string()));
    }

    #[test]
    fn test_get_nonexistent() {
        let store = test_store();
        assert!(store.get("no-such-key").unwrap().is_none());
    }

    #[test]
    fn test_overwrite() {
        let store = test_store();
        store.set("ping_count", "4").unwrap();
        store.set("ping_count", "10").unwrap();
        assert_eq!(store.get("ping_count").unwrap(), Some("10".to_string()));
    }

    #[test]
    fn test_delete() {
        let store = test_store();
        store.set("key1", "val1").unwrap();
        store.set("key2", "val2").unwrap();
        assert_eq!(store.get_all().unwrap().len(), 2);

        store.delete("key1").unwrap();
        let all = store.get_all().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].key, "key2");
    }

    #[test]
    fn test_delete_nonexistent() {
        let store = test_store();
        store.delete("no-such-key").unwrap();
    }

    #[test]
    fn test_get_all() {
        let store = test_store();
        assert!(store.get_all().unwrap().is_empty());

        store.set("a", "1").unwrap();
        store.set("b", "2").unwrap();
        let all = store.get_all().unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_clear() {
        let store = test_store();
        store.set("a", "1").unwrap();
        store.set("b", "2").unwrap();
        assert_eq!(store.get_all().unwrap().len(), 2);

        store.clear().unwrap();
        assert!(store.get_all().unwrap().is_empty());
    }
}
