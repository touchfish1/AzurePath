use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortRange {
    pub start: u16,
    pub end: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanOptions {
    pub concurrency: u32,
    pub timeout_ms: u64,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            concurrency: 100,
            timeout_ms: 3000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    pub task_id: String,
    pub scanned: u32,
    pub total: u32,
    pub open: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortFound {
    pub task_id: String,
    pub port: u16,
    pub service: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanComplete {
    pub task_id: String,
    pub target: String,
    pub open_ports: Vec<OpenPort>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenPort {
    pub port: u16,
    pub service: Option<String>,
}
