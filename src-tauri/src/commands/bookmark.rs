use crate::core::bookmark::{Bookmark, BookmarkStore};
use chrono::Utc;
use uuid::Uuid;

#[tauri::command]
pub fn list_bookmarks() -> Result<Vec<Bookmark>, String> {
    BookmarkStore::new()?.list_all()
}

#[tauri::command]
pub fn add_bookmark(label: String, target: String, tags: Vec<String>) -> Result<Bookmark, String> {
    let bm = Bookmark {
        id: Uuid::new_v4().to_string(),
        label,
        target,
        tags,
        created_at: Utc::now().to_rfc3339(),
    };
    BookmarkStore::new()?.add(&bm)?;
    Ok(bm)
}

#[tauri::command]
pub fn delete_bookmark(id: String) -> Result<(), String> {
    BookmarkStore::new()?.delete(&id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::TempDir;

    // Serialize env-dependent tests since they mutate process-global HOME/USERPROFILE.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Sets HOME and USERPROFILE to a temporary directory and restores them on drop.
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

    // ── add_bookmark ─────────────────────────────────────────────

    #[test]
    fn test_add_bookmark_creates_valid_uuid() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new();

        let result = add_bookmark("Test".into(), "192.168.1.1".into(), vec![]);
        assert!(result.is_ok());
        let bm = result.unwrap();
        assert_eq!(bm.id.len(), 36);
        assert_eq!(bm.id.chars().filter(|&c| c == '-').count(), 4);
        assert!(uuid::Uuid::parse_str(&bm.id).is_ok());
    }

    #[test]
    fn test_add_bookmark_creates_rfc3339_timestamp() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new();

        let result = add_bookmark("Time Test".into(), "10.0.0.1".into(), vec![]);
        assert!(result.is_ok());
        let bm = result.unwrap();
        assert!(bm.created_at.contains('T'), "Expected RFC 3339 timestamp");
        assert!(
            bm.created_at.ends_with('Z') || bm.created_at.contains('+'),
            "Expected timezone in timestamp: {}",
            bm.created_at
        );
        assert!(
            chrono::DateTime::parse_from_rfc3339(&bm.created_at).is_ok(),
            "created_at '{}' is not valid RFC 3339",
            bm.created_at
        );
    }

    #[test]
    fn test_add_bookmark_preserves_fields() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new();

        let result = add_bookmark(
            "My Server".into(),
            "192.168.1.100".into(),
            vec!["web".into(), "prod".into()],
        );
        assert!(result.is_ok());
        let bm = result.unwrap();
        assert_eq!(bm.label, "My Server");
        assert_eq!(bm.target, "192.168.1.100");
        assert_eq!(bm.tags, vec!["web", "prod"]);
    }

    #[test]
    fn test_add_bookmark_empty_label_is_accepted() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new();

        let result = add_bookmark("".into(), "10.0.0.1".into(), vec![]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().label, "");
    }

    #[test]
    fn test_add_bookmark_special_chars() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new();

        let result = add_bookmark(
            "\"quoted\" & <tag>".into(),
            "https://example.com/path?q=1".into(),
            vec!["a,b".into(), "c:d".into()],
        );
        assert!(result.is_ok());
        let bm = result.unwrap();
        assert_eq!(bm.label, "\"quoted\" & <tag>");
        assert_eq!(bm.target, "https://example.com/path?q=1");
        assert_eq!(bm.tags, vec!["a,b", "c:d"]);
    }

    #[test]
    fn test_add_bookmark_empty_tags() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new();

        let result = add_bookmark("No Tags".into(), "10.0.0.1".into(), vec![]);
        assert!(result.is_ok());
        assert!(result.unwrap().tags.is_empty());
    }

    // ── list_bookmarks ───────────────────────────────────────────

    #[test]
    fn test_list_bookmarks_empty() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new();

        let list = list_bookmarks().unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn test_add_and_list_bookmarks() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new();

        add_bookmark("A".into(), "10.0.0.1".into(), vec![]).unwrap();
        add_bookmark("B".into(), "10.0.0.2".into(), vec![]).unwrap();

        let list = list_bookmarks().unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_list_bookmarks_order_desc_by_created_at() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new();

        add_bookmark("First".into(), "10.0.0.1".into(), vec![]).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        add_bookmark("Second".into(), "10.0.0.2".into(), vec![]).unwrap();

        let list = list_bookmarks().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].label, "Second");
        assert_eq!(list[1].label, "First");
    }

    // ── delete_bookmark ──────────────────────────────────────────

    #[test]
    fn test_delete_bookmark_removes_it() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new();

        let bm = add_bookmark("Delete Me".into(), "10.0.0.1".into(), vec![]).unwrap();
        assert_eq!(list_bookmarks().unwrap().len(), 1);

        delete_bookmark(bm.id.clone()).unwrap();
        assert!(list_bookmarks().unwrap().is_empty());
    }

    #[test]
    fn test_delete_nonexistent_is_ok() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new();

        let result = delete_bookmark("no-such-id".into());
        assert!(result.is_ok());
    }

    #[test]
    fn test_delete_and_add_again() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new();

        let bm = add_bookmark("Temp".into(), "10.0.0.1".into(), vec![]).unwrap();
        delete_bookmark(bm.id.clone()).unwrap();
        assert!(list_bookmarks().unwrap().is_empty());

        add_bookmark("New".into(), "10.0.0.2".into(), vec![]).unwrap();
        assert_eq!(list_bookmarks().unwrap().len(), 1);
    }

    // ─── End-to-end workﬂow ──────────────────────────────────────

    #[test]
    fn test_full_workflow() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new();

        // Start empty
        assert!(list_bookmarks().unwrap().is_empty());

        // Add three bookmarks
        let b1 = add_bookmark("Web".into(), "192.168.1.10".into(), vec!["http".into()]).unwrap();
        let b2 = add_bookmark("DB".into(), "192.168.1.20".into(), vec!["mysql".into(), "prod".into()]).unwrap();
        let b3 = add_bookmark("DNS".into(), "8.8.8.8".into(), vec![]).unwrap();

        assert_eq!(list_bookmarks().unwrap().len(), 3);

        // Delete the middle one
        delete_bookmark(b2.id.clone()).unwrap();
        let list = list_bookmarks().unwrap();
        assert_eq!(list.len(), 2);

        // Ensure correct items remain
        let ids: Vec<&str> = list.iter().map(|b| b.id.as_str()).collect();
        assert!(ids.contains(&b1.id.as_str()));
        assert!(ids.contains(&b3.id.as_str()));
        assert!(!ids.contains(&b2.id.as_str()));

        // Delete remaining
        delete_bookmark(b1.id.clone()).unwrap();
        delete_bookmark(b3.id.clone()).unwrap();
        assert!(list_bookmarks().unwrap().is_empty());
    }
}
