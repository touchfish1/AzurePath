use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecordType {
    #[serde(rename = "a")]
    A,
    #[serde(rename = "aaaa")]
    Aaaa,
    #[serde(rename = "cname")]
    Cname,
    #[serde(rename = "mx")]
    Mx,
    #[serde(rename = "ns")]
    Ns,
    #[serde(rename = "soa")]
    Soa,
    #[serde(rename = "txt")]
    Txt,
    #[serde(rename = "all")]
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecord {
    pub name: String,
    #[serde(rename = "type")]
    pub record_type: String,
    pub value: String,
    pub ttl: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsResult {
    pub task_id: String,
    pub target: String,
    pub records: Vec<DnsRecord>,
}
