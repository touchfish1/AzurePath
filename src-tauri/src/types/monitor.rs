use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorTarget {
    pub id: String,
    pub host: String,
    pub label: String,
    pub interval_secs: u64,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PingRecord {
    pub id: i64,
    pub target_id: String,
    pub target_host: String,
    pub timestamp: String,
    pub latency_ms: Option<f64>,
    pub loss_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorUpdatePayload {
    pub target_id: String,
    pub target_host: String,
    pub label: String,
    pub timestamp: String,
    pub latency_ms: Option<f64>,
    pub loss_rate: f64,
    pub min_ms: f64,
    pub avg_ms: f64,
    pub max_ms: f64,
    pub sent: u32,
    pub received: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorStatusPayload {
    pub running: bool,
    pub target_count: usize,
}
