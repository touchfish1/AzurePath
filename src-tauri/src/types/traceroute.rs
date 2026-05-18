use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceOptions {
    pub max_hops: u32,
    pub timeout_ms: u64,
    pub probes_per_hop: u32,
}

impl Default for TraceOptions {
    fn default() -> Self {
        Self {
            max_hops: 30,
            timeout_ms: 5000,
            probes_per_hop: 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceHop {
    pub hop: u32,
    pub addr: Option<String>,
    pub hostname: Option<String>,
    pub latencies: Vec<Option<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceComplete {
    pub task_id: String,
    pub target: String,
    pub hops: Vec<TraceHop>,
}
