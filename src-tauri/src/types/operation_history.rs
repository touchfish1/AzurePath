use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationRecord {
    pub id: String,
    pub op_type: String,
    pub target: String,
    pub status: String,
    pub summary: String,
    pub result_meta: serde_json::Value,
    pub created_at: String,
}
