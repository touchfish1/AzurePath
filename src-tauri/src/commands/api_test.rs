use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::sync::LazyLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiRequest {
    pub method: String, // GET, POST, PUT, DELETE, PATCH
    pub url: String,
    pub headers: Vec<Vec<String>>, // [[key, value], ...]
    pub body: Option<String>,
    pub body_type: Option<String>, // "json", "form", "text"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<Vec<String>>,
    pub body: String,
    pub duration_ms: u64,
    pub body_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedRequest {
    pub id: String,
    pub name: String,
    pub request: ApiRequest,
    pub created_at: String,
    pub updated_at: String,
}

// In-memory save for collections (use a simple file-based JSON store)
static SAVED_REQUESTS: LazyLock<Mutex<Vec<SavedRequest>>> = LazyLock::new(|| {
    let requests = load_from_disk().unwrap_or_default();
    Mutex::new(requests)
});

fn storage_path() -> Result<std::path::PathBuf, String> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map_err(|_| "No home dir".to_string())?;
    Ok(std::path::PathBuf::from(home).join("AzurePath/api_requests.json"))
}

fn load_from_disk() -> Result<Vec<SavedRequest>, String> {
    let path = storage_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

fn save_to_disk(requests: &[SavedRequest]) -> Result<(), String> {
    let path = storage_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(requests).map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn send_api_request(request: ApiRequest) -> Result<ApiResponse, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .danger_accept_invalid_certs(false)
        .build()
        .map_err(|e| format!("Failed to create client: {}", e))?;

    let start = std::time::Instant::now();

    // Build request
    let mut req = match request.method.to_uppercase().as_str() {
        "GET" => client.get(&request.url),
        "POST" => client.post(&request.url),
        "PUT" => client.put(&request.url),
        "DELETE" => client.delete(&request.url),
        "PATCH" => client.patch(&request.url),
        _ => return Err(format!("Unsupported method: {}", request.method)),
    };

    // Add headers
    for pair in &request.headers {
        if pair.len() == 2 {
            req = req.header(&pair[0], &pair[1]);
        }
    }

    // Add body
    if let Some(body) = &request.body {
        if !body.is_empty() {
            match request.body_type.as_deref() {
                Some("json") => {
                    req = req.header("Content-Type", "application/json");
                    req = req.body(body.clone());
                }
                Some("form") => {
                    req = req.header("Content-Type", "application/x-www-form-urlencoded");
                    req = req.body(body.clone());
                }
                _ => {
                    req = req.body(body.clone());
                }
            }
        }
    }

    // Send
    let response = req.send().await.map_err(|e| format!("Request failed: {}", e))?;
    let duration = start.elapsed().as_millis() as u64;

    let status = response.status().as_u16();
    let status_text = response
        .status()
        .canonical_reason()
        .unwrap_or("Unknown")
        .to_string();

    // Collect response headers
    let headers: Vec<Vec<String>> = response
        .headers()
        .iter()
        .map(|(name, value)| {
            vec![
                name.as_str().to_string(),
                value.to_str().unwrap_or("").to_string(),
            ]
        })
        .collect();

    // Read body
    let body_bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;
    let body_size = body_bytes.len();

    // Try to format JSON response
    let body = if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&body_bytes) {
        serde_json::to_string_pretty(&json)
            .unwrap_or_else(|_| String::from_utf8_lossy(&body_bytes).to_string())
    } else {
        String::from_utf8_lossy(&body_bytes).to_string()
    };

    Ok(ApiResponse {
        status,
        status_text,
        headers,
        body,
        duration_ms: duration,
        body_size,
    })
}

#[tauri::command]
pub fn list_api_requests() -> Result<Vec<SavedRequest>, String> {
    let guard = SAVED_REQUESTS.lock().map_err(|e| e.to_string())?;
    Ok(guard.clone())
}

#[tauri::command]
pub fn save_api_request(
    id: Option<String>,
    name: String,
    request: ApiRequest,
) -> Result<SavedRequest, String> {
    let mut guard = SAVED_REQUESTS.lock().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();

    if let Some(existing_id) = id {
        if let Some(existing) = guard.iter_mut().find(|r| r.id == existing_id) {
            existing.name = name;
            existing.request = request;
            existing.updated_at = now.clone();
            let result = existing.clone();
            save_to_disk(&guard)?;
            return Ok(result);
        }
        return Err("Request not found".to_string());
    }

    let saved = SavedRequest {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        request,
        created_at: now.clone(),
        updated_at: now,
    };
    guard.push(saved.clone());
    save_to_disk(&guard)?;
    Ok(saved)
}

#[tauri::command]
pub fn delete_api_request(id: String) -> Result<(), String> {
    let mut guard = SAVED_REQUESTS.lock().map_err(|e| e.to_string())?;
    guard.retain(|r| r.id != id);
    save_to_disk(&guard)?;
    Ok(())
}
