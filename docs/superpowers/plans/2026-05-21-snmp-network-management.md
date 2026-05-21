# SNMP 网络管理 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add SNMP v2c device discovery, interface monitoring, and performance data collection to AzurePath.

**Architecture:** Three-layer Rust structure (types → core/snmp/ → commands) following existing patterns. The `snmp2` crate provides SNMP get/walk operations. A dedicated worker thread handles periodic data collection and pushes results via Tauri events.

**Tech Stack:** `snmp2 = "0.5"` (pure Rust SNMP), `rusqlite` (history storage), Tauri events (real-time push)

---

### Task 1: Add snmp2 dependency and module declarations

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/types/mod.rs`
- Modify: `src-tauri/src/core/mod.rs`
- Modify: `src-tauri/src/commands/mod.rs`

- [ ] **Step 1: Add snmp2 to Cargo.toml**

```toml
# In [dependencies] section, add:
snmp2 = "0.5"
```

- [ ] **Step 2: Declare modules in mod.rs files**

```rust
// src-tauri/src/types/mod.rs — add:
pub mod snmp;

// src-tauri/src/core/mod.rs — add:
pub mod snmp;

// src-tauri/src/commands/mod.rs — add:
pub mod snmp;
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/types/mod.rs src-tauri/src/core/mod.rs src-tauri/src/commands/mod.rs
git commit -m "feat(snmp): add snmp2 dependency and module declarations"
```

---

### Task 2: Create SNMP data types

**Files:**
- Create: `src-tauri/src/types/snmp.rs`

- [ ] **Step 1: Create the types file**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnmpSessionConfig {
    pub host: String,
    pub port: u16,
    pub community: String,
    pub timeout_ms: u64,
}

impl Default for SnmpSessionConfig {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: 161,
            community: "public".into(),
            timeout_ms: 3000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnmpDevice {
    pub id: String,
    pub ip: String,
    pub hostname: String,
    pub sys_descr: String,
    pub sys_object_id: String,
    pub vendor: String,
    pub model: String,
    pub uptime: u64,
    pub community: String,
    pub last_seen: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnmpInterface {
    pub index: u32,
    pub name: String,
    pub description: String,
    pub mac: String,
    pub ip: String,
    pub speed: u64,
    pub admin_status: u8,
    pub oper_status: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnmpSample {
    pub device_id: String,
    pub timestamp: String,
    pub if_index: u32,
    pub in_bps: f64,
    pub out_bps: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnmpArpEntry {
    pub ip: String,
    pub mac: String,
    pub interface: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnmpRouteEntry {
    pub destination: String,
    pub next_hop: String,
    pub interface: String,
    pub metric: u32,
    pub route_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverProgress {
    pub scanned: u32,
    pub total: u32,
    pub found: u32,
    pub current_ip: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceResource {
    pub cpu_usage: Option<f32>,
    pub memory_usage: Option<f32>,
    pub timestamp: String,
}
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/types/snmp.rs
git commit -m "feat(snmp): create SNMP data types"
```

---

### Task 3: Create OID constants

**Files:**
- Create: `src-tauri/src/core/snmp/oids.rs`

- [ ] **Step 1: Create OID constants file**

```rust
//! Well-known SNMP OID constants for system info, interfaces, and monitoring.

/// system
pub const SYS_DESCR: &str = "1.3.6.1.2.1.1.1.0";
pub const SYS_OBJECT_ID: &str = "1.3.6.1.2.1.1.2.0";
pub const SYS_UPTIME: &str = "1.3.6.1.2.1.1.3.0";
pub const SYS_NAME: &str = "1.3.6.1.2.1.1.5.0";

/// interfaces table (ifEntry)
pub const IF_NUMBER: &str = "1.3.6.1.2.1.2.1.0";
pub const IF_TABLE: &str = "1.3.6.1.2.1.2.2.1";
pub const IF_INDEX: &str = "1.3.6.1.2.1.2.2.1.1";
pub const IF_DESCR: &str = "1.3.6.1.2.1.2.2.1.2";
pub const IF_TYPE: &str = "1.3.6.1.2.1.2.2.1.3";
pub const IF_MTU: &str = "1.3.6.1.2.1.2.2.1.4";
pub const IF_SPEED: &str = "1.3.6.1.2.1.2.2.1.5";
pub const IF_PHYS_ADDRESS: &str = "1.3.6.1.2.1.2.2.1.6";
pub const IF_ADMIN_STATUS: &str = "1.3.6.1.2.1.2.2.1.7";
pub const IF_OPER_STATUS: &str = "1.3.6.1.2.1.2.2.1.8";
pub const IF_IN_OCTETS: &str = "1.3.6.1.2.1.2.2.1.10";
pub const IF_OUT_OCTETS: &str = "1.3.6.1.2.1.2.2.1.16";

/// 64-bit interface counters (ifXTable)
pub const IF_HC_IN_OCTETS: &str = "1.3.6.1.2.1.31.1.1.1.6";
pub const IF_HC_OUT_OCTETS: &str = "1.3.6.1.2.1.31.1.1.1.10";

/// IP-MIB
pub const IP_NET_TO_MEDIA_TABLE: &str = "1.3.6.1.2.1.4.22.1";
pub const IP_ROUTE_TABLE: &str = "1.3.6.1.2.1.4.24.2";

/// host resources
pub const HR_PROCESSOR_LOAD: &str = "1.3.6.1.2.1.25.3.3.1.2";
pub const HR_STORAGE_USED: &str = "1.3.6.1.2.1.25.2.3.1.6";
pub const HR_STORAGE_SIZE: &str = "1.3.6.1.2.1.25.2.3.1.5";

/// Common vendor OID prefixes for device type detection
pub const VENDOR_CISCO: &str = "1.3.6.1.4.1.9";
pub const VENDOR_HUAWEI: &str = "1.3.6.1.4.1.2011";
pub const VENDOR_HP: &str = "1.3.6.1.4.1.11";
pub const VENDOR_H3C: &str = "1.3.6.1.4.1.25506";
pub const VENDOR_HIKVISION: &str = "1.3.6.1.4.1.42623";
pub const VENDOR_TP_LINK: &str = "1.3.6.1.4.1.11863";
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/core/snmp/oids.rs
git commit -m "feat(snmp): add OID constant definitions"
```

---

### Task 4: Create core/snmp/mod.rs — SnmpSession wrapper

**Files:**
- Create: `src-tauri/src/core/snmp/mod.rs`

- [ ] **Step 1: Create SnmpSession wrapper with get, walk, and bulkwalk**

```rust
//! SNMP session wrapper using the `snmp2` crate.
//!
//! Provides high-level get/walk/bulkwalk operations with error handling.

use std::net::SocketAddrV4;
use std::str::FromStr;
use std::time::Duration;

use snmp2::SyncSession;
use snmp2::Value;

use crate::types::snmp::SnmpSessionConfig;

pub struct SnmpSession {
    session: SyncSession,
}

impl SnmpSession {
    pub fn open(config: &SnmpSessionConfig) -> Result<Self, String> {
        let addr = SocketAddrV4::from_str(&format!("{}:{}", config.host, config.port))
            .map_err(|e| format!("invalid SNMP address: {e}"))?;

        let session = SyncSession::new(
            addr,
            config.community.clone(),
            Some(Duration::from_millis(config.timeout_ms)),
            0, // retries
        )
        .map_err(|e| format!("SNMP session open failed: {e}"))?;

        Ok(Self { session })
    }

    pub fn get(&self, oid: &str) -> Result<String, String> {
        let response = self
            .session
            .get(&[oid.into()])
            .map_err(|e| format!("SNMP get {oid} failed: {e}"))?;

        response
            .into_iter()
            .next()
            .and_then(|v| value_to_string(&v))
            .ok_or_else(|| format!("SNMP get {oid}: empty response"))
    }

    pub fn get_multiple(&self, oids: &[&str]) -> Result<Vec<Option<String>>, String> {
        let request: Vec<snmp2::Oid> = oids.iter().map(|o| (*o).into()).collect();
        let response = self
            .session
            .get(&request)
            .map_err(|e| format!("SNMP get multiple failed: {e}"))?;

        Ok(response
            .into_iter()
            .map(|v| value_to_string(&v))
            .collect())
    }

    pub fn walk(&self, oid: &str) -> Result<Vec<(String, String)>, String> {
        let results = self
            .session
            .walk(oid)
            .map_err(|e| format!("SNMP walk {oid} failed: {e}"))?;

        Ok(results
            .into_iter()
            .filter_map(|(oid, value)| {
                value_to_string(&value).map(|s| (oid.to_string(), s))
            })
            .collect())
    }
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::Integer(i) => Some(i.to_string()),
        Value::OctetString(bytes) => {
            // Try UTF-8 first, fall back to hex
            String::from_utf8(bytes.clone()).ok()
                .or_else(|| {
                    if bytes.len() <= 6 {
                        // Likely a MAC address
                        Some(bytes.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(":"))
                    } else {
                        Some(bytes.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(""))
                    }
                })
        }
        Value::Oid(oid) => Some(oid.to_string()),
        Value::IpAddress(ip) => Some(ip.to_string()),
        Value::Counter32(n) => Some(n.to_string()),
        Value::Counter64(n) => Some(n.to_string()),
        Value::Gauge32(n) => Some(n.to_string()),
        Value::Timeticks(n) => Some(n.to_string()),
        Value::Null => None,
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/core/snmp/mod.rs
git commit -m "feat(snmp): add SnmpSession wrapper"
```

---

### Task 5: Create SNMP scanner (device discovery)

**Files:**
- Create: `src-tauri/src/core/snmp/scanner.rs`

- [ ] **Step 1: Create scanner with concurrent probing**

```rust
//! SNMP device scanner — probes a CIDR range for SNMP-enabled devices.

use crate::core::snmp::mod as snmp_session;
use crate::core::snmp::oids;
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
        let mut handles = vec![];

        for ip in ips {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let config = self.config.clone();
            let found = found.clone();
            let scanned = scanned.clone();
            let on_progress = Arc::new(on_progress);
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

                if let Ok(session) = snmp_session::SnmpSession::open(&probe_config) {
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

fn extract_model<'a>(sys_descr: &'a str, keywords: &str) -> String {
    for line in sys_descr.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    sys_descr.to_string()
}
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/core/snmp/scanner.rs
git commit -m "feat(snmp): add device scanner with CIDR expansion"
```

---

### Task 6: Create SNMP collector (periodic data collection)

**Files:**
- Create: `src-tauri/src/core/snmp/collector.rs`

- [ ] **Step 1: Create collector with periodic polling**

```rust
//! SNMP data collector — periodically polls devices for interface statistics.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;
use tokio::time::interval;

use crate::core::snmp::mod as snmp_session;
use crate::core::snmp::oids;
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

                let session = match snmp_session::SnmpSession::open(&config) {
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

                for (oid, name) in &entries {
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
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/core/snmp/collector.rs
git commit -m "feat(snmp): add periodic data collector"
```

---

### Task 7: Create SQLite store

**Files:**
- Create: `src-tauri/src/core/snmp/store.rs`

- [ ] **Step 1: Create SQLite store for device and sample persistence**

```rust
//! SQLite persistence for SNMP devices and collected data.

use rusqlite::{params, Connection};
use std::sync::Mutex;

use crate::types::snmp::SnmpDevice;

pub struct SnmpStore {
    conn: Mutex<Connection>,
}

impl SnmpStore {
    pub fn new() -> Result<Self, String> {
        let conn = Connection::open_in_memory()
            .map_err(|e| e.to_string())?;
        let store = Self { conn: Mutex::new(conn) };
        store.init_tables()?;
        Ok(store)
    }

    fn init_tables(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS snmp_devices (
                id TEXT PRIMARY KEY,
                ip TEXT NOT NULL,
                hostname TEXT,
                sys_descr TEXT,
                sys_object_id TEXT,
                vendor TEXT,
                model TEXT,
                uptime INTEGER DEFAULT 0,
                community TEXT,
                last_seen TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS snmp_samples (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                device_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                if_index INTEGER NOT NULL,
                in_bps REAL DEFAULT 0,
                out_bps REAL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_snmp_samples_device ON snmp_samples(device_id, timestamp);
            ",
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn upsert_device(&self, device: &SnmpDevice) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO snmp_devices (id, ip, hostname, sys_descr, sys_object_id, vendor, model, uptime, community, last_seen) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                device.id, device.ip, device.hostname, device.sys_descr,
                device.sys_object_id, device.vendor, device.model,
                device.uptime, device.community, device.last_seen
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_devices(&self) -> Result<Vec<SnmpDevice>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, ip, hostname, sys_descr, sys_object_id, vendor, model, uptime, community, last_seen \
                 FROM snmp_devices ORDER BY ip"
            )
            .map_err(|e| e.to_string())?;
        let devices = stmt
            .query_map([], |row| {
                Ok(SnmpDevice {
                    id: row.get(0)?,
                    ip: row.get(1)?,
                    hostname: row.get(2)?,
                    sys_descr: row.get(3)?,
                    sys_object_id: row.get(4)?,
                    vendor: row.get(5)?,
                    model: row.get(6)?,
                    uptime: row.get(7)?,
                    community: row.get(8)?,
                    last_seen: row.get(9)?,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        Ok(devices)
    }

    pub fn delete_device(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM snmp_samples WHERE device_id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM snmp_devices WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn insert_sample(
        &self,
        device_id: &str,
        timestamp: &str,
        if_index: u32,
        in_bps: f64,
        out_bps: f64,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO snmp_samples (device_id, timestamp, if_index, in_bps, out_bps) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![device_id, timestamp, if_index, in_bps, out_bps],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_history(
        &self,
        device_id: &str,
        limit: u32,
    ) -> Result<Vec<SnmpSample>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT device_id, timestamp, if_index, in_bps, out_bps FROM snmp_samples \
                 WHERE device_id = ?1 ORDER BY timestamp DESC LIMIT ?2"
            )
            .map_err(|e| e.to_string())?;
        let samples = stmt
            .query_map(params![device_id, limit], |row| {
                Ok(SnmpSample {
                    device_id: row.get(0)?,
                    timestamp: row.get(1)?,
                    if_index: row.get(2)?,
                    in_bps: row.get(3)?,
                    out_bps: row.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        Ok(samples)
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/core/snmp/store.rs
git commit -m "feat(snmp): add SQLite store for devices and samples"
```

---

### Task 8: Create Tauri commands

**Files:**
- Create: `src-tauri/src/commands/snmp.rs`

- [ ] **Step 1: Create commands module**

```rust
//! SNMP Tauri commands — device discovery, interface query, data collection.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;

use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

use crate::core::snmp::collector::SnmpCollector;
use crate::core::snmp::mod as snmp_session;
use crate::core::snmp::oids;
use crate::core::snmp::scanner::SnmpScanner;
use crate::core::snmp::store::SnmpStore;
use crate::types::snmp::{
    DiscoverProgress, SnmpArpEntry, SnmpDevice, SnmpInterface, SnmpRouteEntry,
    SnmpSample, SnmpSessionConfig,
};

static STORE: OnceLock<Arc<SnmpStore>> = OnceLock::new();

fn store() -> &'static Arc<SnmpStore> {
    STORE.get().expect("SnmpStore not initialized")
}

#[tauri::command]
pub async fn snmp_init() -> Result<(), String> {
    let s = SnmpStore::new()?;
    STORE.get_or_init(|| Arc::new(s));
    Ok(())
}

#[tauri::command]
pub async fn snmp_discover(
    app: AppHandle,
    cidr: String,
    community: String,
) -> Result<Vec<SnmpDevice>, String> {
    let config = SnmpSessionConfig {
        host: "0.0.0.0".into(),
        port: 161,
        community,
        timeout_ms: 2000,
    };

    let scanner = SnmpScanner::new(config.clone());
    let app_clone = app.clone();

    let devices = scanner
        .scan(&cidr, move |progress| {
            let _ = app_clone.emit("snmp:progress", &progress);
        })
        .await?;

    // Save discovered devices
    for device in &devices {
        store().upsert_device(device)?;
    }

    let _ = app.emit("snmp:discover-complete", &devices);
    Ok(devices)
}

#[tauri::command]
pub async fn snmp_list_devices() -> Result<Vec<SnmpDevice>, String> {
    store().list_devices()
}

#[tauri::command]
pub async fn snmp_delete_device(id: String) -> Result<(), String> {
    store().delete_device(&id)
}

#[tauri::command]
pub async fn snmp_get_interfaces(
    host: String,
    community: String,
) -> Result<Vec<SnmpInterface>, String> {
    let config = SnmpSessionConfig {
        host,
        port: 161,
        community,
        timeout_ms: 3000,
    };
    let session = snmp_session::SnmpSession::open(&config)?;

    let entries = session.walk(oids::IF_TABLE)?;
    let mut interfaces: HashMap<u32, SnmpInterface> = HashMap::new();

    for (oid, value) in &entries {
        let parts: Vec<&str> = oid.rsplitn(2, '.').collect();
        let index: u32 = parts[0].parse().unwrap_or(0);
        if index == 0 {
            continue;
        }

        let iface = interfaces.entry(index).or_insert(SnmpInterface {
            index,
            name: String::new(),
            description: String::new(),
            mac: String::new(),
            ip: String::new(),
            speed: 0,
            admin_status: 0,
            oper_status: 0,
        });

        if oid.starts_with(oids::IF_DESCR) {
            iface.description = value.clone();
            iface.name = value.clone();
        } else if oid.starts_with(oids::IF_PHYS_ADDRESS) && value != "00:00:00:00:00:00" {
            iface.mac = value.clone();
        } else if oid.starts_with(oids::IF_SPEED) {
            iface.speed = value.parse().unwrap_or(0);
        } else if oid.starts_with(oids::IF_ADMIN_STATUS) {
            iface.admin_status = value.parse().unwrap_or(0);
        } else if oid.starts_with(oids::IF_OPER_STATUS) {
            iface.oper_status = value.parse().unwrap_or(0);
        }
    }

    let mut result: Vec<SnmpInterface> = interfaces.into_values().collect();
    result.sort_by_key(|i| i.index);
    Ok(result)
}

#[tauri::command]
pub async fn snmp_get_arp_table(
    host: String,
    community: String,
) -> Result<Vec<SnmpArpEntry>, String> {
    let config = SnmpSessionConfig {
        host,
        port: 161,
        community,
        timeout_ms: 3000,
    };
    let session = snmp_session::SnmpSession::open(&config)?;
    let entries = session.walk("1.3.6.1.2.1.4.22.1")?;

    let mut arp_entries: HashMap<String, (String, String)> = HashMap::new();

    for (oid, value) in &entries {
        let parts: Vec<&str> = oid.rsplitn(2, '.').collect();
        let key = parts[0].to_string();

        let entry = arp_entries.entry(key).or_default();
        if oid.starts_with("1.3.6.1.2.1.4.22.1.2") {
            entry.1 = value.clone(); // MAC
        } else if oid.starts_with("1.3.6.1.2.1.4.22.1.3") {
            entry.0 = value.clone(); // IP
        }
    }

    let mut parsed_entries: Vec<SnmpArpEntry> = arp_entries
        .into_values()
        .filter(|(ip, _)| !ip.is_empty())
        .take(200)
        .map(|(ip, mac)| SnmpArpEntry {
            ip,
            mac,
            interface: String::new(),
        })
        .collect();

    parsed_entries.sort_by(|a, b| {
        a.ip.split('.')
            .filter_map(|p| p.parse::<u32>().ok())
            .cmp(b.ip.split('.').filter_map(|p| p.parse::<u32>().ok()))
    });

    Ok(parsed_entries)
}

#[tauri::command]
pub async fn snmp_get_history(
    device_id: String,
    limit: Option<u32>,
) -> Result<Vec<SnmpSample>, String> {
    store().get_history(&device_id, limit.unwrap_or(300))
}

// Global collector instances
static COLLECTORS: OnceLock<Arc<Mutex<HashMap<String, SnmpCollector>>>> = OnceLock::new();

fn collectors() -> &'static Arc<Mutex<HashMap<String, SnmpCollector>>> {
    COLLECTORS.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

#[tauri::command]
pub async fn snmp_start_collect(
    app: AppHandle,
    host: String,
    community: String,
    interval_secs: Option<u64>,
) -> Result<(), String> {
    let config = SnmpSessionConfig {
        host: host.clone(),
        port: 161,
        community,
        timeout_ms: 3000,
    };

    let collector = SnmpCollector::new();
    let app_clone = app.clone();
    let device_id = host.clone();

    collector
        .start(config, interval_secs.unwrap_or(10), move |sample| {
            let _ = app_clone.emit("snmp:sample", &sample);
        })
        .await?;

    collectors()
        .lock()
        .await
        .insert(host, collector);

    Ok(())
}

#[tauri::command]
pub async fn snmp_stop_collect(host: String) -> Result<(), String> {
    let mut map = collectors().lock().await;
    if let Some(collector) = map.remove(&host) {
        collector.stop().await;
    }
    Ok(())
}
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/commands/snmp.rs
git commit -m "feat(snmp): add Tauri commands for SNMP operations"
```

---

### Task 9: Register commands in lib.rs

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add snmp_init to setup and register commands**

In the `setup` closure, after the remote desktop init:
```rust
// Initialize snmp store
if let Err(e) = commands::snmp::snmp_init() {
    eprintln!("[azurepath] snmp init warning: {e}");
}
```

In the `invoke_handler`:
```rust
// SNMP
commands::snmp::snmp_discover,
commands::snmp::snmp_list_devices,
commands::snmp::snmp_delete_device,
commands::snmp::snmp_get_interfaces,
commands::snmp::snmp_get_arp_table,
commands::snmp::snmp_get_history,
commands::snmp::snmp_start_collect,
commands::snmp::snmp_stop_collect,
```

Also add `commands::snmp::snmp_init` if needed (not a `#[tauri::command]` so no need).

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(snmp): register SNMP commands in lib.rs"
```

---

### Task 10: Frontend — Tauri bindings and store

**Files:**
- Modify: `src/lib/tauri.ts`
- Create: `src/stores/snmp.ts`

- [ ] **Step 1: Add TypeScript bindings to tauri.ts**

```typescript
// ── SNMP ──

export interface SnmpDevice {
  id: string;
  ip: string;
  hostname: string;
  sysDescr: string;
  sysObjectId: string;
  vendor: string;
  model: string;
  uptime: number;
  community: string;
  lastSeen: string;
}

export interface SnmpInterface {
  index: number;
  name: string;
  description: string;
  mac: string;
  ip: string;
  speed: number;
  adminStatus: number;
  operStatus: number;
}

export interface SnmpSample {
  deviceId: string;
  timestamp: string;
  ifIndex: number;
  inBps: number;
  outBps: number;
}

export interface SnmpArpEntry {
  ip: string;
  mac: string;
  interface: string;
}

export interface SnmpDiscoverProgress {
  scanned: number;
  total: number;
  found: number;
  currentIp: string;
}

export function snmpDiscover(cidr: string, community: string): Promise<SnmpDevice[]> {
  return invoke<SnmpDevice[]>("snmp_discover", { cidr, community });
}

export function snmpListDevices(): Promise<SnmpDevice[]> {
  return invoke<SnmpDevice[]>("snmp_list_devices");
}

export function snmpDeleteDevice(id: string): Promise<void> {
  return invoke<void>("snmp_delete_device", { id });
}

export function snmpGetInterfaces(host: string, community: string): Promise<SnmpInterface[]> {
  return invoke<SnmpInterface[]>("snmp_get_interfaces", { host, community });
}

export function snmpGetArpTable(host: string, community: string): Promise<SnmpArpEntry[]> {
  return invoke<SnmpArpEntry[]>("snmp_get_arp_table", { host, community });
}

export function snmpGetHistory(deviceId: string, limit?: number): Promise<SnmpSample[]> {
  return invoke<SnmpSample[]>("snmp_get_history", { deviceId, limit });
}

export function snmpStartCollect(host: string, community: string, intervalSecs?: number): Promise<void> {
  return invoke<void>("snmp_start_collect", { host, community, intervalSecs });
}

export function snmpStopCollect(host: string): Promise<void> {
  return invoke<void>("snmp_stop_collect", { host });
}

// Event listeners
export function onSnmpProgress(callback: (progress: SnmpDiscoverProgress) => void): Promise<UnlistenFn> {
  return listen<SnmpDiscoverProgress>("snmp:progress", (event) => callback(event.payload));
}

export function onSnmpSample(callback: (sample: SnmpSample) => void): Promise<UnlistenFn> {
  return listen<SnmpSample>("snmp:sample", (event) => callback(event.payload));
}
```

- [ ] **Step 2: Create Pinia store**

```typescript
// src/stores/snmp.ts

import { defineStore } from "pinia";
import { ref } from "vue";
import {
  snmpDiscover,
  snmpListDevices,
  snmpDeleteDevice,
  snmpGetInterfaces,
  snmpGetArpTable,
  snmpGetHistory,
  snmpStartCollect,
  snmpStopCollect,
  onSnmpProgress,
  onSnmpSample,
  type SnmpDevice,
  type SnmpInterface,
  type SnmpSample,
  type SnmpArpEntry,
  type SnmpDiscoverProgress,
} from "@/lib/tauri";

export const useSnmpStore = defineStore("snmp", () => {
  const devices = ref<SnmpDevice[]>([]);
  const interfaces = ref<SnmpInterface[]>([]);
  const arpTable = ref<SnmpArpEntry[]>([]);
  const samples = ref<SnmpSample[]>([]);
  const discoverProgress = ref<SnmpDiscoverProgress | null>(null);
  const isLoading = ref(false);
  const isCollecting = ref(false);

  async function loadDevices() {
    isLoading.value = true;
    try {
      devices.value = await snmpListDevices();
    } finally {
      isLoading.value = false;
    }
  }

  async function discover(cidr: string, community: string) {
    const unlisten = await onSnmpProgress((p) => {
      discoverProgress.value = p;
    });
    try {
      const result = await snmpDiscover(cidr, community);
      devices.value = result;
      return result;
    } finally {
      unlisten();
      discoverProgress.value = null;
    }
  }

  async function deleteDevice(id: string) {
    await snmpDeleteDevice(id);
    devices.value = devices.value.filter((d) => d.id !== id);
  }

  async function fetchInterfaces(host: string, community: string) {
    interfaces.value = await snmpGetInterfaces(host, community);
  }

  async function fetchArpTable(host: string, community: string) {
    arpTable.value = await snmpGetArpTable(host, community);
  }

  async function startCollection(host: string, community: string) {
    isCollecting.value = true;
    const unlisten = await onSnmpSample((sample) => {
      samples.value.push(sample);
      if (samples.value.length > 1000) {
        samples.value = samples.value.slice(-500);
      }
    });
    await snmpStartCollect(host, community);
  }

  async function stopCollection(host: string) {
    await snmpStopCollect(host);
    isCollecting.value = false;
  }

  return {
    devices,
    interfaces,
    arpTable,
    samples,
    discoverProgress,
    isLoading,
    isCollecting,
    loadDevices,
    discover,
    deleteDevice,
    fetchInterfaces,
    fetchArpTable,
    startCollection,
    stopCollection,
  };
});
```

- [ ] **Step 3: Commit**

```bash
git add src/lib/tauri.ts src/stores/snmp.ts
git commit -m "feat(snmp): add frontend bindings and store"
```

---

### Task 11: Frontend — Vue page

**Files:**
- Create: `src/pages/snmp/Page.vue`
- Modify: `src/router/index.ts`

- [ ] **Step 1: Create Vue page with device discovery and detail views**

```vue
<script setup lang="ts">
import { ref, onMounted, watch } from "vue";
import { useSnmpStore } from "@/stores/snmp";
import { useToast } from "@/components/Toast.vue";

const store = useSnmpStore();
const toast = useToast();

const cidr = ref("192.168.1.0/24");
const community = ref("public");
const selectedDevice = ref<SnmpDevice | null>(null);
const activeTab = ref<"interfaces" | "arp" | "traffic">("interfaces");
const isScanning = ref(false);
const intervalSecs = ref(10);

onMounted(() => {
  store.loadDevices();
});

async function startScan() {
  isScanning.value = true;
  try {
    await store.discover(cidr.value, community.value);
    toast.add("success", `发现 ${store.devices.length} 个设备`);
  } catch (e: any) {
    toast.add("error", String(e));
  } finally {
    isScanning.value = false;
  }
}

async function selectDevice(device: SnmpDevice) {
  selectedDevice.value = device;
  activeTab.value = "interfaces";
  store.fetchInterfaces(device.ip, device.community);
  store.fetchArpTable(device.ip, device.community);
}

async function handleDelete(id: string) {
  await store.deleteDevice(id);
  if (selectedDevice.value?.id === id) selectedDevice.value = null;
  toast.add("info", "设备已删除");
}

async function toggleCollection() {
  if (!selectedDevice.value) return;
  if (store.isCollecting) {
    await store.stopCollection(selectedDevice.value.ip);
  } else {
    await store.startCollection(selectedDevice.value.ip, selectedDevice.value.community);
  }
}

function switchTab(tab: "interfaces" | "arp" | "traffic") {
  activeTab.value = tab;
  if (!selectedDevice.value) return;
  if (tab === "arp") {
    store.fetchArpTable(selectedDevice.value.ip, selectedDevice.value.community);
  }
}
</script>

<template>
  <div class="flex h-full gap-4 p-4">
    <!-- Left panel: device list + scanner -->
    <div class="flex w-80 shrink-0 flex-col gap-3">
      <!-- Scanner -->
      <div class="rounded-xl border border-paper-deep/60 bg-paper p-3">
        <h3 class="mb-2 text-sm font-semibold text-ink">SNMP 扫描</h3>
        <div class="flex flex-col gap-2">
          <div class="flex gap-2">
            <input v-model="cidr" type="text" placeholder="192.168.1.0/24"
              class="flex-1 rounded-lg border border-paper-deep/60 bg-paper-warm/50 px-2 py-1.5 text-xs text-ink outline-none focus:border-bamboo/50" />
            <input v-model="community" type="text" placeholder="public"
              class="w-20 rounded-lg border border-paper-deep/60 bg-paper-warm/50 px-2 py-1.5 text-xs text-ink outline-none focus:border-bamboo/50" />
          </div>
          <button @click="startScan" :disabled="isScanning"
            class="rounded-lg bg-bamboo px-3 py-1.5 text-xs font-medium text-white transition-colors hover:bg-bamboo/90 disabled:opacity-50">
            {{ isScanning ? '扫描中...' : '扫描' }}
          </button>
          <!-- Progress -->
          <div v-if="store.discoverProgress" class="text-xs text-ink-faint">
            已扫描 {{ store.discoverProgress.scanned }}/{{ store.discoverProgress.total }}，发现 {{ store.discoverProgress.found }} 个设备
          </div>
        </div>
      </div>

      <!-- Device list -->
      <div class="flex-1 overflow-y-auto rounded-xl border border-paper-deep/60 bg-paper p-2">
        <div v-if="store.devices.length === 0" class="py-8 text-center text-xs text-ink-faint">
          暂无设备，请先扫描
        </div>
        <div v-for="device in store.devices" :key="device.id"
          @click="selectDevice(device)"
          class="cursor-pointer rounded-lg p-2.5 transition-colors hover:bg-paper-deep/50"
          :class="{ 'bg-bamboo/10': selectedDevice?.id === device.id }">
          <div class="flex items-center justify-between">
            <span class="text-xs font-medium text-ink">{{ device.hostname || device.ip }}</span>
            <span class="rounded bg-paper-deep/40 px-1.5 py-0.5 text-[10px] text-ink-faint">{{ device.vendor }}</span>
          </div>
          <div class="mt-0.5 text-[10px] text-ink-faint">{{ device.ip }} · {{ device.model }}</div>
        </div>
      </div>
    </div>

    <!-- Right panel: device detail -->
    <div v-if="selectedDevice" class="flex-1 rounded-xl border border-paper-deep/60 bg-paper p-4">
      <!-- Device header -->
      <div class="mb-4 flex items-center justify-between">
        <div>
          <h2 class="text-base font-semibold text-ink">{{ selectedDevice.hostname || selectedDevice.ip }}</h2>
          <p class="text-xs text-ink-faint">{{ selectedDevice.vendor }} {{ selectedDevice.model }} · {{ selectedDevice.ip }}</p>
        </div>
        <div class="flex items-center gap-2">
          <button @click="toggleCollection"
            class="rounded-lg px-3 py-1.5 text-xs font-medium transition-colors"
            :class="store.isCollecting
              ? 'bg-red-50 text-red-600 hover:bg-red-100 dark:bg-red-900/20 dark:text-red-400'
              : 'bg-bamboo/10 text-bamboo hover:bg-bamboo/20'">
            {{ store.isCollecting ? '停止采集' : '开始采集' }}
          </button>
          <button @click="handleDelete(selectedDevice.id)"
            class="rounded-lg px-3 py-1.5 text-xs font-medium text-red-500 transition-colors hover:bg-red-50 dark:hover:bg-red-900/20">
            删除
          </button>
        </div>
      </div>

      <!-- Tabs -->
      <div class="mb-4 flex gap-1 border-b border-paper-deep/60">
        <button v-for="tab in ([{k:'interfaces',l:'接口'},{k:'arp',l:'ARP表'},{k:'traffic',l:'流量'}])" :key="tab.k"
          @click="switchTab(tab.k as any)"
          class="border-b-2 px-3 py-2 text-xs font-medium transition-colors"
          :class="activeTab === tab.k
            ? 'border-bamboo text-bamboo'
            : 'border-transparent text-ink-faint hover:text-ink'">
          {{ tab.l }}
        </button>
      </div>

      <!-- Interfaces tab -->
      <div v-if="activeTab === 'interfaces'" class="overflow-y-auto" style="max-height: calc(100vh - 320px)">
        <table class="w-full text-xs">
          <thead>
            <tr class="text-ink-faint">
              <th class="px-2 py-1 text-left">接口</th>
              <th class="px-2 py-1 text-left">MAC</th>
              <th class="px-2 py-1 text-right">速率</th>
              <th class="px-2 py-1 text-center">状态</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="iface in store.interfaces" :key="iface.index"
              class="border-t border-paper-deep/30">
              <td class="px-2 py-1.5 font-medium text-ink">{{ iface.name }}</td>
              <td class="px-2 py-1.5 font-mono text-ink-faint">{{ iface.mac }}</td>
              <td class="px-2 py-1.5 text-right text-ink-faint">
                {{ iface.speed >= 1_000_000_000 ? (iface.speed / 1_000_000_000).toFixed(1) + ' Gbps' : (iface.speed / 1_000_000).toFixed(0) + ' Mbps' }}
              </td>
              <td class="px-2 py-1.5 text-center">
                <span class="inline-block h-2 w-2 rounded-full"
                  :class="iface.operStatus === 1 ? 'bg-green-500' : 'bg-red-500'"></span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <!-- ARP tab -->
      <div v-if="activeTab === 'arp'" class="overflow-y-auto" style="max-height: calc(100vh - 320px)">
        <table class="w-full text-xs">
          <thead>
            <tr class="text-ink-faint">
              <th class="px-2 py-1 text-left">IP</th>
              <th class="px-2 py-1 text-left">MAC</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(entry, i) in store.arpTable" :key="i"
              class="border-t border-paper-deep/30">
              <td class="px-2 py-1.5 font-mono text-ink">{{ entry.ip }}</td>
              <td class="px-2 py-1.5 font-mono text-ink-faint">{{ entry.mac }}</td>
            </tr>
          </tbody>
        </table>
      </div>

      <!-- Traffic tab -->
      <div v-if="activeTab === 'traffic'" class="flex flex-col items-center justify-center py-12 text-ink-faint">
        <p class="text-sm">流量趋势图</p>
        <p class="mt-1 text-xs">（启动采集后显示数据）</p>
        <div v-if="store.samples.length > 0" class="mt-4 text-xs text-ink">
          已采集 {{ store.samples.length }} 个样本
        </div>
      </div>
    </div>

    <!-- Empty state -->
    <div v-else class="flex flex-1 items-center justify-center text-sm text-ink-faint">
      选择一个设备查看详情
    </div>
  </div>
</template>
```

- [ ] **Step 2: Add route to router**

```typescript
// src/router/index.ts — add:
{
  path: "/snmp",
  name: "snmp",
  component: () => import("@/pages/snmp/Page.vue"),
}
```

- [ ] **Step 3: Commit**

```bash
git add src/pages/snmp/Page.vue src/router/index.ts
git commit -m "feat(snmp): add SNMP frontend page with discovery and detail views"
```

---

### Task 12: Cargo build verification

**Files:** None (verification only)

- [ ] **Step 1: Run cargo check**

```bash
cd src-tauri && cargo check 2>&1
```

Expected: Build succeeds with no errors (pre-existing warnings OK).

- [ ] **Step 2: Run frontend type check**

```bash
cd .. && npx vue-tsc --noEmit 2>&1
```

Expected: No type errors.

- [ ] **Step 3: Run frontend build**

```bash
npm run build 2>&1
```

Expected: Vite build succeeds.

- [ ] **Step 4: Commit if any fixes were needed**

```bash
git add -A
git commit -m "fix(snmp): fix build issues"
```

---

### Spec Coverage

| Spec Requirement | Task |
|---|---|
| SnmpSession wrapper (get/walk) | Task 4 |
| OID constants | Task 3 |
| CIDR device scanner | Task 5 |
| Periodic data collector | Task 6 |
| SQLite persistence | Task 7 |
| Tauri commands | Task 8 |
| Frontend discovery page | Task 11 |
| Frontend device detail (interfaces/ARP/traffic) | Task 11 |
| TypeScript bindings | Task 10 |
| Pinia store | Task 10 |
