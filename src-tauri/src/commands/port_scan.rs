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

fn validate_scan_inputs(target: &str, port_range: &PortRange, opts: &ScanOptions) -> Result<(), String> {
    if target.trim().is_empty() {
        return Err("Target cannot be empty".to_string());
    }
    if port_range.start > port_range.end {
        return Err(format!(
            "Port range start ({}) must be <= end ({})",
            port_range.start, port_range.end
        ));
    }
    if opts.concurrency == 0 {
        return Err("Concurrency must be greater than 0".to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn port_scan_start(
    app: AppHandle,
    target: String,
    port_range: PortRange,
    options: Option<ScanOptions>,
) -> Result<String, String> {
    let opts = options.unwrap_or_default();

    // Validate inputs before allocating any resources
    validate_scan_inputs(&target, &port_range, &opts)?;

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
    // Pre-resolve target to an IP string once so check_port never re-resolves
    let resolved_target = port_scan::tcp_connect::resolve_target(target).await?;

    // Compute total ports using u32 arithmetic to avoid u16 overflow
    let total_ports = port_range.end as u32 - port_range.start as u32 + 1;
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
    let resolved_arc = Arc::new(resolved_target);
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Option<u16>>(total_ports as usize);

    // Spawn all scan tasks — each sends its result through the channel
    for port in port_range.start..=port_range.end {
        if cancel_flag.load(Ordering::SeqCst) {
            break;
        }

        let sem_clone = semaphore.clone();
        let target_clone = resolved_arc.clone();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::port_scan::ScanOptions;

    #[test]
    fn test_validate_scan_inputs_empty_target() {
        let range = PortRange { start: 1, end: 100 };
        let opts = ScanOptions::default();
        let result = validate_scan_inputs("", &range, &opts);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn test_validate_scan_inputs_whitespace_target() {
        let range = PortRange { start: 1, end: 100 };
        let opts = ScanOptions::default();
        let result = validate_scan_inputs("   ", &range, &opts);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn test_validate_scan_inputs_start_greater_than_end() {
        let range = PortRange { start: 50000, end: 100 };
        let opts = ScanOptions::default();
        let result = validate_scan_inputs("127.0.0.1", &range, &opts);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("50000"));
        assert!(err.contains("100"));
    }

    #[test]
    fn test_validate_scan_inputs_zero_concurrency() {
        let range = PortRange { start: 1, end: 100 };
        let opts = ScanOptions {
            concurrency: 0,
            timeout_ms: 1000,
        };
        let result = validate_scan_inputs("127.0.0.1", &range, &opts);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Concurrency"));
    }

    #[test]
    fn test_validate_scan_inputs_valid() {
        let range = PortRange { start: 1, end: 1024 };
        let opts = ScanOptions::default();
        let result = validate_scan_inputs("127.0.0.1", &range, &opts);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_scan_inputs_single_port() {
        let range = PortRange {
            start: 443,
            end: 443,
        };
        let opts = ScanOptions::default();
        let result = validate_scan_inputs("example.com", &range, &opts);
        assert!(result.is_ok());
    }

    /// Verify that total_ports calculation does not overflow with
    /// worst-case u16 range 0..=65535 (65536 ports total).
    #[test]
    fn test_total_ports_u16_range_no_overflow() {
        let range = PortRange {
            start: 0,
            end: 65535,
        };
        // This reproduces the original overflow bug:
        //   (65535u16.saturating_sub(0) + 1) as u32  →  0u32
        // The fix uses u32 arithmetic:
        //   65535u32 - 0u32 + 1u32  →  65536u32
        let total = range.end as u32 - range.start as u32 + 1;
        assert_eq!(total, 65536, "total_ports must be 65536, not 0");

        let opts = ScanOptions::default();
        let result = validate_scan_inputs("127.0.0.1", &range, &opts);
        assert!(result.is_ok());
    }

    /// Verify that mid-range (32768..=65535) also works correctly.
    #[test]
    fn test_total_ports_mid_range_no_overflow() {
        let range = PortRange {
            start: 32768,
            end: 65535,
        };
        let total = range.end as u32 - range.start as u32 + 1;
        assert_eq!(total, 32768);

        let opts = ScanOptions::default();
        let result = validate_scan_inputs("127.0.0.1", &range, &opts);
        assert!(result.is_ok());
    }

    /// Edge: start == end == 0 (u16 minimum)
    #[test]
    fn test_total_ports_zero() {
        let range = PortRange { start: 0, end: 0 };
        let total = range.end as u32 - range.start as u32 + 1;
        assert_eq!(total, 1);

        let opts = ScanOptions::default();
        let result = validate_scan_inputs("127.0.0.1", &range, &opts);
        assert!(result.is_ok());
    }
}
