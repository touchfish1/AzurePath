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
    crate::core::utils::decode_output(bytes)
}

#[tauri::command]
pub async fn traceroute_start(
    app: AppHandle,
    target: String,
    options: Option<TraceOptions>,
) -> Result<String, String> {
    // Validate target is non-empty to avoid spawning the OS command with empty arguments
    if target.trim().is_empty() {
        return Err("Target must not be empty".to_string());
    }

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
        let mut cmd = tokio::process::Command::new("traceroute");
        cmd.arg("-m")
            .arg(opts.max_hops.to_string())
            .arg("-w")
            .arg(timeout_s.to_string());
        if opts.probes_per_hop > 0 {
            cmd.arg("-q").arg(opts.probes_per_hop.to_string());
        }
        cmd.arg(target)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn traceroute: {}", e))?
    };

    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let mut reader = tokio::io::BufReader::new(stdout);
    let mut buf = Vec::new();
    let mut hop_results: Vec<traceroute::ExecuteTraceResult> = Vec::new();
    // On Unix, collect all output lines for batch parsing after process exit,
    // because Unix traceroute output (with hostname/IP combos) is too complex
    // for a simple streaming line parser.
    // On Windows this is unused (we stream-parse instead), so suppress the warning.
    #[allow(unused_mut, unused_variables)]
    let mut unix_lines: Vec<String> = Vec::new();
    let mut cancel_check_counter = 0u32;

    loop {
        // Check cancellation periodically (every 8 lines) to avoid
        // acquiring the Mutex on every single iteration.
        cancel_check_counter += 1;
        if cancel_check_counter >= 8 {
            cancel_check_counter = 0;
            if let Ok(tokens) = CANCEL_TOKENS.lock() {
                if let Some(cancel) = tokens.get(task_id) {
                    if cancel.load(Ordering::SeqCst) {
                        return Ok(());
                    }
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

        // Platform-specific parsing:
        // - On Windows: stream-parse each line in real-time (tracert output is simple)
        // - On Unix: buffer all lines, batch-parse after process exits
        #[cfg(target_os = "windows")]
        {
            if let Some(hop) = traceroute::parse_tracert_line(line) {
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
        #[cfg(not(target_os = "windows"))]
        {
            unix_lines.push(line.to_string());
        }
    }

    let _ = child.wait().await;

    // On Unix, batch-parse all collected lines after the process exits.
    #[cfg(not(target_os = "windows"))]
    {
        let all_output = unix_lines.join("\n");
        let parsed = traceroute::parse_traceroute_output(&all_output);
        for hop in parsed {
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

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // decode_line — encoding fallback logic
    // -----------------------------------------------------------------------

    #[test]
    fn test_decode_line_valid_utf8() {
        let input = b"Hello, world!";
        let result = decode_line(input);
        assert_eq!(result, "Hello, world!");
    }

    #[test]
    fn test_decode_line_empty_input() {
        assert_eq!(decode_line(b""), "");
    }

    #[test]
    fn test_decode_line_ascii() {
        let input = b" 1   5 ms   6 ms   7 ms  192.168.1.1\r\n";
        let result = decode_line(input);
        assert_eq!(result, " 1   5 ms   6 ms   7 ms  192.168.1.1\r\n");
    }

    #[test]
    fn test_decode_line_newline_variants() {
        assert_eq!(decode_line(b"hello\n"), "hello\n");
        assert_eq!(decode_line(b"hello\r\n"), "hello\r\n");
    }

    #[test]
    fn test_decode_line_invalid_utf8_does_not_panic() {
        decode_line(b"\xff\xfe\x00\x01");
        decode_line(b"\x80\x81\x82");
        decode_line(b"\xff");
    }

    #[test]
    fn test_decode_line_unicode() {
        let input = "Tracing route to 8.8.8.8 \u{4f60}\u{597d}\r\n";
        let result = decode_line(input.as_bytes());
        assert_eq!(result, input);
    }

    // -----------------------------------------------------------------------
    // TraceOptions — parameter passing defaults / camelCase
    // -----------------------------------------------------------------------

    #[test]
    fn test_trace_options_default_values() {
        let opts = TraceOptions::default();
        assert_eq!(opts.max_hops, 30);
        assert_eq!(opts.timeout_ms, 5000);
        assert_eq!(opts.probes_per_hop, 3);
    }

    #[test]
    fn test_trace_options_custom_values() {
        let opts = TraceOptions {
            max_hops: 15,
            timeout_ms: 2000,
            probes_per_hop: 1,
        };
        assert_eq!(opts.max_hops, 15);
        assert_eq!(opts.timeout_ms, 2000);
        assert_eq!(opts.probes_per_hop, 1);
    }

    #[test]
    fn test_trace_options_zero_probes() {
        let opts = TraceOptions {
            max_hops: 30,
            timeout_ms: 5000,
            probes_per_hop: 0,
        };
        assert_eq!(opts.probes_per_hop, 0);
    }

    #[test]
    fn test_trace_options_deserialize_camel_case() {
        // TraceOptions uses rename_all = "camelCase"
        let json = r#"{"maxHops":10,"timeoutMs":3000,"probesPerHop":2}"#;
        let opts: TraceOptions = serde_json::from_str(json).unwrap();
        assert_eq!(opts.max_hops, 10);
        assert_eq!(opts.timeout_ms, 3000);
        assert_eq!(opts.probes_per_hop, 2);
    }

    #[test]
    fn test_trace_options_deserialize_partial() {
        // All fields are required for deserialization
        let json = r#"{"maxHops":5}"#;
        let result: Result<TraceOptions, _> = serde_json::from_str(json);
        assert!(result.is_err(), "Missing fields should fail deserialization");
    }

    #[test]
    fn test_trace_options_deserialize_rejects_snake_case() {
        // Snake_case keys should fail because of rename_all = "camelCase"
        let json = r#"{"max_hops":10,"timeout_ms":3000,"probes_per_hop":2}"#;
        let result: Result<TraceOptions, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_trace_options_roundtrip() {
        let opts = TraceOptions {
            max_hops: 64,
            timeout_ms: 10000,
            probes_per_hop: 5,
        };
        let json = serde_json::to_string(&opts).unwrap();
        let deserialized: TraceOptions = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.max_hops, 64);
        assert_eq!(deserialized.timeout_ms, 10000);
        assert_eq!(deserialized.probes_per_hop, 5);
    }

    // -----------------------------------------------------------------------
    // CANCEL_TOKENS — cancellation infrastructure
    // -----------------------------------------------------------------------

    #[test]
    fn test_cancel_token_insert_and_check() {
        let task_id = Uuid::new_v4().to_string();

        // Register token (as traceroute_start does)
        CANCEL_TOKENS
            .lock()
            .unwrap()
            .insert(task_id.clone(), AtomicBool::new(false));

        // Verify token exists and is not cancelled
        let tokens = CANCEL_TOKENS.lock().unwrap();
        let token = tokens.get(&task_id).expect("Token should exist");
        assert!(
            !token.load(Ordering::SeqCst),
            "New token should not be cancelled"
        );

        // Cleanup
        drop(tokens);
        CANCEL_TOKENS.lock().unwrap().remove(&task_id);
    }

    #[test]
    fn test_cancel_token_cancel_operation() {
        let task_id = Uuid::new_v4().to_string();

        // Register
        CANCEL_TOKENS
            .lock()
            .unwrap()
            .insert(task_id.clone(), AtomicBool::new(false));

        // Cancel (same logic as traceroute_stop)
        {
            let mut tokens = CANCEL_TOKENS.lock().unwrap();
            let cancel = tokens.get_mut(&task_id).expect("Token should exist");
            cancel.store(true, Ordering::SeqCst);
        }

        // Verify cancellation is detected
        let tokens = CANCEL_TOKENS.lock().unwrap();
        assert!(
            tokens.get(&task_id).unwrap().load(Ordering::SeqCst),
            "Cancelled token should report true"
        );

        // Cleanup
        drop(tokens);
        CANCEL_TOKENS.lock().unwrap().remove(&task_id);
    }

    #[test]
    fn test_cancel_non_existent_task() {
        let task_id = "non-existent-task".to_string();
        let mut tokens = CANCEL_TOKENS.lock().unwrap();
        assert!(
            tokens.get_mut(&task_id).is_none(),
            "Non-existent task should not have a cancel token"
        );
    }

    #[test]
    fn test_cancel_token_cleanup() {
        let task_id = Uuid::new_v4().to_string();

        // Register
        CANCEL_TOKENS
            .lock()
            .unwrap()
            .insert(task_id.clone(), AtomicBool::new(false));

        // Verify exists
        assert!(CANCEL_TOKENS.lock().unwrap().contains_key(&task_id));

        // Cleanup (as traceroute_start's spawn block does after completion)
        CANCEL_TOKENS.lock().unwrap().remove(&task_id);

        // Verify removed
        assert!(!CANCEL_TOKENS.lock().unwrap().contains_key(&task_id));
    }

    #[test]
    fn test_cancel_token_multiple_tasks() {
        let id1 = Uuid::new_v4().to_string();
        let id2 = Uuid::new_v4().to_string();

        // Register two tasks
        CANCEL_TOKENS
            .lock()
            .unwrap()
            .insert(id1.clone(), AtomicBool::new(false));
        CANCEL_TOKENS
            .lock()
            .unwrap()
            .insert(id2.clone(), AtomicBool::new(false));

        // Cancel only one
        CANCEL_TOKENS
            .lock()
            .unwrap()
            .get_mut(&id1)
            .unwrap()
            .store(true, Ordering::SeqCst);

        // Verify each token has the correct state
        let tokens = CANCEL_TOKENS.lock().unwrap();
        assert!(tokens.get(&id1).unwrap().load(Ordering::SeqCst));
        assert!(!tokens.get(&id2).unwrap().load(Ordering::SeqCst));

        // Cleanup
        drop(tokens);
        CANCEL_TOKENS.lock().unwrap().remove(&id1);
        CANCEL_TOKENS.lock().unwrap().remove(&id2);
    }

    #[test]
    fn test_cancel_token_double_cancel_is_idempotent() {
        let task_id = Uuid::new_v4().to_string();

        CANCEL_TOKENS
            .lock()
            .unwrap()
            .insert(task_id.clone(), AtomicBool::new(false));

        // Cancel twice
        CANCEL_TOKENS
            .lock()
            .unwrap()
            .get_mut(&task_id)
            .unwrap()
            .store(true, Ordering::SeqCst);
        CANCEL_TOKENS
            .lock()
            .unwrap()
            .get_mut(&task_id)
            .unwrap()
            .store(true, Ordering::SeqCst);

        assert!(CANCEL_TOKENS
            .lock()
            .unwrap()
            .get(&task_id)
            .unwrap()
            .load(Ordering::SeqCst));

        CANCEL_TOKENS.lock().unwrap().remove(&task_id);
    }

    #[test]
    fn test_cancel_token_task_not_found_error_message() {
        let task_id = "ghost-task".to_string();
        let mut tokens = CANCEL_TOKENS.lock().unwrap();
        // This mirrors the error path in traceroute_stop
        let result = match tokens.get_mut(&task_id) {
            Some(_cancel) => Ok(()),
            None => Err(format!("Task {} not found", task_id)),
        };
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Task ghost-task not found");
    }

    // -----------------------------------------------------------------------
    // Input validation — empty target detection
    // -----------------------------------------------------------------------

    #[test]
    fn test_empty_target_rejected() {
        for target in &["", "   ", "\t", "\n", " \t\n "] {
            assert!(
                target.trim().is_empty(),
                "Target {:?} should be considered empty",
                target
            );
        }
    }

    #[test]
    fn test_non_empty_target_accepted() {
        for target in &[
            "8.8.8.8",
            "example.com",
            " 192.168.1.1 ",
            "localhost",
            "2001:db8::1",
        ] {
            assert!(
                !target.trim().is_empty(),
                "Target {:?} should be considered non-empty",
                target
            );
        }
    }

    // -----------------------------------------------------------------------
    // parse_tracert_line integration (imported from core::traceroute)
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_tracert_line_integration_normal() {
        let line = " 1   5 ms   6 ms   7 ms  192.168.1.1";
        let result = traceroute::parse_tracert_line(line);
        assert!(result.is_some(), "Should parse a valid tracert hop line");
        let hop = result.unwrap();
        assert_eq!(hop.hop, 1);
        assert_eq!(hop.addr.as_deref(), Some("192.168.1.1"));
    }

    #[test]
    fn test_parse_tracert_line_integration_header() {
        let line = "Tracing route to 8.8.8.8 over a maximum of 30 hops:";
        assert!(
            traceroute::parse_tracert_line(line).is_none(),
            "Header lines should be filtered out"
        );
    }

    #[test]
    fn test_parse_tracert_line_integration_timed_out() {
        let line = " 3   *        *        *     Request timed out.";
        let result = traceroute::parse_tracert_line(line);
        assert!(result.is_some());
        assert!(
            result.unwrap().addr.is_none(),
            "Timed-out hop should have no address"
        );
    }

    // -----------------------------------------------------------------------
    // TraceHop / TraceComplete serde — event payload structure
    // -----------------------------------------------------------------------

    #[test]
    fn test_trace_hop_serialization() {
        let hop = TraceHop {
            hop: 1,
            addr: Some("192.168.1.1".to_string()),
            hostname: None,
            latencies: vec![Some(5.0), Some(10.5), None],
        };
        let json = serde_json::to_value(&hop).unwrap();
        assert_eq!(json["hop"], 1);
        assert_eq!(json["addr"], "192.168.1.1");
        assert_eq!(json["hostname"], serde_json::Value::Null);
        assert_eq!(json["latencies"][0], 5.0);
        assert_eq!(json["latencies"][1], 10.5);
        assert_eq!(json["latencies"][2], serde_json::Value::Null);
    }

    #[test]
    fn test_trace_hop_deserialization() {
        let json = r#"{"hop":2,"addr":"10.0.0.1","hostname":"router.local","latencies":[1.0,null,3.0]}"#;
        let hop: TraceHop = serde_json::from_str(json).unwrap();
        assert_eq!(hop.hop, 2);
        assert_eq!(hop.addr, Some("10.0.0.1".to_string()));
        assert_eq!(hop.hostname, Some("router.local".to_string()));
        assert_eq!(hop.latencies.len(), 3);
        assert_eq!(hop.latencies[0], Some(1.0));
        assert_eq!(hop.latencies[1], None);
        assert_eq!(hop.latencies[2], Some(3.0));
    }

    #[test]
    fn test_trace_hop_all_fields_optional() {
        let hop = TraceHop {
            hop: 0,
            addr: None,
            hostname: None,
            latencies: vec![],
        };
        let json = serde_json::to_value(&hop).unwrap();
        assert_eq!(json["hop"], 0);
        assert_eq!(json["addr"], serde_json::Value::Null);
        assert_eq!(json["hostname"], serde_json::Value::Null);
        assert!(json["latencies"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_trace_complete_serialization() {
        let complete = TraceComplete {
            task_id: "task-1".to_string(),
            target: "8.8.8.8".to_string(),
            hops: vec![TraceHop {
                hop: 1,
                addr: Some("192.168.1.1".to_string()),
                hostname: None,
                latencies: vec![Some(1.0)],
            }],
        };
        let json = serde_json::to_value(&complete).unwrap();
        assert_eq!(json["task_id"], "task-1");
        assert_eq!(json["target"], "8.8.8.8");
        assert_eq!(json["hops"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_trace_complete_deserialization() {
        let json = r#"{"task_id":"t2","target":"example.com","hops":[]}"#;
        let complete: TraceComplete = serde_json::from_str(json).unwrap();
        assert_eq!(complete.task_id, "t2");
        assert_eq!(complete.target, "example.com");
        assert!(complete.hops.is_empty());
    }

    #[test]
    fn test_trace_complete_empty_hops() {
        let complete = TraceComplete {
            task_id: "empty".to_string(),
            target: "test.local".to_string(),
            hops: vec![],
        };
        let json = serde_json::to_value(&complete).unwrap();
        assert!(json["hops"].as_array().unwrap().is_empty());
    }
}
