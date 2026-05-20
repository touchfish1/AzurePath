use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubnetResult {
    pub network_address: String,
    pub broadcast_address: String,
    pub subnet_mask: String,
    pub wildcard_mask: String,
    pub usable_hosts: u64,
    pub ip_range: String,
    pub cidr: u8,
    pub ip_version: String,
    pub classification: IpClassification,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpClassification {
    pub is_private: bool,
    pub is_loopback: bool,
    pub is_link_local: bool,
    pub is_multicast: bool,
    pub is_public: bool,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubnetSplitResult {
    pub subnets: Vec<SubnetResult>,
    pub total_usable: u64,
}
