use std::path::Path;

/// Save a string to a file at the given path.
///
/// Used by the frontend to export HTML reports.
#[tauri::command]
pub fn save_report(path: String, content: String) -> Result<(), String> {
    let path = Path::new(&path);

    // Ensure parent directory exists.
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent directory: {}", e))?;
    }

    std::fs::write(path, &content)
        .map_err(|e| format!("Failed to write report file: {}", e))?;

    Ok(())
}
