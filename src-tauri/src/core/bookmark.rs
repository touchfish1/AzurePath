use rusqlite::Connection;
use serde::{Serialize, Deserialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bookmark {
    pub id: String,
    pub label: String,
    pub target: String,
    pub tags: Vec<String>,
    pub created_at: String,
}

pub struct BookmarkStore {
    conn: Connection,
}

impl BookmarkStore {
    pub fn new() -> Result<Self, String> {
        let db_path = Self::db_path()?;
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
        let store = Self { conn };
        store.init_tables()?;
        Ok(store)
    }

    fn db_path() -> Result<PathBuf, String> {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map_err(|_| "No home dir".to_string())?;
        Ok(PathBuf::from(home).join("AzurePath/bookmarks.db"))
    }

    fn init_tables(&self) -> Result<(), String> {
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS bookmarks (
                id TEXT PRIMARY KEY,
                label TEXT NOT NULL,
                target TEXT NOT NULL,
                tags TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL
            );",
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_all(&self) -> Result<Vec<Bookmark>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, label, target, tags, created_at FROM bookmarks ORDER BY created_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let items = stmt
            .query_map([], |row| {
                let tags_str: String = row.get(3)?;
                let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
                Ok(Bookmark {
                    id: row.get(0)?,
                    label: row.get(1)?,
                    target: row.get(2)?,
                    tags,
                    created_at: row.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        Ok(items)
    }

    pub fn add(&self, bookmark: &Bookmark) -> Result<(), String> {
        let tags_json = serde_json::to_string(&bookmark.tags).map_err(|e| e.to_string())?;
        self.conn
            .execute(
                "INSERT INTO bookmarks (id, label, target, tags, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![bookmark.id, bookmark.label, bookmark.target, tags_json, bookmark.created_at],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        self.conn
            .execute("DELETE FROM bookmarks WHERE id = ?1", [id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
