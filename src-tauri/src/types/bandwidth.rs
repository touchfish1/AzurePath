use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterfaceInfo {
    pub name: String,
    pub friendly_name: String,
    pub ip: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BandwidthSample {
    pub interface: String,
    pub download_bps: u64,
    pub upload_bps: u64,
    pub total_rx: u64,
    pub total_tx: u64,
    pub timestamp: String,
}
