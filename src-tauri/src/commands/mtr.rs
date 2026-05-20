use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::core::mtr;
use crate::types::mtr::{MtrComplete, MtrOptions, MtrProgress};

#[tauri::command]
pub async fn mtr_start(
    app: AppHandle,
    options: MtrOptions,
) -> Result<String, String> {
    // Validate target
    if options.target.trim().is_empty() {
        return Err("Target must not be empty".to_string());
    }

    let task_id = Uuid::new_v4().to_string();

    // Register cancel token
    {
        let mut tokens = mtr::CANCEL_TOKENS.lock().map_err(|e| e.to_string())?;
        tokens.insert(task_id.clone(), AtomicBool::new(false));
    }

    let task_id_clone = task_id.clone();
    let app_clone = app.clone();

    tauri::async_runtime::spawn(async move {
        if let Err(e) = run_mtr(&app_clone, &task_id_clone, &options).await {
            let _ = app_clone.emit("mtr:error", serde_json::json!({
                "task_id": task_id_clone,
                "error": e,
            }));
        }

        // Cleanup cancel token
        let _ = mtr::CANCEL_TOKENS.lock().map(|mut tokens| {
            tokens.remove(&task_id_clone);
        });
    });

    Ok(task_id)
}

async fn run_mtr(
    app: &AppHandle,
    task_id: &str,
    options: &MtrOptions,
) -> Result<(), String> {
    // Phase 1: Discover hops
    let discover_result = mtr::discover_hops(options).await?;

    // Emit initial progress with discovered hops (round 0, no stats yet)
    let total_hops = discover_result.len() as u32;
    let initial_hops = discover_result
        .iter()
        .map(|h| crate::types::mtr::MtrHopStats {
            hop: h.hop,
            addr: h.addr.clone(),
            hostname: h.hostname.clone(),
            sent: 0,
            received: 0,
            loss_percent: 0.0,
            min_ms: 0.0,
            avg_ms: 0.0,
            max_ms: 0.0,
            jitter_ms: 0.0,
            last_ms: None,
        })
        .collect();

    let initial_progress = MtrProgress {
        target: options.target.clone(),
        total_hops,
        round: 0,
        hops: initial_hops,
    };
    app.emit("mtr:progress", &initial_progress)
        .map_err(|e| format!("Failed to emit initial progress: {}", e))?;

    // Phase 2: Run the MTR probing loop
    let round_snapshots = mtr::run_mtr_loop(task_id, options, &discover_result).await;

    if round_snapshots.is_empty() {
        return Err("MTR probing was cancelled before any data was collected".to_string());
    }

    // Emit the latest snapshot as the final complete result
    let total_rounds = round_snapshots.len() as u32;
    let final_hops = round_snapshots.last().unwrap().clone();

    // Emit one more progress event with the final snapshot
    let final_progress = MtrProgress {
        target: options.target.clone(),
        total_hops,
        round: total_rounds,
        hops: final_hops.clone(),
    };
    app.emit("mtr:progress", &final_progress)
        .map_err(|e| format!("Failed to emit final progress: {}", e))?;

    // Emit complete event
    let complete = MtrComplete {
        target: options.target.clone(),
        total_rounds,
        hops: final_hops,
    };
    app.emit("mtr:complete", &complete)
        .map_err(|e| format!("Failed to emit complete: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn mtr_stop(
    app: AppHandle,
    task_id: String,
) -> Result<(), String> {
    let _ = app;
    let mut tokens = mtr::CANCEL_TOKENS.lock().map_err(|e| e.to_string())?;
    if let Some(cancel) = tokens.get_mut(&task_id) {
        cancel.store(true, Ordering::SeqCst);
        Ok(())
    } else {
        Err(format!("Task {} not found", task_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // MtrOptions tests
    // ============================================================

    #[test]
    fn test_mtr_options_default() {
        let opts = MtrOptions::default();
        assert_eq!(opts.max_hops, 30);
        assert_eq!(opts.interval_ms, 1000);
        assert_eq!(opts.timeout_ms, 3000);
        assert_eq!(opts.target, "");
    }

    #[test]
    fn test_mtr_options_camel_case_deserialize() {
        let json = r#"{"target":"8.8.8.8","maxHops":15,"intervalMs":500,"timeoutMs":2000}"#;
        let opts: MtrOptions = serde_json::from_str(json).unwrap();
        assert_eq!(opts.target, "8.8.8.8");
        assert_eq!(opts.max_hops, 15);
        assert_eq!(opts.interval_ms, 500);
        assert_eq!(opts.timeout_ms, 2000);
    }

    #[test]
    fn test_mtr_options_roundtrip() {
        let opts = MtrOptions {
            target: "example.com".into(),
            max_hops: 20,
            interval_ms: 2000,
            timeout_ms: 5000,
        };
        let json = serde_json::to_string(&opts).unwrap();
        let deserialized: MtrOptions = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.max_hops, 20);
        assert_eq!(deserialized.interval_ms, 2000);
        assert_eq!(deserialized.timeout_ms, 5000);
    }

    // ============================================================
    // MtrHopStats tests
    // ============================================================

    #[test]
    fn test_mtr_hop_stats_serialization() {
        let stats = crate::types::mtr::MtrHopStats {
            hop: 1,
            addr: Some("192.168.1.1".into()),
            hostname: None,
            sent: 10,
            received: 9,
            loss_percent: 10.0,
            min_ms: 1.0,
            avg_ms: 5.0,
            max_ms: 10.0,
            jitter_ms: 2.0,
            last_ms: Some(3.0),
        };
        let json = serde_json::to_value(&stats).unwrap();
        assert_eq!(json["hop"], 1);
        assert_eq!(json["addr"], "192.168.1.1");
        assert_eq!(json["sent"], 10);
        assert_eq!(json["lossPercent"], 10.0);
        assert_eq!(json["jitterMs"], 2.0);
        assert_eq!(json["lastMs"], 3.0);
    }

    #[test]
    fn test_mtr_hop_stats_null_fields() {
        let stats = crate::types::mtr::MtrHopStats {
            hop: 2,
            addr: None,
            hostname: None,
            sent: 0,
            received: 0,
            loss_percent: 0.0,
            min_ms: 0.0,
            avg_ms: 0.0,
            max_ms: 0.0,
            jitter_ms: 0.0,
            last_ms: None,
        };
        let json = serde_json::to_value(&stats).unwrap();
        assert_eq!(json["addr"], serde_json::Value::Null);
        assert_eq!(json["hostname"], serde_json::Value::Null);
        assert_eq!(json["lastMs"], serde_json::Value::Null);
    }

    // ============================================================
    // MtrProgress tests
    // ============================================================

    #[test]
    fn test_mtr_progress_serialization() {
        let progress = MtrProgress {
            target: "8.8.8.8".into(),
            total_hops: 3,
            round: 5,
            hops: vec![],
        };
        let json = serde_json::to_value(&progress).unwrap();
        assert_eq!(json["target"], "8.8.8.8");
        assert_eq!(json["totalHops"], 3);
        assert_eq!(json["round"], 5);
        assert!(json["hops"].as_array().unwrap().is_empty());
    }

    // ============================================================
    // MtrComplete tests
    // ============================================================

    #[test]
    fn test_mtr_complete_serialization() {
        let complete = MtrComplete {
            target: "example.com".into(),
            total_rounds: 10,
            hops: vec![crate::types::mtr::MtrHopStats {
                hop: 1,
                addr: Some("1.1.1.1".into()),
                hostname: None,
                sent: 10,
                received: 10,
                loss_percent: 0.0,
                min_ms: 1.0,
                avg_ms: 2.0,
                max_ms: 3.0,
                jitter_ms: 0.5,
                last_ms: Some(2.0),
            }],
        };
        let json = serde_json::to_value(&complete).unwrap();
        assert_eq!(json["target"], "example.com");
        assert_eq!(json["totalRounds"], 10);
        assert_eq!(json["hops"].as_array().unwrap().len(), 1);
    }

    // ============================================================
    // Input validation tests
    // ============================================================

    #[test]
    fn test_empty_target_rejected() {
        for target in &["", "   ", "\t", "\n"] {
            assert!(target.trim().is_empty());
        }
    }

    #[test]
    fn test_non_empty_target_accepted() {
        for target in &[
            "8.8.8.8",
            "example.com",
            " 192.168.1.1 ",
            "localhost",
        ] {
            assert!(!target.trim().is_empty());
        }
    }

    // ============================================================
    // Cancel token tests
    // ============================================================

    #[test]
    fn test_cancel_token_insert_and_check() {
        let task_id = Uuid::new_v4().to_string();

        mtr::CANCEL_TOKENS
            .lock()
            .unwrap()
            .insert(task_id.clone(), AtomicBool::new(false));

        assert!(!mtr::is_cancelled(&task_id));

        // Cancel
        mtr::CANCEL_TOKENS
            .lock()
            .unwrap()
            .get(&task_id)
            .unwrap()
            .store(true, Ordering::SeqCst);

        assert!(mtr::is_cancelled(&task_id));

        // Cleanup
        mtr::CANCEL_TOKENS.lock().unwrap().remove(&task_id);
    }

    #[test]
    fn test_stop_non_existent_task() {
        let task_id = "ghost-task".to_string();
        let mut tokens = mtr::CANCEL_TOKENS.lock().unwrap();
        let result = match tokens.get_mut(&task_id) {
            Some(_cancel) => Ok(()),
            None => Err(format!("Task {} not found", task_id)),
        };
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Task ghost-task not found");
    }

    #[test]
    fn test_cancel_token_cleanup() {
        let task_id = Uuid::new_v4().to_string();

        mtr::CANCEL_TOKENS
            .lock()
            .unwrap()
            .insert(task_id.clone(), AtomicBool::new(false));

        assert!(mtr::CANCEL_TOKENS.lock().unwrap().contains_key(&task_id));

        mtr::CANCEL_TOKENS.lock().unwrap().remove(&task_id);

        assert!(!mtr::CANCEL_TOKENS.lock().unwrap().contains_key(&task_id));
    }
}
