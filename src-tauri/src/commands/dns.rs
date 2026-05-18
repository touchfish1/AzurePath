use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::core::dns;
use crate::types::dns::{DnsResult, RecordType};

#[tauri::command]
pub async fn dns_lookup(
    app: AppHandle,
    target: String,
    record_type: RecordType,
) -> Result<String, String> {
    let task_id = Uuid::new_v4().to_string();

    let result = dns::resolve(&target, &record_type).await;

    match result {
        Ok(records) => {
            let dns_result = DnsResult {
                task_id: task_id.clone(),
                target: target.clone(),
                records: records.clone(),
            };

            app.emit("dns:result", &dns_result)
                .map_err(|e| format!("Failed to emit dns:result: {}", e))?;

            Ok(serde_json::to_string(&records).map_err(|e| e.to_string())?)
        }
        Err(e) => {
            app.emit(
                "dns:error",
                serde_json::json!({
                    "task_id": task_id,
                    "target": target,
                    "error": e,
                }),
            )
            .map_err(|err| format!("Failed to emit dns:error: {}", err))?;

            Err(e)
        }
    }
}
