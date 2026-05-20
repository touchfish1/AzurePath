use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;
use std::sync::Mutex;

use crate::core::ping;
use crate::core::traceroute;
use crate::types::mtr::{MtrHopStats, MtrOptions};

use tokio::time::{sleep, Duration};

/// Global cancel tokens for MTR tasks.
pub(crate) static CANCEL_TOKENS: LazyLock<Mutex<HashMap<String, AtomicBool>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Check whether the given task has been cancelled.
pub(crate) fn is_cancelled(task_id: &str) -> bool {
    if let Ok(tokens) = CANCEL_TOKENS.lock() {
        tokens
            .get(task_id)
            .map(|c| c.load(Ordering::SeqCst))
            .unwrap_or(false)
    } else {
        false
    }
}

/// Phase 1: Discover the route hops to the target.
/// Returns a list of hop results with IP addresses.
pub(crate) async fn discover_hops(
    options: &MtrOptions,
) -> Result<Vec<traceroute::ExecuteTraceResult>, String> {
    let output = traceroute::execute_traceroute(
        &options.target,
        options.max_hops,
        options.timeout_ms,
        1, // single probe per hop is sufficient for discovery
    )
    .await?;

    let hops = traceroute::parse_traceroute_output(&output);

    if hops.is_empty() {
        return Err(format!(
            "目标 {} 不可达，未发现路由节点",
            options.target
        ));
    }

    Ok(hops)
}

/// Statistics accumulator for a single hop across multiple rounds.
#[derive(Debug, Clone)]
struct HopAccumulator {
    hop: u32,
    addr: Option<String>,
    hostname: Option<String>,
    latencies: Vec<f64>,
    timeouts: u32,
}

impl HopAccumulator {
    fn new(hop: u32, addr: Option<String>, hostname: Option<String>) -> Self {
        Self {
            hop,
            addr,
            hostname,
            latencies: Vec::new(),
            timeouts: 0,
        }
    }

    fn add_result(&mut self, latency: Option<f64>) {
        match latency {
            Some(ms) => self.latencies.push(ms),
            None => self.timeouts += 1,
        }
    }

    fn compute_stats(&self) -> MtrHopStats {
        let sent = self.latencies.len() as u32 + self.timeouts;
        let received = self.latencies.len() as u32;
        let loss_percent = if sent > 0 {
            (self.timeouts as f64 / sent as f64) * 100.0
        } else {
            0.0
        };

        if self.latencies.is_empty() {
            return MtrHopStats {
                hop: self.hop,
                addr: self.addr.clone(),
                hostname: self.hostname.clone(),
                sent,
                received,
                loss_percent,
                min_ms: 0.0,
                avg_ms: 0.0,
                max_ms: 0.0,
                jitter_ms: 0.0,
                last_ms: None,
            };
        }

        let min_ms = self
            .latencies
            .iter()
            .copied()
            .reduce(f64::min)
            .unwrap_or(0.0);
        let max_ms = self
            .latencies
            .iter()
            .copied()
            .reduce(f64::max)
            .unwrap_or(0.0);
        let avg_ms = self.latencies.iter().sum::<f64>() / self.latencies.len() as f64;

        // Jitter = mean deviation = Σ|latency_i - avg| / n
        let jitter_ms = self
            .latencies
            .iter()
            .map(|&l| (l - avg_ms).abs())
            .sum::<f64>()
            / self.latencies.len() as f64;

        let last_ms = self.latencies.last().copied();

        MtrHopStats {
            hop: self.hop,
            addr: self.addr.clone(),
            hostname: self.hostname.clone(),
            sent,
            received,
            loss_percent,
            min_ms,
            avg_ms,
            max_ms,
            jitter_ms,
            last_ms,
        }
    }
}

/// Phase 2: Run the MTR loop — continuously ping all discovered hops.
///
/// For each round:
/// 1. Ping every hop concurrently with `ping -n 1` (Windows) / `ping -c 1` (Unix)
/// 2. Parse each response
/// 3. Aggregate stats across all rounds
/// 4. Build and return MtrHopStats for each hop (so the caller can emit progress)
pub(crate) async fn run_mtr_loop(
    task_id: &str,
    options: &MtrOptions,
    discovered_hops: &[traceroute::ExecuteTraceResult],
) -> Vec<Vec<MtrHopStats>> {
    // Collect discovered hop addresses
    let hop_addrs: Vec<(u32, Option<String>, Option<String>)> = discovered_hops
        .iter()
        .map(|h| (h.hop, h.addr.clone(), h.hostname.clone()))
        .collect();

    // Build accumulators for each hop
    let mut accumulators: Vec<HopAccumulator> = hop_addrs
        .iter()
        .map(|(hop, addr, hostname)| {
            HopAccumulator::new(*hop, addr.clone(), hostname.clone())
        })
        .collect();

    let mut round_snapshots: Vec<Vec<MtrHopStats>> = Vec::new();

    loop {
        // Check cancellation
        if is_cancelled(task_id) {
            break;
        }

        // Ping all hops with addr concurrently
        let mut ping_futures = Vec::new();
        for (hop_idx, (hop, addr, _hostname)) in hop_addrs.iter().enumerate() {
            if let Some(ip) = addr {
                ping_futures.push(async move {
                    let result = ping_single_hop(ip, options.timeout_ms).await;
                    (hop_idx, *hop, result)
                });
            } else {
                // Hop had no address (timeout in traceroute), count as timeout
                accumulators[hop_idx].timeouts += 1;
            }
        }

        if !ping_futures.is_empty() {
            let results = futures::future::join_all(ping_futures).await;
            for (hop_idx, _hop, latency) in results {
                if hop_idx < accumulators.len() {
                    accumulators[hop_idx].add_result(latency);
                }
            }
        }

        // Build snapshot for this round
        let snapshot: Vec<MtrHopStats> = accumulators.iter().map(|a| a.compute_stats()).collect();
        round_snapshots.push(snapshot);

        // Check cancellation again before sleeping
        if is_cancelled(task_id) {
            break;
        }

        // Wait for the next round interval
        sleep(Duration::from_millis(options.interval_ms)).await;
    }

    round_snapshots
}

/// Ping a single hop with count=1 and return the latency in ms.
/// Returns None on timeout/unreachable.
async fn ping_single_hop(target: &str, timeout_ms: u64) -> Option<f64> {
    let output = match ping::execute_ping(target, 1, timeout_ms).await {
        Ok(o) => o,
        Err(_) => return None,
    };

    let results = ping::parse_ping_output(&output);
    for r in &results {
        if r.status == "success" && r.latency_ms >= 0.0 {
            return Some(r.latency_ms);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // HopAccumulator tests
    // ============================================================

    #[test]
    fn test_hop_accumulator_new() {
        let acc = HopAccumulator::new(1, Some("192.168.1.1".into()), None);
        assert_eq!(acc.hop, 1);
        assert_eq!(acc.addr, Some("192.168.1.1".into()));
        assert!(acc.latencies.is_empty());
        assert_eq!(acc.timeouts, 0);
    }

    #[test]
    fn test_hop_accumulator_all_success() {
        let mut acc = HopAccumulator::new(1, Some("8.8.8.8".into()), None);
        acc.add_result(Some(10.0));
        acc.add_result(Some(20.0));
        acc.add_result(Some(30.0));

        let stats = acc.compute_stats();
        assert_eq!(stats.sent, 3);
        assert_eq!(stats.received, 3);
        assert!((stats.loss_percent - 0.0).abs() < 1e-9);
        assert!((stats.min_ms - 10.0).abs() < 1e-9);
        assert!((stats.max_ms - 30.0).abs() < 1e-9);
        assert!((stats.avg_ms - 20.0).abs() < 1e-9);
        assert!((stats.last_ms.unwrap() - 30.0).abs() < 1e-9);
    }

    #[test]
    fn test_hop_accumulator_all_timeout() {
        let mut acc = HopAccumulator::new(2, None, None);
        acc.add_result(None);
        acc.add_result(None);

        let stats = acc.compute_stats();
        assert_eq!(stats.sent, 2);
        assert_eq!(stats.received, 0);
        assert!((stats.loss_percent - 100.0).abs() < 1e-9);
        assert!(stats.min_ms < 0.001);
        assert!(stats.avg_ms < 0.001);
        assert!(stats.max_ms < 0.001);
        assert!(stats.last_ms.is_none());
    }

    #[test]
    fn test_hop_accumulator_mixed() {
        let mut acc = HopAccumulator::new(3, Some("10.0.0.1".into()), None);
        acc.add_result(Some(15.0));
        acc.add_result(None);
        acc.add_result(Some(25.0));

        let stats = acc.compute_stats();
        assert_eq!(stats.sent, 3);
        assert_eq!(stats.received, 2);
        assert!((stats.loss_percent - 33.33333333333333).abs() < 1e-9);
        assert!((stats.min_ms - 15.0).abs() < 1e-9);
        assert!((stats.max_ms - 25.0).abs() < 1e-9);
        assert!((stats.avg_ms - 20.0).abs() < 1e-9);
    }

    #[test]
    fn test_hop_accumulator_single_result() {
        let mut acc = HopAccumulator::new(1, Some("1.1.1.1".into()), None);
        acc.add_result(Some(42.5));

        let stats = acc.compute_stats();
        assert_eq!(stats.sent, 1);
        assert_eq!(stats.received, 1);
        assert!((stats.min_ms - 42.5).abs() < 1e-9);
        assert!((stats.avg_ms - 42.5).abs() < 1e-9);
        assert!((stats.max_ms - 42.5).abs() < 1e-9);
        assert!((stats.jitter_ms - 0.0).abs() < 1e-9);
    }

    // ============================================================
    // Jitter computation tests (mean deviation)
    // ============================================================

    #[test]
    fn test_jitter_constant_latency() {
        let mut acc = HopAccumulator::new(1, Some("1.1.1.1".into()), None);
        acc.add_result(Some(10.0));
        acc.add_result(Some(10.0));
        acc.add_result(Some(10.0));

        let stats = acc.compute_stats();
        assert!((stats.jitter_ms - 0.0).abs() < 1e-9, "Constant latency should have zero jitter");
    }

    #[test]
    fn test_jitter_variable_latency() {
        let mut acc = HopAccumulator::new(1, Some("1.1.1.1".into()), None);
        acc.add_result(Some(10.0));
        acc.add_result(Some(20.0));
        acc.add_result(Some(30.0));

        // avg = 20, deviations: |10-20| + |20-20| + |30-20| = 10 + 0 + 10 = 20, jitter = 20/3 ≈ 6.667
        let stats = acc.compute_stats();
        assert!((stats.jitter_ms - 6.666666666666667).abs() < 1e-9);
    }

    #[test]
    fn test_jitter_empty_returns_zero() {
        let acc = HopAccumulator::new(1, Some("1.1.1.1".into()), None);
        let stats = acc.compute_stats();
        assert!((stats.jitter_ms - 0.0).abs() < 1e-9);
    }

    // ============================================================
    // CANCEL_TOKENS tests
    // ============================================================

    #[test]
    fn test_is_cancelled_no_token() {
        // Non-existent token should return false (not cancelled)
        assert!(!is_cancelled("non-existent"));
    }

    #[test]
    fn test_is_cancelled_token_set() {
        let task_id = "test-cancel-1";
        CANCEL_TOKENS
            .lock()
            .unwrap()
            .insert(task_id.into(), AtomicBool::new(false));

        assert!(!is_cancelled(task_id));

        // Set cancelled
        CANCEL_TOKENS
            .lock()
            .unwrap()
            .get(task_id)
            .unwrap()
            .store(true, Ordering::SeqCst);

        assert!(is_cancelled(task_id));

        // Cleanup
        CANCEL_TOKENS.lock().unwrap().remove(task_id);
    }

    #[test]
    fn test_is_cancelled_multiple_tokens() {
        let id1 = "multi-1";
        let id2 = "multi-2";

        CANCEL_TOKENS.lock().unwrap().insert(id1.into(), AtomicBool::new(false));
        CANCEL_TOKENS.lock().unwrap().insert(id2.into(), AtomicBool::new(true));

        assert!(!is_cancelled(id1));
        assert!(is_cancelled(id2));

        CANCEL_TOKENS.lock().unwrap().clear();
    }

    // ============================================================
    // Ping single hop (unit test with mock output)
    // ============================================================

    #[test]
    fn test_ping_single_hop_parse() {
        // Unit-test the inner parsing logic that ping_single_hop uses.
        let output = "Reply from 8.8.8.8: bytes=32 time=11ms TTL=118";
        let results = ping::parse_ping_output(output);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, "success");
        assert!((results[0].latency_ms - 11.0).abs() < 1e-9);
    }

    #[test]
    fn test_ping_single_hop_timeout_parse() {
        let output = "Request timed out.";
        let results = ping::parse_ping_output(output);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, "timeout");
    }

    #[test]
    fn test_ping_single_hop_unreachable_parse() {
        let output = "Reply from 10.0.0.99: Destination host unreachable.";
        let results = ping::parse_ping_output(output);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, "unreachable");
    }
}
