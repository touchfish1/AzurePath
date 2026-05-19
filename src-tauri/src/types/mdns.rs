use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdnsService {
    pub service_type: String,
    pub hostname: String,
    pub ip: String,
    pub port: u16,
    pub txt: HashMap<String, String>,
}
