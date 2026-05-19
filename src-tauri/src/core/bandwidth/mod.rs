use std::collections::HashMap;

use crate::types::bandwidth::{BandwidthSample, InterfaceInfo};
use chrono::Local;

/// Raw counter snapshot for an interface.
#[derive(Debug, Clone)]
pub struct CounterSnapshot {
    pub rx: u64,
    pub tx: u64,
}

/// Retrieve a list of network interfaces that are enabled and have an IP.
pub fn get_interfaces() -> Result<Vec<InterfaceInfo>, String> {
    // On Windows, use wmic to get interface info.
    #[cfg(windows)]
    {
        get_interfaces_wmic()
    }

    // On non-Windows, provide a placeholder or minimal implementation.
    #[cfg(not(windows))]
    {
        get_interfaces_default()
    }
}

/// Read the current raw byte counters for all network interfaces.
pub fn get_counters() -> Result<HashMap<String, CounterSnapshot>, String> {
    #[cfg(windows)]
    {
        get_counters_wmic()
    }

    #[cfg(not(windows))]
    {
        get_counters_procfs()
    }
}

/// Compute bandwidth samples by comparing current counters against a previous snapshot.
///
/// The `interval_secs` parameter is the number of seconds between the two samples.
pub fn compute_samples(
    previous: &HashMap<String, CounterSnapshot>,
    current: &HashMap<String, CounterSnapshot>,
    interval_secs: f64,
) -> Vec<BandwidthSample> {
    let timestamp = Local::now().format("%Y-%m-%dT%H:%M:%S%.3f").to_string();
    let mut samples = Vec::new();

    // Accumulate totals for "all interfaces" aggregate.
    let mut agg_rx: u64 = 0;
    let mut agg_tx: u64 = 0;
    let mut agg_prev_rx: u64 = 0;
    let mut agg_prev_tx: u64 = 0;

    for (name, curr) in current {
        if let Some(prev) = previous.get(name) {
            let raw_rx = curr.rx.saturating_sub(prev.rx);
            let raw_tx = curr.tx.saturating_sub(prev.tx);

            // Convert from raw counter difference to bytes per second.
            let download_bps = if interval_secs > 0.0 {
                (raw_rx as f64 / interval_secs) as u64
            } else {
                0
            };
            let upload_bps = if interval_secs > 0.0 {
                (raw_tx as f64 / interval_secs) as u64
            } else {
                0
            };

            samples.push(BandwidthSample {
                interface: name.clone(),
                download_bps,
                upload_bps,
                total_rx: curr.rx,
                total_tx: curr.tx,
                timestamp: timestamp.clone(),
            });

            agg_rx += curr.rx;
            agg_tx += curr.tx;
            agg_prev_rx += prev.rx;
            agg_prev_tx += prev.tx;
        }
    }

    // Add an aggregate "*" entry representing total across all interfaces.
    let raw_rx = agg_rx.saturating_sub(agg_prev_rx);
    let raw_tx = agg_tx.saturating_sub(agg_prev_tx);
    let agg_download = if interval_secs > 0.0 {
        (raw_rx as f64 / interval_secs) as u64
    } else {
        0
    };
    let agg_upload = if interval_secs > 0.0 {
        (raw_tx as f64 / interval_secs) as u64
    } else {
        0
    };
    samples.push(BandwidthSample {
        interface: "*".to_string(),
        download_bps: agg_download,
        upload_bps: agg_upload,
        total_rx: agg_rx,
        total_tx: agg_tx,
        timestamp,
    });

    samples
}

// ---------------------------------------------------------------------------
// Windows implementation (wmic)
// ---------------------------------------------------------------------------

#[cfg(windows)]
fn get_interfaces_wmic() -> Result<Vec<InterfaceInfo>, String> {
    let output = std::process::Command::new("wmic")
        .args([
            "nic",
            "where",
            "NetEnabled=TRUE",
            "get",
            "Index,NetConnectionID,Name",
            "/format:csv",
        ])
        .output()
        .map_err(|e| format!("Failed to run wmic for interfaces: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let interfaces = parse_interface_csv(&stdout)?;

    // Get IP addresses per interface index.
    let ip_output = std::process::Command::new("wmic")
        .args([
            "nicconfig",
            "where",
            "IPEnabled=TRUE",
            "get",
            "Index,IPAddress",
            "/format:csv",
        ])
        .output()
        .map_err(|e| format!("Failed to run wmic for IP config: {}", e))?;

    let ip_stdout = String::from_utf8_lossy(&ip_output.stdout);
    let ip_map = parse_ip_config_csv(&ip_stdout);

    // Merge IP addresses into interfaces.
    let mut result: Vec<InterfaceInfo> = interfaces
        .into_iter()
        .map(|mut info| {
            // Extract index from name (we stored index in name temporarily)
            let idx = info.name.clone();
            if let Some(ip) = ip_map.get(&idx) {
                info.ip = ip.clone();
            }
            info
        })
        .collect();

    // Sort: aggregate "*" last
    result.sort_by_key(|i| if i.name == "*" { 1 } else { 0 });

    Ok(result)
}

#[cfg(windows)]
fn parse_interface_csv(output: &str) -> Result<Vec<InterfaceInfo>, String> {
    let mut interfaces = Vec::new();
    let mut lines = output.lines().filter(|l| !l.is_empty());

    // Skip header line.
    let _header = lines.next();

    for line in lines {
        let line = line.trim();
        if line.is_empty() || line.to_lowercase().contains("node") {
            continue;
        }
        let parts: Vec<&str> = line.split(',').collect();
        // Format: Node,Index,NetConnectionID,Name
        if parts.len() >= 4 {
            let index = parts[1].trim().to_string();
            let friendly_name = parts[2].trim().to_string();
            let name = parts[3].trim().to_string();
            if !name.is_empty() {
                interfaces.push(InterfaceInfo {
                    name: index.clone(), // Store index for later IP lookup
                    friendly_name,
                    ip: String::new(),
                });
            }
        }
    }

    Ok(interfaces)
}

#[cfg(windows)]
fn parse_ip_config_csv(output: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut lines = output.lines().filter(|l| !l.is_empty());

    let _header = lines.next();

    for line in lines {
        let line = line.trim();
        if line.is_empty() || line.to_lowercase().contains("node") {
            continue;
        }
        let parts: Vec<&str> = line.split(',').collect();
        // Format: Node,Index,IPAddress
        if parts.len() >= 3 {
            let index = parts[1].trim().to_string();
            let ip = parts[2].trim().to_string();
            // IPAddress is semicolon-separated list; take the first IPv4.
            let first_ip = ip.split(';').next().unwrap_or("").to_string();
            map.insert(index, first_ip);
        }
    }

    map
}

#[cfg(windows)]
fn get_counters_wmic() -> Result<HashMap<String, CounterSnapshot>, String> {
    let output = std::process::Command::new("wmic")
        .args([
            "path",
            "Win32_PerfRawData_Tcpip_NetworkInterface",
            "get",
            "Name,BytesReceivedPersec,BytesSentPersec",
            "/format:csv",
        ])
        .output()
        .map_err(|e| format!("Failed to run wmic for counters: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_counter_csv(&stdout)
}

#[cfg(windows)]
fn parse_counter_csv(output: &str) -> Result<HashMap<String, CounterSnapshot>, String> {
    let mut counters = HashMap::new();
    let mut lines = output.lines().filter(|l| !l.is_empty());

    let _header = lines.next();

    for line in lines {
        let line = line.trim();
        if line.is_empty() || line.to_lowercase().contains("node") {
            continue;
        }
        let parts: Vec<&str> = line.split(',').collect();
        // Format: Node,Name,BytesReceivedPersec,BytesSentPersec
        if parts.len() >= 4 {
            let name = parts[1].trim().to_string();
            let rx: u64 = parts[2].trim().parse().unwrap_or(0);
            let tx: u64 = parts[3].trim().parse().unwrap_or(0);
            counters.insert(name, CounterSnapshot { rx, tx });
        }
    }

    Ok(counters)
}

// ---------------------------------------------------------------------------
// Non-Windows fallback (Linux /proc/net/dev)
// ---------------------------------------------------------------------------

#[cfg(not(windows))]
fn get_interfaces_default() -> Result<Vec<InterfaceInfo>, String> {
    let content = std::fs::read_to_string("/proc/net/dev")
        .map_err(|e| format!("Failed to read /proc/net/dev: {}", e))?;

    let mut interfaces = Vec::new();
    for line in content.lines().skip(2) {
        // Skip header lines
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() >= 2 {
            let name = parts[0].trim().to_string();
            // Skip loopback
            if name == "lo" {
                continue;
            }
            interfaces.push(InterfaceInfo {
                name: name.clone(),
                friendly_name: name.clone(),
                ip: String::new(),
            });
        }
    }

    Ok(interfaces)
}

#[cfg(not(windows))]
fn get_counters_procfs() -> Result<HashMap<String, CounterSnapshot>, String> {
    let content =
        std::fs::read_to_string("/proc/net/dev").map_err(|e| format!("Failed to read /proc/net/dev: {}", e))?;

    let mut counters = HashMap::new();
    for line in content.lines().skip(2) {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() >= 2 {
            let name = parts[0].trim().to_string();
            let values: Vec<&str> = parts[1].split_whitespace().collect();
            if values.len() >= 10 {
                let rx: u64 = values[0].parse().unwrap_or(0);
                let tx: u64 = values[8].parse().unwrap_or(0);
                counters.insert(name, CounterSnapshot { rx, tx });
            }
        }
    }

    Ok(counters)
}
