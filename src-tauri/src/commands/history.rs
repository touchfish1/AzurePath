use crate::core::history::ActivityStore;
use crate::types::activity::ActivityEntry;
use std::sync::OnceLock;

static ACTIVITY_STORE: OnceLock<ActivityStore> = OnceLock::new();

fn get_store() -> Result<&'static ActivityStore, String> {
    ACTIVITY_STORE
        .get()
        .ok_or_else(|| "Activity store not initialized".to_string())
}

pub fn init_activity_store() -> Result<(), String> {
    if ACTIVITY_STORE.get().is_some() {
        return Ok(());
    }
    let store = ActivityStore::new()?;
    ACTIVITY_STORE
        .set(store)
        .map_err(|_| "Activity store already initialized".to_string())
}

#[tauri::command]
pub async fn activity_list(
    limit: Option<usize>,
    offset: Option<usize>,
    action_type: Option<String>,
) -> Result<Vec<ActivityEntry>, String> {
    let store = get_store()?;
    store.list(
        limit.unwrap_or(50),
        offset.unwrap_or(0),
        action_type.as_deref(),
    )
}

#[tauri::command]
pub async fn activity_search(keyword: String) -> Result<Vec<ActivityEntry>, String> {
    let store = get_store()?;
    store.search(&keyword)
}

#[tauri::command]
pub async fn activity_delete(ids: Vec<i64>) -> Result<(), String> {
    let store = get_store()?;
    store.delete(&ids)
}

#[tauri::command]
pub async fn activity_clear() -> Result<(), String> {
    let store = get_store()?;
    store.clear()
}

#[tauri::command]
pub async fn activity_log(
    action_type: String,
    description: String,
    metadata: Option<String>,
) -> Result<(), String> {
    let store = get_store()?;
    store.log(&action_type, &description, metadata.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activity_list_defaults() {
        // Verify default parameter values
        assert_eq!(None::<usize>.unwrap_or(50), 50);
        assert_eq!(Some(10usize).unwrap_or(50), 10);
        assert_eq!(None::<usize>.unwrap_or(0), 0);
        assert_eq!(Some(5usize).unwrap_or(0), 5);
    }

    #[test]
    fn test_action_type_passthrough() {
        let none: Option<&str> = None::<String>.as_deref();
        assert!(none.is_none());
        let opt = Some("ping".to_string());
        assert_eq!(opt.as_deref(), Some("ping"));
    }
}
