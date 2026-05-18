use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::core::port_scan;
use crate::types::port_scan::{
    OpenPort, PortFound, PortRange, ScanComplete, ScanOptions, ScanProgress,
};

static CANCEL_TOKENS: LazyLock<Mutex<HashMap<String, Arc<AtomicBool>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[tauri::command]
pub async fn port_scan_start(
    app: AppHandle,
    target: String,
    port_range: PortRange,
    options: Option<ScanOptions>,
) -> Result<String, String> {
    let opts = options.unwrap_or_default();
    let task_id = Uuid::new_v4().to_string();
    let cancel_flag = Arc::new(AtomicBool::new(false));

    // Register cancel token
    {
        let mut tokens = CANCEL_TOKENS.lock().map_err(|e| e.to_string())?;
        tokens.insert(task_id.clone(), cancel_flag.clone());
    }

    let task_id_clone = task_id.clone();
    let app_clone = app.clone();

    tauri::async_runtime::spawn(async move {
        let result =
            run_port_scan(&app_clone, &task_id_clone, &target, &port_range, &opts, cancel_flag)
                .await;
        if let Err(e) = result {
            let _ = app_clone.emit("port:error", serde_json::json!({
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

async fn run_port_scan(
    app: &AppHandle,
    task_id: &str,
    target: &str,
    port_range: &PortRange,
    opts: &ScanOptions,
    cancel_flag: Arc<AtomicBool>,
) -> Result<(), String> {
    let total_ports = (port_range.end.saturating_sub(port_range.start) + 1) as u32;
    let mut open_ports: Vec<OpenPort> = Vec::new();

    // Emit initial progress
    app.emit(
        "port:progress",
        &ScanProgress {
            task_id: task_id.to_string(),
            scanned: 0,
            total: total_ports,
            open: 0,
        },
    )
    .map_err(|e| format!("Failed to emit progress: {}", e))?;

    let semaphore = Arc::new(tokio::sync::Semaphore::new(opts.concurrency as usize));
    let target_arc = Arc::new(target.to_string());
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Option<u16>>(total_ports as usize);

    // Spawn all scan tasks — each sends its result through the channel
    for port in port_range.start..=port_range.end {
        if cancel_flag.load(Ordering::SeqCst) {
            break;
        }

        let sem_clone = semaphore.clone();
        let target_clone = target_arc.clone();
        let cancel_clone = cancel_flag.clone();
        let tx_clone = tx.clone();
        let timeout_ms = opts.timeout_ms;

        tokio::spawn(async move {
            let _permit = match sem_clone.acquire_owned().await {
                Ok(p) => p,
                Err(_) => return,
            };
            if cancel_clone.load(Ordering::SeqCst) {
                return;
            }
            let is_open = port_scan::tcp_connect::check_port(&target_clone, port, timeout_ms).await;
            let _ = tx_clone.send(if is_open { Some(port) } else { None }).await;
        });
    }

    // Drop the sender so rx.recv() knows when all senders are gone
    drop(tx);

    // Collect results as they arrive (real-time, not in-order)
    let mut scanned: u32 = 0;
    while let Some(result) = rx.recv().await {
        scanned += 1;

        if let Some(port) = result {
            let open_port = OpenPort {
                port,
                service: port_scan::guess_service(port),
            };
            open_ports.push(open_port.clone());

            // Emit found immediately
            app.emit(
                "port:found",
                &PortFound {
                    task_id: task_id.to_string(),
                    port,
                    service: open_port.service.clone(),
                },
            )
            .map_err(|e| format!("Failed to emit port:found: {}", e))?;
        }

        // Emit progress periodically
        if scanned % 10 == 0 || scanned == total_ports {
            app.emit(
                "port:progress",
                &ScanProgress {
                    task_id: task_id.to_string(),
                    scanned,
                    total: total_ports,
                    open: open_ports.len() as u32,
                },
            )
            .map_err(|e| format!("Failed to emit progress: {}", e))?;
        }
    }

    open_ports.sort_by_key(|p| p.port);

    // Emit final progress
    app.emit(
        "port:progress",
        &ScanProgress {
            task_id: task_id.to_string(),
            scanned,
            total: total_ports,
            open: open_ports.len() as u32,
        },
    )
    .map_err(|e| format!("Failed to emit final progress: {}", e))?;

    // Emit complete
    app.emit(
        "port:complete",
        &ScanComplete {
            task_id: task_id.to_string(),
            target: target.to_string(),
            open_ports,
        },
    )
    .map_err(|e| format!("Failed to emit complete: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn port_scan_stop(
    app: AppHandle,
    task_id: String,
) -> Result<(), String> {
    let _ = app;
    let cancel = {
        let tokens = CANCEL_TOKENS.lock().map_err(|e| e.to_string())?;
        tokens.get(&task_id).cloned()
    };

    if let Some(cancel) = cancel {
        cancel.store(true, Ordering::SeqCst);
        Ok(())
    } else {
        Err(format!("Task {} not found", task_id))
    }
}
