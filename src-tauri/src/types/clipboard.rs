use serde::{Deserialize, Serialize};

/// Maximum clipboard entry content size in bytes (20 MB).
/// Entries larger than this are not stored or synced.
pub const MAX_CLIPBOARD_BYTES: u64 = 20 * 1024 * 1024;

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

impl ClipboardEntry {
    /// Approximate content size in bytes.
    /// For images this is the raw RGBA size (only available before saving);
    /// for synced entries it returns 0 (caller should check on the sending side).
    pub fn content_bytes(&self) -> u64 {
        match self.content_type.as_str() {
            "text" => self.text_content.as_ref().map(|t| t.len() as u64).unwrap_or(0),
            "file" => self.file_paths.as_ref()
                .map(|files| {
                    files.iter().filter_map(|p| std::fs::metadata(p).ok()).map(|m| m.len()).sum()
                })
                .unwrap_or(0),
            _ => 0,
        }
    }
}
