use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::core::ping;
use crate::types::ping::{PingComplete, PingOptions, PingProgress};

static CANCEL_TOKENS: LazyLock<Mutex<HashMap<String, AtomicBool>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[tauri::command]
pub async fn ping_start(
    app: AppHandle,
    target: String,
    options: Option<PingOptions>,
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

    // Spawn the ping execution on a background task
    tauri::async_runtime::spawn(async move {
        let result = run_ping(&app_clone, &task_id_clone, &target, &opts).await;
        if let Err(e) = result {
            let _ = app_clone.emit("ping:error", serde_json::json!({
                "task_id": task_id_clone,
                "error": e,
            }));
        }

        // Cleanup cancel token
        let _ = CANCEL_TOKENS.lock().map(|mut tokens| {
            tokens.remove(&task_id_clone);
        });
    });

    Ok(task_id)
}

async fn run_ping(
    app: &AppHandle,
    task_id: &str,
    target: &str,
    opts: &PingOptions,
) -> Result<(), String> {
    use std::process::Stdio;
    use tokio::io::AsyncBufReadExt;

    let mut child = if cfg!(target_os = "windows") {
        tokio::process::Command::new("ping")
            .arg("-n")
            .arg(opts.count.to_string())
            .arg("-w")
            .arg(opts.timeout_ms.to_string())
            .arg(target)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn ping: {}", e))?
    } else {
        let timeout_s = (opts.timeout_ms / 1000).max(1);
        tokio::process::Command::new("ping")
            .arg("-c")
            .arg(opts.count.to_string())
            .arg("-W")
            .arg(timeout_s.to_string())
            .arg(target)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn ping: {}", e))?
    };

    let stdout = child.stdout.take().ok_or("Failed to capture ping stdout")?;
    let mut reader = tokio::io::BufReader::new(stdout);
    let mut buf = Vec::new();
    let mut ping_results: Vec<ping::PingResult> = Vec::new();

    loop {
        // Check cancellation
        if let Ok(tokens) = CANCEL_TOKENS.lock() {
            if let Some(cancel) = tokens.get(task_id) {
                if cancel.load(Ordering::SeqCst) {
                    return Ok(());
                }
            }
        }

        buf.clear();
        let n = reader
            .read_until(b'\n', &mut buf)
            .await
            .map_err(|e| format!("Failed to read ping output: {}", e))?;
        if n == 0 {
            break; // EOF
        }

        // Decode with encoding-aware conversion (handles GBK on Chinese Windows)
        let line = ping::decode_ping_output(&buf);
        let line = line.trim_end_matches('\n').trim_end_matches('\r');

        if line.is_empty() {
            continue;
        }

        if let Some(result) = ping::parse_ping_line(line) {
            let seq = ping_results.len() as u32 + 1;
            let progress = PingProgress {
                task_id: task_id.to_string(),
                seq,
                ttl: result.ttl,
                latency_ms: if result.latency_ms >= 0.0 {
                    Some(result.latency_ms)
                } else {
                    None
                },
                status: result.status.clone(),
            };

            app.emit("ping:progress", &progress)
                .map_err(|e| format!("Failed to emit progress: {}", e))?;

            ping_results.push(result);
        }
    }

    // Wait for process to exit
    let _ = child.wait().await;

    // Compute and emit stats
    let stats = ping::compute_stats(&ping_results);
    let complete = PingComplete {
        task_id: task_id.to_string(),
        sent: stats.sent,
        received: stats.received,
        loss_percent: stats.loss_percent,
        min_ms: stats.min_ms,
        avg_ms: stats.avg_ms,
        max_ms: stats.max_ms,
    };

    app.emit("ping:complete", &complete)
        .map_err(|e| format!("Failed to emit complete: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn ping_stop(
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
