use crate::core::operation_history::HistoryStore;
use crate::types::operation_history::OperationRecord;
use std::sync::Arc;
use std::sync::OnceLock;

static HISTORY_STORE: OnceLock<Arc<HistoryStore>> = OnceLock::new();

pub fn history_store() -> Option<&'static Arc<HistoryStore>> {
    HISTORY_STORE.get()
}

fn ensure_store() -> Result<&'static Arc<HistoryStore>, String> {
    HISTORY_STORE
        .get()
        .ok_or_else(|| "History store not initialized".to_string())
}

#[tauri::command]
pub async fn history_init() -> Result<(), String> {
    if HISTORY_STORE.get().is_some() {
        return Ok(());
    }
    let store = Arc::new(HistoryStore::new()?);
    HISTORY_STORE
        .set(store)
        .map_err(|_| "Already initialized".to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn history_list(
    op_type: Option<String>,
    search: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<OperationRecord>, String> {
    let store = ensure_store()?;
    store.list(
        op_type.as_deref(),
        search.as_deref(),
        limit.unwrap_or(100),
        offset.unwrap_or(0),
    )
}

#[tauri::command]
pub async fn history_get(id: String) -> Result<OperationRecord, String> {
    let store = ensure_store()?;
    store
        .get_by_id(&id)?
        .ok_or_else(|| format!("Record not found: {}", id))
}

#[tauri::command]
pub async fn history_delete(id: String) -> Result<(), String> {
    let store = ensure_store()?;
    store.delete(&id)
}

#[tauri::command]
pub async fn history_clear() -> Result<(), String> {
    let store = ensure_store()?;
    store.clear()
}

#[tauri::command]
pub async fn history_set_max_entries(count: u32) -> Result<(), String> {
    let store = ensure_store()?;
    store.set_max_entries(count)
}

#[tauri::command]
pub async fn history_get_max_entries() -> Result<u32, String> {
    let store = ensure_store()?;
    store.get_max_entries()
}

/// Public helper called by tool command modules to record operations.
/// Silently skipped if history store is not initialized.
pub(crate) fn record_operation(
    op_type: &str,
    target: &str,
    status: &str,
    summary: &str,
    result_meta: serde_json::Value,
) {
    if let Some(store) = history_store() {
        let record = OperationRecord {
            id: uuid::Uuid::new_v4().to_string(),
            op_type: op_type.to_string(),
            target: target.to_string(),
            status: status.to_string(),
            summary: summary.to_string(),
            result_meta,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        let _ = store.insert(&record);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn init_state() {
        if HISTORY_STORE.get().is_none() {
            let store = Arc::new(HistoryStore::test_store());
            let _ = HISTORY_STORE.set(store);
        }
    }

    fn store() -> &'static HistoryStore {
        HISTORY_STORE.get().expect("store not initialized").as_ref()
    }

    #[test]
    fn test_record_and_list() {
        init_state();

        record_operation("ping", "8.8.8.8", "success", "4/4", json!({"sent": 4}));
        record_operation("dns", "google.com", "success", "2 records", json!({}));

        let entries = store().list(None, None, 100, 0).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_get_by_id() {
        init_state();

        record_operation("ping", "1.1.1.1", "success", "4/4", json!({}));

        let entries = store().list(None, None, 1, 0).unwrap();
        let id = entries[0].id.clone();

        let found = store().get_by_id(&id).unwrap().expect("should exist");
        assert_eq!(found.target, "1.1.1.1");
    }

    #[test]
    fn test_get_nonexistent() {
        init_state();
        assert!(store().get_by_id("no-such-id").unwrap().is_none());
    }

    #[test]
    fn test_delete() {
        init_state();

        record_operation("ping", "8.8.8.8", "success", "4/4", json!({}));
        let entries = store().list(None, None, 100, 0).unwrap();
        assert_eq!(entries.len(), 1);

        store().delete(&entries[0].id).unwrap();
        let entries = store().list(None, None, 100, 0).unwrap();
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_clear() {
        init_state();

        record_operation("ping", "8.8.8.8", "success", "4/4", json!({}));
        record_operation("dns", "google.com", "success", "2", json!({}));
        assert_eq!(store().list(None, None, 100, 0).unwrap().len(), 2);

        store().clear().unwrap();
        assert_eq!(store().list(None, None, 100, 0).unwrap().len(), 0);
    }

    #[test]
    fn test_set_max_entries() {
        init_state();
        store().set_max_entries(100).unwrap();
        assert_eq!(store().get_max_entries().unwrap(), 100);
    }
}
