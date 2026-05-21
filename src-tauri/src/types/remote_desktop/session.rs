use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Rdp,
    Vnc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopSession {
    pub id: String,
    pub name: String,
    pub protocol: Protocol,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub quality: u8,
    pub desktop_width: u16,
    pub desktop_height: u16,
    pub domain: Option<String>,
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
    pub quality: Option<u8>,
    pub desktop_width: Option<u16>,
    pub desktop_height: Option<u16>,
    pub domain: Option<String>,
}

impl SessionInput {
    pub fn into_session(self) -> DesktopSession {
        let now = chrono::Utc::now().to_rfc3339();
        DesktopSession {
            id: uuid::Uuid::new_v4().to_string(),
            name: self.name,
            protocol: self.protocol,
            host: self.host,
            port: self.port,
            username: self.username,
            quality: self.quality.unwrap_or(75),
            desktop_width: self.desktop_width.unwrap_or(1280),
            desktop_height: self.desktop_height.unwrap_or(720),
            domain: self.domain,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub id: String,
    pub name: String,
    pub protocol: Protocol,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub quality: u8,
    pub is_connected: bool,
    pub desktop_width: Option<u16>,
    pub desktop_height: Option<u16>,
    pub domain: Option<String>,
}
