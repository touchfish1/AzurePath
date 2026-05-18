use tokio::process::Command;

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

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Ping can still produce useful stdout even with non-zero exit (e.g. partial loss)
        if output.stdout.is_empty() {
            return Err(format!("Ping failed: {}", stderr));
        }
    }

    String::from_utf8(output.stdout)
        .map_err(|e| format!("Invalid UTF-8 from ping output: {}", e))
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
/// Example: "Reply from 8.8.8.8: bytes=32 time=11ms TTL=118"
/// Example: "Reply from 192.168.1.1: Destination host unreachable."
/// Example: "Request timed out."
fn parse_windows_ping_line(line: &str) -> Option<PingResult> {
    // Check for timeout
    if line.contains("timed out") || line.contains("unreachable") {
        return Some(PingResult {
            seq: 0,
            latency_ms: -1.0,
            ttl: 0,
            status: if line.contains("timed out") {
                "timeout".to_string()
            } else {
                "unreachable".to_string()
            },
        });
    }

    if !line.starts_with("Reply from") {
        return None;
    }

    let mut latency_ms = -1.0_f64;
    let mut ttl = 0_u32;

    // Parse "time=Xms" or "time=Xms "
    if let Some(time_start) = line.find("time=") {
        let after_time = &line[time_start + 5..];
        let time_str = after_time
            .split(|c: char| !c.is_ascii_digit() && c != '.')
            .next()
            .unwrap_or("");
        if let Ok(val) = time_str.parse::<f64>() {
            latency_ms = val;
        }
    }

    // Parse "TTL=X"
    if let Some(ttl_start) = line.find("TTL=") {
        let after_ttl = &line[ttl_start + 4..];
        let ttl_str = after_ttl
            .split(|c: char| !c.is_ascii_digit())
            .next()
            .unwrap_or("");
        if let Ok(val) = ttl_str.parse::<u32>() {
            ttl = val;
        }
    }

    Some(PingResult {
        seq: 0,
        latency_ms,
        ttl,
        status: "success".to_string(),
    })
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
