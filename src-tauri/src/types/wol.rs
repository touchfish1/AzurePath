use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WolRecord {
    pub id: String,
    pub mac: String,
    pub broadcast_ip: String,
    pub port: u16,
    pub label: String,
    pub last_used: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WolResult {
    pub success: bool,
    pub message: String,
}
