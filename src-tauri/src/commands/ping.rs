use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::core::cancel::CANCEL_REGISTRY;
use crate::core::ping;
use crate::types::ping::{PingComplete, PingOptions, PingProgress};

#[tauri::command]
pub async fn ping_start(
    app: AppHandle,
    target: String,
    options: Option<PingOptions>,
) -> Result<String, String> {
    let opts = options.unwrap_or_default();
    let task_id = Uuid::new_v4().to_string();

    // Register cancel token
    CANCEL_REGISTRY.register(&task_id);

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
        CANCEL_REGISTRY.unregister(&task_id_clone);
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
            .stderr(Stdio::null())
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
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to spawn ping: {}", e))?
    };

    let stdout = child.stdout.take().ok_or("Failed to capture ping stdout")?;
    let mut reader = tokio::io::BufReader::new(stdout);
    let mut buf = Vec::new();
    let mut ping_results: Vec<ping::PingResult> = Vec::new();

    loop {
        // Check cancellation via CancelRegistry
        if CANCEL_REGISTRY.is_cancelled(task_id) {
            return Ok(());
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
pub async fn ping_stop(task_id: String) -> Result<(), String> {
    if CANCEL_REGISTRY.cancel(&task_id) {
        Ok(())
    } else {
        Err(format!("Task {} not found", task_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cancel_registry_basic() {
        let task_id = Uuid::new_v4().to_string();
        assert!(!CANCEL_REGISTRY.is_cancelled(&task_id));

        CANCEL_REGISTRY.register(&task_id);
        assert!(!CANCEL_REGISTRY.is_cancelled(&task_id));

        assert!(CANCEL_REGISTRY.cancel(&task_id));
        assert!(CANCEL_REGISTRY.is_cancelled(&task_id));

        CANCEL_REGISTRY.unregister(&task_id);
        assert!(!CANCEL_REGISTRY.is_cancelled(&task_id));
    }

    #[test]
    fn test_cancel_registry_missing_key() {
        let task_id = Uuid::new_v4().to_string();
        assert!(!CANCEL_REGISTRY.is_cancelled(&task_id));
        assert!(!CANCEL_REGISTRY.cancel(&task_id));
    }

    #[test]
    fn test_cancel_registry_multiple_operations() {
        let id_a = Uuid::new_v4().to_string();
        let id_b = Uuid::new_v4().to_string();

        CANCEL_REGISTRY.register(&id_a);
        CANCEL_REGISTRY.register(&id_b);

        assert!(!CANCEL_REGISTRY.is_cancelled(&id_a));
        assert!(!CANCEL_REGISTRY.is_cancelled(&id_b));

        CANCEL_REGISTRY.cancel(&id_a);
        assert!(CANCEL_REGISTRY.is_cancelled(&id_a));
        assert!(!CANCEL_REGISTRY.is_cancelled(&id_b));

        CANCEL_REGISTRY.unregister(&id_a);
        CANCEL_REGISTRY.unregister(&id_b);
    }

    #[test]
    fn test_ping_stop_non_existent() {
        let result = ping_stop("nonexistent".to_string());
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt.block_on(result).unwrap_err();
        assert!(err.contains("not found"));
    }
}
