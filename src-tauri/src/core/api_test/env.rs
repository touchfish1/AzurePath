//! Environment variable management for API testing.

use std::sync::Mutex;
use std::sync::LazyLock;

use crate::types::api_test::Environment;

static ENVIRONMENTS: LazyLock<Mutex<Vec<Environment>>> = LazyLock::new(|| {
    let envs = load_from_disk().unwrap_or_default();
    Mutex::new(envs)
});

fn storage_path() -> Result<std::path::PathBuf, String> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map_err(|_| "No home directory".to_string())?;
    Ok(std::path::PathBuf::from(home).join("AzurePath/environments.json"))
}

fn load_from_disk() -> Result<Vec<Environment>, String> {
    let path = storage_path()?;
    if !path.exists() {
        return Ok(vec![Environment {
            id: "default".into(),
            name: "默认环境".into(),
            variables: vec![],
        }]);
    }
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

fn save_to_disk(envs: &[Environment]) -> Result<(), String> {
    let path = storage_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(envs).map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

pub struct EnvironmentManager;

impl EnvironmentManager {
    pub fn list() -> Result<Vec<Environment>, String> {
        let guard = ENVIRONMENTS.lock().map_err(|e| e.to_string())?;
        Ok(guard.clone())
    }

    pub fn save(env: Environment) -> Result<Environment, String> {
        let mut guard = ENVIRONMENTS.lock().map_err(|e| e.to_string())?;
        if let Some(existing) = guard.iter_mut().find(|e| e.id == env.id) {
            existing.name = env.name.clone();
            existing.variables = env.variables.clone();
        } else {
            guard.push(env.clone());
        }
        save_to_disk(&guard)?;
        Ok(env)
    }

    pub fn delete(id: &str) -> Result<(), String> {
        let mut guard = ENVIRONMENTS.lock().map_err(|e| e.to_string())?;
        guard.retain(|e| e.id != id);
        save_to_disk(&guard)?;
        Ok(())
    }

    /// Replace {{var_name}} placeholders in the input string with variable values.
    pub fn substitute(input: &str, variables: &[Vec<String>]) -> String {
        let mut result = input.to_string();
        for pair in variables {
            if pair.len() == 2 {
                let placeholder = format!("{{{{{}}}}}", pair[0]);
                result = result.replace(&placeholder, &pair[1]);
            }
        }
        result
    }
}
