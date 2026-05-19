use crate::core::target_group::{TargetGroup, TargetGroupStore};
use chrono::Utc;
use uuid::Uuid;

fn get_store() -> Result<TargetGroupStore, String> {
    TargetGroupStore::new()
}

#[tauri::command]
pub fn list_target_groups() -> Result<Vec<TargetGroup>, String> {
    get_store()?.list_groups()
}

#[tauri::command]
pub fn get_target_group(id: String) -> Result<Option<TargetGroup>, String> {
    get_store()?.get_group(&id)
}

#[tauri::command]
pub fn save_target_group(
    id: Option<String>,
    name: String,
    targets: Vec<String>,
) -> Result<TargetGroup, String> {
    let store = get_store()?;
    let now = Utc::now().to_rfc3339();

    if let Some(existing_id) = id {
        // Update existing
        let mut group = store
            .get_group(&existing_id)?
            .ok_or("Group not found")?;
        group.name = name;
        group.targets = targets;
        group.updated_at = now;
        store.save_group(&group)?;
        Ok(group)
    } else {
        // Create new
        let group = TargetGroup {
            id: Uuid::new_v4().to_string(),
            name,
            targets,
            created_at: now.clone(),
            updated_at: now,
        };
        store.save_group(&group)?;
        Ok(group)
    }
}

#[tauri::command]
pub fn delete_target_group(id: String) -> Result<(), String> {
    get_store()?.delete_group(&id)
}
