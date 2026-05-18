use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::core::traceroute;
use crate::types::traceroute::{TraceComplete, TraceHop, TraceOptions};

static CANCEL_TOKENS: LazyLock<Mutex<HashMap<String, AtomicBool>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[tauri::command]
pub async fn traceroute_start(
    app: AppHandle,
    target: String,
    options: Option<TraceOptions>,
) -> Result<String, String> {
    let opts = options.unwrap_or_default();
    let task_id = Uuid::new_v4().to_string();

    // Register cancel token
    {
        let mut tokens = CANCEL_TOKENS.lock().map_err(|e| e.to_string())?;
        tokens.insert(task_id.clone(), AtomicBool::new(false));
    }

    let task_id_clone = task_id.clone();
    let app_clone = app.clone();

    tauri::async_runtime::spawn(async move {
        let result = run_traceroute(&app_clone, &task_id_clone, &target, &opts).await;
        if let Err(e) = result {
            let _ = app_clone.emit("trace:error", serde_json::json!({
                "task_id": task_id_clone,
                "error": e,
            }));
        }

        let _ = CANCEL_TOKENS.lock().map(|mut tokens| {
            tokens.remove(&task_id_clone);
        });
    });

    Ok(task_id)
}

async fn run_traceroute(
    app: &AppHandle,
    task_id: &str,
    target: &str,
    opts: &TraceOptions,
) -> Result<(), String> {
    let output =
        traceroute::execute_traceroute(target, opts.max_hops, opts.timeout_ms).await?;
    let results = traceroute::parse_traceroute_output(&output);

    for result in &results {
        // Check cancellation
        if let Ok(tokens) = CANCEL_TOKENS.lock() {
            if let Some(cancel) = tokens.get(task_id) {
                if cancel.load(Ordering::SeqCst) {
                    return Ok(());
                }
            }
        }

        let hop = TraceHop {
            hop: result.hop,
            addr: result.addr.clone(),
            hostname: result.hostname.clone(),
            latencies: result.latencies.clone(),
        };

        app.emit("trace:hop", &hop)
            .map_err(|e| format!("Failed to emit hop: {}", e))?;
    }

    let complete = TraceComplete {
        task_id: task_id.to_string(),
        target: target.to_string(),
        hops: results
            .iter()
            .map(|r| TraceHop {
                hop: r.hop,
                addr: r.addr.clone(),
                hostname: r.hostname.clone(),
                latencies: r.latencies.clone(),
            })
            .collect(),
    };

    app.emit("trace:complete", &complete)
        .map_err(|e| format!("Failed to emit complete: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn traceroute_stop(
    app: AppHandle,
    task_id: String,
) -> Result<(), String> {
    let _ = app;
    let mut tokens = CANCEL_TOKENS.lock().map_err(|e| e.to_string())?;
    if let Some(cancel) = tokens.get_mut(&task_id) {
        cancel.store(true, Ordering::SeqCst);
        Ok(())
    } else {
        Err(format!("Task {} not found", task_id))
    }
}
