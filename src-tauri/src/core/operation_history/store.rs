use crate::types::operation_history::OperationRecord;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::Mutex;

const DEFAULT_MAX_ENTRIES: u32 = 500;

pub struct HistoryStore {
    conn: Mutex<Connection>,
    max_entries: Mutex<u32>,
}

impl HistoryStore {
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
            max_entries: Mutex::new(DEFAULT_MAX_ENTRIES),
        };
        store.init_tables()?;
        Ok(store)
    }

    #[cfg(test)]
    pub fn test_store() -> Self {
        let conn = Connection::open_in_memory().unwrap();
        let store = Self {
            conn: Mutex::new(conn),
            max_entries: Mutex::new(DEFAULT_MAX_ENTRIES),
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
            CREATE TABLE IF NOT EXISTS operation_history (
                id          TEXT PRIMARY KEY,
                op_type     TEXT NOT NULL,
                target      TEXT NOT NULL,
                status      TEXT NOT NULL,
                summary     TEXT NOT NULL DEFAULT '',
                result_meta TEXT NOT NULL DEFAULT '{}',
                created_at  TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_op_history_created ON operation_history(created_at);
            CREATE INDEX IF NOT EXISTS idx_op_history_type ON operation_history(op_type);
            ",
        )
        .map_err(|e| format!("Failed to init history tables: {}", e))?;
        Ok(())
    }

    pub fn insert(&self, entry: &OperationRecord) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO operation_history (id, op_type, target, status, summary, result_meta, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                entry.id,
                entry.op_type,
                entry.target,
                entry.status,
                entry.summary,
                serde_json::to_string(&entry.result_meta).unwrap_or_default(),
                entry.created_at,
            ],
        )
        .map_err(|e| format!("Failed to insert history entry: {}", e))?;

        drop(conn);
        let _ = self.evict_old();
        Ok(())
    }

    pub fn list(
        &self,
        op_type: Option<&str>,
        search: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<OperationRecord>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        let mut conditions: Vec<String> = Vec::new();
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(t) = op_type {
            conditions.push(format!("op_type = ?{}", param_values.len() + 1));
            param_values.push(Box::new(t.to_string()));
        }
        if let Some(s) = search {
            if !s.is_empty() {
                let escaped = s
                    .replace("\\", "\\\\")
                    .replace("%", "\\%")
                    .replace("_", "\\_");
                conditions.push(format!(
                    "target LIKE ?{} ESCAPE '\\'",
                    param_values.len() + 1
                ));
                param_values.push(Box::new(format!("%{}%", escaped)));
            }
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let sql = format!(
            "SELECT id, op_type, target, status, summary, result_meta, created_at
             FROM operation_history
             {}
             ORDER BY created_at DESC
             LIMIT ?{} OFFSET ?{}",
            where_clause,
            param_values.len() + 1,
            param_values.len() + 2,
        );

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| format!("Failed to prepare: {}", e))?;

        let params_refs: Vec<&dyn rusqlite::types::ToSql> = param_values
            .iter()
            .map(|p| p.as_ref())
            .collect();
        let limit_param = rusqlite::types::Value::Integer(limit as i64);
        let offset_param = rusqlite::types::Value::Integer(offset as i64);

        let mut all_params = params_refs;
        all_params.push(&limit_param);
        all_params.push(&offset_param);

        let rows = stmt
            .query_map(all_params.as_slice(), Self::map_row)
            .map_err(|e| format!("Failed to query: {}", e))?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row.map_err(|e| format!("Failed to read row: {}", e))?);
        }
        Ok(entries)
    }

    pub fn get_by_id(&self, id: &str) -> Result<Option<OperationRecord>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, op_type, target, status, summary, result_meta, created_at
                 FROM operation_history WHERE id = ?1",
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
        conn.execute("DELETE FROM operation_history WHERE id = ?1", params![id])
            .map_err(|e| format!("Failed to delete: {}", e))?;
        Ok(())
    }

    pub fn clear(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute_batch("DELETE FROM operation_history")
            .map_err(|e| format!("Failed to clear: {}", e))?;
        Ok(())
    }

    pub fn count(&self) -> Result<u32, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let count: u32 = conn
            .query_row("SELECT COUNT(*) FROM operation_history", [], |row| {
                row.get(0)
            })
            .map_err(|e| format!("Failed to count: {}", e))?;
        Ok(count)
    }

    pub fn set_max_entries(&self, count: u32) -> Result<(), String> {
        let mut max = self.max_entries.lock().map_err(|e| e.to_string())?;
        *max = count;
        Ok(())
    }

    pub fn get_max_entries(&self) -> Result<u32, String> {
        let max = self.max_entries.lock().map_err(|e| e.to_string())?;
        Ok(*max)
    }

    fn evict_old(&self) -> Result<(), String> {
        let max = {
            let m = self.max_entries.lock().map_err(|e| e.to_string())?;
            *m
        };
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "DELETE FROM operation_history WHERE id IN (
                SELECT id FROM operation_history
                ORDER BY created_at ASC
                LIMIT MAX(0, (SELECT COUNT(*) - ?1 FROM operation_history))
            )",
            params![max],
        )
        .map_err(|e| format!("Failed to evict old entries: {}", e))?;
        Ok(())
    }

    fn map_row(row: &rusqlite::Row) -> rusqlite::Result<OperationRecord> {
        let meta_str: String = row.get(5)?;
        let result_meta = serde_json::from_str(&meta_str).unwrap_or(serde_json::Value::Null);
        Ok(OperationRecord {
            id: row.get(0)?,
            op_type: row.get(1)?,
            target: row.get(2)?,
            status: row.get(3)?,
            summary: row.get(4)?,
            result_meta,
            created_at: row.get(6)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_store() -> HistoryStore {
        let conn = Connection::open_in_memory().unwrap();
        let store = HistoryStore {
            conn: Mutex::new(conn),
            max_entries: Mutex::new(DEFAULT_MAX_ENTRIES),
        };
        store.init_tables().unwrap();
        store
    }

    fn make_record(
        op_type: &str,
        target: &str,
        status: &str,
        summary: &str,
        meta: serde_json::Value,
    ) -> OperationRecord {
        OperationRecord {
            id: uuid::Uuid::new_v4().to_string(),
            op_type: op_type.to_string(),
            target: target.to_string(),
            status: status.to_string(),
            summary: summary.to_string(),
            result_meta: meta,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    #[test]
    fn test_insert_and_list() {
        let store = test_store();
        let r = make_record("ping", "8.8.8.8", "success", "4/4 packets, 0% loss", json!({"sent": 4}));
        store.insert(&r).unwrap();

        let entries = store.list(None, None, 100, 0).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].op_type, "ping");
        assert_eq!(entries[0].target, "8.8.8.8");
        assert_eq!(entries[0].summary, "4/4 packets, 0% loss");
    }

    #[test]
    fn test_insert_and_get_by_id() {
        let store = test_store();
        let r = make_record("dns", "example.com", "success", "2 records", json!({}));
        store.insert(&r).unwrap();

        let found = store.get_by_id(&r.id).unwrap().expect("should exist");
        assert_eq!(found.target, "example.com");
    }

    #[test]
    fn test_get_by_id_nonexistent() {
        let store = test_store();
        assert!(store.get_by_id("no-such-id").unwrap().is_none());
    }

    #[test]
    fn test_list_with_type_filter() {
        let store = test_store();
        store.insert(&make_record("ping", "8.8.8.8", "success", "", json!({}))).unwrap();
        store.insert(&make_record("dns", "example.com", "success", "", json!({}))).unwrap();
        store.insert(&make_record("ping", "1.1.1.1", "success", "", json!({}))).unwrap();

        let pings = store.list(Some("ping"), None, 100, 0).unwrap();
        assert_eq!(pings.len(), 2);

        let dns = store.list(Some("dns"), None, 100, 0).unwrap();
        assert_eq!(dns.len(), 1);

        let empty = store.list(Some("sniffer"), None, 100, 0).unwrap();
        assert_eq!(empty.len(), 0);
    }

    #[test]
    fn test_list_with_search() {
        let store = test_store();
        store.insert(&make_record("ping", "8.8.8.8", "success", "", json!({}))).unwrap();
        store.insert(&make_record("ping", "1.1.1.1", "success", "", json!({}))).unwrap();
        store.insert(&make_record("dns", "google.com", "success", "", json!({}))).unwrap();

        let results = store.list(None, Some("8.8"), 100, 0).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].target, "8.8.8.8");
    }

    #[test]
    fn test_list_empty_search_returns_all() {
        let store = test_store();
        store.insert(&make_record("ping", "8.8.8.8", "success", "", json!({}))).unwrap();
        store.insert(&make_record("dns", "google.com", "success", "", json!({}))).unwrap();

        let results = store.list(None, Some(""), 100, 0).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_list_offset_and_limit() {
        let store = test_store();
        for i in 0..10 {
            store
                .insert(&make_record("ping", &format!("10.0.0.{}", i), "success", "", json!({})))
                .unwrap();
        }

        let page1 = store.list(None, None, 3, 0).unwrap();
        assert_eq!(page1.len(), 3);

        let page2 = store.list(None, None, 3, 3).unwrap();
        assert_eq!(page2.len(), 3);

        let all = store.list(None, None, 100, 0).unwrap();
        assert_eq!(all.len(), 10);
    }

    #[test]
    fn test_delete() {
        let store = test_store();
        let r = make_record("ping", "8.8.8.8", "success", "", json!({}));
        store.insert(&r).unwrap();
        assert_eq!(store.count().unwrap(), 1);

        store.delete(&r.id).unwrap();
        assert_eq!(store.count().unwrap(), 0);
    }

    #[test]
    fn test_delete_nonexistent() {
        let store = test_store();
        store.delete("no-such-id").unwrap();
    }

    #[test]
    fn test_clear() {
        let store = test_store();
        store.insert(&make_record("ping", "a", "success", "", json!({}))).unwrap();
        store.insert(&make_record("dns", "b", "success", "", json!({}))).unwrap();
        assert_eq!(store.count().unwrap(), 2);

        store.clear().unwrap();
        assert_eq!(store.count().unwrap(), 0);
    }

    #[test]
    fn test_count() {
        let store = test_store();
        assert_eq!(store.count().unwrap(), 0);
        store.insert(&make_record("ping", "a", "success", "", json!({}))).unwrap();
        assert_eq!(store.count().unwrap(), 1);
        store.insert(&make_record("dns", "b", "success", "", json!({}))).unwrap();
        assert_eq!(store.count().unwrap(), 2);
    }

    #[test]
    fn test_set_max_entries() {
        let store = test_store();
        assert_eq!(store.get_max_entries().unwrap(), 500);
        store.set_max_entries(100).unwrap();
        assert_eq!(store.get_max_entries().unwrap(), 100);
    }

    #[test]
    fn test_evict_old_basic() {
        let store = test_store();
        store.set_max_entries(5).unwrap();

        for i in 0..10 {
            store
                .insert(&make_record(
                    "ping",
                    &format!("10.0.0.{}", i),
                    "success",
                    "",
                    json!({}),
                ))
                .unwrap();
        }

        let count = store.count().unwrap();
        assert!(count <= 5, "expected <= 5 entries after eviction, got {count}");
    }

    #[test]
    fn test_result_meta_json_roundtrip() {
        let store = test_store();
        let meta = json!({
            "sent": 4,
            "received": 4,
            "min_ms": 10.5,
            "max_ms": 20.3,
            "avg_ms": 15.2,
            "open_ports": [80, 443],
        });
        let r = make_record("ping", "8.8.8.8", "success", "4/4", meta.clone());
        store.insert(&r).unwrap();

        let found = store.get_by_id(&r.id).unwrap().unwrap();
        assert_eq!(found.result_meta, meta);
    }

    #[test]
    fn test_evict_old_zero_max_preserves_nothing() {
        let store = test_store();
        store.set_max_entries(0).unwrap();
        store
            .insert(&make_record("ping", "10.0.0.1", "success", "", json!({})))
            .unwrap();
        assert_eq!(store.count().unwrap(), 0);
    }

    #[test]
    fn test_sql_injection_via_search() {
        let store = test_store();
        let r = make_record(
            "ping",
            "'; DROP TABLE operation_history; --",
            "success",
            "",
            json!({}),
        );
        store.insert(&r).unwrap();

        let results = store.list(None, Some("'; DROP"), 100, 0).unwrap();
        assert_eq!(results.len(), 1);

        assert_eq!(store.count().unwrap(), 1);
    }
}
