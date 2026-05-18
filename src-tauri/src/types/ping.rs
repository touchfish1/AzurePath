use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PingOptions {
    pub count: u32,
    pub interval_ms: u64,
    pub timeout_ms: u64,
    pub payload_size: u32,
}

impl Default for PingOptions {
    fn default() -> Self {
        Self {
            count: 4,
            interval_ms: 1000,
            timeout_ms: 5000,
            payload_size: 32,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingProgress {
    pub task_id: String,
    pub seq: u32,
    pub ttl: u32,
    pub latency_ms: Option<f64>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingComplete {
    pub task_id: String,
    pub sent: u32,
    pub received: u32,
    pub loss_percent: f64,
    pub min_ms: f64,
    pub avg_ms: f64,
    pub max_ms: f64,
}
