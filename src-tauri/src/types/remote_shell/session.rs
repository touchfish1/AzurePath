use serde::{Deserialize, Serialize};
use uuid::Uuid;

fn default_environment() -> String {
    "default".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Ssh,
    Telnet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSession {
    pub id: Uuid,
    #[serde(default = "default_environment")]
    pub environment: String,
    pub name: String,
    pub protocol: Protocol,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub encoding: String,
    pub keepalive_secs: u64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInput {
    pub name: String,
    pub protocol: Protocol,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub encoding: Option<String>,
    pub keepalive_secs: Option<u64>,
    pub environment: Option<String>,
}

impl SessionInput {
    pub fn into_session(self) -> RemoteSession {
        let now = chrono::Utc::now().to_rfc3339();
        RemoteSession {
            id: Uuid::new_v4(),
            environment: self.environment.unwrap_or_else(default_environment),
            name: self.name,
            protocol: self.protocol,
            host: self.host,
            port: self.port,
            username: self.username,
            encoding: self.encoding.unwrap_or_else(|| "utf-8".to_string()),
            keepalive_secs: self.keepalive_secs.unwrap_or(30),
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub id: Uuid,
    pub environment: String,
    pub name: String,
    pub protocol: Protocol,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub is_connected: bool,
}
