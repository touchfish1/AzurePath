use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub feature: String, // "ping", "port_scan", "sniffer"
    pub params: serde_json::Value, // flexible JSON params
    pub created_at: String,
    pub updated_at: String,
}
