use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MtrOptions {
    pub target: String,
    pub max_hops: u32,
    pub interval_ms: u64,
    pub timeout_ms: u64,
}

impl Default for MtrOptions {
    fn default() -> Self {
        Self {
            target: String::new(),
            max_hops: 30,
            interval_ms: 1000,
            timeout_ms: 3000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MtrHopStats {
    pub hop: u32,
    pub addr: Option<String>,
    pub hostname: Option<String>,
    pub sent: u32,
    pub received: u32,
    pub loss_percent: f64,
    pub min_ms: f64,
    pub avg_ms: f64,
    pub max_ms: f64,
    pub jitter_ms: f64,
    pub last_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MtrProgress {
    pub target: String,
    pub total_hops: u32,
    pub round: u32,
    pub hops: Vec<MtrHopStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MtrComplete {
    pub target: String,
    pub total_rounds: u32,
    pub hops: Vec<MtrHopStats>,
}
