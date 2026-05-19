use tokio::process::Command;

/// Decode process output bytes to UTF-8, handling system locale encoding (e.g. GBK on Chinese Windows).
fn decode_output(bytes: &[u8]) -> String {
    #[cfg(target_os = "windows")]
    {
        String::from_utf8(bytes.to_vec())
            .unwrap_or_else(|_| encoding_rs::GBK.decode(bytes).0.to_string())
    }
    #[cfg(not(target_os = "windows"))]
    {
        String::from_utf8_lossy(bytes).to_string()
    }
}

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
    probes_per_hop: u32,
) -> Result<String, String> {
    let output = if cfg!(target_os = "windows") {
        // Windows: tracert -h <max_hops> -w <timeout_ms> <target>
        // tracert does not support changing probes per hop.
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
        // Unix: traceroute -m <max_hops> -w <timeout_s> -q <probes_per_hop> <target>
        let timeout_s = (timeout_ms / 1000).max(1);
        let mut cmd = Command::new("traceroute");
        cmd.arg("-m")
            .arg(max_hops.to_string())
            .arg("-w")
            .arg(timeout_s.to_string());
        if probes_per_hop > 0 {
            cmd.arg("-q").arg(probes_per_hop.to_string());
        }
        cmd.arg(target);
        cmd.output().await.map_err(|e| format!("Failed to execute traceroute: {}", e))?
    };

    let output_str = decode_output(&output.stdout);

    if !output.status.success() && output_str.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Traceroute failed: {}", stderr));
    }

    Ok(output_str)
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

/// Parse a single Windows tracert output line.
/// Returns `None` if the line is not a valid hop line.
pub fn parse_tracert_line(line: &str) -> Option<ExecuteTraceResult> {
    let line = line.trim();
    if line.is_empty() || !line.chars().next().map_or(false, |c| c.is_ascii_digit()) {
        return None;
    }

    // Skip header/summary lines
    if line.to_lowercase().contains("tracing") || line.to_lowercase().contains("trace complete") {
        return None;
    }

    let hop = match line.split_whitespace().next() {
        Some(n) => n.parse::<u32>().unwrap_or(0),
        None => return None,
    };

    if hop == 0 {
        return None;
    }

    // Windows tracert "Request timed out." lines look like:
    //   3  *        *        *     Request timed out.
    // The addr would incorrectly be "out." without this check.
    if line.to_lowercase().contains("request timed out") {
        return Some(ExecuteTraceResult {
            hop,
            addr: None,
            hostname: None,
            latencies: vec![None; 3],
        });
    }

    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }

    // Extract latencies (parts[1..parts.len()-1]) and address (last part)
    let addr_str = parts[parts.len() - 1];
    let latency_parts = &parts[1..parts.len() - 1];

    let mut latencies = Vec::new();
    for &lat_str in latency_parts {
        // Skip "ms" unit tokens — tracert intersperses values and units
        if lat_str == "ms" {
            continue;
        }
        if lat_str == "*" || lat_str.to_lowercase().contains("timeout") {
            latencies.push(None);
        } else {
            let cleaned: String = lat_str
                .chars()
                .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '<')
                .collect();
            if cleaned.starts_with('<') {
                // Values like "<1" mean less than 1 ms, approximate as 0
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

    Some(ExecuteTraceResult {
        hop,
        addr,
        hostname: None,
        latencies,
    })
}

/// Parse Windows tracert output using the single-line parser.
fn parse_windows_tracert_output(output: &str, results: &mut Vec<ExecuteTraceResult>) {
    for line in output.lines() {
        if let Some(result) = parse_tracert_line(line) {
            results.push(result);
        }
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
                // All tokens are "*" (timeout) — none of them is an address,
                // so keep the entire rest as latency entries.
                (None, rest.clone())
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

#[cfg(test)]
mod tests {
    use super::*;

    // ============ Windows parse_tracert_line tests ============

    #[test]
    fn test_parse_tracert_line_normal() {
        // Standard line with 3 probe latencies
        let line = " 1   5 ms   6 ms   7 ms  192.168.1.1";
        let result = parse_tracert_line(line).unwrap();
        assert_eq!(result.hop, 1);
        assert_eq!(result.addr.unwrap(), "192.168.1.1");
        assert_eq!(result.latencies.len(), 3);
        assert!((result.latencies[0].unwrap() - 5.0).abs() < 0.001);
        assert!((result.latencies[1].unwrap() - 6.0).abs() < 0.001);
        assert!((result.latencies[2].unwrap() - 7.0).abs() < 0.001);
    }

    #[test]
    fn test_parse_tracert_line_sub_ms() {
        // Line with "<1 ms" values — should produce 3 entries with 0.0
        let line = " 1   <1 ms   <1 ms   <1 ms  192.168.1.1";
        let result = parse_tracert_line(line).unwrap();
        assert_eq!(result.hop, 1);
        assert_eq!(result.addr.unwrap(), "192.168.1.1");
        assert_eq!(result.latencies.len(), 3);
        for lat in &result.latencies {
            assert_eq!(*lat, Some(0.0));
        }
    }

    #[test]
    fn test_parse_tracert_line_all_timeout() {
        // Line with all "*" (timeout) probes
        let line = " 2   *        *        *    192.168.1.2";
        let result = parse_tracert_line(line).unwrap();
        assert_eq!(result.hop, 2);
        assert_eq!(result.addr.unwrap(), "192.168.1.2");
        assert_eq!(result.latencies.len(), 3);
        for lat in &result.latencies {
            assert!(lat.is_none());
        }
    }

    #[test]
    fn test_parse_tracert_line_request_timed_out() {
        // Line ending with "Request timed out." — addr must be None (bug fix)
        let line = " 3   *        *        *     Request timed out.";
        let result = parse_tracert_line(line).unwrap();
        assert_eq!(result.hop, 3);
        assert!(result.addr.is_none(), "addr should be None for 'Request timed out.'");
        assert_eq!(result.latencies.len(), 3);
        for lat in &result.latencies {
            assert!(lat.is_none());
        }
    }

    #[test]
    fn test_parse_tracert_line_mixed_timeout_and_values() {
        // Some probes timed out, some have values
        let line = " 4   10 ms   *       20 ms  10.0.0.1";
        let result = parse_tracert_line(line).unwrap();
        assert_eq!(result.hop, 4);
        assert_eq!(result.addr.unwrap(), "10.0.0.1");
        assert_eq!(result.latencies.len(), 3);
        assert!((result.latencies[0].unwrap() - 10.0).abs() < 0.001);
        assert!(result.latencies[1].is_none());
        assert!((result.latencies[2].unwrap() - 20.0).abs() < 0.001);
    }

    #[test]
    fn test_parse_tracert_line_header_tracing() {
        // "Tracing route to ..." header must be ignored
        let line = "Tracing route to 8.8.8.8 over a maximum of 30 hops:";
        assert!(parse_tracert_line(line).is_none());
    }

    #[test]
    fn test_parse_tracert_line_header_trace_complete() {
        // "Trace complete." summary must be ignored
        let line = "Trace complete.";
        assert!(parse_tracert_line(line).is_none());
    }

    #[test]
    fn test_parse_tracert_line_empty() {
        assert!(parse_tracert_line("").is_none());
        assert!(parse_tracert_line("   ").is_none());
    }

    #[test]
    fn test_parse_tracert_line_non_digit_start() {
        // Lines not starting with a digit should be ignored
        assert!(parse_tracert_line("  *    *    *    *").is_none());
        // Request timed out without a hop number prefix
        assert!(parse_tracert_line("Request timed out.").is_none());
    }

    #[test]
    fn test_parse_tracert_line_large_hop() {
        let line = " 30   100 ms   110 ms   120 ms  203.0.113.1";
        let result = parse_tracert_line(line).unwrap();
        assert_eq!(result.hop, 30);
        assert_eq!(result.addr.unwrap(), "203.0.113.1");
        assert_eq!(result.latencies.len(), 3);
    }

    // ============ Windows batch parsing tests ============

    #[test]
    fn test_parse_windows_tracert_output_full() {
        let output = "Tracing route to 8.8.8.8 over a maximum of 30 hops:\r\n\
                      1   5 ms   6 ms   7 ms  192.168.1.1\r\n\
                      2   *       *       *     Request timed out.\r\n\
                      3   10 ms   11 ms   12 ms  8.8.8.8\r\n\
                      Trace complete.\r\n";
        let mut results = Vec::new();
        parse_windows_tracert_output(output, &mut results);
        assert_eq!(results.len(), 3);

        assert_eq!(results[0].hop, 1);
        assert_eq!(results[0].addr.as_deref(), Some("192.168.1.1"));
        assert_eq!(results[0].latencies.len(), 3);

        assert_eq!(results[1].hop, 2);
        // Bug fix: "Request timed out." should give None addr
        assert!(results[1].addr.is_none(), "Hop 2 addr should be None");
        assert_eq!(results[1].latencies.len(), 3);
        for lat in &results[1].latencies {
            assert!(lat.is_none());
        }

        assert_eq!(results[2].hop, 3);
        assert_eq!(results[2].addr.as_deref(), Some("8.8.8.8"));
        assert_eq!(results[2].latencies.len(), 3);
    }

    // ============ Unix batch parsing tests ============

    #[test]
    fn test_parse_unix_traceroute_output_full() {
        let output = "traceroute to 8.8.8.8 (8.8.8.8), 30 hops max, 60 byte packets\n\
                       1  192.168.1.1 (192.168.1.1)  0.542 ms  0.489 ms  0.476 ms\n\
                       2  * * *\n\
                       3  8.8.8.8 (8.8.8.8)  11.234 ms  12.567 ms  11.890 ms\n";
        let mut results = Vec::new();
        parse_unix_traceroute_output(output, &mut results);
        assert_eq!(results.len(), 3);

        assert_eq!(results[0].hop, 1);
        assert_eq!(results[0].addr.as_deref(), Some("192.168.1.1"));
        assert_eq!(results[0].hostname.as_deref(), Some("192.168.1.1"));
        assert_eq!(results[0].latencies.len(), 3);
        assert!((results[0].latencies[0].unwrap() - 0.542).abs() < 0.001);
        assert!((results[0].latencies[1].unwrap() - 0.489).abs() < 0.001);
        assert!((results[0].latencies[2].unwrap() - 0.476).abs() < 0.001);

        assert_eq!(results[1].hop, 2);
        assert!(results[1].addr.is_none(), "Hop 2 addr should be None for * * *");
        assert!(results[1].hostname.is_none());
        assert_eq!(results[1].latencies.len(), 3);
        for lat in &results[1].latencies {
            assert!(lat.is_none());
        }

        assert_eq!(results[2].hop, 3);
        assert_eq!(results[2].addr.as_deref(), Some("8.8.8.8"));
        assert_eq!(results[2].latencies.len(), 3);
    }

    #[test]
    fn test_parse_unix_traceroute_output_hostname() {
        // Line with hostname before IP
        let output = "traceroute to example.com (93.184.216.34), 30 hops max, 60 byte packets\n\
                       1  router.local (10.0.0.1)  5.123 ms  4.987 ms  5.234 ms\n\
                       2  93.184.216.34 (93.184.216.34)  42.000 ms  43.000 ms  44.000 ms\n";
        let mut results = Vec::new();
        parse_unix_traceroute_output(output, &mut results);
        assert_eq!(results.len(), 2);

        assert_eq!(results[0].hop, 1);
        assert_eq!(results[0].addr.as_deref(), Some("10.0.0.1"));
        assert_eq!(results[0].hostname.as_deref(), Some("router.local"));

        assert_eq!(results[1].hop, 2);
        assert_eq!(results[1].addr.as_deref(), Some("93.184.216.34"));
        assert_eq!(results[1].hostname.as_deref(), Some("93.184.216.34"));
    }

    #[test]
    fn test_parse_unix_traceroute_output_empty() {
        let mut results = Vec::new();
        parse_unix_traceroute_output("", &mut results);
        assert!(results.is_empty());

        parse_unix_traceroute_output("traceroute to example.com (1.2.3.4), 30 hops max\n", &mut results);
        // Only header, no hop lines
    }

    // ============ parse_traceroute_output (platform dispatch) tests ============

    #[test]
    fn test_parse_traceroute_output_empty() {
        let results = parse_traceroute_output("");
        assert!(results.is_empty());
    }

    // ============ compute_trace_stats tests ============

    #[test]
    fn test_compute_trace_stats_empty() {
        let stats = compute_trace_stats(&[]);
        assert_eq!(stats.hops_completed, 0);
        assert!(!stats.destinations_reached);
    }

    #[test]
    fn test_compute_trace_stats_with_destination() {
        let results = vec![
            ExecuteTraceResult {
                hop: 1,
                addr: Some("192.168.1.1".to_string()),
                hostname: None,
                latencies: vec![Some(5.0), Some(6.0), Some(7.0)],
            },
            ExecuteTraceResult {
                hop: 2,
                addr: Some("8.8.8.8".to_string()),
                hostname: None,
                latencies: vec![Some(10.0), None, Some(12.0)],
            },
        ];
        let stats = compute_trace_stats(&results);
        assert_eq!(stats.hops_completed, 2);
        assert!(stats.destinations_reached);
    }

    #[test]
    fn test_compute_trace_stats_no_destination() {
        let results = vec![
            ExecuteTraceResult {
                hop: 1,
                addr: None,
                hostname: None,
                latencies: vec![None, None, None],
            },
        ];
        let stats = compute_trace_stats(&results);
        assert_eq!(stats.hops_completed, 1);
        assert!(!stats.destinations_reached);
    }

    // ============ Edge cases ============

    #[test]
    fn test_parse_tracert_line_with_leading_zero_hop() {
        // Hop numbers are normal integers, no leading zeros
        let line = " 10   1 ms   2 ms   3 ms  10.0.0.1";
        let result = parse_tracert_line(line).unwrap();
        assert_eq!(result.hop, 10);
    }

    #[test]
    fn test_parse_tracert_line_with_brackets_in_addr() {
        // Some Windows tracert versions show domain [IP] format
        let line = " 1   1 ms   2 ms   3 ms  router.local [192.168.1.1]";
        let result = parse_tracert_line(line).unwrap();
        assert_eq!(result.hop, 1);
        // The addr will include the brackets since we take the last whitespace token
        let addr = result.addr.unwrap();
        assert_eq!(addr, "[192.168.1.1]");
    }

    #[test]
    fn test_decode_output_valid_utf8() {
        let bytes = b"hello world";
        let s = decode_output(bytes);
        assert_eq!(s, "hello world");
    }

    #[test]
    fn test_decode_output_invalid_utf8() {
        // Invalid UTF-8 should not panic
        let bytes = b"\xff\xfe\x00\x01";
        // Should not panic
        let _ = decode_output(bytes);
    }
}
