//! SNMP device scanner — probes a CIDR range for SNMP-enabled devices.

use crate::core::snmp::oids;
use crate::core::snmp::SnmpSession;
use crate::types::snmp::{DiscoverProgress, SnmpDevice, SnmpSessionConfig};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct SnmpScanner {
    config: SnmpSessionConfig,
    concurrency: usize,
}

impl SnmpScanner {
    pub fn new(config: SnmpSessionConfig) -> Self {
        Self {
            config,
            concurrency: 50,
        }
    }

    /// Scan a CIDR range and discover SNMP devices.
    /// Sends progress updates via the provided callback.
    pub async fn scan<F>(
        &self,
        cidr: &str,
        on_progress: F,
    ) -> Result<Vec<SnmpDevice>, String>
    where
        F: Fn(DiscoverProgress) + Send + Sync + 'static,
    {
        let ips = self.expand_cidr(cidr)?;
        let total = ips.len() as u32;
        let found = Arc::new(Mutex::new(Vec::new()));
        let scanned = Arc::new(Mutex::new(0u32));

        // Process IPs concurrently with a semaphore
        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.concurrency));
        let on_progress = Arc::new(on_progress);
        let mut handles = vec![];

        for ip in ips {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let config = self.config.clone();
            let found = found.clone();
            let scanned = scanned.clone();
            let on_progress = on_progress.clone();
            let ip_clone = ip.clone();

            let handle = tokio::spawn(async move {
                let _permit = permit;

                // Update progress
                {
                    let mut s = scanned.lock().await;
                    *s += 1;
                    let current = *s;
                    let found_count = found.lock().await.len() as u32;
                    on_progress(DiscoverProgress {
                        scanned: current,
                        total,
                        found: found_count,
                        current_ip: ip_clone.clone(),
                    });
                }

                // Try to probe this IP
                let probe_config = SnmpSessionConfig {
                    host: ip_clone.clone(),
                    ..config.clone()
                };

                if let Ok(session) = SnmpSession::open(&probe_config) {
                    let results = session.get_multiple(&[
                        oids::SYS_NAME,
                        oids::SYS_DESCR,
                        oids::SYS_OBJECT_ID,
                        oids::SYS_UPTIME,
                    ]);

                    if let Ok(values) = results {
                        if values.iter().any(|v| v.is_some()) {
                            let hostname = values[0].clone().unwrap_or_default();
                            let sys_descr = values[1].clone().unwrap_or_default();
                            let sys_oid = values[2].clone().unwrap_or_default();
                            let uptime_str = values[3].clone().unwrap_or_default();
                            let (vendor, model) = detect_device(&sys_descr, &sys_oid);

                            let device = SnmpDevice {
                                id: Uuid::new_v4().to_string(),
                                ip: ip_clone,
                                hostname,
                                sys_descr,
                                sys_object_id: sys_oid,
                                vendor,
                                model,
                                uptime: uptime_str.parse().unwrap_or(0),
                                community: config.community.clone(),
                                last_seen: chrono::Utc::now().to_rfc3339(),
                            };

                            found.lock().await.push(device);
                        }
                    }
                }
            });

            handles.push(handle);
        }

        // Wait for all probes
        for h in handles {
            let _ = h.await;
        }

        let result = found.lock().await.clone();
        Ok(result)
    }

    fn expand_cidr(&self, cidr: &str) -> Result<Vec<String>, String> {
        // Simple CIDR expansion for /24 only
        let parts: Vec<&str> = cidr.split('/').collect();
        if parts.len() != 2 {
            return Err("invalid CIDR format, use x.x.x.x/24".into());
        }

        let prefix_len: u8 = parts[1].parse().map_err(|_| "invalid prefix".to_string())?;
        if prefix_len != 24 {
            return Err("only /24 subnets are supported".into());
        }

        let mut octets: Vec<u8> = parts[0]
            .split('.')
            .filter_map(|o| o.parse().ok())
            .collect();
        if octets.len() != 4 {
            return Err("invalid IP address".into());
        }

        // Set host bits to 0
        octets[3] = 0;

        let base = ((octets[0] as u32) << 24)
            | ((octets[1] as u32) << 16)
            | ((octets[2] as u32) << 8);

        let mut ips = Vec::with_capacity(254);
        for i in 1..255 {
            let ip = base | i;
            ips.push(format!(
                "{}.{}.{}.{}",
                (ip >> 24) & 0xFF,
                (ip >> 16) & 0xFF,
                (ip >> 8) & 0xFF,
                ip & 0xFF,
            ));
        }
        Ok(ips)
    }
}

fn detect_device(sys_descr: &str, sys_oid: &str) -> (String, String) {
    if sys_oid.starts_with("1.3.6.1.4.1.9") {
        ("Cisco".into(), extract_model(sys_descr, "Cisco"))
    } else if sys_oid.starts_with("1.3.6.1.4.1.2011") {
        ("Huawei".into(), extract_model(sys_descr, "Huawei"))
    } else if sys_oid.starts_with("1.3.6.1.4.1.11") {
        ("HP".into(), extract_model(sys_descr, "HP|ProCurve|Aruba"))
    } else if sys_oid.starts_with("1.3.6.1.4.1.25506") {
        ("H3C".into(), extract_model(sys_descr, "H3C|Comware"))
    } else if sys_oid.starts_with("1.3.6.1.4.1.42623") {
        ("Hikvision".into(), extract_model(sys_descr, "Hikvision|DS-"))
    } else if sys_oid.starts_with("1.3.6.1.4.1.11863") {
        ("TP-Link".into(), extract_model(sys_descr, "TP-Link"))
    } else {
        ("Unknown".into(), sys_descr.lines().next().unwrap_or("").to_string())
    }
}

fn extract_model<'a>(sys_descr: &'a str, _keywords: &str) -> String {
    for line in sys_descr.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    sys_descr.to_string()
}
