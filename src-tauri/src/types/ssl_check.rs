use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SslCheckResult {
    pub hostname: String,
    pub issuer: Option<String>,
    pub subject: Option<String>,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub is_expired: bool,
    pub is_self_signed: bool,
    pub days_remaining: Option<i64>,
}
