pub mod icmp;

use tokio::process::Command;

/// Decode ping output bytes to a UTF-8 string.
/// On non-Windows platforms, bytes are expected to be UTF-8.
/// On Windows, the system locale encoding (e.g. GBK for Chinese) is used as fallback.
pub(crate) fn decode_ping_output(bytes: &[u8]) -> String {
    crate::core::utils::decode_output(bytes)
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

pub fn parse_ping_line(line: &str) -> Option<PingResult> {
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
            let end = i;
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

    // If we have "bytes from" but no timing info, this is not a valid success line.
    // Could be a malformed or unexpected output line.
    if latency_ms < 0.0 {
        return None;
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

    // Use .reduce() for cleaner semantics; latencies is guaranteed non-empty at this point.
    // SAFETY: we already returned early if latencies.is_empty(), so unwrap is safe.
    let min_ms = latencies.iter().copied().reduce(f64::min).unwrap_or(0.0);
    let max_ms = latencies.iter().copied().reduce(f64::max).unwrap_or(0.0);
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

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // parse_windows_ping_line tests
    // ============================================================

    #[test]
    fn test_windows_success_english() {
        let line = "Reply from 8.8.8.8: bytes=32 time=11ms TTL=118";
        let result = parse_windows_ping_line(line).unwrap();
        assert_eq!(result.status, "success");
        assert!((result.latency_ms - 11.0).abs() < 1e-9);
        assert_eq!(result.ttl, 118);
    }

    #[test]
    fn test_windows_success_decimal() {
        let line = "Reply from 8.8.8.8: bytes=32 time=11.2ms TTL=118";
        let result = parse_windows_ping_line(line).unwrap();
        assert_eq!(result.status, "success");
        assert!((result.latency_ms - 11.2).abs() < 1e-9);
        assert_eq!(result.ttl, 118);
    }

    #[test]
    fn test_windows_success_chinese() {
        let line = "来自 8.8.8.8 的回复: 字节=32 时间=11ms TTL=118";
        let result = parse_windows_ping_line(line).unwrap();
        assert_eq!(result.status, "success");
        assert!((result.latency_ms - 11.0).abs() < 1e-9);
        assert_eq!(result.ttl, 118);
    }

    #[test]
    fn test_windows_timeout_english() {
        let line = "Request timed out.";
        let result = parse_windows_ping_line(line).unwrap();
        assert_eq!(result.status, "timeout");
        assert_eq!(result.latency_ms, -1.0);
        assert_eq!(result.ttl, 0);
    }

    #[test]
    fn test_windows_timeout_chinese() {
        let line = "请求超时。";
        let result = parse_windows_ping_line(line).unwrap();
        assert_eq!(result.status, "timeout");
        assert_eq!(result.latency_ms, -1.0);
    }

    #[test]
    fn test_windows_unreachable_english() {
        let line = "Reply from 10.0.0.99: Destination host unreachable.";
        let result = parse_windows_ping_line(line).unwrap();
        assert_eq!(result.status, "unreachable");
        assert_eq!(result.latency_ms, -1.0);
    }

    #[test]
    fn test_windows_unreachable_chinese() {
        let line = "来自 10.0.0.99 的回复: 无法访问目标主机。";
        let result = parse_windows_ping_line(line).unwrap();
        assert_eq!(result.status, "unreachable");
    }

    #[test]
    fn test_windows_no_match() {
        let line = "Pinging 8.8.8.8 with 32 bytes of data:";
        assert!(parse_windows_ping_line(line).is_none());
    }

    #[test]
    fn test_windows_empty_line() {
        assert!(parse_windows_ping_line("").is_none());
    }

    // ============================================================
    // parse_unix_ping_line tests
    // ============================================================

    #[test]
    fn test_unix_success() {
        let line = "64 bytes from 8.8.8.8: icmp_seq=1 ttl=118 time=11.2 ms";
        let result = parse_unix_ping_line(line).unwrap();
        assert_eq!(result.status, "success");
        assert!((result.latency_ms - 11.2).abs() < 1e-9);
        assert_eq!(result.ttl, 118);
        assert_eq!(result.seq, 1);
    }

    #[test]
    fn test_unix_success_no_space_before_ms() {
        let line = "64 bytes from 8.8.8.8: icmp_seq=1 ttl=118 time=11.2ms";
        let result = parse_unix_ping_line(line).unwrap();
        assert_eq!(result.status, "success");
        assert!((result.latency_ms - 11.2).abs() < 1e-9);
        assert_eq!(result.ttl, 118);
    }

    #[test]
    fn test_unix_missing_time_is_not_success() {
        // Line has "bytes from" but no "time=" -- should NOT be success
        let line = "64 bytes from 8.8.8.8: icmp_seq=1 ttl=118";
        assert!(parse_unix_ping_line(line).is_none());
    }

    #[test]
    fn test_unix_no_match() {
        let line = "PING 8.8.8.8 (8.8.8.8): 56 data bytes";
        assert!(parse_unix_ping_line(line).is_none());
    }

    #[test]
    fn test_unix_empty_line() {
        assert!(parse_unix_ping_line("").is_none());
    }

    #[test]
    fn test_unix_large_seq() {
        let line = "64 bytes from 8.8.8.8: icmp_seq=99999 ttl=118 time=10.0 ms";
        let result = parse_unix_ping_line(line).unwrap();
        assert_eq!(result.seq, 99999);
        assert!((result.latency_ms - 10.0).abs() < 1e-9);
    }

    // ============================================================
    // extract_latency_ms tests
    // ============================================================

    #[test]
    fn test_extract_latency_normal() {
        assert!((extract_latency_ms("time=42ms") - 42.0).abs() < 1e-9);
    }

    #[test]
    fn test_extract_latency_decimal() {
        assert!((extract_latency_ms("time=42.5ms") - 42.5).abs() < 1e-9);
    }

    #[test]
    fn test_extract_latency_zero() {
        assert!((extract_latency_ms("time=0ms") - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_extract_latency_no_ms() {
        assert!((extract_latency_ms("no milliseconds here") - (-1.0)).abs() < 1e-9);
    }

    #[test]
    fn test_extract_latency_empty() {
        assert!((extract_latency_ms("") - (-1.0)).abs() < 1e-9);
    }

    #[test]
    fn test_extract_latency_chinese_utf8() {
        // Chinese locale: "时间=11ms"
        let line = "来自 8.8.8.8 的回复: 字节=32 时间=11ms TTL=118";
        assert!((extract_latency_ms(line) - 11.0).abs() < 1e-9);
    }

    // ============================================================
    // extract_ttl tests
    // ============================================================

    #[test]
    fn test_extract_ttl_normal() {
        assert_eq!(extract_ttl("TTL=118"), 118);
    }

    #[test]
    fn test_extract_ttl_large() {
        assert_eq!(extract_ttl("TTL=255"), 255);
    }

    #[test]
    fn test_extract_ttl_no_match() {
        assert_eq!(extract_ttl("no ttl here"), 0);
    }

    #[test]
    fn test_extract_ttl_empty() {
        assert_eq!(extract_ttl(""), 0);
    }

    #[test]
    fn test_extract_ttl_prefix() {
        // TTL= is a substring match, but this case is unrealistic for ping output
        assert_eq!(extract_ttl("PREFIX_TTL=42_SUFFIX"), 42);
    }

    // ============================================================
    // compute_stats tests
    // ============================================================

    #[test]
    fn test_stats_normal() {
        let results = vec![
            PingResult { seq: 1, latency_ms: 10.0, ttl: 64, status: "success".to_string() },
            PingResult { seq: 2, latency_ms: 20.0, ttl: 64, status: "success".to_string() },
            PingResult { seq: 3, latency_ms: 30.0, ttl: 64, status: "success".to_string() },
        ];
        let stats = compute_stats(&results);
        assert_eq!(stats.sent, 3);
        assert_eq!(stats.received, 3);
        assert!((stats.loss_percent - 0.0).abs() < 1e-9);
        assert!((stats.min_ms - 10.0).abs() < 1e-9);
        assert!((stats.max_ms - 30.0).abs() < 1e-9);
        assert!((stats.avg_ms - 20.0).abs() < 1e-9);
    }

    #[test]
    fn test_stats_all_timeout() {
        let results = vec![
            PingResult { seq: 0, latency_ms: -1.0, ttl: 0, status: "timeout".to_string() },
            PingResult { seq: 0, latency_ms: -1.0, ttl: 0, status: "timeout".to_string() },
        ];
        let stats = compute_stats(&results);
        assert_eq!(stats.sent, 2);
        assert_eq!(stats.received, 0);
        assert!((stats.loss_percent - 100.0).abs() < 1e-9);
        assert!((stats.min_ms - 0.0).abs() < 1e-9);
        assert!((stats.max_ms - 0.0).abs() < 1e-9);
        assert!((stats.avg_ms - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_stats_empty() {
        let results = vec![];
        let stats = compute_stats(&results);
        assert_eq!(stats.sent, 0);
        assert_eq!(stats.received, 0);
        assert!((stats.loss_percent - 0.0).abs() < 1e-9);
        assert!((stats.min_ms - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_stats_single_result() {
        let results = vec![
            PingResult { seq: 1, latency_ms: 42.0, ttl: 64, status: "success".to_string() },
        ];
        let stats = compute_stats(&results);
        assert!((stats.min_ms - 42.0).abs() < 1e-9);
        assert!((stats.max_ms - 42.0).abs() < 1e-9);
        assert!((stats.avg_ms - 42.0).abs() < 1e-9);
    }

    #[test]
    fn test_stats_zero_latency() {
        // Edge case: latency of exactly 0.0 ms should not cause f64::MIN confusion
        let results = vec![
            PingResult { seq: 1, latency_ms: 0.0, ttl: 64, status: "success".to_string() },
        ];
        let stats = compute_stats(&results);
        assert!((stats.min_ms - 0.0).abs() < 1e-9);
        assert!((stats.max_ms - 0.0).abs() < 1e-9);
        assert!((stats.avg_ms - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_stats_mixed_success_timeout() {
        let results = vec![
            PingResult { seq: 1, latency_ms: 15.0, ttl: 64, status: "success".to_string() },
            PingResult { seq: 0, latency_ms: -1.0, ttl: 0, status: "timeout".to_string() },
            PingResult { seq: 2, latency_ms: 25.0, ttl: 64, status: "success".to_string() },
        ];
        let stats = compute_stats(&results);
        assert_eq!(stats.sent, 3);
        assert_eq!(stats.received, 2);
        assert!((stats.loss_percent - 33.33333333333333).abs() < 1e-9);
        assert!((stats.min_ms - 15.0).abs() < 1e-9);
        assert!((stats.max_ms - 25.0).abs() < 1e-9);
        assert!((stats.avg_ms - 20.0).abs() < 1e-9);
    }

    // ============================================================
    // parse_ping_output integration tests
    // ============================================================

    #[test]
    fn test_parse_output_empty() {
        let results = parse_ping_output("");
        assert!(results.is_empty());
    }

    #[test]
    fn test_parse_output_mixed_lines() {
        // Should work on the current platform's parser regardless
        let output = "\n  \nline without match\n";
        let results = parse_ping_output(output);
        assert!(results.is_empty());
    }

    // ============================================================
    // decode_ping_output tests
    // ============================================================

    #[test]
    fn test_decode_ascii() {
        let decoded = decode_ping_output(b"Reply from 8.8.8.8: bytes=32 time=11ms TTL=118");
        assert_eq!(decoded, "Reply from 8.8.8.8: bytes=32 time=11ms TTL=118");
    }

    #[test]
    fn test_decode_utf8() {
        let decoded = decode_ping_output("来自 8.8.8.8 的回复: 时间=11ms".as_bytes());
        assert_eq!(decoded, "来自 8.8.8.8 的回复: 时间=11ms");
    }

    #[test]
    fn test_decode_empty() {
        let decoded = decode_ping_output(b"");
        assert_eq!(decoded, "");
    }

    // ============================================================
    // PingResult struct tests
    // ============================================================

    #[test]
    fn test_ping_stats_default() {
        let stats = PingStats::default();
        assert_eq!(stats.sent, 0);
        assert_eq!(stats.received, 0);
        assert_eq!(stats.loss_percent, 0.0);
    }
}
