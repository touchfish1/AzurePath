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

/// Decode process output bytes to UTF-8, handling system locale encoding (e.g. GBK on Chinese Windows).
fn decode_line(bytes: &[u8]) -> String {
    #[cfg(target_os = "windows")]
    {
        String::from_utf8(bytes.to_vec())
            .unwrap_or_else(|_| encoding_rs::GBK.decode(bytes).0.to_string())
    }
    #[cfg(not(target_os = "windows"))]
    {
        String::from_utf8_lossy(bytes).to_string()
    }
}

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
    use std::process::Stdio;
    use tokio::io::AsyncBufReadExt;

    let mut child = if cfg!(target_os = "windows") {
        tokio::process::Command::new("tracert")
            .arg("-h")
            .arg(opts.max_hops.to_string())
            .arg("-w")
            .arg(opts.timeout_ms.to_string())
            .arg(target)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn tracert: {}", e))?
    } else {
        let timeout_s = (opts.timeout_ms / 1000).max(1);
        tokio::process::Command::new("traceroute")
            .arg("-m")
            .arg(opts.max_hops.to_string())
            .arg("-w")
            .arg(timeout_s.to_string())
            .arg(target)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn traceroute: {}", e))?
    };

    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let mut reader = tokio::io::BufReader::new(stdout);
    let mut buf = Vec::new();
    let mut hop_results = Vec::new();

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
            .map_err(|e| format!("Failed to read output: {}", e))?;
        if n == 0 {
            break;
        }

        // Decode with encoding fallback
        let line = decode_line(&buf);
        let line = line.trim_end_matches('\n').trim_end_matches('\r');

        if line.is_empty() {
            continue;
        }

        // Use platform-appropriate single-line parser
        #[allow(unused_assignments)]
        let mut parsed_hop = None;
        #[cfg(target_os = "windows")]
        {
            parsed_hop = traceroute::parse_tracert_line(line);
        }
        #[cfg(not(target_os = "windows"))]
        {
            // Unix line parsing not yet implemented for streaming
            // falls back to batch parsing after process exit
        }

        if let Some(hop) = parsed_hop {
            let hop_event = TraceHop {
                hop: hop.hop,
                addr: hop.addr.clone(),
                hostname: hop.hostname.clone(),
                latencies: hop.latencies.clone(),
            };

            app.emit("trace:hop", &hop_event)
                .map_err(|e| format!("Failed to emit hop: {}", e))?;

            hop_results.push(hop);
        }
    }

    let _ = child.wait().await;

    let complete = TraceComplete {
        task_id: task_id.to_string(),
        target: target.to_string(),
        hops: hop_results
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
