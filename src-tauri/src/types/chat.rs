use serde::{Deserialize, Serialize};

/// Wire format — length-prefixed JSON frames over TCP.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Frame {
    #[serde(rename = "hello")]
    Hello { id: String },
    #[serde(rename = "chat")]
    ChatMsg {
        id: String,
        from: String,
        from_name: String,
        from_ip: String,
        from_os: String,
        content: String,
        to: String,
        created_at: String,
    },
    #[serde(rename = "system")]
    System { content: String },
    #[serde(rename = "file_request")]
    FileRequest {
        file_id: String,
        filename: String,
        size: u64,
        from: String,
    },
    #[serde(rename = "file_response")]
    FileResponse {
        file_id: String,
        accepted: bool,
        data_port: u16,
    },
    #[serde(rename = "file_progress")]
    FileProgress {
        file_id: String,
        received: u64,
        total: u64,
        speed: f64,
    },
    #[serde(rename = "file_complete")]
    FileComplete { file_id: String },
    #[serde(rename = "file_ack")]
    FileAck { file_id: String },
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "pong")]
    Pong,
    #[serde(rename = "clipboard_sync")]
    ClipboardSync {
        entries: Vec<crate::types::clipboard::ClipboardEntry>,
    },
}

/// Stored chat message (SQLite row).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredMessage {
    pub id: String,
    pub peer_id: String,
    pub peer_name: String,
    pub peer_ip: String,
    pub peer_os: Option<String>,
    pub content: String,
    pub is_broadcast: bool,
    pub is_incoming: bool,
    pub file_ref: Option<String>,
    pub created_at: String,
}
