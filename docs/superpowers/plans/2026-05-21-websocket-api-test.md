# WebSocket/API 测试工具 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enhance the API test tool with WebSocket client, environment variables, request collections, code generation, and authentication helpers.

**Architecture:** Add `tokio-tungstenite` for WebSocket, extend existing `commands/api_test.rs` with new commands, create `core/api_test/` for WebSocket/env/codegen logic, enhance the Vue page with tabs and panels.

**Tech Stack:** `tokio-tungstenite` (WebSocket), `reqwest` (HTTP, existing), `futures` (streaming, existing)

---

## File Structure

| File | Change | Responsibility |
|------|--------|----------------|
| `src-tauri/Cargo.toml` | **Modify** | Add `tokio-tungstenite`, `futures-util` |
| `src-tauri/src/types/api_test.rs` | **Create** | `WsMessage`, `Environment`, `RequestCollection`, `CollectionItem`, `AuthConfig` |
| `src-tauri/src/types/mod.rs` | **Modify** | Add `pub mod api_test;` |
| `src-tauri/src/core/api_test/mod.rs` | **Create** | Module declarations |
| `src-tauri/src/core/api_test/ws.rs` | **Create** | `WsSession` — WebSocket connect/send/close with Tauri events |
| `src-tauri/src/core/api_test/env.rs` | **Create** | `EnvironmentManager` — CRUD + `{{var}}` substitution |
| `src-tauri/src/core/api_test/codegen.rs` | **Create** | `generate_curl`, `generate_javascript`, `generate_python` |
| `src-tauri/src/core/mod.rs` | **Modify** | Add `pub mod api_test;` |
| `src-tauri/src/commands/api_test.rs` | **Modify** | Add WebSocket/env/collection/codegen commands + auth |
| `src-tauri/src/lib.rs` | **Modify** | Register new commands |
| `src/lib/tauri.ts` | **Modify** | Add TypeScript interfaces and invoke wrappers |
| `src/stores/apiTest.ts` | **Modify** | Add WebSocket, env, collection state + actions |
| `src/pages/api-test/Page.vue` | **Modify** | Add tabs (HTTP/WebSocket), auth panel, env selector, collection tree, code gen |

---

### Task 1: Add dependency and data types

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/types/mod.rs`
- Create: `src-tauri/src/types/api_test.rs`

- [ ] **Step 1: Add tokio-tungstenite to Cargo.toml**

```toml
# In [dependencies] section, add:
tokio-tungstenite = { version = "0.21", features = ["native-tls"] }
```

- [ ] **Step 2: Create types/api_test.rs**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsMessage {
    pub id: String,
    pub direction: String,
    pub content: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    pub id: String,
    pub name: String,
    pub variables: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestCollection {
    pub id: String,
    pub name: String,
    pub requests: Vec<CollectionItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionItem {
    pub id: String,
    pub name: String,
    pub request: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthConfig {
    pub auth_type: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub token: Option<String>,
    pub api_key: Option<String>,
    pub api_key_name: Option<String>,
    pub api_key_location: Option<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            auth_type: "none".into(),
            username: None,
            password: None,
            token: None,
            api_key: None,
            api_key_name: None,
            api_key_location: None,
        }
    }
}
```

- [ ] **Step 3: Add module declaration**

```rust
// src-tauri/src/types/mod.rs — add:
pub mod api_test;
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/types/api_test.rs src-tauri/src/types/mod.rs
git commit -m "feat(api-test): add tokio-tungstenite dependency and data types"
```

---

### Task 2: Create WebSocket session module

**Files:**
- Create: `src-tauri/src/core/api_test/mod.rs`
- Create: `src-tauri/src/core/api_test/ws.rs`
- Modify: `src-tauri/src/core/mod.rs`

- [ ] **Step 1: Create core module declarations**

```rust
// src-tauri/src/core/api_test/mod.rs
pub mod codegen;
pub mod env;
pub mod ws;
```

```rust
// src-tauri/src/core/mod.rs — add:
pub mod api_test;
```

- [ ] **Step 2: Create ws.rs**

```rust
//! WebSocket session management.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use tauri::AppHandle;
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use crate::core::utils::emit_or_warn;

pub struct WsSession {
    pub url: String,
    pub connected: Arc<AtomicBool>,
    tx: mpsc::Sender<String>,
}

impl WsSession {
    pub async fn connect(url: &str, app: AppHandle) -> Result<Self, String> {
        let (ws_stream, _) = connect_async(url)
            .await
            .map_err(|e| format!("WebSocket connection failed: {e}"))?;

        let (mut write, mut read) = ws_stream.split();
        let (tx, mut rx) = mpsc::channel::<String>(256);
        let connected = Arc::new(AtomicBool::new(true));
        let connected_clone = connected.clone();
        let app_clone = app.clone();
        let url_owned = url.to_string();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(msg) = rx.recv() => {
                        if write.send(Message::Text(msg)).await.is_err() {
                            break;
                        }
                    }
                    msg = read.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                emit_or_warn(&app, "ws:message", &serde_json::json!({
                                    "direction": "received",
                                    "content": text,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                }));
                            }
                            Some(Ok(Message::Close(_))) | None => break,
                            Some(Err(e)) => {
                                emit_or_warn(&app, "ws:error", &serde_json::json!({
                                    "error": format!("{e}")
                                }));
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }

            connected_clone.store(false, Ordering::Relaxed);
            emit_or_warn(&app, "ws:disconnected", &serde_json::json!({
                "code": 1000, "reason": "Connection closed"
            }));
        });

        emit_or_warn(&app_clone, "ws:connected", &serde_json::json!({
            "url": url_owned
        }));

        Ok(Self {
            url: url.to_string(),
            connected,
            tx,
        })
    }

    pub async fn send(&self, text: String) -> Result<(), String> {
        if !self.connected.load(Ordering::Relaxed) {
            return Err("WebSocket not connected".into());
        }
        self.tx
            .send(text)
            .await
            .map_err(|_| "WebSocket send channel closed".to_string())
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }
}
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/core/api_test/mod.rs src-tauri/src/core/api_test/ws.rs src-tauri/src/core/mod.rs
git commit -m "feat(api-test): add WebSocket session module"
```

---

### Task 3: Create environment variable manager

**Files:**
- Create: `src-tauri/src/core/api_test/env.rs`

- [ ] **Step 1: Create env.rs**

```rust
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
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/core/api_test/env.rs
git commit -m "feat(api-test): add environment variable manager"
```

---

### Task 4: Create code generation module

**Files:**
- Create: `src-tauri/src/core/api_test/codegen.rs`

- [ ] **Step 1: Create codegen.rs**

```rust
//! Code generation for API requests — curl, JavaScript, Python.

use serde_json::Value;

/// The ApiRequest subset needed for code generation.
pub struct CodegenRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub body_type: Option<String>,
}

pub fn generate_curl(req: &CodegenRequest) -> String {
    let mut parts = vec![format!("curl -X {}", req.method)];

    parts.push(format!("'{}'", req.url));

    for (k, v) in &req.headers {
        parts.push(format!("-H '{}: {}'", k, v));
    }

    if let Some(body) = &req.body {
        if !body.is_empty() {
            parts.push(format!("-d '{}'", body.replace('\'', "'\\''")));
        }
    }

    parts.join(" \\\n  ")
}

pub fn generate_javascript(req: &CodegenRequest) -> String {
    let mut indent = "  ";
    let mut lines = vec![format!("fetch('{}', {{", req.url)];

    lines.push(format!("{}method: '{}',", indent, req.method));

    if !req.headers.is_empty() {
        lines.push(format!("{}headers: {{", indent));
        for (k, v) in &req.headers {
            lines.push(format!("{}  '{}': '{}',", indent, k, v));
        }
        lines.push(format!("{}}},", indent));
    }

    if let Some(body) = &req.body {
        if !body.is_empty() {
            if req.body_type.as_deref() == Some("json") {
                lines.push(format!("{}body: JSON.stringify({}),", indent, body));
            } else {
                lines.push(format!("{}body: '{}',", indent, body));
            }
        }
    }

    lines.push("})".to_string());
    lines.push(".then(res => res.json())".to_string());
    lines.push(".then(console.log)".to_string());
    lines.push(".catch(console.error);".to_string());

    lines.join("\n")
}

pub fn generate_python(req: &CodegenRequest) -> String {
    let mut lines = vec!["import requests".to_string(), String::new()];

    let method_lower = req.method.to_lowercase();
    let method_call = if method_lower == "get" {
        "requests.get"
    } else {
        &format!("requests.{}", method_lower)
    };

    lines.push(format!("response = {}.\\", method_call));
    lines.push(format!("    '{}',\\", req.url));

    if !req.headers.is_empty() {
        let header_items: Vec<String> = req
            .headers
            .iter()
            .map(|(k, v)| format!("        '{}': '{}'", k, v))
            .collect();
        lines.push("    headers={".to_string());
        lines.extend(header_items);
        lines.push("    },".to_string());
    }

    if let Some(body) = &req.body {
        if !body.is_empty() {
            if req.body_type.as_deref() == Some("json") {
                lines.push(format!("    json={},".format_json_arg(body)));
            } else {
                lines.push(format!("    data='{}',", body));
            }
        }
    }

    lines.push(String::new());
    lines.push("print(response.status_code)".to_string());
    lines.push("print(response.text)".to_string());

    lines.join("\n")
}

trait JsonFormat {
    fn format_json_arg(&self, body: &str) -> String;
}

impl JsonFormat for String {
    fn format_json_arg(&self, body: &str) -> String {
        // Try to parse as JSON and re-serialize as a Python-like dict
        if let Ok(val) = serde_json::from_str::<Value>(body) {
            json_to_python(&val)
        } else {
            format!("'{}'", body)
        }
    }
}

fn json_to_python(val: &Value) -> String {
    match val {
        Value::Object(map) => {
            let items: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("    '{}': {}", k, json_to_python(v)))
                .collect();
            format!("{{\n{}\n}}", items.join(",\n"))
        }
        Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(json_to_python).collect();
            format!("[{}]", items.join(", "))
        }
        Value::String(s) => format!("'{}'", s),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "None".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_req() -> CodegenRequest {
        CodegenRequest {
            method: "POST".into(),
            url: "https://api.example.com/data".into(),
            headers: vec![
                ("Content-Type".into(), "application/json".into()),
                ("Authorization".into(), "Bearer tok_123".into()),
            ],
            body: Some(r#"{"name":"test"}"#.into()),
            body_type: Some("json".into()),
        }
    }

    #[test]
    fn test_generate_curl() {
        let code = generate_curl(&make_req());
        assert!(code.contains("curl -X POST"));
        assert!(code.contains("api.example.com/data"));
        assert!(code.contains("Authorization: Bearer tok_123"));
        assert!(code.contains("-d"));
    }

    #[test]
    fn test_generate_javascript() {
        let code = generate_javascript(&make_req());
        assert!(code.contains("fetch("));
        assert!(code.contains("method: 'POST'"));
        assert!(code.contains("JSON.stringify"));
    }

    #[test]
    fn test_generate_python() {
        let code = generate_python(&make_req());
        assert!(code.contains("import requests"));
        assert!(code.contains("requests.post"));
        assert!(code.contains("json="));
    }

    #[test]
    fn test_generate_curl_get() {
        let req = CodegenRequest {
            method: "GET".into(),
            url: "https://example.com".into(),
            headers: vec![],
            body: None,
            body_type: None,
        };
        let code = generate_curl(&req);
        assert!(code.contains("curl -X GET"));
        assert!(!code.contains("-d"));
    }

    #[test]
    fn test_empty_headers() {
        let req = CodegenRequest {
            method: "GET".into(),
            url: "https://example.com".into(),
            headers: vec![],
            body: None,
            body_type: None,
        };
        let js = generate_javascript(&req);
        assert!(js.contains("fetch("));
        assert!(!js.contains("headers:"));
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cd src-tauri && cargo test -- codegen --nocapture
```

Expected: All 5 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/core/api_test/codegen.rs
git commit -m "feat(api-test): add code generation module with tests"
```

---

### Task 5: Enhance API test commands

**Files:**
- Modify: `src-tauri/src/commands/api_test.rs`

- [ ] **Step 1: Add imports and new commands**

At the top of the file, add:
```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::Mutex;

use crate::core::api_test::codegen::{self, CodegenRequest};
use crate::core::api_test::env::EnvironmentManager;
use crate::core::api_test::ws::WsSession;
use crate::core::utils::emit_or_warn;
use crate::types::api_test::{AuthConfig, Environment, RequestCollection, WsMessage};
```

Add a global WS session store:
```rust
static WS_SESSION: LazyLock<Mutex<Option<WsSession>>> = LazyLock::new(|| Mutex::new(None));
static WS_MESSAGES: LazyLock<Mutex<Vec<WsMessage>>> = LazyLock::new(|| Mutex::new(Vec::new()));
```

Modify `ApiRequest` to add auth:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<Vec<String>>,
    pub body: Option<String>,
    pub body_type: Option<String>,
    pub auth: Option<AuthConfig>,  // NEW
}
```

Modify `send_api_request` to apply auth and env substitution before sending. After building the client, add:
```rust
// Apply auth
let mut final_headers = request.headers.clone();
if let Some(ref auth) = request.auth {
    match auth.auth_type.as_str() {
        "basic" => {
            if let (Some(user), Some(pass)) = (&auth.username, &auth.password) {
                let credentials = base64::engine::general_purpose::STANDARD
                    .encode(format!("{}:{}", user, pass));
                final_headers.push(vec!["Authorization".into(), format!("Basic {}", credentials)]);
            }
        }
        "bearer" => {
            if let Some(token) = &auth.token {
                final_headers.push(vec!["Authorization".into(), format!("Bearer {}", token)]);
            }
        }
        "apiKey" => {
            if let (Some(key_name), Some(key_val)) = (&auth.api_key_name, &auth.api_key) {
                if auth.api_key_location.as_deref() == Some("query") {
                    // Append to URL — handled below
                } else {
                    final_headers.push(vec![key_name.clone(), key_val.clone()]);
                }
            }
        }
        _ => {}
    }
}
request.headers = final_headers;

// Apply env substitution
let env_vars = EnvironmentManager::list().unwrap_or_default();
let active_vars = env_vars.first().map(|e| e.variables.clone()).unwrap_or_default();
request.url = EnvironmentManager::substitute(&request.url, &active_vars);
```

Add new WebSocket commands after the existing ones:
```rust
// ── WebSocket Commands ──

#[tauri::command]
pub async fn ws_connect(app: AppHandle, url: String) -> Result<(), String> {
    let session = WsSession::connect(&url, app).await?;
    let mut guard = WS_SESSION.lock().await;
    *guard = Some(session);
    Ok(())
}

#[tauri::command]
pub async fn ws_send(message: String) -> Result<(), String> {
    let mut guard = WS_SESSION.lock().await;
    let session = guard.as_mut().ok_or("WebSocket not connected")?;
    session.send(message.clone()).await?;

    // Record sent message
    let mut msgs = WS_MESSAGES.lock().map_err(|e| e.to_string())?;
    msgs.push(WsMessage {
        id: uuid::Uuid::new_v4().to_string(),
        direction: "sent".into(),
        content: message,
        timestamp: chrono::Utc::now().to_rfc3339(),
    });
    Ok(())
}

#[tauri::command]
pub async fn ws_close() -> Result<(), String> {
    let mut guard = WS_SESSION.lock().await;
    *guard = None;
    Ok(())
}

#[tauri::command]
pub fn ws_get_messages() -> Result<Vec<WsMessage>, String> {
    let guard = WS_MESSAGES.lock().map_err(|e| e.to_string())?;
    Ok(guard.clone())
}

#[tauri::command]
pub fn ws_clear_messages() -> Result<(), String> {
    let mut guard = WS_MESSAGES.lock().map_err(|e| e.to_string())?;
    guard.clear();
    Ok(())
}

// ── Environment Commands ──

#[tauri::command]
pub fn env_list() -> Result<Vec<Environment>, String> {
    EnvironmentManager::list()
}

#[tauri::command]
pub fn env_save(environment: Environment) -> Result<Environment, String> {
    EnvironmentManager::save(environment)
}

#[tauri::command]
pub fn env_delete(id: String) -> Result<(), String> {
    EnvironmentManager::delete(&id)
}

// ── Collection Commands ──

static COLLECTIONS: LazyLock<Mutex<Vec<RequestCollection>>> = LazyLock::new(|| {
    let collections = load_collections_from_disk().unwrap_or_default();
    Mutex::new(collections)
});

fn collections_path() -> Result<std::path::PathBuf, String> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map_err(|_| "No home directory".to_string())?;
    Ok(std::path::PathBuf::from(home).join("AzurePath/collections.json"))
}

fn load_collections_from_disk() -> Result<Vec<RequestCollection>, String> {
    let path = collections_path()?;
    if !path.exists() { return Ok(vec![]); }
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

fn save_collections_to_disk(collections: &[RequestCollection]) -> Result<(), String> {
    let path = collections_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(collections).map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn collection_list() -> Result<Vec<RequestCollection>, String> {
    let guard = COLLECTIONS.lock().map_err(|e| e.to_string())?;
    Ok(guard.clone())
}

#[tauri::command]
pub fn collection_save(
    name: String,
    id: Option<String>,
    requests: Vec<CollectionItem>,
) -> Result<RequestCollection, String> {
    let mut guard = COLLECTIONS.lock().map_err(|e| e.to_string())?;

    if let Some(existing_id) = id {
        if let Some(existing) = guard.iter_mut().find(|c| c.id == existing_id) {
            existing.name = name;
            existing.requests = requests;
            save_collections_to_disk(&guard)?;
            return Ok(existing.clone());
        }
        return Err("Collection not found".to_string());
    }

    let collection = RequestCollection {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        requests,
    };
    guard.push(collection.clone());
    save_collections_to_disk(&guard)?;
    Ok(collection)
}

#[tauri::command]
pub fn collection_delete(id: String) -> Result<(), String> {
    let mut guard = COLLECTIONS.lock().map_err(|e| e.to_string())?;
    guard.retain(|c| c.id != id);
    save_collections_to_disk(&guard)?;
    Ok(())
}

// ── Code Generation ──

#[tauri::command]
pub fn generate_http_code(
    method: String,
    url: String,
    headers: Vec<Vec<String>>,
    body: Option<String>,
    body_type: Option<String>,
    lang: String,
) -> Result<String, String> {
    let req = CodegenRequest {
        method,
        url,
        headers: headers.into_iter().filter_map(|h| {
            if h.len() == 2 { Some((h[0].clone(), h[1].clone())) } else { None }
        }).collect(),
        body,
        body_type,
    };

    match lang.as_str() {
        "curl" => Ok(codegen::generate_curl(&req)),
        "javascript" => Ok(codegen::generate_javascript(&req)),
        "python" => Ok(codegen::generate_python(&req)),
        _ => Err(format!("Unsupported language: {lang}")),
    }
}
```

Also add `use base64::Engine;` to the top imports (base64 is already in Cargo.toml).

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/commands/api_test.rs
git commit -m "feat(api-test): add WebSocket, env, collection, and codegen commands"
```

---

### Task 6: Register new commands in lib.rs

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add new commands to invoke_handler**

After the existing `commands::api_test::delete_api_request,` line:
```rust
// API Test WebSocket
commands::api_test::ws_connect,
commands::api_test::ws_send,
commands::api_test::ws_close,
commands::api_test::ws_get_messages,
commands::api_test::ws_clear_messages,
// API Test Environment
commands::api_test::env_list,
commands::api_test::env_save,
commands::api_test::env_delete,
// API Test Collections
commands::api_test::collection_list,
commands::api_test::collection_save,
commands::api_test::collection_delete,
// API Test Code Generation
commands::api_test::generate_http_code,
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(api-test): register new API test commands"
```

---

### Task 7: Frontend — TypeScript bindings and store

**Files:**
- Modify: `src/lib/tauri.ts`
- Modify: `src/stores/apiTest.ts`

- [ ] **Step 1: Add new TypeScript interfaces to tauri.ts**

After the existing API test section (after line ~1063), add:
```typescript
// ── WebSocket ──

export interface WsMessage {
  id: string;
  direction: string;
  content: string;
  timestamp: string;
}

export function wsConnect(url: string): Promise<void> {
  return invoke("ws_connect", { url });
}

export function wsSend(message: string): Promise<void> {
  return invoke("ws_send", { message });
}

export function wsClose(): Promise<void> {
  return invoke("ws_close");
}

export function wsGetMessages(): Promise<WsMessage[]> {
  return invoke("ws_get_messages");
}

export function wsClearMessages(): Promise<void> {
  return invoke("ws_clear_messages");
}

export function onWsMessage(cb: (payload: WsMessage) => void): Promise<UnlistenFn> {
  return listen<WsMessage>("ws:message", (event) => cb(event.payload));
}

export function onWsConnected(cb: (payload: { url: string }) => void): Promise<UnlistenFn> {
  return listen("ws:connected", (event) => cb(event.payload as any));
}

export function onWsDisconnected(cb: (payload: { code: number; reason: string }) => void): Promise<UnlistenFn> {
  return listen("ws:disconnected", (event) => cb(event.payload as any));
}

// ── Environment ──

export interface Environment {
  id: string;
  name: string;
  variables: string[][];
}

export function envList(): Promise<Environment[]> {
  return invoke("env_list");
}

export function envSave(environment: Environment): Promise<Environment> {
  return invoke("env_save", { environment });
}

export function envDelete(id: string): Promise<void> {
  return invoke("env_delete", { id });
}

// ── Collections ──

export interface CollectionItem {
  id: string;
  name: string;
  request: any;
}

export interface RequestCollection {
  id: string;
  name: string;
  requests: CollectionItem[];
}

export function collectionList(): Promise<RequestCollection[]> {
  return invoke("collection_list");
}

export function collectionSave(name: string, id: string | null, requests: CollectionItem[]): Promise<RequestCollection> {
  return invoke("collection_save", { name, id, requests });
}

export function collectionDelete(id: string): Promise<void> {
  return invoke("collection_delete", { id });
}

// ── Code Generation ──

export function generateHttpCode(method: string, url: string, headers: string[][], body: string | null, bodyType: string | null, lang: string): Promise<string> {
  return invoke("generate_http_code", { method, url, headers, body, bodyType, lang });
}

// ── Auth Config ──

export interface AuthConfig {
  authType: string;
  username: string | null;
  password: string | null;
  token: string | null;
  apiKey: string | null;
  apiKeyName: string | null;
  apiKeyLocation: string | null;
}
```

Update the existing `ApiRequest` interface to add `auth`:
```typescript
export interface ApiRequest {
  method: string;
  url: string;
  headers: string[][];
  body: string | null;
  bodyType: string | null;
  auth?: AuthConfig | null;  // ADD THIS LINE
}
```

- [ ] **Step 2: Enhance Pinia store**

Replace `src/stores/apiTest.ts` with the enhanced version:

```typescript
import { defineStore } from "pinia";
import { ref } from "vue";
import {
  sendApiRequest,
  listApiRequests,
  saveApiRequest,
  deleteApiRequest,
  wsConnect,
  wsSend,
  wsClose,
  wsGetMessages,
  wsClearMessages,
  onWsMessage,
  onWsConnected,
  onWsDisconnected,
  envList,
  envSave,
  envDelete,
  collectionList,
  collectionSave,
  collectionDelete,
  generateHttpCode,
  type ApiRequest,
  type ApiResponse,
  type SavedRequest,
  type WsMessage,
  type Environment,
  type RequestCollection,
} from "@/lib/tauri";
import type { UnlistenFn } from "@tauri-apps/api/event";

export const useApiTestStore = defineStore("apiTest", () => {
  // ── HTTP Request State ──
  const savedRequests = ref<SavedRequest[]>([]);
  const currentRequest = ref<ApiRequest>({
    method: "GET",
    url: "",
    headers: [],
    body: null,
    bodyType: "json",
    auth: { authType: "none", username: null, password: null, token: null, apiKey: null, apiKeyName: null, apiKeyLocation: null },
  });
  const response = ref<ApiResponse | null>(null);
  const sending = ref(false);
  const requestName = ref("");
  const error = ref("");

  // ── WebSocket State ──
  const wsUrl = ref("");
  const wsConnected = ref(false);
  const wsMessages = ref<WsMessage[]>([]);
  const wsSending = ref(false);
  const activeTab = ref<"http" | "websocket">("http");
  let unlistenWsMessage: UnlistenFn | null = null;
  let unlistenWsConnected: UnlistenFn | null = null;
  let unlistenWsDisconnected: UnlistenFn | null = null;

  // ── Environment State ──
  const environments = ref<Environment[]>([]);
  const activeEnvId = ref<string | null>(null);

  // ── Collection State ──
  const collections = ref<RequestCollection[]>([]);
  const showCollectionPanel = ref(false);
  const showCodeGen = ref(false);
  const generatedCode = ref("");

  // ── HTTP Actions ──
  async function send() {
    if (!currentRequest.value.url.trim()) { error.value = "请输入 URL"; return; }
    sending.value = true; error.value = ""; response.value = null;
    try { response.value = await sendApiRequest(currentRequest.value); }
    catch (e) { error.value = String(e); }
    finally { sending.value = false; }
  }

  async function loadSaved() {
    try { savedRequests.value = await listApiRequests(); } catch {}
  }

  async function saveCurrent() {
    if (!requestName.value.trim()) { error.value = "请输入请求名称"; return; }
    if (!currentRequest.value.url.trim()) { error.value = "请输入 URL"; return; }
    try {
      await saveApiRequest(null, requestName.value.trim(), currentRequest.value);
      requestName.value = ""; await loadSaved();
    } catch (e) { error.value = String(e); }
  }

  async function deleteSaved(id: string) {
    try { await deleteApiRequest(id); await loadSaved(); }
    catch (e) { error.value = String(e); }
  }

  function loadRequest(item: SavedRequest) {
    currentRequest.value = { ...item.request };
    requestName.value = item.name; response.value = null; error.value = "";
  }

  function newRequest() {
    currentRequest.value = {
      method: "GET", url: "", headers: [], body: null, bodyType: "json",
      auth: { authType: "none", username: null, password: null, token: null, apiKey: null, apiKeyName: null, apiKeyLocation: null },
    };
    requestName.value = ""; response.value = null; error.value = "";
  }

  // ── WebSocket Actions ──
  async function wsConnectAction() {
    if (!wsUrl.value.trim()) { error.value = "请输入 WebSocket URL"; return; }
    wsSending.value = true; error.value = "";
    try {
      await wsConnect(wsUrl.value);
      wsConnected.value = true;
      // Register listeners
      unlistenWsMessage = await onWsMessage((msg) => { wsMessages.value.push(msg); });
      unlistenWsConnected = await onWsConnected(() => { wsConnected.value = true; });
      unlistenWsDisconnected = await onWsDisconnected(() => { wsConnected.value = false; });
    } catch (e) { error.value = String(e); }
    finally { wsSending.value = false; }
  }

  async function wsSendAction(msg: string) {
    if (!msg.trim()) return;
    try {
      await wsSend(msg);
      wsMessages.value.push({
        id: crypto.randomUUID(), direction: "sent", content: msg, timestamp: new Date().toISOString(),
      });
    } catch (e) { error.value = String(e); }
  }

  async function wsDisconnect() {
    try { await wsClose(); } catch {}
    wsConnected.value = false;
    unlistenWsMessage?.(); unlistenWsConnected?.(); unlistenWsDisconnected?.();
  }

  async function wsLoadMessages() {
    try { wsMessages.value = await wsGetMessages(); } catch {}
  }

  async function wsClear() {
    try { await wsClearMessages(); wsMessages.value = []; } catch {}
  }

  // ── Environment Actions ──
  async function loadEnvironments() {
    try {
      environments.value = await envList();
      if (!activeEnvId.value && environments.value.length > 0) {
        activeEnvId.value = environments.value[0].id;
      }
    } catch {}
  }

  async function saveEnvironment(env: Environment) {
    const saved = await envSave(env);
    await loadEnvironments();
    return saved;
  }

  async function deleteEnvironment(id: string) {
    await envDelete(id);
    await loadEnvironments();
  }

  // ── Collection Actions ──
  async function loadCollections() {
    try { collections.value = await collectionList(); } catch {}
  }

  async function saveCollection(name: string, id: string | null, requests: any[]) {
    const result = await collectionSave(name, id, requests);
    await loadCollections();
    return result;
  }

  async function deleteCollection(id: string) {
    await collectionDelete(id);
    await loadCollections();
  }

  // ── Code Generation ──
  async function generateCode(lang: string) {
    const req = currentRequest.value;
    try {
      generatedCode.value = await generateHttpCode(req.method, req.url, req.headers, req.body, req.bodyType, lang);
      showCodeGen.value = true;
    } catch (e) { error.value = String(e); }
  }

  return {
    // HTTP
    savedRequests, currentRequest, response, sending, requestName, error,
    send, loadSaved, saveCurrent, deleteSaved, loadRequest, newRequest,
    // WebSocket
    wsUrl, wsConnected, wsMessages, wsSending, activeTab,
    wsConnectAction, wsSendAction, wsDisconnect, wsLoadMessages, wsClear,
    // Environment
    environments, activeEnvId, loadEnvironments, saveEnvironment, deleteEnvironment,
    // Collections
    collections, showCollectionPanel, loadCollections, saveCollection, deleteCollection,
    // Code Gen
    showCodeGen, generatedCode, generateCode,
  };
});
```

- [ ] **Step 3: Commit**

```bash
git add src/lib/tauri.ts src/stores/apiTest.ts
git commit -m "feat(api-test): add frontend WebSocket, env, collection, codegen support"
```

---

### Task 8: Enhance API test Vue page

**Files:**
- Modify: `src/pages/api-test/Page.vue`

- [ ] **Step 1: Add tab switching between HTTP and WebSocket**

Replace the top bar section with a tabbed layout:

```vue
<!-- Top bar with tabs -->
<div class="flex items-center border-b border-paper-deep/40 bg-paper-warm/20 px-5 py-2">
  <div class="flex gap-1">
    <button
      v-for="tab in [{k:'http',l:'HTTP 请求'},{k:'websocket',l:'WebSocket'}]" :key="tab.k"
      class="rounded-lg px-4 py-1.5 text-xs font-medium transition-colors"
      :class="store.activeTab === tab.k
        ? 'bg-bamboo/15 text-bamboo'
        : 'text-ink-faint hover:text-ink hover:bg-paper-deep/30'"
      @click="store.activeTab = tab.k"
    >{{ tab.l }}</button>
  </div>

  <!-- HTTP tab toolbar -->
  <template v-if="store.activeTab === 'http'">
    <div class="ml-4 flex items-center gap-2">
      <input v-model="store.requestName" type="text" placeholder="请求名称"
        class="h-7 w-40 rounded border border-paper-deep/30 bg-paper-warm/50 px-2 text-xs text-ink outline-none focus:border-bamboo/40" />
      <button class="rounded px-2 py-1 text-xs text-ink-soft hover:bg-paper-deep/30" @click="store.saveCurrent()">保存</button>
    </div>

    <div class="ml-auto flex items-center gap-2">
      <!-- Environment selector -->
      <select v-model="store.activeEnvId" @change="store.loadEnvironments()"
        class="rounded border border-paper-deep/30 bg-paper-deep/20 px-2 py-1 text-xs text-ink outline-none">
        <option v-for="env in store.environments" :key="env.id" :value="env.id">{{ env.name }}</option>
      </select>
      <!-- Collection toggle -->
      <button class="rounded px-2 py-1 text-xs text-ink-faint hover:text-ink"
        @click="store.showCollectionPanel = !store.showCollectionPanel">
        集合 ({{ store.collections.length }})
      </button>
      <!-- Code gen -->
      <button class="rounded px-2 py-1 text-xs text-ink-faint hover:text-ink"
        @click="store.generateCode('curl')">
        生成代码
      </button>
      <button class="rounded px-2 py-1 text-xs text-ink-faint hover:text-ink" @click="store.newRequest()">新建</button>
    </div>
  </template>
</div>
```

- [ ] **Step 2: Add WebSocket tab content**

After the main content two-panel layout, add the WebSocket tab (shown when `store.activeTab === 'websocket'`):

```vue
<!-- WebSocket Tab -->
<div v-if="store.activeTab === 'websocket'" class="flex flex-1 flex-col overflow-hidden">
  <!-- Connection bar -->
  <div class="flex items-center gap-2 border-b border-paper-deep/30 px-5 py-3">
    <div class="flex items-center gap-2">
      <span class="inline-block h-2 w-2 rounded-full"
        :class="store.wsConnected ? 'bg-green-500' : 'bg-red-500'" />
      <span class="text-xs text-ink-faint">{{ store.wsConnected ? '已连接' : '未连接' }}</span>
    </div>
    <input v-model="store.wsUrl" type="text" placeholder="ws://192.168.1.100:8080/ws"
      class="flex-1 rounded-lg border border-paper-deep/40 bg-paper-deep/20 px-3 py-1.5 text-sm font-mono text-ink outline-none focus:border-bamboo/40"
      :disabled="store.wsConnected" />
    <button v-if="!store.wsConnected"
      class="rounded-lg bg-bamboo px-4 py-1.5 text-xs font-medium text-white hover:bg-bamboo/90"
      :disabled="store.wsSending" @click="store.wsConnectAction()">
      {{ store.wsSending ? '连接中...' : '连接' }}
    </button>
    <button v-else
      class="rounded-lg bg-red-500 px-4 py-1.5 text-xs font-medium text-white hover:bg-red-600"
      @click="store.wsDisconnect()">断开</button>
    <button class="rounded px-2 py-1 text-xs text-ink-faint hover:text-ink"
      @click="store.wsClear()">清除</button>
  </div>

  <!-- Messages -->
  <div class="flex-1 overflow-y-auto p-4 space-y-2">
    <div v-for="msg in store.wsMessages" :key="msg.id"
      class="flex" :class="msg.direction === 'sent' ? 'justify-end' : 'justify-start'">
      <div class="max-w-[70%] rounded-xl px-4 py-2 text-sm"
        :class="msg.direction === 'sent'
          ? 'bg-bamboo/15 text-ink rounded-br-md'
          : 'bg-paper-deep/40 text-ink rounded-bl-md'">
        <pre class="whitespace-pre-wrap font-mono text-xs">{{ msg.content }}</pre>
        <div class="mt-1 text-[10px] text-ink-faint">
          {{ new Date(msg.timestamp).toLocaleTimeString() }}
        </div>
      </div>
    </div>
    <div v-if="store.wsMessages.length === 0" class="py-12 text-center text-sm text-ink-faint">
      暂无消息，连接 WebSocket 后开始通信
    </div>
  </div>

  <!-- Message input -->
  <div class="border-t border-paper-deep/30 px-5 py-3">
    <div class="flex gap-2">
      <input ref="wsInputRef" type="text" placeholder="输入消息..."
        class="flex-1 rounded-lg border border-paper-deep/40 bg-paper-deep/20 px-3 py-2 text-sm text-ink outline-none focus:border-bamboo/40 font-mono"
        @keyup.enter="sendWsMessage" />
      <button class="rounded-lg bg-bamboo px-4 py-2 text-xs font-medium text-white hover:bg-bamboo/90"
        :disabled="!store.wsConnected" @click="sendWsMessage">
        发送
      </button>
    </div>
  </div>
</div>
```

Add the `sendWsMessage` function in `<script setup>`:
```typescript
const wsInput = ref("");
async function sendWsMessage() {
  if (!wsInput.value.trim()) return;
  await store.wsSendAction(wsInput.value);
  wsInput.value = "";
}
```

- [ ] **Step 3: Add auth panel to HTTP request section**

In the left panel, above the headers section, add:
```vue
<!-- Auth section -->
<div class="mt-4">
  <h3 class="mb-2 text-xs font-semibold uppercase tracking-wider text-ink-soft">认证</h3>
  <select v-model="store.currentRequest.auth!.authType"
    class="mb-2 w-full rounded-lg border border-paper-deep/30 bg-paper-deep/15 px-2.5 py-1.5 text-xs text-ink outline-none">
    <option value="none">无认证</option>
    <option value="basic">Basic Auth</option>
    <option value="bearer">Bearer Token</option>
    <option value="apiKey">API Key</option>
  </select>
  <template v-if="store.currentRequest.auth?.authType === 'basic'">
    <input v-model="store.currentRequest.auth!.username" placeholder="用户名"
      class="mb-1 w-full rounded border border-paper-deep/30 bg-paper-deep/15 px-2 py-1 text-xs text-ink outline-none" />
    <input v-model="store.currentRequest.auth!.password" type="password" placeholder="密码"
      class="w-full rounded border border-paper-deep/30 bg-paper-deep/15 px-2 py-1 text-xs text-ink outline-none" />
  </template>
  <template v-if="store.currentRequest.auth?.authType === 'bearer'">
    <input v-model="store.currentRequest.auth!.token" placeholder="输入 Token"
      class="w-full rounded border border-paper-deep/30 bg-paper-deep/15 px-2 py-1 text-xs text-ink outline-none" />
  </template>
  <template v-if="store.currentRequest.auth?.authType === 'apiKey'">
    <input v-model="store.currentRequest.auth!.apiKeyName" placeholder="Key 名称"
      class="mb-1 w-full rounded border border-paper-deep/30 bg-paper-deep/15 px-2 py-1 text-xs text-ink outline-none" />
    <input v-model="store.currentRequest.auth!.apiKey" placeholder="Key 值"
      class="mb-1 w-full rounded border border-paper-deep/30 bg-paper-deep/15 px-2 py-1 text-xs text-ink outline-none" />
    <select v-model="store.currentRequest.auth!.apiKeyLocation"
      class="w-full rounded border border-paper-deep/30 bg-paper-deep/15 px-2 py-1 text-xs text-ink outline-none">
      <option value="header">Header</option>
      <option value="query">Query</option>
    </select>
  </template>
</div>
```

- [ ] **Step 4: Add code generation dialog**

Add to template (inside the main content area, as an overlay):
```vue
<!-- Code Generation Dialog -->
<div v-if="store.showCodeGen" class="fixed inset-0 z-50 flex items-center justify-center bg-black/20 backdrop-blur-sm"
  @click.self="store.showCodeGen = false">
  <div class="w-[600px] rounded-xl border border-paper-deep/60 bg-paper shadow-xl">
    <div class="flex items-center justify-between border-b border-paper-deep/30 px-5 py-3">
      <h3 class="text-sm font-semibold text-ink">生成代码</h3>
      <div class="flex gap-1">
        <button v-for="lang in ['curl','javascript','python']" :key="lang"
          class="rounded px-3 py-1 text-xs font-medium transition-colors"
          :class="selectedLang === lang ? 'bg-bamboo/15 text-bamboo' : 'text-ink-faint hover:text-ink'"
          @click="selectCodeLang(lang)">{{ lang }}</button>
        <button class="ml-2 rounded p-1 text-ink-faint hover:text-ink" @click="store.showCodeGen = false">
          ✕
        </button>
      </div>
    </div>
    <div class="p-5">
      <pre class="max-h-96 overflow-y-auto rounded-lg bg-slate-900 p-4 text-xs text-green-400 font-mono whitespace-pre-wrap">{{ store.generatedCode }}</pre>
      <button class="mt-3 rounded-lg bg-bamboo px-4 py-1.5 text-xs font-medium text-white"
        @click="copyCode">复制</button>
    </div>
  </div>
</div>
```

Add script functions:
```typescript
const selectedLang = ref("curl");
async function selectCodeLang(lang: string) {
  selectedLang.value = lang;
  await store.generateCode(lang);
}
function copyCode() {
  navigator.clipboard.writeText(store.generatedCode);
}
```

- [ ] **Step 5: Update onMounted to load envs and collections**

```typescript
onMounted(() => {
  store.loadSaved();
  store.loadEnvironments();
  store.loadCollections();
});
```

- [ ] **Step 6: Commit**

```bash
git add src/pages/api-test/Page.vue
git commit -m "feat(api-test): add WebSocket tab, auth panel, env selector, code gen dialog"
```

---

### Task 9: Build verification

**Files:** None (verification only)

- [ ] **Step 1: Run cargo check**

```bash
cd src-tauri && cargo check 2>&1
```

Expected: Build succeeds. Fix any compile errors:
- `tokio-tungstenite` requires `futures-util` for `StreamExt`/`SinkExt` — if needed, add `futures-util = "0.3"` or use `futures::StreamExt`/`futures::SinkExt` since `futures = "0.3"` is already in deps.
- The `base64::Engine` trait import requires `use base64::engine::general_purpose;`

- [ ] **Step 2: Verify frontend build**

```bash
npm run build 2>&1
```

Expected: Build succeeds.

- [ ] **Step 3: Commit any fixes**

```bash
git add -A
git commit -m "fix(api-test): fix build issues"
```

---

### Spec Coverage

| Spec Requirement | Task |
|---|---|
| WebSocket client (connect/send/close/events) | Tasks 2, 5, 7, 8 |
| Environment variables (CRUD + {{var}} substitution) | Tasks 3, 5, 7, 8 |
| Request collections (nested tree, CRUD) | Tasks 5, 7, 8 |
| Code generation (curl/JS/Python) | Tasks 4, 5, 7, 8 |
| Authentication helpers (Basic/Bearer/API Key) | Tasks 1 (AuthConfig), 5, 7, 8 |
| Frontend tab switching (HTTP/WebSocket) | Task 8 |
| Build verification | Task 9 |
