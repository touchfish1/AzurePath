use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetGroup {
    pub id: String,
    pub name: String,
    pub targets: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct TargetGroupStore {
    conn: Connection,
}

impl TargetGroupStore {
    pub fn new() -> Result<Self, String> {
        let db_path = Self::db_path()?;
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create dir: {}", e))?;
        }
        let conn = Connection::open(&db_path).map_err(|e| format!("Failed to open DB: {}", e))?;
        let store = Self { conn };
        store.init_tables()?;
        Ok(store)
    }

    fn db_path() -> Result<PathBuf, String> {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map_err(|_| "No home dir".to_string())?;
        Ok(PathBuf::from(home).join("AzurePath/target_groups.db"))
    }

    fn init_tables(&self) -> Result<(), String> {
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS target_groups (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                targets TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );",
            )
            .map_err(|e| format!("DB init error: {}", e))?;
        Ok(())
    }

    pub fn list_groups(&self) -> Result<Vec<TargetGroup>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, name, targets, created_at, updated_at FROM target_groups ORDER BY updated_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let groups = stmt
            .query_map([], |row| {
                let targets_str: String = row.get(2)?;
                let targets: Vec<String> =
                    serde_json::from_str(&targets_str).unwrap_or_default();
                Ok(TargetGroup {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    targets,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        Ok(groups)
    }

    pub fn get_group(&self, id: &str) -> Result<Option<TargetGroup>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, name, targets, created_at, updated_at FROM target_groups WHERE id = ?1",
            )
            .map_err(|e| e.to_string())?;
        let mut rows = stmt
            .query_map([id], |row| {
                let targets_str: String = row.get(2)?;
                let targets: Vec<String> =
                    serde_json::from_str(&targets_str).unwrap_or_default();
                Ok(TargetGroup {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    targets,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?;
        Ok(rows.next().and_then(|r| r.ok()))
    }

    pub fn save_group(&self, group: &TargetGroup) -> Result<(), String> {
        let targets_json =
            serde_json::to_string(&group.targets).map_err(|e| e.to_string())?;
        self.conn
            .execute(
                "INSERT OR REPLACE INTO target_groups (id, name, targets, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![group.id, group.name, targets_json, group.created_at, group.updated_at],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_group(&self, id: &str) -> Result<(), String> {
        self.conn
            .execute("DELETE FROM target_groups WHERE id = ?1", [id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
