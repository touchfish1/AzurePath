use tauri::{AppHandle, Emitter};

use crate::core::mdns;
use crate::types::mdns::MdnsService;

/// Discover LAN services via mDNS.
///
/// Emits `mdns:progress` events during discovery and returns the full
/// list of discovered services upon completion.
#[tauri::command]
pub async fn mdns_discover(app: AppHandle) -> Result<Vec<MdnsService>, String> {
    app.emit(
        "mdns:progress",
        serde_json::json!({ "status": "querying", "message": "正在扫描 mDNS 服务..." }),
    )
    .map_err(|e| format!("Failed to emit progress: {}", e))?;

    let services = mdns::discover().await?;

    app.emit(
        "mdns:progress",
        serde_json::json!({
            "status": "complete",
            "count": services.len(),
            "message": format!("发现 {} 个服务", services.len()),
        }),
    )
    .map_err(|e| format!("Failed to emit progress: {}", e))?;

    Ok(services)
}
