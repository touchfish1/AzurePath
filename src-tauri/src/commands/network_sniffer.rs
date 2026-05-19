use crate::core::network_sniffer::{discovery, fingerprint, os_detect, port_scanner};
use crate::types::network_sniffer::{
    DeviceResult, PortPreset, PortResult, SnifferOptions, SnifferProgress,
};
use serde_json;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

static SCAN_RESULTS: LazyLock<Mutex<HashMap<String, Vec<DeviceResult>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static CANCEL_TOKENS: LazyLock<Mutex<HashMap<String, Arc<AtomicBool>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Parse a CIDR notation string or a single IP into a list of IP addresses.
fn parse_cidr(cidr: &str) -> Result<Vec<IpAddr>, String> {
    let (base, prefix_len) = match cidr.split_once('/') {
        Some((ip, len)) => {
            let ip: IpAddr = ip.parse().map_err(|e| format!("Invalid IP: {}", e))?;
            let len: u8 = len.parse().map_err(|_| "Invalid CIDR prefix".to_string())?;
            (ip, len)
        }
        None => {
            let ip: IpAddr = cidr.parse().map_err(|e| format!("Invalid IP: {}", e))?;
            return Ok(vec![ip]);
        }
    };

    let ip_u32 = match base {
        IpAddr::V4(v4) => u32::from(v4),
        IpAddr::V6(_) => return Err("IPv6 not supported yet".to_string()),
    };

    let mask = if prefix_len == 0 {
        0u32
    } else {
        !0u32 << (32 - prefix_len)
    };
    let network = ip_u32 & mask;
    let broadcast = network | !mask;
    let host_count = broadcast.saturating_sub(network).saturating_sub(1);
    if host_count > 65534 {
        return Err("CIDR range too large (max /16)".to_string());
    }

    let mut ips = Vec::with_capacity(host_count as usize + 1);
    for i in 1..=host_count {
        if let Some(host_ip) = network.checked_add(i) {
            if host_ip < broadcast {
                ips.push(IpAddr::V4(host_ip.into()));
            }
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
                let _ = app.emit(
                    "sniffer:error",
                    serde_json::json!({
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
        let _ = app.emit(
            "sniffer:error",
            serde_json::json!({
                "taskId": task_id,
                "error": "No valid targets specified".to_string(),
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

    // 3. Get cancel token
    let cancel = {
        let tokens = match CANCEL_TOKENS.lock() {
            Ok(t) => t,
            Err(_) => {
                let _ = app.emit(
                    "sniffer:error",
                    serde_json::json!({
                        "taskId": task_id,
                        "error": "Internal lock error".to_string(),
                    }),
                );
                return;
            }
        };
        match tokens.get(&task_id).cloned() {
            Some(c) => c,
            None => {
                let _ = app.emit(
                    "sniffer:error",
                    serde_json::json!({
                        "taskId": task_id,
                        "error": "Task not found".to_string(),
                    }),
                );
                return;
            }
        }
    };

    // 4. Emit initial progress
    let progress = SnifferProgress {
        total_hosts: total_ips as u32,
        scanned_hosts: 0,
        services_found: 0,
        current_target: String::new(),
    };
    let _ = app.emit("sniffer:progress", &progress);

    // 5. Scan loop
    let mut all_devices: Vec<DeviceResult> = Vec::new();
    let mut scanned: u32 = 0;
    let mut services_found: u32 = 0;

    for ip in &all_ips {
        if cancel.load(Ordering::SeqCst) {
            break;
        }

        scanned += 1;
        let ip_str = ip.to_string();

        // Emit progress for current target
        let progress = SnifferProgress {
            total_hosts: total_ips as u32,
            scanned_hosts: scanned,
            services_found,
            current_target: ip_str.clone(),
        };
        let _ = app.emit("sniffer:progress", &progress);

        // TCP ping to check if host is alive
        let alive = discovery::is_host_alive(*ip, options.timeout_ms).await;

        if alive {
            // Scan ports
            let scanned_ports = port_scanner::scan_ports(
                &ip_str,
                &ports,
                options.concurrency_ports as usize,
                options.timeout_ms,
                cancel.clone(),
            )
            .await;

            // Process each open port: detect service or emit plain port event
            let mut open_ports: Vec<PortResult> = Vec::with_capacity(scanned_ports.len());
            for pr in scanned_ports {
                if options.probe_services {
                    let detected =
                        fingerprint::detect_service(&ip_str, pr, options.timeout_ms).await;

                    let _ = app.emit(
                        "sniffer:port",
                        serde_json::json!({
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
                        services_found += 1;
                    }
                    open_ports.push(detected);
                } else {
                    let _ = app.emit(
                        "sniffer:port",
                        serde_json::json!({
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
            let hostname = discovery::resolve_hostname(&ip_str);
            let (mac, vendor) = match discovery::resolve_mac(&ip_str) {
                Some((m, v)) => (Some(m), v),
                None => (None, None),
            };

            // Assemble device result
            let device = os_detect::assemble_device(
                *ip,
                hostname,
                mac,
                vendor,
                open_ports,
                &options.mode,
            );

            let _ = app.emit("sniffer:device", &device);
            all_devices.push(device);
        }
    }

    // 6. Save results to SCAN_RESULTS
    {
        let mut results = match SCAN_RESULTS.lock() {
            Ok(r) => r,
            Err(_) => {
                let _ = app.emit(
                    "sniffer:error",
                    serde_json::json!({
                        "taskId": task_id,
                        "error": "Internal lock error saving results".to_string(),
                    }),
                );
                cleanup_task(&task_id);
                return;
            }
        };
        results.insert(task_id.clone(), all_devices);
    }

    // 7. Emit completion event
    let _ = app.emit(
        "sniffer:complete",
        serde_json::json!({
            "taskId": task_id,
        }),
    );

    // Cleanup cancel token
    cleanup_task(&task_id);
}

/// Remove a task's cancel token from the global map.
fn cleanup_task(task_id: &str) {
    if let Ok(mut tokens) = CANCEL_TOKENS.lock() {
        tokens.remove(task_id);
    }
}

#[tauri::command]
pub async fn sniffer_start(
    app: AppHandle,
    options: SnifferOptions,
) -> Result<String, String> {
    let task_id = Uuid::new_v4().to_string();
    let cancel = Arc::new(AtomicBool::new(false));

    {
        let mut tokens = CANCEL_TOKENS.lock().map_err(|e| e.to_string())?;
        tokens.insert(task_id.clone(), cancel);
    }

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
    let cancel = {
        let tokens = CANCEL_TOKENS.lock().map_err(|e| e.to_string())?;
        tokens.get(&task_id).cloned()
    };
    match cancel {
        Some(c) => {
            c.store(true, Ordering::SeqCst);
            Ok(())
        }
        None => Err(format!("Task {} not found", task_id)),
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
}
