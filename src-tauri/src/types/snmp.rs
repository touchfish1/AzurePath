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
