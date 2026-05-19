use crate::core::bookmark::{Bookmark, BookmarkStore};
use chrono::Utc;
use uuid::Uuid;

#[tauri::command]
pub fn list_bookmarks() -> Result<Vec<Bookmark>, String> {
    BookmarkStore::new()?.list_all()
}

#[tauri::command]
pub fn add_bookmark(label: String, target: String, tags: Vec<String>) -> Result<Bookmark, String> {
    let bm = Bookmark {
        id: Uuid::new_v4().to_string(),
        label,
        target,
        tags,
        created_at: Utc::now().to_rfc3339(),
    };
    BookmarkStore::new()?.add(&bm)?;
    Ok(bm)
}

#[tauri::command]
pub fn delete_bookmark(id: String) -> Result<(), String> {
    BookmarkStore::new()?.delete(&id)
}
