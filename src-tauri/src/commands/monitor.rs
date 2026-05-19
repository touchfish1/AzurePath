use crate::core::monitor;
use crate::types::monitor::{MonitorTarget, PingRecord};

use tauri::AppHandle;

#[tauri::command]
pub async fn monitor_start(app: AppHandle) -> Result<(), String> {
    monitor::start_monitoring(app)
}

#[tauri::command]
pub async fn monitor_stop() -> Result<(), String> {
    monitor::stop_monitoring();
    Ok(())
}

#[tauri::command]
pub async fn monitor_status() -> Result<bool, String> {
    Ok(monitor::is_monitoring())
}

#[tauri::command]
pub async fn monitor_add_target(
    host: String,
    label: String,
    interval_secs: u64,
) -> Result<MonitorTarget, String> {
    let store = monitor::MonitorStore::new()?;
    store.add_target(&host, &label, interval_secs)
}

#[tauri::command]
pub async fn monitor_list_targets() -> Result<Vec<MonitorTarget>, String> {
    let store = monitor::MonitorStore::new()?;
    store.list_targets()
}

#[tauri::command]
pub async fn monitor_delete_target(id: String) -> Result<(), String> {
    let store = monitor::MonitorStore::new()?;
    store.delete_target(&id)
}

#[tauri::command]
pub async fn monitor_get_history(
    target_id: String,
    since_days: i64,
) -> Result<Vec<PingRecord>, String> {
    let store = monitor::MonitorStore::new()?;
    store.get_history(&target_id, since_days)
}

#[tauri::command]
pub async fn monitor_get_all_recent_history(
    since_days: i64,
) -> Result<Vec<PingRecord>, String> {
    let store = monitor::MonitorStore::new()?;
    store.get_all_recent_history(since_days)
}
