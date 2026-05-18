use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub id: String,
    pub hostname: String,
    pub ip: String,
    pub os: String,
    pub listen_port: u16,
    pub last_seen: String,
    pub status: String, // "online" | "offline"
}
