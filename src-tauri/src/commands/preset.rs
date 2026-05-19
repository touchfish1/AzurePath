use tauri::AppHandle;
use tauri::Manager;

use crate::core::preset;
use crate::types::preset::Preset;

#[tauri::command]
pub async fn save_preset(
    name: String,
    feature: String,
    params: String,
    app: AppHandle,
) -> Result<Preset, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let params_value: serde_json::Value =
        serde_json::from_str(&params).map_err(|e| format!("Invalid params JSON: {}", e))?;

    preset::save_preset(name, feature, params_value, &app_data_dir)
}

#[tauri::command]
pub async fn load_presets(
    feature: Option<String>,
    app: AppHandle,
) -> Result<Vec<Preset>, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    preset::load_presets(feature, &app_data_dir)
}

#[tauri::command]
pub async fn delete_preset(id: String, app: AppHandle) -> Result<(), String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    preset::delete_preset(id, &app_data_dir)
}
