use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::core::ping;
use crate::types::ping::{PingComplete, PingOptions, PingProgress};

static CANCEL_TOKENS: LazyLock<Mutex<HashMap<String, bool>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Access CANCEL_TOKENS with automatic Mutex poisoning recovery.
/// If the Mutex is poisoned (another thread panicked while holding the lock),
/// we recover via `into_inner()` to keep the system functional.
fn with_cancel_tokens<F, R>(f: F) -> R
where
    F: FnOnce(&mut HashMap<String, bool>) -> R,
{
    let mut tokens = CANCEL_TOKENS
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    f(&mut *tokens)
}

#[tauri::command]
pub async fn ping_start(
    app: AppHandle,
    target: String,
    options: Option<PingOptions>,
) -> Result<String, String> {
    let opts = options.unwrap_or_default();
    let task_id = Uuid::new_v4().to_string();

    // Register cancel token
    with_cancel_tokens(|tokens| {
        tokens.insert(task_id.clone(), false);
    });

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

        // Cleanup cancel token (handles Mutex poisoning gracefully)
        with_cancel_tokens(|tokens| {
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
        // Check cancellation (handles Mutex poisoning gracefully)
        if with_cancel_tokens(|tokens| tokens.get(task_id).copied().unwrap_or(false)) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_cancel_tokens_basic_insert_and_read() {
        // Insert a token
        with_cancel_tokens(|tokens| {
            tokens.insert("test1".to_string(), true);
        });
        // Read it back
        let val = with_cancel_tokens(|tokens| tokens.get("test1").copied().unwrap_or(false));
        assert!(val);

        // Update it
        with_cancel_tokens(|tokens| {
            if let Some(v) = tokens.get_mut("test1") {
                *v = false;
            }
        });
        let val = with_cancel_tokens(|tokens| tokens.get("test1").copied().unwrap_or(true));
        assert!(!val);

        // Clean up
        with_cancel_tokens(|tokens| {
            tokens.remove("test1");
        });
    }

    #[test]
    fn test_with_cancel_tokens_missing_key() {
        let val = with_cancel_tokens(|tokens| tokens.get("nonexistent").copied().unwrap_or(false));
        assert!(!val);
    }

    #[test]
    fn test_with_cancel_tokens_multiple_operations() {
        with_cancel_tokens(|tokens| {
            tokens.insert("a".to_string(), false);
            tokens.insert("b".to_string(), true);
            tokens.insert("c".to_string(), false);
        });

        let a = with_cancel_tokens(|tokens| tokens.get("a").copied().unwrap_or(true));
        let b = with_cancel_tokens(|tokens| tokens.get("b").copied().unwrap_or(false));
        let c = with_cancel_tokens(|tokens| tokens.get("c").copied().unwrap_or(true));
        assert!(!a);
        assert!(b);
        assert!(!c);

        with_cancel_tokens(|tokens| {
            tokens.clear();
        });
    }

    #[test]
    fn test_with_cancel_tokens_poison_recovery() {
        // Poisone the Mutex by panicking while holding the lock
        let _ = std::panic::catch_unwind(|| {
            let _lock = CANCEL_TOKENS.lock().unwrap();
            panic!("intentional poison for test");
        });

        // After poisoning, with_cancel_tokens should still work via into_inner()
        with_cancel_tokens(|tokens| {
            tokens.insert("poison_test".to_string(), true);
        });

        let val = with_cancel_tokens(|tokens| {
            tokens.get("poison_test").copied().unwrap_or(false)
        });
        assert!(val, "Poisoned mutex should still allow token operations");

        // Clean up
        with_cancel_tokens(|tokens| {
            tokens.remove("poison_test");
        });
    }
}

#[tauri::command]
pub async fn ping_stop(task_id: String) -> Result<(), String> {
    if with_cancel_tokens(|tokens| {
        if let Some(cancel) = tokens.get_mut(&task_id) {
            *cancel = true;
            true
        } else {
            false
        }
    }) {
        Ok(())
    } else {
        Err(format!("Task {} not found", task_id))
    }
}
