use tokio::process::Command;

#[cfg(target_os = "windows")]
use encoding_rs::GBK;

/// Decode ping output bytes to a UTF-8 string.
/// On non-Windows platforms, bytes are expected to be UTF-8.
/// On Windows, the system locale encoding (e.g. GBK for Chinese) is used as fallback.
fn decode_ping_output(bytes: &[u8]) -> String {
    #[cfg(target_os = "windows")]
    {
        String::from_utf8(bytes.to_vec())
            .unwrap_or_else(|_| GBK.decode(bytes).0.to_string())
    }
    #[cfg(not(target_os = "windows"))]
    {
        String::from_utf8_lossy(bytes).to_string()
    }
}

#[derive(Debug, Clone)]
pub struct PingResult {
    pub seq: u32,
    pub latency_ms: f64,
    pub ttl: u32,
    pub status: String,
}

#[derive(Debug, Clone, Default)]
pub struct PingStats {
    pub sent: u32,
    pub received: u32,
    pub loss_percent: f64,
    pub min_ms: f64,
    pub avg_ms: f64,
    pub max_ms: f64,
}

/// Execute the system ping command and return raw stdout.
pub async fn execute_ping(
    target: &str,
    count: u32,
    timeout_ms: u64,
) -> Result<String, String> {
    let output = if cfg!(target_os = "windows") {
        // Windows: ping -n <count> -w <timeout_ms> <target>
        Command::new("ping")
            .arg("-n")
            .arg(count.to_string())
            .arg("-w")
            .arg(timeout_ms.to_string())
            .arg(target)
            .output()
            .await
            .map_err(|e| format!("Failed to execute ping: {}", e))?
    } else {
        // Unix: ping -c <count> -W <timeout_s> <target>
        let timeout_s = (timeout_ms / 1000).max(1);
        Command::new("ping")
            .arg("-c")
            .arg(count.to_string())
            .arg("-W")
            .arg(timeout_s.to_string())
            .arg(target)
            .output()
            .await
            .map_err(|e| format!("Failed to execute ping: {}", e))?
    };

    // On Windows, system tools may output in the system locale encoding (e.g. GBK for Chinese).
    // Try UTF-8 first, then fall back to the platform encoding.
    let output_str = decode_ping_output(&output.stdout);

    if !output.status.success() && output.stdout.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Ping failed: {}", stderr));
    }

    Ok(output_str)
}

/// Parse ping output into individual PingResult entries.
pub fn parse_ping_output(output: &str) -> Vec<PingResult> {
    let mut results = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(result) = parse_ping_line(line) {
            results.push(result);
        }
    }

    results
}

fn parse_ping_line(line: &str) -> Option<PingResult> {
    if cfg!(target_os = "windows") {
        parse_windows_ping_line(line)
    } else {
        parse_unix_ping_line(line)
    }
}

/// Parse a Windows ping reply line.
///
/// English:  "Reply from 8.8.8.8: bytes=32 time=11ms TTL=118"
/// Chinese:  "来自 8.8.8.8 的回复: 字节=32 时间=11ms TTL=118"
/// Timeouts: "Request timed out." / "请求超时。"
/// Unreach:  "Destination host unreachable." / "无法访问目标主机。"
fn parse_windows_ping_line(line: &str) -> Option<PingResult> {
    // Timeout — check both English and Chinese
    if line.contains("timed out") || line.contains("超时") {
        return Some(PingResult {
            seq: 0,
            latency_ms: -1.0,
            ttl: 0,
            status: "timeout".to_string(),
        });
    }

    // Unreachable — check both languages
    if line.contains("unreachable") || line.contains("无法访问") {
        return Some(PingResult {
            seq: 0,
            latency_ms: -1.0,
            ttl: 0,
            status: "unreachable".to_string(),
        });
    }

    // Success: TTL= is universal across all locale variants on Windows
    if !line.contains("TTL=") {
        return None;
    }

    let latency_ms = extract_latency_ms(line);
    let ttl = extract_ttl(line);

    Some(PingResult {
        seq: 0,
        latency_ms,
        ttl,
        status: "success".to_string(),
    })
}

/// Extract latency value from a ping reply line.
/// Works for both "time=11ms" and "时间=11ms" by scanning for "ms" backwards.
fn extract_latency_ms(line: &str) -> f64 {
    let bytes = line.as_bytes();
    // Scan backwards from "ms" to find the number before it, preceded by "="
    for i in (1..bytes.len().saturating_sub(1)).rev() {
        if bytes[i] == b'm' && bytes[i + 1] == b's' {
            // Walk backwards to find the start of the number
            let mut end = i;
            let mut start = end;
            while start > 0
                && (bytes[start - 1].is_ascii_digit() || bytes[start - 1] == b'.')
            {
                start -= 1;
            }
            if start > 0 && bytes[start - 1] == b'=' && start < end {
                if let Some(num) = std::str::from_utf8(&bytes[start..end])
                    .ok()
                    .and_then(|s| s.parse::<f64>().ok())
                {
                    return num;
                }
            }
        }
    }
    -1.0
}

/// Extract TTL value from a ping reply line. "TTL=" is universal.
fn extract_ttl(line: &str) -> u32 {
    if let Some(start) = line.find("TTL=") {
        let after = &line.as_bytes()[start + 4..];
        let mut ttl_str = String::new();
        for &b in after {
            if b.is_ascii_digit() {
                ttl_str.push(b as char);
            } else {
                break;
            }
        }
        return ttl_str.parse::<u32>().unwrap_or(0);
    }
    0
}

/// Parse a Unix ping reply line.
/// Example: "64 bytes from 8.8.8.8: icmp_seq=1 ttl=118 time=11.2 ms"
fn parse_unix_ping_line(line: &str) -> Option<PingResult> {
    if !line.contains("bytes from") {
        return None;
    }

    let mut seq = 0_u32;
    let mut latency_ms = -1.0_f64;
    let mut ttl = 0_u32;

    // Parse "icmp_seq=X"
    if let Some(seq_start) = line.find("icmp_seq=") {
        let after_seq = &line[seq_start + 9..];
        let seq_str = after_seq
            .split(|c: char| !c.is_ascii_digit())
            .next()
            .unwrap_or("");
        if let Ok(val) = seq_str.parse::<u32>() {
            seq = val;
        }
    }

    // Parse "ttl=X"
    if let Some(ttl_start) = line.find("ttl=") {
        let after_ttl = &line[ttl_start + 4..];
        let ttl_str = after_ttl
            .split(|c: char| !c.is_ascii_digit())
            .next()
            .unwrap_or("");
        if let Ok(val) = ttl_str.parse::<u32>() {
            ttl = val;
        }
    }

    // Parse "time=X ms" or "time=Xms"
    if let Some(time_start) = line.find(" time=") {
        let after_time = &line[time_start + 6..];
        let time_str = after_time
            .split(|c: char| !c.is_ascii_digit() && c != '.')
            .next()
            .unwrap_or("");
        if let Ok(val) = time_str.parse::<f64>() {
            latency_ms = val;
        }
    }

    Some(PingResult {
        seq,
        latency_ms,
        ttl,
        status: "success".to_string(),
    })
}

/// Compute statistics from a list of PingResults.
pub fn compute_stats(results: &[PingResult]) -> PingStats {
    let sent = results.len() as u32;
    let received = results
        .iter()
        .filter(|r| r.status == "success")
        .count() as u32;

    let latencies: Vec<f64> = results
        .iter()
        .filter(|r| r.status == "success" && r.latency_ms >= 0.0)
        .map(|r| r.latency_ms)
        .collect();

    let loss_percent = if sent > 0 {
        ((sent - received) as f64 / sent as f64) * 100.0
    } else {
        0.0
    };

    if latencies.is_empty() {
        return PingStats {
            sent,
            received,
            loss_percent,
            min_ms: 0.0,
            avg_ms: 0.0,
            max_ms: 0.0,
        };
    }

    let min_ms = latencies.iter().cloned().fold(f64::MAX, f64::min);
    let max_ms = latencies.iter().cloned().fold(f64::MIN, f64::max);
    let avg_ms = latencies.iter().sum::<f64>() / latencies.len() as f64;

    PingStats {
        sent,
        received,
        loss_percent,
        min_ms,
        avg_ms,
        max_ms,
    }
}
