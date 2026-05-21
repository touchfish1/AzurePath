use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopologyNode {
    pub id: String,
    pub ip: String,
    pub hostname: String,
    pub device_type: String,
    pub vendor: String,
    pub model: String,
    pub os: String,
    pub cpu_usage: Option<f32>,
    pub memory_usage: Option<f32>,
    pub status: String,
    pub x: f64,
    pub y: f64,
    pub group_id: Option<String>,
    pub mac: String,
    pub interfaces: Vec<InterfaceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterfaceInfo {
    pub index: u32,
    pub name: String,
    pub mac: String,
    pub speed: u64,
    pub oper_status: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopologyLink {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub link_type: String,
    pub speed: Option<u64>,
    pub latency_ms: Option<f64>,
    pub bandwidth_usage: Option<f64>,
    pub source_iface: Option<String>,
    pub target_iface: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopologySnapshot {
    pub id: String,
    pub name: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotDetail {
    pub id: String,
    pub name: String,
    pub layout_algorithm: String,
    pub created_at: String,
    pub nodes: Vec<TopologyNode>,
    pub links: Vec<TopologyLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotDiff {
    pub added_nodes: Vec<String>,
    pub removed_nodes: Vec<String>,
    pub changed_nodes: Vec<String>,
}
