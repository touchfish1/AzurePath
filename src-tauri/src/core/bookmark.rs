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

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helpers ──────────────────────────────────────────────────

    fn test_store() -> BookmarkStore {
        let conn = Connection::open_in_memory().unwrap();
        let store = BookmarkStore { conn };
        store.init_tables().unwrap();
        store
    }

    fn sample_bookmark(id: &str) -> Bookmark {
        Bookmark {
            id: id.to_string(),
            label: "My Server".to_string(),
            target: "192.168.1.100".to_string(),
            tags: vec!["web".to_string(), "prod".to_string()],
            created_at: "2025-01-01T00:00:00Z".to_string(),
        }
    }

    // ── Bookmark struct ──────────────────────────────────────────

    #[test]
    fn test_bookmark_creation() {
        let bm = sample_bookmark("id-1");
        assert_eq!(bm.id, "id-1");
        assert_eq!(bm.label, "My Server");
        assert_eq!(bm.target, "192.168.1.100");
        assert_eq!(bm.tags, vec!["web", "prod"]);
        assert_eq!(bm.created_at, "2025-01-01T00:00:00Z");
    }

    #[test]
    fn test_bookmark_camel_case_serialization() {
        let bm = sample_bookmark("ser-1");
        let json = serde_json::to_string(&bm).unwrap();
        assert!(
            json.contains(r#""createdAt""#),
            "Expected camelCase 'createdAt' in JSON, got: {}",
            json
        );
        assert!(
            !json.contains(r#""created_at""#),
            "Unexpected snake_case 'created_at' in JSON: {}",
            json
        );
    }

    #[test]
    fn test_bookmark_camel_case_deserialization() {
        let json = r#"{
            "id": "ds-1",
            "label": "Deser",
            "target": "10.0.0.1",
            "tags": ["a", "b"],
            "createdAt": "2025-06-01T00:00:00Z"
        }"#;
        let bm: Bookmark = serde_json::from_str(json).unwrap();
        assert_eq!(bm.id, "ds-1");
        assert_eq!(bm.label, "Deser");
        assert_eq!(bm.tags, vec!["a", "b"]);
        assert_eq!(bm.created_at, "2025-06-01T00:00:00Z");
    }

    #[test]
    fn test_bookmark_serde_roundtrip() {
        let bm = sample_bookmark("rt-1");
        let json = serde_json::to_string(&bm).unwrap();
        let back: Bookmark = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, bm.id);
        assert_eq!(back.label, bm.label);
        assert_eq!(back.target, bm.target);
        assert_eq!(back.tags, bm.tags);
        assert_eq!(back.created_at, bm.created_at);
    }

    #[test]
    fn test_bookmark_derived_traits() {
        let bm = sample_bookmark("tr-1");
        let cloned = bm.clone();
        assert_eq!(cloned.id, bm.id);
        let debug = format!("{:?}", bm);
        assert!(debug.contains("My Server"));
    }

    // ── BookmarkStore CRUD ───────────────────────────────────────

    #[test]
    fn test_store_list_empty() {
        let store = test_store();
        let items = store.list_all().unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn test_store_add_single() {
        let store = test_store();
        store.add(&sample_bookmark("a1")).unwrap();
        let items = store.list_all().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "a1");
        assert_eq!(items[0].label, "My Server");
    }

    #[test]
    fn test_store_add_multiple() {
        let store = test_store();
        store.add(&sample_bookmark("a")).unwrap();
        store.add(&sample_bookmark("b")).unwrap();
        store.add(&sample_bookmark("c")).unwrap();
        assert_eq!(store.list_all().unwrap().len(), 3);
    }

    #[test]
    fn test_store_list_order_desc_by_created_at() {
        let store = test_store();
        store
            .add(&Bookmark {
                id: "old".into(),
                label: "Older".into(),
                target: "10.0.0.1".into(),
                tags: vec![],
                created_at: "2024-01-01T00:00:00Z".into(),
            })
            .unwrap();
        store
            .add(&Bookmark {
                id: "new".into(),
                label: "Newer".into(),
                target: "10.0.0.2".into(),
                tags: vec![],
                created_at: "2025-01-01T00:00:00Z".into(),
            })
            .unwrap();
        let items = store.list_all().unwrap();
        assert_eq!(items[0].id, "new");
        assert_eq!(items[1].id, "old");
    }

    #[test]
    fn test_store_delete_existing() {
        let store = test_store();
        store.add(&sample_bookmark("del-me")).unwrap();
        store.delete("del-me").unwrap();
        assert!(store.list_all().unwrap().is_empty());
    }

    #[test]
    fn test_store_delete_nonexistent_is_noop() {
        let store = test_store();
        store.add(&sample_bookmark("a")).unwrap();
        store.delete("no-such-id").unwrap();
        assert_eq!(store.list_all().unwrap().len(), 1);
    }

    #[test]
    fn test_store_delete_one_of_many() {
        let store = test_store();
        store.add(&sample_bookmark("keep")).unwrap();
        store.add(&sample_bookmark("remove")).unwrap();
        store.delete("remove").unwrap();
        let items = store.list_all().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "keep");
    }

    #[test]
    fn test_store_add_after_delete() {
        let store = test_store();
        store.add(&sample_bookmark("tmp")).unwrap();
        store.delete("tmp").unwrap();
        assert!(store.list_all().unwrap().is_empty());
        store.add(&sample_bookmark("new")).unwrap();
        assert_eq!(store.list_all().unwrap().len(), 1);
    }

    // ── Edge cases ───────────────────────────────────────────────

    #[test]
    fn test_store_empty_tags() {
        let store = test_store();
        let bm = Bookmark {
            id: "no-tags".into(),
            label: "Empty Tags".into(),
            target: "10.0.0.1".into(),
            tags: vec![],
            created_at: "2025-01-01T00:00:00Z".into(),
        };
        store.add(&bm).unwrap();
        assert!(store.list_all().unwrap()[0].tags.is_empty());
    }

    #[test]
    fn test_store_many_tags() {
        let store = test_store();
        let tags: Vec<String> = (0..100).map(|i| format!("tag-{}", i)).collect();
        let bm = Bookmark {
            id: "many-tags".into(),
            label: "Many Tags".into(),
            target: "10.0.0.1".into(),
            tags,
            created_at: "2025-01-01T00:00:00Z".into(),
        };
        store.add(&bm).unwrap();
        assert_eq!(store.list_all().unwrap()[0].tags.len(), 100);
    }

    #[test]
    fn test_store_special_chars_in_label_and_target() {
        let store = test_store();
        let bm = Bookmark {
            id: "special-1".into(),
            label: "It's a \"nice\" server & more".into(),
            target: "https://example.com/path?q=1&r=2#frag".into(),
            tags: vec!["tag-a".into(), "tag,b".into(), "tag:c".into()],
            created_at: "2025-01-01T00:00:00Z".into(),
        };
        store.add(&bm).unwrap();
        let items = store.list_all().unwrap();
        assert_eq!(items[0].label, "It's a \"nice\" server & more");
        assert_eq!(
            items[0].target,
            "https://example.com/path?q=1&r=2#frag"
        );
        assert_eq!(items[0].tags, vec!["tag-a", "tag,b", "tag:c"]);
    }

    #[test]
    fn test_store_unicode_text() {
        let store = test_store();
        let bm = Bookmark {
            id: "uni-1".into(),
            label: "服务器".into(),
            target: "例子.测试".into(),
            tags: vec!["中文".into(), "标签".into()],
            created_at: "2025-01-01T00:00:00Z".into(),
        };
        store.add(&bm).unwrap();
        let items = store.list_all().unwrap();
        assert_eq!(items[0].label, "服务器");
        assert_eq!(items[0].target, "例子.测试");
        assert_eq!(items[0].tags, vec!["中文", "标签"]);
    }

    #[test]
    fn test_store_long_values() {
        let store = test_store();
        let long_label = "X".repeat(500);
        let long_target = "Y".repeat(1000);
        let bm = Bookmark {
            id: "long-1".into(),
            label: long_label.clone(),
            target: long_target.clone(),
            tags: vec![],
            created_at: "2025-01-01T00:00:00Z".into(),
        };
        store.add(&bm).unwrap();
        let items = store.list_all().unwrap();
        assert_eq!(items[0].label.len(), 500);
        assert_eq!(items[0].target.len(), 1000);
    }

    #[test]
    fn test_store_duplicate_id_fails() {
        let store = test_store();
        store.add(&sample_bookmark("dup")).unwrap();
        let dup = Bookmark {
            id: "dup".into(),
            label: "Duplicate".into(),
            target: "10.0.0.1".into(),
            tags: vec![],
            created_at: "2025-06-01T00:00:00Z".into(),
        };
        let result = store.add(&dup);
        assert!(result.is_err());
    }

    #[test]
    fn test_store_tags_with_empty_string() {
        let store = test_store();
        let bm = Bookmark {
            id: "empty-tag".into(),
            label: "Empty Tag".into(),
            target: "10.0.0.1".into(),
            tags: vec!["".into()],
            created_at: "2025-01-01T00:00:00Z".into(),
        };
        store.add(&bm).unwrap();
        let items = store.list_all().unwrap();
        assert_eq!(items[0].tags, vec![""]);
    }

    #[test]
    fn test_store_whitespace_preserved() {
        let store = test_store();
        let bm = Bookmark {
            id: "ws-1".into(),
            label: "  spaces all around  ".into(),
            target: "  192.168.1.1  ".into(),
            tags: vec![" prod ".into()],
            created_at: "2025-01-01T00:00:00Z".into(),
        };
        store.add(&bm).unwrap();
        let items = store.list_all().unwrap();
        assert_eq!(items[0].label, "  spaces all around  ");
        assert_eq!(items[0].target, "  192.168.1.1  ");
        assert_eq!(items[0].tags, vec![" prod "]);
    }

    #[test]
    fn test_store_init_tables_is_idempotent() {
        let store = test_store();
        // Calling init_tables again should not error
        store.init_tables().unwrap();
        store.init_tables().unwrap();
        store.add(&sample_bookmark("idem")).unwrap();
        assert_eq!(store.list_all().unwrap().len(), 1);
    }
}
