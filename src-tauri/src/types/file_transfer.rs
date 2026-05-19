use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransfer {
    pub id: String,
    pub filename: String,
    pub path: Option<String>,
    pub size: u64,
    pub received: u64,
    pub status: String, // "pending" | "transferring" | "completed" | "rejected" | "error"
    pub peer_id: String,
    pub is_incoming: bool,
    pub created_at: String,
    #[serde(default)]
    pub download_url: Option<String>,
}
