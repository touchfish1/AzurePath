use tauri::AppHandle;
use tauri::Manager;

use crate::core::wol;
use crate::types::wol::{WolRecord, WolResult};

#[tauri::command]
pub async fn wol_send(
    mac: String,
    broadcast_ip: String,
    port: u16,
    app: AppHandle,
) -> Result<WolResult, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    // Update last_used for this record (look up by mac)
    let records = wol::load_records(&app_data_dir)?;
    for record in &records {
        if record.mac == mac {
            wol::touch_record(&record.id, &app_data_dir);
            break;
        }
    }

    let message = wol::send_magic_packet(&mac, &broadcast_ip, port).await?;
    Ok(WolResult {
        success: true,
        message,
    })
}

#[tauri::command]
pub async fn wol_save(
    mac: String,
    broadcast_ip: String,
    label: String,
    app: AppHandle,
) -> Result<WolRecord, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    wol::save_record(mac, broadcast_ip, label, &app_data_dir)
}

#[tauri::command]
pub async fn wol_list(app: AppHandle) -> Result<Vec<WolRecord>, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    wol::load_records(&app_data_dir)
}

#[tauri::command]
pub async fn wol_delete(id: String, app: AppHandle) -> Result<(), String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    wol::delete_record(id, &app_data_dir)
}
