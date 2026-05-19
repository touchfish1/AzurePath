use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardEntry {
    pub id: String,
    pub content_type: String,    // "text" | "image" | "file"
    pub text_content: Option<String>,
    pub image_path: Option<String>,
    pub file_paths: Option<Vec<String>>,
    pub content_hash: String,
    pub is_favorite: bool,
    pub created_at: String,
}
