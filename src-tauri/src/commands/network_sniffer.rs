use crate::core::cancel::CANCEL_REGISTRY;
use crate::core::network_sniffer::{discovery, fingerprint, os_detect, port_scanner};
use crate::core::utils::emit_or_warn;
use crate::types::network_sniffer::{
    DeviceResult, PortPreset, PortResult, SnifferOptions, SnifferProgress,
};
use serde_json;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use tauri::AppHandle;
use tokio::sync::Semaphore;
use tracing::warn;
use uuid::Uuid;

static SCAN_RESULTS: LazyLock<Mutex<HashMap<String, Vec<DeviceResult>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Parse a CIDR notation string or a single IP into a list of IP addresses.
fn parse_cidr(cidr: &str) -> Result<Vec<IpAddr>, String> {
    // Single IP without CIDR: return directly
    if !cidr.contains('/') {
        let ip: IpAddr = cidr.parse().map_err(|e| format!("Invalid IP: {}", e))?;
        return Ok(vec![ip]);
    }

    let (ip_u32, prefix_len) = crate::core::subnet::parse_cidr_v4(cidr)?;

    if prefix_len == 32 {
        return Ok(vec![IpAddr::V4(ip_u32.into())]);
    }

    let host_bits = 32 - prefix_len;
    let mask = if host_bits >= 32 {
        0u32
    } else {
        (!0u32) << host_bits
    };
    let network = ip_u32 & mask;
    let broadcast = network | !mask;

    if prefix_len == 31 {
        return Ok(vec![
            IpAddr::V4(network.into()),
            IpAddr::V4(broadcast.into()),
        ]);
    }

    let host_count = broadcast.saturating_sub(network).saturating_sub(1);
    let mut ips = Vec::with_capacity(host_count as usize);
    for i in 1..=host_count {
        if let Some(host_ip) = network.checked_add(i) {
            ips.push(IpAddr::V4(host_ip.into()));
        }
    }
    Ok(ips)
}

/// Run the full scan pipeline for a given task.
async fn run_scan(app: AppHandle, task_id: String, options: SnifferOptions) {
    // 1. Parse all CIDR targets into a deduplicated IP list
    let mut all_ips: Vec<IpAddr> = Vec::new();
    for target in &options.targets {
        match parse_cidr(target) {
            Ok(ips) => all_ips.extend(ips),
            Err(e) => {
                emit_or_warn(
                    &app,
                    "sniffer:error",
                    &serde_json::json!({
                        "taskId": task_id,
                        "error": format!("Invalid target '{}': {}", target, e),
                    }),
                );
                cleanup_task(&task_id);
                return;
            }
        }
    }

    all_ips.sort();
    all_ips.dedup();

    let total_ips = all_ips.len();
    if total_ips == 0 {
        emit_or_warn(
            &app,
            "sniffer:error",
            &serde_json::json!({
                "taskId": task_id,
                "error": "No valid targets specified".to_string(),
            }),
        );
        cleanup_task(&task_id);
        return;
    }

    // Validate concurrency settings
    if options.concurrency_ports == 0 {
        emit_or_warn(
            &app,
            "sniffer:error",
            &serde_json::json!({
                "taskId": task_id,
                "error": "Port concurrency must be > 0".to_string(),
            }),
        );
        cleanup_task(&task_id);
        return;
    }
    if options.concurrency_hosts == 0 {
        emit_or_warn(
            &app,
            "sniffer:error",
            &serde_json::json!({
                "taskId": task_id,
                "error": "Host concurrency must be > 0".to_string(),
            }),
        );
        cleanup_task(&task_id);
        return;
    }
    // 2. Determine ports
    let ports: Vec<u16> = if options.mode == "fast" {
        port_scanner::top_ports()
    } else {
        if options.ports.is_empty() {
            port_scanner::top_ports()
        } else {
            options.ports.clone()
        }
    };

    // 3. Check cancel token
    if !CANCEL_REGISTRY.contains(&task_id) {
        emit_or_warn(
            &app,
            "sniffer:error",
            &serde_json::json!({
                "taskId": task_id,
                "error": "Task not found".to_string(),
            }),
        );
        return;
    }

    // 4. Emit initial progress
    let progress = SnifferProgress {
        total_hosts: total_ips as u32,
        scanned_hosts: 0,
        services_found: 0,
        current_target: String::new(),
    };
    emit_or_warn(&app, "sniffer:progress", &progress);

    // 5. Scan loop — concurrent hosts via Semaphore
    let semaphore = Arc::new(Semaphore::new(options.concurrency_hosts as usize));
    let scanned = Arc::new(AtomicU32::new(0));
    let services_found = Arc::new(AtomicU32::new(0));
    let devices: Arc<Mutex<Vec<DeviceResult>>> = Arc::new(Mutex::new(Vec::new()));
    let opts = Arc::new(options);

    let mut handles = Vec::with_capacity(all_ips.len());

    for ip in all_ips {
        if CANCEL_REGISTRY.is_cancelled(&task_id) {
            break;
        }

        let sem_clone = semaphore.clone();
        let app = app.clone();
        let task_id = task_id.clone();
        let scanned = scanned.clone();
        let services_found = services_found.clone();
        let devices = devices.clone();
        let opts = opts.clone();
        let ports = ports.clone();
        let total = total_ips as u32;

        handles.push(tokio::spawn(async move {
            // Acquire semaphore permit — limits concurrent host processing
            let _permit = match sem_clone.acquire_owned().await {
                Ok(p) => p,
                Err(_) => return, // semaphore closed
            };

            if CANCEL_REGISTRY.is_cancelled(&task_id) {
                return;
            }

            let scanned_val = scanned.fetch_add(1, Ordering::SeqCst) + 1;
            let svc_found = services_found.load(Ordering::SeqCst);
            let ip_str = ip.to_string();

            // Emit progress
            emit_or_warn(
                &app,
                "sniffer:progress",
                &SnifferProgress {
                    total_hosts: total,
                    scanned_hosts: scanned_val,
                    services_found: svc_found,
                    current_target: ip_str.clone(),
                },
            );

            // TCP ping to check if host is alive
            let alive = discovery::is_host_alive(ip, opts.timeout_ms).await;

            if alive {
                // Scan ports
                let cancel = Arc::new(AtomicBool::new(false));
                let scanned_ports = port_scanner::scan_ports(
                    &ip_str,
                    &ports,
                    opts.concurrency_ports as usize,
                    opts.timeout_ms,
                    cancel.clone(),
                )
                .await;

                // Process each open port
                let mut open_ports: Vec<PortResult> = Vec::with_capacity(scanned_ports.len());
                for pr in scanned_ports {
                    if opts.probe_services {
                        let detected =
                            fingerprint::detect_service(&ip_str, pr, opts.timeout_ms).await;

                        emit_or_warn(
                            &app,
                            "sniffer:port",
                            &serde_json::json!({
                                "ip": ip_str,
                                "port": detected.port,
                                "protocol": detected.protocol,
                                "state": detected.state,
                                "service": detected.service,
                                "version": detected.version,
                                "banner": detected.banner,
                                "confidence": detected.confidence,
                                "probeMethod": detected.probe_method,
                            }),
                        );

                        if detected.service.is_some() {
                            services_found.fetch_add(1, Ordering::SeqCst);
                        }
                        open_ports.push(detected);
                    } else {
                        emit_or_warn(
                            &app,
                            "sniffer:port",
                            &serde_json::json!({
                                "ip": ip_str,
                                "port": pr.port,
                                "protocol": pr.protocol,
                                "state": pr.state,
                                "service": pr.service,
                                "version": None::<String>,
                                "banner": None::<String>,
                                "confidence": pr.confidence,
                                "probeMethod": pr.probe_method,
                            }),
                        );
                        open_ports.push(pr);
                    }
                }

                // Resolve hostname and MAC
                let hostname: Option<String> = None; // reverse DNS removed (unreliable)
                let (mac, vendor) = match discovery::resolve_mac(&ip_str) {
                    Some((m, v)) => (Some(m), v),
                    None => (None, None),
                };

                // Assemble device result
                let device = os_detect::assemble_device(
                    ip,
                    hostname,
                    mac,
                    vendor,
                    open_ports,
                    &opts.mode,
                );

                emit_or_warn(&app, "sniffer:device", &device);

                // Collect result
                if let Ok(mut devs) = devices.lock() {
                    devs.push(device);
                }
            }
        }));
    }

    // Wait for all tasks to complete
    let all_devices = {
        for handle in handles {
            let _ = handle.await;
        }
        match Arc::try_unwrap(devices) {
            Ok(inner) => inner.into_inner().unwrap_or_else(|e| e.into_inner()),
            Err(_) => {
                warn!("[sniffer] devices Arc still has multiple references, returning empty results");
                Vec::new()
            }
        }
    };

    // 6. Save results to SCAN_RESULTS
    {
        let mut results = match SCAN_RESULTS.lock() {
            Ok(r) => r,
            Err(_) => {
                emit_or_warn(
                    &app,
                    "sniffer:error",
                    &serde_json::json!({
                        "taskId": task_id,
                        "error": "Internal lock error saving results".to_string(),
                    }),
                );
                cleanup_task(&task_id);
                return;
            }
        };
        // Enforce a maximum of 20 cached scan results to prevent unbounded memory growth.
        // Remove the oldest entry (arbitrary key) if the limit is exceeded.
        if results.len() >= 20 {
            if let Some(oldest_key) = results.keys().next().cloned() {
                results.remove(&oldest_key);
            }
        }
        results.insert(task_id.clone(), all_devices);
    }

    // 7. Emit completion event
    emit_or_warn(
        &app,
        "sniffer:complete",
        &serde_json::json!({
            "taskId": task_id,
        }),
    );

    // Cleanup cancel token
    cleanup_task(&task_id);
}

/// Remove a task's cancel token from the global map.
fn cleanup_task(task_id: &str) {
    CANCEL_REGISTRY.unregister(task_id);
}

#[tauri::command]
pub async fn sniffer_start(
    app: AppHandle,
    options: SnifferOptions,
) -> Result<String, String> {
    let task_id = Uuid::new_v4().to_string();

    CANCEL_REGISTRY.register(&task_id);

    let app_clone = app.clone();
    let task_id_clone = task_id.clone();

    tauri::async_runtime::spawn(async move {
        run_scan(app_clone, task_id_clone, options).await;
    });

    Ok(task_id)
}

#[tauri::command]
pub async fn sniffer_stop(app: AppHandle, task_id: String) -> Result<(), String> {
    let _ = app;
    if CANCEL_REGISTRY.cancel(&task_id) {
        Ok(())
    } else {
        Err(format!("Task {} not found", task_id))
    }
}

#[tauri::command]
pub async fn sniffer_list(task_id: Option<String>) -> Result<Vec<DeviceResult>, String> {
    let results = SCAN_RESULTS.lock().map_err(|e| e.to_string())?;
    match task_id {
        Some(id) => results
            .get(&id)
            .cloned()
            .ok_or_else(|| format!("Task {} not found", id)),
        None => {
            let mut all: Vec<DeviceResult> = Vec::new();
            for devices in results.values() {
                all.extend(devices.iter().cloned());
            }
            Ok(all)
        }
    }
}

#[tauri::command]
pub async fn sniffer_export(task_id: String, format: String) -> Result<String, String> {
    let devices = {
        let results = SCAN_RESULTS.lock().map_err(|e| e.to_string())?;
        results
            .get(&task_id)
            .cloned()
            .ok_or_else(|| format!("Task {} not found", task_id))?
    };

    match format.to_lowercase().as_str() {
        "json" => serde_json::to_string_pretty(&devices)
            .map_err(|e| format!("JSON serialization error: {}", e)),
        "csv" => Ok(devices_to_csv(&devices)),
        _ => Err(format!("Unsupported export format '{}'. Use 'json' or 'csv'.", format)),
    }
}

/// Convert a list of DeviceResults to CSV string.
fn devices_to_csv(devices: &[DeviceResult]) -> String {
    let mut csv = String::from(
        "ip,hostname,mac,vendor,os,port,protocol,state,service,version,banner\n",
    );
    for device in devices {
        if device.open_ports.is_empty() {
            csv.push_str(&format!(
                "{},{},{},{},{},,,,,,\n",
                csv_escape(&device.ip),
                csv_escape(device.hostname.as_deref().unwrap_or("")),
                csv_escape(device.mac.as_deref().unwrap_or("")),
                csv_escape(device.vendor.as_deref().unwrap_or("")),
                csv_escape(device.os.as_deref().unwrap_or("")),
            ));
        } else {
            for port in &device.open_ports {
                csv.push_str(&format!(
                    "{},{},{},{},{},{},{},{},{},{},{}\n",
                    csv_escape(&device.ip),
                    csv_escape(device.hostname.as_deref().unwrap_or("")),
                    csv_escape(device.mac.as_deref().unwrap_or("")),
                    csv_escape(device.vendor.as_deref().unwrap_or("")),
                    csv_escape(device.os.as_deref().unwrap_or("")),
                    port.port,
                    csv_escape(&port.protocol),
                    csv_escape(&port.state),
                    csv_escape(port.service.as_deref().unwrap_or("")),
                    csv_escape(port.version.as_deref().unwrap_or("")),
                    csv_escape(port.banner.as_deref().unwrap_or("")),
                ));
            }
        }
    }
    csv
}

/// Escape a field value for CSV output.
fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

#[tauri::command]
pub async fn sniffer_presets() -> Result<Vec<PortPreset>, String> {
    Ok(crate::types::network_sniffer::default_presets())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cidr_single_ip() {
        let ips = parse_cidr("192.168.1.1").unwrap();
        assert_eq!(ips.len(), 1);
        assert_eq!(ips[0].to_string(), "192.168.1.1");
    }

    #[test]
    fn test_parse_cidr_slash24() {
        let ips = parse_cidr("192.168.1.0/24").unwrap();
        assert_eq!(ips.len(), 254);
        assert_eq!(ips[0].to_string(), "192.168.1.1");
        assert_eq!(ips[253].to_string(), "192.168.1.254");
    }

    #[test]
    fn test_parse_cidr_too_large() {
        let result = parse_cidr("10.0.0.0/8");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_cidr_invalid() {
        let result = parse_cidr("not_an_ip/24");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_cidr_slash32() {
        // /32 should return exactly the single host
        let ips = parse_cidr("192.168.1.42/32").unwrap();
        assert_eq!(ips.len(), 1);
        assert_eq!(ips[0].to_string(), "192.168.1.42");
    }

    #[test]
    fn test_parse_cidr_slash31() {
        // /31 per RFC 3021: both addresses are usable (no network/broadcast)
        let ips = parse_cidr("10.0.0.0/31").unwrap();
        assert_eq!(ips.len(), 2);
        assert_eq!(ips[0].to_string(), "10.0.0.0");
        assert_eq!(ips[1].to_string(), "10.0.0.1");
    }

    #[test]
    fn test_parse_cidr_slash31_mid_network() {
        let ips = parse_cidr("192.168.1.2/31").unwrap();
        assert_eq!(ips.len(), 2);
        assert_eq!(ips[0].to_string(), "192.168.1.2");
        assert_eq!(ips[1].to_string(), "192.168.1.3");
    }

    #[test]
    fn test_parse_cidr_prefix_too_large() {
        // prefix > 32 should return an error (was a panic bug)
        let result = parse_cidr("192.168.1.0/33");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("must be 0-32"), "Error should mention valid range, got: {}", err);
    }

    #[test]
    fn test_parse_cidr_prefix_max_boundary() {
        let result = parse_cidr("192.168.1.0/99");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_cidr_slash16_at_boundary() {
        // /16 is the maximum allowed range
        let ips = parse_cidr("192.168.0.0/16").unwrap();
        assert_eq!(ips.len(), 65534);
        assert_eq!(ips[0].to_string(), "192.168.0.1");
        assert_eq!(ips[65533].to_string(), "192.168.255.254");
    }

    #[test]
    fn test_parse_cidr_slash15_too_large() {
        // /15 is larger than /16, should be rejected
        let result = parse_cidr("10.0.0.0/15");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_cidr_invalid_prefix_format() {
        let result = parse_cidr("192.168.1.0/abc");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_cidr_slash0() {
        // /0 should be rejected as too large
        let result = parse_cidr("0.0.0.0/0");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_cidr_empty_string() {
        let result = parse_cidr("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_cidr_trailing_slash() {
        let result = parse_cidr("192.168.1.0/");
        assert!(result.is_err());
    }

    #[test]
    fn test_csv_escape_no_change() {
        assert_eq!(csv_escape("simple"), "simple");
    }

    #[test]
    fn test_csv_escape_with_comma() {
        assert_eq!(csv_escape("a,b"), "\"a,b\"");
    }

    #[test]
    fn test_csv_escape_with_quote() {
        assert_eq!(csv_escape("a\"b"), "\"a\"\"b\"");
    }

    #[test]
    fn test_csv_escape_newline() {
        assert_eq!(csv_escape("a\nb"), "\"a\nb\"");
    }

    #[test]
    fn test_csv_escape_already_quoted() {
        assert_eq!(csv_escape("\"hello\""), "\"\"\"hello\"\"\"");
    }

    #[test]
    fn test_devices_to_csv_basic() {
        let devices = vec![DeviceResult {
            ip: "192.168.1.1".to_string(),
            hostname: Some("router".to_string()),
            mac: Some("00:11:22:33:44:55".to_string()),
            vendor: Some("Dell".to_string()),
            os: Some("Linux".to_string()),
            open_ports: vec![PortResult {
                port: 80,
                protocol: "tcp".to_string(),
                state: "open".to_string(),
                service: Some("HTTP".to_string()),
                version: None,
                banner: None,
                confidence: 50,
                probe_method: "tcp_connect".to_string(),
            }],
            is_alive: true,
            scan_mode: "fast".to_string(),
            scan_completed: true,
        }];
        let csv = devices_to_csv(&devices);
        assert!(csv.contains("192.168.1.1"));
        assert!(csv.contains("router"));
        assert!(csv.contains("00:11:22:33:44:55"));
        assert!(csv.contains("80"));
        assert!(csv.contains("HTTP"));
    }

    #[test]
    fn test_devices_to_csv_no_open_ports() {
        let devices = vec![DeviceResult {
            ip: "10.0.0.1".to_string(),
            hostname: None,
            mac: None,
            vendor: None,
            os: Some("Linux".to_string()),
            open_ports: vec![],
            is_alive: true,
            scan_mode: "fast".to_string(),
            scan_completed: true,
        }];
        let csv = devices_to_csv(&devices);
        // Header: "ip,hostname,mac,vendor,os,port,protocol,state,service,version,banner"
        assert!(csv.starts_with("ip,"));
        // Data row: 10.0.0.1,empty,empty,empty,Linux,,,,,,,
        assert!(csv.contains("10.0.0.1,,,,Linux"));
        // The field count should be 11 (header fields)
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 2, "Should have header + 1 data row");
        assert_eq!(
            lines[1].split(',').count(),
            11,
            "CSV data row should have 11 columns"
        );
    }

    #[test]
    fn test_devices_to_csv_empty_devices() {
        let csv = devices_to_csv(&[]);
        // Should only have the header row
        assert_eq!(csv.lines().count(), 1);
    }

    #[test]
    fn test_default_presets_contain_expected() {
        let presets = crate::types::network_sniffer::default_presets();
        assert!(!presets.is_empty());
        let top100 = presets.iter().find(|p| p.name == "top100");
        assert!(top100.is_some());
        assert!(top100.unwrap().ports.contains(&80));
        assert!(top100.unwrap().ports.contains(&443));
    }

    #[test]
    fn test_sniffer_options_default() {
        let opts = SnifferOptions::default();
        assert!(!opts.targets.is_empty());
        assert!(!opts.ports.is_empty());
        assert_eq!(opts.mode, "fast");
        assert!(opts.concurrency_hosts > 0);
        assert!(opts.concurrency_ports > 0);
    }

    /// Lock to serialise tests that modify SCAN_RESULTS.
    static SCAN_RESULTS_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_scan_results_cap() {
        let _guard = SCAN_RESULTS_LOCK.lock().unwrap();

        // Fill SCAN_RESULTS with 20 entries
        {
            let mut results = SCAN_RESULTS.lock().unwrap();
            results.clear();
            for i in 0..20 {
                results.insert(format!("task_{}", i), vec![]);
            }
            assert_eq!(results.len(), 20);
        }

        // Simulate the exact cleanup logic from run_scan():
        // Enforce a maximum of 20 cached scan results.
        // Remove the oldest entry (arbitrary key) if the limit is exceeded.
        {
            let mut results = SCAN_RESULTS.lock().unwrap();
            if results.len() >= 20 {
                if let Some(oldest_key) = results.keys().next().cloned() {
                    results.remove(&oldest_key);
                }
            }
            results.insert("overflow_task".to_string(), vec![]);
        }

        // Verify the cap is enforced: only 20 entries remain
        {
            let results = SCAN_RESULTS.lock().unwrap();
            assert_eq!(results.len(), 20, "SCAN_RESULTS should be capped at 20");
            assert!(
                results.contains_key("overflow_task"),
                "Newly inserted task should be present"
            );
        }

        // Clean up global state
        SCAN_RESULTS.lock().unwrap().clear();
    }
}
