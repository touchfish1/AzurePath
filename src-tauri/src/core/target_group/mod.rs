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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::TempDir;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Sets HOME and USERPROFILE to a temp directory, restores on drop.
    struct EnvGuard {
        old_home: Option<String>,
        old_profile: Option<String>,
        _temp_dir: TempDir,
    }

    impl EnvGuard {
        fn new() -> Self {
            let dir = TempDir::new().expect("temp dir");
            let old_home = std::env::var("HOME").ok();
            let old_profile = std::env::var("USERPROFILE").ok();
            std::env::set_var("HOME", dir.path());
            std::env::set_var("USERPROFILE", dir.path());
            EnvGuard {
                old_home,
                old_profile,
                _temp_dir: dir,
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            if let Some(ref h) = self.old_home {
                std::env::set_var("HOME", h);
            } else {
                std::env::remove_var("HOME");
            }
            if let Some(ref p) = self.old_profile {
                std::env::set_var("USERPROFILE", p);
            } else {
                std::env::remove_var("USERPROFILE");
            }
        }
    }

    fn create_store() -> TargetGroupStore {
        TargetGroupStore::new().expect("Failed to create TargetGroupStore")
    }

    fn make_group(name: &str, targets: &[&str]) -> TargetGroup {
        TargetGroup {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            targets: targets.iter().map(|s| s.to_string()).collect(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    // ── CRUD tests ─────────────────────────────────────────────────

    #[test]
    fn test_create_and_list() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new();
        let store = create_store();

        let group = make_group("servers", &["192.168.1.1"]);
        store.save_group(&group).unwrap();

        let list = store.list_groups().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "servers");
        assert_eq!(list[0].targets, vec!["192.168.1.1"]);
    }

    #[test]
    fn test_get_group_by_id() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new();
        let store = create_store();

        let group = make_group("web-servers", &["10.0.0.1", "10.0.0.2"]);
        store.save_group(&group).unwrap();

        let found = store.get_group(&group.id).unwrap();
        assert!(found.is_some());
        let g = found.unwrap();
        assert_eq!(g.name, "web-servers");
        assert_eq!(g.targets, vec!["10.0.0.1", "10.0.0.2"]);
    }

    #[test]
    fn test_get_group_not_found() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new();
        let store = create_store();

        let found = store.get_group("non-existent-id").unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn test_update_group() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new();
        let store = create_store();

        let mut group = make_group("old-name", &["10.0.0.1"]);
        store.save_group(&group).unwrap();

        // Update fields
        group.name = "new-name".to_string();
        group.targets = vec!["10.0.0.2".to_string()];
        group.updated_at = chrono::Utc::now().to_rfc3339();
        store.save_group(&group).unwrap();

        let found = store.get_group(&group.id).unwrap().unwrap();
        assert_eq!(found.name, "new-name");
        assert_eq!(found.targets, vec!["10.0.0.2"]);
    }

    #[test]
    fn test_delete_group() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new();
        let store = create_store();

        let group = make_group("delete-me", &["10.0.0.1"]);
        store.save_group(&group).unwrap();
        assert_eq!(store.list_groups().unwrap().len(), 1);

        store.delete_group(&group.id).unwrap();
        assert!(store.list_groups().unwrap().is_empty());
    }

    #[test]
    fn test_delete_nonexistent_is_ok() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new();
        let store = create_store();

        // Deleting a non-existent ID should not error
        let result = store.delete_group("no-such-id");
        assert!(result.is_ok());
    }

    #[test]
    fn test_delete_and_recreate() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new();
        let store = create_store();

        let g1 = make_group("temp", &["10.0.0.1"]);
        store.save_group(&g1).unwrap();
        store.delete_group(&g1.id).unwrap();
        assert!(store.list_groups().unwrap().is_empty());

        let g2 = make_group("new", &["10.0.0.2"]);
        store.save_group(&g2).unwrap();
        assert_eq!(store.list_groups().unwrap().len(), 1);
    }

    // ── Multiple targets ───────────────────────────────────────────

    #[test]
    fn test_multiple_targets() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new();
        let store = create_store();

        let ips = vec!["192.168.1.1", "10.0.0.1", "172.16.0.1", "8.8.8.8"];
        let group = make_group("multi", &ips);
        store.save_group(&group).unwrap();

        let found = store.get_group(&group.id).unwrap().unwrap();
        assert_eq!(found.targets.len(), 4);
        assert_eq!(found.targets, ips);
    }

    #[test]
    fn test_empty_targets() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new();
        let store = create_store();

        let group = make_group("empty-targets", &[]);
        store.save_group(&group).unwrap();

        let found = store.get_group(&group.id).unwrap().unwrap();
        assert!(found.targets.is_empty());
    }

    // ── List ordering ──────────────────────────────────────────────

    #[test]
    fn test_list_order_desc_by_updated_at() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new();
        let store = create_store();

        let g1 = make_group("first", &[]);
        store.save_group(&g1).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));

        let g2 = make_group("second", &[]);
        store.save_group(&g2).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));

        let g3 = make_group("third", &[]);
        store.save_group(&g3).unwrap();

        let list = store.list_groups().unwrap();
        assert_eq!(list.len(), 3);
        // Should be ordered by updated_at DESC: third, second, first
        assert_eq!(list[0].name, "third");
        assert_eq!(list[1].name, "second");
        assert_eq!(list[2].name, "first");
    }

    // ── Boundary tests ─────────────────────────────────────────────

    #[test]
    fn test_unicode_name() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new();
        let store = create_store();

        let group = make_group("中文组名", &["192.168.1.1"]);
        store.save_group(&group).unwrap();

        let found = store.get_group(&group.id).unwrap().unwrap();
        assert_eq!(found.name, "中文组名");
    }

    #[test]
    fn test_empty_name() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new();
        let store = create_store();

        let group = make_group("", &["10.0.0.1"]);
        store.save_group(&group).unwrap();
        let found = store.get_group(&group.id).unwrap().unwrap();
        assert_eq!(found.name, "");
    }

    #[test]
    fn test_sql_injection_in_name() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new();
        let store = create_store();

        let group = make_group("'; DROP TABLE target_groups; --", &["10.0.0.1"]);
        store.save_group(&group).unwrap();

        // The group should be saved safely (SQL injection prevented by parameterized queries)
        let found = store.get_group(&group.id).unwrap().unwrap();
        assert_eq!(found.name, "'; DROP TABLE target_groups; --");

        // Listing should still work and other data should be intact
        let list = store.list_groups().unwrap();
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn test_long_name() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new();
        let store = create_store();

        let long_name = "a".repeat(1000);
        let group = make_group(&long_name, &["10.0.0.1"]);
        store.save_group(&group).unwrap();

        let found = store.get_group(&group.id).unwrap().unwrap();
        assert_eq!(found.name.len(), 1000);
        assert_eq!(found.name, long_name);
    }

    #[test]
    fn test_long_targets() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new();
        let store = create_store();

        let targets: Vec<String> = (0..100).map(|i| format!("10.0.{}.{}", i / 256, i % 256)).collect();
        let target_refs: Vec<&str> = targets.iter().map(|s| s.as_str()).collect();
        let group = make_group("many-targets", &target_refs);
        store.save_group(&group).unwrap();

        let found = store.get_group(&group.id).unwrap().unwrap();
        assert_eq!(found.targets.len(), 100);
        assert_eq!(found.targets, targets);
    }

    #[test]
    fn test_special_chars_in_targets() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new();
        let store = create_store();

        let group = make_group("special", &["192.168.1.1", "hostname.local", "2001:db8::1"]);
        store.save_group(&group).unwrap();

        let found = store.get_group(&group.id).unwrap().unwrap();
        assert_eq!(found.targets, vec!["192.168.1.1", "hostname.local", "2001:db8::1"]);
    }

    // ── Full workflow ──────────────────────────────────────────────

    #[test]
    fn test_full_workflow() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new();
        let store = create_store();

        // Start empty
        assert!(store.list_groups().unwrap().is_empty());

        // Create three groups
        let g1 = make_group("Web", &["192.168.1.10"]);
        let g2 = make_group("DB", &["192.168.1.20", "192.168.1.21"]);
        let g3 = make_group("DNS", &["8.8.8.8"]);
        store.save_group(&g1).unwrap();
        store.save_group(&g2).unwrap();
        store.save_group(&g3).unwrap();
        assert_eq!(store.list_groups().unwrap().len(), 3);

        // Get by ID
        let found = store.get_group(&g2.id).unwrap().unwrap();
        assert_eq!(found.name, "DB");
        assert_eq!(found.targets, vec!["192.168.1.20", "192.168.1.21"]);

        // Update
        let mut updated = g3.clone();
        updated.name = "Google DNS".to_string();
        updated.targets = vec!["8.8.8.8".to_string(), "8.8.4.4".to_string()];
        store.save_group(&updated).unwrap();

        let found = store.get_group(&g3.id).unwrap().unwrap();
        assert_eq!(found.name, "Google DNS");
        assert_eq!(found.targets, vec!["8.8.8.8", "8.8.4.4"]);

        // Delete middle group
        store.delete_group(&g2.id).unwrap();
        let list = store.list_groups().unwrap();
        assert_eq!(list.len(), 2);

        let ids: Vec<&str> = list.iter().map(|g| g.id.as_str()).collect();
        assert!(ids.contains(&g1.id.as_str()));
        assert!(ids.contains(&g3.id.as_str()));
        assert!(!ids.contains(&g2.id.as_str()));

        // Delete remaining
        store.delete_group(&g1.id).unwrap();
        store.delete_group(&g3.id).unwrap();
        assert!(store.list_groups().unwrap().is_empty());
    }
}
