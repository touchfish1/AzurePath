use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct ExecuteTraceResult {
    pub hop: u32,
    pub addr: Option<String>,
    pub hostname: Option<String>,
    pub latencies: Vec<Option<f64>>,
}

#[derive(Debug, Clone, Default)]
pub struct TraceStats {
    pub hops_completed: u32,
    pub destinations_reached: bool,
}

/// Execute the system traceroute/tracert command and return raw stdout.
pub async fn execute_traceroute(
    target: &str,
    max_hops: u32,
    timeout_ms: u64,
) -> Result<String, String> {
    let output = if cfg!(target_os = "windows") {
        // Windows: tracert -h <max_hops> -w <timeout_ms> <target>
        Command::new("tracert")
            .arg("-h")
            .arg(max_hops.to_string())
            .arg("-w")
            .arg(timeout_ms.to_string())
            .arg(target)
            .output()
            .await
            .map_err(|e| format!("Failed to execute tracert: {}", e))?
    } else {
        // Unix: traceroute -m <max_hops> -w <timeout_s> <target>
        let timeout_s = (timeout_ms / 1000).max(1);
        Command::new("traceroute")
            .arg("-m")
            .arg(max_hops.to_string())
            .arg("-w")
            .arg(timeout_s.to_string())
            .arg(target)
            .output()
            .await
            .map_err(|e| format!("Failed to execute traceroute: {}", e))?
    };

    if !output.status.success() && output.stdout.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Traceroute failed: {}", stderr));
    }

    String::from_utf8(output.stdout)
        .map_err(|e| format!("Invalid UTF-8 from traceroute output: {}", e))
}

/// Parse traceroute/tracert output into structured hop results.
pub fn parse_traceroute_output(output: &str) -> Vec<ExecuteTraceResult> {
    let mut results = Vec::new();

    if cfg!(target_os = "windows") {
        parse_windows_tracert_output(output, &mut results);
    } else {
        parse_unix_traceroute_output(output, &mut results);
    }

    results
}

/// Parse Windows tracert output.
/// Windows tracert format:
///   1    <1 ms    <1 ms    <1 ms  192.168.1.1
///   2     *        *        *     Request timed out.
///   3    11 ms    12 ms    11 ms  8.8.8.8
fn parse_windows_tracert_output(output: &str, results: &mut Vec<ExecuteTraceResult>) {
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() || !line.chars().next().map_or(false, |c| c.is_ascii_digit()) {
            continue;
        }

        // Skip header lines that start with "Tracing" or "over"
        if line.to_lowercase().contains("tracing")
            || line.to_lowercase().contains("trace complete")
        {
            continue;
        }

        let hop = match line.split_whitespace().next() {
            Some(n) => n.parse::<u32>().unwrap_or(0),
            None => continue,
        };

        if hop == 0 {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }

        // Extract latencies (parts[1..parts.len()-1]) and address (last part)
        let addr_str = parts[parts.len() - 1];
        let latency_parts = &parts[1..parts.len() - 1];

        let mut latencies = Vec::new();
        for &lat_str in latency_parts {
            if lat_str == "*" || lat_str.to_lowercase().contains("timeout") {
                latencies.push(None);
            } else {
                // Parse "<1ms", "11ms", "12 ms" etc.
                let cleaned: String = lat_str
                    .chars()
                    .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '<')
                    .collect();
                if cleaned == "<" {
                    latencies.push(Some(0.0));
                } else if let Ok(val) = cleaned.parse::<f64>() {
                    latencies.push(Some(val));
                } else {
                    latencies.push(None);
                }
            }
        }

        let addr = if addr_str == "*" || addr_str.to_lowercase().contains("request timed out") {
            None
        } else {
            Some(addr_str.to_string())
        };

        results.push(ExecuteTraceResult {
            hop,
            addr,
            hostname: None, // Windows tracert doesn't separate hostname from addr easily
            latencies,
        });
    }
}

/// Parse Unix traceroute output.
/// Unix traceroute format:
///  1  192.168.1.1 (192.168.1.1)  0.542 ms  0.489 ms  0.476 ms
///  2  * * *
///  3  8.8.8.8 (8.8.8.8)  11.234 ms  12.567 ms  11.890 ms
///  4  router.local (10.0.0.1)  5.123 ms  4.987 ms  5.234 ms
fn parse_unix_traceroute_output(output: &str, results: &mut Vec<ExecuteTraceResult>) {
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Skip header lines
        if line.to_lowercase().contains("traceroute to")
            || line.to_lowercase().contains("traceroute")
        {
            continue;
        }

        let hop = match line.split_whitespace().next() {
            Some(n) => n.trim_end_matches('.').parse::<u32>().unwrap_or(0),
            None => continue,
        };

        if hop == 0 {
            continue;
        }

        // Try to extract hostname (if present) and IP from format: "hostname (ip)"
        let rest = line
            .split_whitespace()
            .skip(1)
            .collect::<Vec<&str>>()
            .join(" ");

        let (hostname, addr, rest_after_addr) = if let Some(open_paren) = rest.find('(') {
            if let Some(close_paren) = rest[open_paren + 1..].find(')') {
                let hostname_str = rest[..open_paren].trim().to_string();
                let addr_str = rest[open_paren + 1..open_paren + 1 + close_paren].to_string();
                let after = rest[open_paren + 1 + close_paren + 1..].trim().to_string();
                (Some(hostname_str), Some(addr_str), after)
            } else {
                (None, None, rest.clone())
            }
        } else {
            (None, None, rest.clone())
        };

        // If no parenthesized IP, the address might be a bare IP or "*"
        let (addr, rest_str) = if addr.is_none() {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.is_empty() {
                (None, String::new())
            } else if parts[0] == "*" {
                (None, parts[1..].join(" "))
            } else {
                (Some(parts[0].to_string()), parts[1..].join(" "))
            }
        } else {
            (addr, rest_after_addr)
        };

        // Parse latency values from the remaining string
        let mut latencies = Vec::new();
        let latency_strs: Vec<&str> = rest_str.split_whitespace().collect();
        let mut i = 0;
        while i < latency_strs.len() {
            if latency_strs[i] == "*" {
                latencies.push(None);
                i += 1;
            } else if latency_strs[i] == "ms" && !latencies.is_empty() {
                // The value was already captured, skip the "ms"
                i += 1;
            } else {
                // Parse numeric value
                let cleaned: String = latency_strs[i]
                    .chars()
                    .filter(|c| c.is_ascii_digit() || *c == '.')
                    .collect();
                if let Ok(val) = cleaned.parse::<f64>() {
                    latencies.push(Some(val));
                }
                i += 1;
            }
        }

        results.push(ExecuteTraceResult {
            hop,
            addr,
            hostname,
            latencies,
        });
    }
}

/// Compute trace statistics.
pub fn compute_trace_stats(results: &[ExecuteTraceResult]) -> TraceStats {
    let hops_completed = results.len() as u32;
    let destinations_reached = results
        .iter()
        .any(|r| r.addr.is_some() && r.latencies.iter().any(|l| l.is_some()));

    TraceStats {
        hops_completed,
        destinations_reached,
    }
}
