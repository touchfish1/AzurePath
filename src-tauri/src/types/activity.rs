use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEntry {
    pub id: i64,
    pub action_type: String,
    pub description: String,
    pub metadata: Option<String>,
    pub created_at: String,
}
