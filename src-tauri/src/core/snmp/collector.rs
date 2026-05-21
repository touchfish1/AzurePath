//! SNMP data collector — periodically polls devices for interface statistics.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;
use tokio::time::interval;

use crate::core::snmp::oids;
use crate::core::snmp::SnmpSession;
use crate::types::snmp::{SnmpSample, SnmpSessionConfig};

/// Tracks the previous counter value to compute delta.
struct CounterCache {
    prev_in: HashMap<u32, u64>,
    prev_out: HashMap<u32, u64>,
}

pub struct SnmpCollector {
    running: Arc<Mutex<bool>>,
    counter_cache: Arc<Mutex<CounterCache>>,
}

impl SnmpCollector {
    pub fn new() -> Self {
        Self {
            running: Arc::new(Mutex::new(false)),
            counter_cache: Arc::new(Mutex::new(CounterCache {
                prev_in: HashMap::new(),
                prev_out: HashMap::new(),
            })),
        }
    }

    /// Start collecting data for a device at the given interval.
    /// Samples are sent via the callback.
    pub async fn start<F>(
        &self,
        config: SnmpSessionConfig,
        interval_secs: u64,
        on_sample: F,
    ) -> Result<(), String>
    where
        F: Fn(SnmpSample) + Send + Sync + 'static,
    {
        let mut running = self.running.lock().await;
        if *running {
            return Err("collector already running".into());
        }
        *running = true;
        drop(running);

        let running = self.running.clone();
        let counter_cache = self.counter_cache.clone();

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(interval_secs));

            while *running.lock().await {
                ticker.tick().await;

                let session = match SnmpSession::open(&config) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("[azurepath] SNMP collector open error: {e}");
                        continue;
                    }
                };

                // Walk ifEntry for interface names and counters
                let entries = match session.walk(oids::IF_DESCR) {
                    Ok(e) => e,
                    Err(e) => {
                        eprintln!("[azurepath] SNMP collector walk error: {e}");
                        continue;
                    }
                };

                let now = chrono::Utc::now().to_rfc3339();
                let mut cache = counter_cache.lock().await;

                for (oid, _name) in &entries {
                    // Extract ifIndex from the OID (last component)
                    let if_index: u32 = oid.rsplit('.').next()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);

                    if if_index == 0 { continue; }

                    // Get current counters
                    let in_oid = format!("{}.{}", oids::IF_HC_IN_OCTETS, if_index);
                    let out_oid = format!("{}.{}", oids::IF_HC_OUT_OCTETS, if_index);

                    let in_val: u64 = session.get(&in_oid).ok()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                    let out_val: u64 = session.get(&out_oid).ok()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);

                    // Compute delta in bps
                    let (in_bps, out_bps) = if let Some(&prev_in) = cache.prev_in.get(&if_index) {
                        let prev_out = cache.prev_out.get(&if_index).copied().unwrap_or(0);
                        let in_delta = in_val.saturating_sub(prev_in) * 8 / interval_secs as u64;
                        let out_delta = out_val.saturating_sub(prev_out) * 8 / interval_secs as u64;
                        (in_delta as f64, out_delta as f64)
                    } else {
                        (0.0, 0.0)
                    };

                    cache.prev_in.insert(if_index, in_val);
                    cache.prev_out.insert(if_index, out_val);

                    on_sample(SnmpSample {
                        device_id: config.host.clone(),
                        timestamp: now.clone(),
                        if_index,
                        in_bps,
                        out_bps,
                    });
                }
            }
        });

        Ok(())
    }

    pub async fn stop(&self) {
        let mut running = self.running.lock().await;
        *running = false;
    }
}
