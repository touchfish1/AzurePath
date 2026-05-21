# WebSocket/API 测试工具增强设计

## 概述

在 AzurePath 现有的 API 测试工具基础上，增加 WebSocket 客户端支持、环境变量系统、请求集合管理、代码生成和认证辅助功能，打造内网 API 调试全功能工具。

## 现有基础

- HTTP API 测试（GET/POST/PUT/DELETE/PATCH）
- 自定义请求头和请求体（JSON/Form/Text）
- 响应展示（状态码、耗时、大小、JSON 自动格式化）
- 请求保存/加载到本地 JSON 文件
- 使用 `reqwest` 作为 HTTP 客户端

## 技术栈

- `tokio-tungstenite` — WebSocket 客户端（需新增依赖）
- `reqwest`（已存在）— HTTP 请求
- `serde_json`（已存在）— JSON 处理和代码生成

## 全功能架构

### 数据模型 (`types/api_test.rs`)

现有 `ApiRequest`、`ApiResponse`、`SavedRequest` 保持不变，新增：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsMessage {
    pub id: String,
    pub direction: String,   // "sent" | "received"
    pub content: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    pub id: String,
    pub name: String,           // "开发环境", "测试环境"
    pub variables: Vec<Vec<String>>, // [["base_url", "http://localhost:8080"], ...]
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
    pub request: ApiRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthConfig {
    pub auth_type: String,      // "none", "basic", "bearer", "apiKey"
    pub username: Option<String>,
    pub password: Option<String>,
    pub token: Option<String>,
    pub api_key: Option<String>,
    pub api_key_name: Option<String>,
    pub api_key_location: Option<String>, // "header" | "query"
}
```

### 核心模块 (`core/api_test/`)

| 文件 | 职责 |
|------|------|
| `ws.rs` | WebSocket 会话管理，连接/断开/收发消息 |
| `env.rs` | 环境变量持久化 + `{{var}}` 替换引擎 |
| `codegen.rs` | 生成 curl / JavaScript / Python 代码 |

### WebSocket 客户端 (`core/api_test/ws.rs`)

```rust
pub struct WsSession {
    pub url: String,
    pub connected: bool,
    // tokio::spawn 后台任务读取 WebSocket 流
    // 通过 tokio::sync::mpsc channel 收发消息
}

impl WsSession {
    pub async fn connect(url: &str) -> Result<Self, String>;
    pub async fn send(text: &str) -> Result<(), String>;
    pub async fn close() -> Result<(), String>;
    // 接收的消息通过 Tauri event 推送到前端
}
```

### 环境变量引擎 (`core/api_test/env.rs`)

```rust
pub struct EnvironmentManager {
    // 从 JSON 文件加载/保存环境
}

impl EnvironmentManager {
    pub fn list() -> Vec<Environment>;
    pub fn save(env: Environment) -> Result<(), String>;
    pub fn delete(id: &str) -> Result<(), String>;
    /// 将 input 中的 {{var_name}} 替换为环境变量值
    pub fn substitute(input: &str, variables: &[(String, String)]) -> String;
}
```

### 代码生成 (`core/api_test/codegen.rs`)

```rust
pub fn generate_curl(request: &ApiRequest) -> String;
pub fn generate_javascript(request: &ApiRequest) -> String;
pub fn generate_python(request: &ApiRequest) -> String;
```

### 命令层 (`commands/api_test.rs`)

新增命令（保留现有 `send_api_request` / `list_api_requests` / `save_api_request` / `delete_api_request`）：

```rust
// WebSocket
ws_connect(url: String) -> Result<(), String>
ws_send(message: String) -> Result<(), String>
ws_close() -> Result<(), String>
ws_get_messages() -> Result<Vec<WsMessage>, String>
ws_clear_messages() -> Result<(), String>

// 环境变量
env_list() -> Result<Vec<Environment>, String>
env_save(environment: Environment) -> Result<(), String>
env_delete(id: String) -> Result<(), String>

// 集合管理
collection_list() -> Result<Vec<RequestCollection>, String>
collection_save(name: String, requests: Vec<CollectionItem>) -> Result<String, String>
collection_delete(id: String) -> Result<(), String>

// 代码生成
generate_http_code(request: ApiRequest, lang: String) -> Result<String, String>
```

### 事件

```
ws:message   — { id, direction, content, timestamp }  新消息
ws:connected  — { url }                                连接成功
ws:disconnected — { code, reason }                      连接断开
ws:error     — { error }                                错误
```

### 前端增强 (`src/pages/api-test/Page.vue`)

在现有页面基础上增加：

**标签切换栏**（页面顶部）：
- "HTTP 请求"（现有功能）
- "WebSocket"（新增）

**HTTP 标签页增强：**
- 认证面板：在请求头上方新增认证类型选择器 + 参数字段
- 环境选择器：URL 输入栏旁新增环境下拉
- 请求集合：左侧新增集合树面板，可切换显示/隐藏
- 代码生成：响应区右上角新增"生成代码"按钮

**WebSocket 标签页：**
- 连接栏：URL 输入 + 连接/断开按钮
- 连接状态指示器
- 消息列表：双向消息流，彩色区分收发
- 消息输入：文本输入框 + 发送按钮
- 清除消息按钮

### 数据流

```
用户输入请求
   ↓
环境变量替换 {{var}} → 注入 Auth Header
   ↓
HTTP: reqwest.send()      WebSocket: tokio-tungstenite
   ↓                            ↓
响应展示                    消息实时推送 (ws:message event)
   ↓
保存/集合/代码生成
```

### 依赖

- 新增 `tokio-tungstenite = { version = "0.21", features = ["native-tls"] }` 到 Cargo.toml
- 其余依赖项目已存在
