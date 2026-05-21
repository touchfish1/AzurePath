use crate::core::app_settings::SettingsStore;
use crate::types::app_settings::SettingEntry;
use std::sync::Arc;
use std::sync::OnceLock;

static SETTINGS_STORE: OnceLock<Arc<SettingsStore>> = OnceLock::new();

fn ensure_store() -> Result<&'static Arc<SettingsStore>, String> {
    SETTINGS_STORE
        .get()
        .ok_or_else(|| "Settings store not initialized".to_string())
}

#[tauri::command]
pub async fn settings_init() -> Result<(), String> {
    if SETTINGS_STORE.get().is_some() {
        return Ok(());
    }
    let store = Arc::new(SettingsStore::new()?);
    SETTINGS_STORE
        .set(store)
        .map_err(|_| "Already initialized".to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn settings_get(key: String) -> Result<Option<String>, String> {
    let store = ensure_store()?;
    store.get(&key)
}

#[tauri::command]
pub async fn settings_set(key: String, value: String) -> Result<(), String> {
    let store = ensure_store()?;
    store.set(&key, &value)
}

#[tauri::command]
pub async fn settings_delete(key: String) -> Result<(), String> {
    let store = ensure_store()?;
    store.delete(&key)
}

#[tauri::command]
pub async fn settings_get_all() -> Result<Vec<SettingEntry>, String> {
    let store = ensure_store()?;
    store.get_all()
}

#[tauri::command]
pub async fn settings_clear() -> Result<(), String> {
    let store = ensure_store()?;
    store.clear()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_state() {
        if SETTINGS_STORE.get().is_none() {
            let store = Arc::new(SettingsStore::test_store());
            let _ = SETTINGS_STORE.set(store);
        }
    }

    fn store() -> &'static SettingsStore {
        SETTINGS_STORE.get().expect("store not initialized").as_ref()
    }

    #[test]
    fn test_get_set() {
        init_state();
        store().set("theme", "dark").unwrap();
        let val = store().get("theme").unwrap();
        assert_eq!(val, Some("dark".to_string()));
    }

    #[test]
    fn test_get_nonexistent() {
        init_state();
        assert!(store().get("no-such").unwrap().is_none());
    }

    #[test]
    fn test_get_all() {
        init_state();
        store().set("a", "1").unwrap();
        store().set("b", "2").unwrap();
        let all = store().get_all().unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_delete() {
        init_state();
        store().set("k", "v").unwrap();
        assert_eq!(store().get_all().unwrap().len(), 1);

        store().delete("k").unwrap();
        assert!(store().get_all().unwrap().is_empty());
    }

    #[test]
    fn test_clear() {
        init_state();
        store().set("a", "1").unwrap();
        store().set("b", "2").unwrap();
        assert_eq!(store().get_all().unwrap().len(), 2);

        store().clear().unwrap();
        assert!(store().get_all().unwrap().is_empty());
    }
}
