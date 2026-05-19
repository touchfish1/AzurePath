use crate::types::preset::Preset;
use chrono::Utc;
use std::path::PathBuf;
use uuid::Uuid;

/// Get the presets file path in the app data directory.
fn presets_path(app_data_dir: &PathBuf) -> PathBuf {
    app_data_dir.join("presets.json")
}

/// Load all presets from the presets file.
fn load_all(app_data_dir: &PathBuf) -> Vec<Preset> {
    let path = presets_path(app_data_dir);
    if !path.exists() {
        return Vec::new();
    }
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

/// Save all presets to the presets file.
fn save_all(app_data_dir: &PathBuf, presets: &[Preset]) -> Result<(), String> {
    let path = presets_path(app_data_dir);

    // Ensure the parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create dir: {}", e))?;
    }

    let content = serde_json::to_string_pretty(presets)
        .map_err(|e| format!("Failed to serialize: {}", e))?;
    std::fs::write(&path, content).map_err(|e| format!("Failed to write presets: {}", e))?;
    Ok(())
}

/// Save a new preset.
pub fn save_preset(
    name: String,
    feature: String,
    params: serde_json::Value,
    app_data_dir: &PathBuf,
) -> Result<Preset, String> {
    let mut presets = load_all(app_data_dir);
    let now = Utc::now().to_rfc3339();

    let preset = Preset {
        id: Uuid::new_v4().to_string(),
        name,
        feature,
        params,
        created_at: now.clone(),
        updated_at: now,
    };

    presets.push(preset.clone());
    save_all(app_data_dir, &presets)?;

    Ok(preset)
}

/// Load presets, optionally filtered by feature.
pub fn load_presets(
    feature: Option<String>,
    app_data_dir: &PathBuf,
) -> Result<Vec<Preset>, String> {
    let presets = load_all(app_data_dir);

    match feature {
        Some(f) => Ok(presets.into_iter().filter(|p| p.feature == f).collect()),
        None => Ok(presets),
    }
}

/// Delete a preset by ID.
pub fn delete_preset(id: String, app_data_dir: &PathBuf) -> Result<(), String> {
    let mut presets = load_all(app_data_dir);
    let len_before = presets.len();
    presets.retain(|p| p.id != id);

    if presets.len() == len_before {
        return Err(format!("Preset with id '{}' not found", id));
    }

    save_all(app_data_dir, &presets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().to_path_buf();
        (dir, path)
    }

    #[test]
    fn test_save_and_load_preset() {
        let (_tmp, dir) = setup();

        let params = serde_json::json!({
            "count": 4,
            "timeout": 3000,
            "target": "8.8.8.8"
        });

        let preset = save_preset(
            "Test Ping".to_string(),
            "ping".to_string(),
            params.clone(),
            &dir,
        )
        .unwrap();

        assert_eq!(preset.name, "Test Ping");
        assert_eq!(preset.feature, "ping");
        assert_eq!(preset.params, params);
        assert!(!preset.id.is_empty());
    }

    #[test]
    fn test_load_presets_filtered() {
        let (_tmp, dir) = setup();

        save_preset(
            "Ping A".to_string(),
            "ping".to_string(),
            serde_json::json!({}),
            &dir,
        )
        .unwrap();
        save_preset(
            "Ping B".to_string(),
            "ping".to_string(),
            serde_json::json!({}),
            &dir,
        )
        .unwrap();
        save_preset(
            "Scan A".to_string(),
            "port_scan".to_string(),
            serde_json::json!({}),
            &dir,
        )
        .unwrap();

        let ping_presets = load_presets(Some("ping".to_string()), &dir).unwrap();
        assert_eq!(ping_presets.len(), 2);

        let all = load_presets(None, &dir).unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_delete_preset() {
        let (_tmp, dir) = setup();

        let preset = save_preset(
            "To Delete".to_string(),
            "ping".to_string(),
            serde_json::json!({}),
            &dir,
        )
        .unwrap();

        let presets = load_presets(None, &dir).unwrap();
        assert_eq!(presets.len(), 1);

        delete_preset(preset.id.clone(), &dir).unwrap();

        let presets = load_presets(None, &dir).unwrap();
        assert_eq!(presets.len(), 0);
    }

    #[test]
    fn test_delete_nonexistent() {
        let (_tmp, dir) = setup();
        let result = delete_preset("nonexistent".to_string(), &dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_load() {
        let (_tmp, dir) = setup();
        let presets = load_presets(None, &dir).unwrap();
        assert!(presets.is_empty());
    }
}
