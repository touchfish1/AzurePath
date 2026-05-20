#[tauri::command]
pub async fn save_file(path: String, content: String) -> Result<(), String> {
    // Create parent directories if they don't exist
    let file_path = std::path::Path::new(&path);
    if let Some(parent) = file_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }
    tokio::fs::write(&file_path, &content)
        .await
        .map_err(|e| format!("Failed to write file: {}", e))?;
    Ok(())
}
