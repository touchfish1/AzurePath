use crate::types::network_sniffer::{DeviceResult, PortPreset, SnifferOptions};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

static CANCEL_TOKENS: LazyLock<Mutex<HashMap<String, Arc<AtomicBool>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[tauri::command]
pub async fn sniffer_start(
    app: tauri::AppHandle,
    options: SnifferOptions,
) -> Result<String, String> {
    let _ = (app, options);
    Err("Not implemented yet".to_string())
}

#[tauri::command]
pub async fn sniffer_stop(app: tauri::AppHandle, task_id: String) -> Result<(), String> {
    let _ = app;
    let cancel = {
        let tokens = CANCEL_TOKENS.lock().map_err(|e| e.to_string())?;
        tokens.get(&task_id).cloned()
    };
    match cancel {
        Some(c) => { c.store(true, Ordering::SeqCst); Ok(()) }
        None => Err(format!("Task {} not found", task_id)),
    }
}

#[tauri::command]
pub async fn sniffer_list() -> Result<Vec<DeviceResult>, String> {
    Err("Not implemented yet".to_string())
}

#[tauri::command]
pub async fn sniffer_export(task_id: String, format: String) -> Result<String, String> {
    let _ = (task_id, format);
    Err("Not implemented yet".to_string())
}

#[tauri::command]
pub async fn sniffer_presets() -> Result<Vec<PortPreset>, String> {
    Ok(crate::types::network_sniffer::default_presets())
}
