# 远程桌面 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Phase 1 — VNC 远程桌面基础功能：连接管理、桌面帧渲染、键盘鼠标输入

**Architecture:** 三层 Rust 结构（types/core/commands）+ Vue 3 前端 Canvas 渲染。Rust 后端使用 `vnc-rs` crate 实现 VNC 客户端，帧编码后通过 Tauri Events 推送到前端。

**Tech Stack:** vnc-rs, rfb-encodings, Tauri 2.0, Vue 3, HTML5 Canvas

---

## File Structure

### Rust 后端 — 新增文件

| 文件 | 职责 |
|------|------|
| `src-tauri/src/types/remote_desktop/mod.rs` | 模块导出 |
| `src-tauri/src/types/remote_desktop/session.rs` | Protocol 枚举、DesktopSession、SessionInput、SessionSummary |
| `src-tauri/src/types/remote_desktop/frame.rs` | DesktopFrame（x, y, width, height, data, encoding） |
| `src-tauri/src/types/remote_desktop/input.rs` | MouseEvent, KeyEvent, Modifiers |
| `src-tauri/src/types/remote_desktop/clipboard.rs` | ClipboardData（Phase 1 预留） |
| `src-tauri/src/core/remote_desktop/mod.rs` | 模块导出 |
| `src-tauri/src/core/remote_desktop/session_store.rs` | SQLite 持久化（复用 remote_shell 的 SessionStore 模式） |
| `src-tauri/src/core/remote_desktop/desktop_client.rs` | DesktopClient trait（统一 VNC/RDP 接口） |
| `src-tauri/src/core/remote_desktop/vnc.rs` | VNC 客户端实现 |
| `src-tauri/src/core/remote_desktop/rdp.rs` | RDP 占位实现（Phase 2） |
| `src-tauri/src/core/remote_desktop/frame_encoder.rs` | 帧差异检测 + JPEG 编码 |
| `src-tauri/src/commands/remote_desktop.rs` | Tauri 命令层 |

### Rust 后端 — 修改文件

| 文件 | 修改内容 |
|------|----------|
| `src-tauri/src/types/mod.rs` | 添加 `pub mod remote_desktop;` |
| `src-tauri/src/core/mod.rs` | 添加 `pub mod remote_desktop;` |
| `src-tauri/src/commands/mod.rs` | 添加 `pub mod remote_desktop;` |
| `src-tauri/src/lib.rs` | 注册命令 + setup 初始化 |
| `src-tauri/Cargo.toml` | 添加依赖 vnc-rs, rfb-encodings, base64, image |

### 前端 — 新增文件

| 文件 | 职责 |
|------|------|
| `src/pages/remote-desktop/Page.vue` | 主页面（侧边栏 + Canvas + 工具栏） |
| `src/stores/remoteDesktop.ts` | Pinia store |
| `src/components/remote-desktop/DesktopCanvas.vue` | Canvas 渲染组件 |
| `src/components/remote-desktop/SessionList.vue` | 会话列表侧边栏 |
| `src/components/remote-desktop/SessionDialog.vue` | 创建/编辑会话弹窗 |
| `src/components/remote-desktop/Toolbar.vue` | 缩放/全屏/剪贴板工具栏 |

### 前端 — 修改文件

| 文件 | 修改内容 |
|------|----------|
| `src/lib/tauri.ts` | 添加所有 remote-desktop invoke 包装函数和事件监听 |
| `src/router/index.ts` | 添加 `/remote-desktop` 路由 |
| `src/components/layout/Sidebar.vue` | 添加远程桌面导航项 |

---

### Task 1: Rust 类型定义

**Files:**
- Create: `src-tauri/src/types/remote_desktop/mod.rs`
- Create: `src-tauri/src/types/remote_desktop/session.rs`
- Create: `src-tauri/src/types/remote_desktop/frame.rs`
- Create: `src-tauri/src/types/remote_desktop/input.rs`
- Create: `src-tauri/src/types/remote_desktop/clipboard.rs`
- Modify: `src-tauri/src/types/mod.rs`

**Key types:**

```rust
// session.rs
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum Protocol { Rdp, Vnc }

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DesktopSession {
    pub id: String,           // UUID
    pub name: String,
    pub protocol: Protocol,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub quality: u8,          // 1-100 JPEG quality
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SessionInput {
    pub name: String,
    pub protocol: Protocol,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub quality: Option<u8>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub id: String,
    pub name: String,
    pub protocol: Protocol,
    pub host: String,
    pub port: u16,
    pub is_connected: bool,
}

// frame.rs
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DesktopFrame {
    pub session_id: String,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub data: Vec<u8>,        // JPEG bytes, base64 encoded for JSON
    pub encoding: String,     // "jpeg"
}

// input.rs
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MouseEvent {
    pub x: u16,
    pub y: u16,
    pub button: u8,           // 0=left, 1=middle, 2=right
    pub pressed: bool,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct KeyEvent {
    pub key_code: u32,        // X11 keysym
    pub pressed: bool,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}
```

---

### Task 2: Rust Core — DesktopClient Trait + FrameEncoder

**Files:**
- Create: `src-tauri/src/core/remote_desktop/mod.rs`
- Create: `src-tauri/src/core/remote_desktop/desktop_client.rs`
- Create: `src-tauri/src/core/remote_desktop/frame_encoder.rs`
- Modify: `src-tauri/src/core/mod.rs`

**DesktopClient trait:**

```rust
#[async_trait]
pub trait DesktopClient: Send {
    async fn connect(&mut self, session: &DesktopSession, password: &str) -> Result<(), String>;
    async fn poll_frame(&mut self) -> Result<Option<DesktopFrame>, String>;
    async fn send_key_event(&mut self, event: KeyEvent) -> Result<(), String>;
    async fn send_mouse_event(&mut self, event: MouseEvent) -> Result<(), String>;
    async fn resize(&mut self, width: u16, height: u16) -> Result<(), String>;
    async fn disconnect(&mut self) -> Result<(), String>;
    fn framebuffer_width(&self) -> u16;
    fn framebuffer_height(&self) -> u16;
}
```

**FrameEncoder:**

```rust
pub struct FrameEncoder {
    prev_frame: Option<Vec<u8>>,
    width: u16,
    height: u16,
    quality: u8,
}

impl FrameEncoder {
    pub fn new(width: u16, height: u16, quality: u8) -> Self;
    
    /// Compare with previous frame, encode changed regions as JPEG.
    /// Returns list of DesktopFrame (one per changed region).
    /// On first call, returns full frame as single region.
    pub fn encode_frame(&mut self, raw_rgb: &[u8]) -> Vec<DesktopFrame>;
    
    /// Split frame into NxN grid tiles for incremental diff
    fn split_into_tiles(&self, data: &[u8], tile_size: u16) -> Vec<(u16, u16, Vec<u8>)>;
}
```

---

### Task 3: Rust Core — VNC Client

**Files:**
- Create: `src-tauri/src/core/remote_desktop/vnc.rs`

```rust
pub struct VncClient {
    session_id: String,
    framebuffer: Option<Vec<u8>>,
    width: u16,
    height: u16,
    encoder: Option<FrameEncoder>,
    // vnc-rs connection handle
    vnc_connection: Option<vnc_rs::VncConnection>,
}

impl VncClient {
    pub fn new(session_id: String) -> Self;
}

#[async_trait]
impl DesktopClient for VncClient {
    async fn connect(&mut self, session: &DesktopSession, password: &str) -> Result<(), String>;
    async fn poll_frame(&mut self) -> Result<Option<DesktopFrame>, String>;
    async fn send_key_event(&mut self, event: KeyEvent) -> Result<(), String>;
    async fn send_mouse_event(&mut self, event: MouseEvent) -> Result<(), String>;
    async fn resize(&mut self, width: u16, height: u16) -> Result<(), String>;
    async fn disconnect(&mut self) -> Result<(), String>;
    fn framebuffer_width(&self) -> u16;
    fn framebuffer_height(&self) -> u16;
}
```

vnc-rs API usage: The `vnc-rs` crate provides async VNC client. Key API:
- `VncConnection::connect(addr, password).await` — connect to VNC server
- `connection.read_event().await` — read RFB events (including framebuffer updates)
- `connection.send_key_event(keysym, pressed).await` — send keyboard event
- `connection.send_pointer_event(x, y, button_mask).await` — send mouse event
- Returns framebuffer data as raw RGB888 pixels in framebuffer update events

---

### Task 4: Rust Core — Session Store (SQLite persistence)

**Files:**
- Create: `src-tauri/src/core/remote_desktop/session_store.rs`

```rust
pub struct DesktopSessionStore {
    conn: Mutex<Connection>,
}

impl DesktopSessionStore {
    pub fn new() -> Result<Self, String>;
    fn init_tables(&self) -> Result<(), String>;
    pub fn list_sessions(&self) -> Result<Vec<DesktopSession>, String>;
    pub fn get_session(&self, id: &str) -> Result<DesktopSession, String>;
    pub fn create_session(&self, input: &SessionInput) -> Result<DesktopSession, String>;
    pub fn update_session(&self, id: &str, input: &SessionInput) -> Result<DesktopSession, String>;
    pub fn delete_session(&self, id: &str) -> Result<(), String>;
    pub fn set_password(&self, session_id: &str, password: &str) -> Result<(), String>;
    pub fn get_password(&self, session_id: &str) -> Result<Option<String>, String>;
}
```

Tables:
```sql
CREATE TABLE IF NOT EXISTS desktop_sessions (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    protocol TEXT NOT NULL,  -- 'rdp' | 'vnc'
    host TEXT NOT NULL,
    port INTEGER NOT NULL,
    username TEXT NOT NULL DEFAULT '',
    quality INTEGER DEFAULT 80,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS desktop_session_secrets (
    session_id TEXT PRIMARY KEY,
    password TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES desktop_sessions(id)
);
```

---

### Task 5: Rust Commands + Registration

**Files:**
- Create: `src-tauri/src/commands/remote_desktop.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/Cargo.toml`

```rust
// commands/remote_desktop.rs
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::AppHandle;
use crate::core::remote_desktop::desktop_client::DesktopClient;
use crate::core::remote_desktop::session_store::DesktopSessionStore;
use crate::types::remote_desktop::session::*;
use crate::types::remote_desktop::input::*;

static STORE: OnceLock<Arc<DesktopSessionStore>> = OnceLock::new();
static ACTIVE_CONNECTIONS: OnceLock<Arc<Mutex<HashMap<String, Box<dyn DesktopClient + Send>>>>> = OnceLock::new();
static CANCEL_TOKENS: OnceLock<Arc<Mutex<HashMap<String, bool>>>> = OnceLock::new();

pub async fn remote_desktop_init() -> Result<(), String> { ... }

// Session CRUD
#[tauri::command] async fn rd_list_sessions() -> Result<Vec<DesktopSession>, String>;
#[tauri::command] async fn rd_create_session(state, input: SessionInput, password: String) -> Result<DesktopSession, String>;
#[tauri::command] async fn rd_update_session(state, id: String, input: SessionInput) -> Result<DesktopSession, String>;
#[tauri::command] async fn rd_delete_session(state, id: String) -> Result<(), String>;

// Connection
#[tauri::command] async fn rd_connect(app: AppHandle, state, session_id: String, password: String) -> Result<(), String>;
#[tauri::command] async fn rd_disconnect(state, session_id: String) -> Result<(), String>;
#[tauri::command] async fn rd_resize(state, session_id: String, width: u16, height: u16) -> Result<(), String>;

// Input
#[tauri::command] async fn rd_send_key(state, session_id: String, event: KeyEvent) -> Result<(), String>;
#[tauri::command] async fn rd_send_mouse(state, session_id: String, event: MouseEvent) -> Result<(), String>;

// Frame polling loop spawned on connect
fn start_frame_polling(app: AppHandle, session_id: String, client: Arc<Mutex<Box<dyn DesktopClient + Send>>>);
```

**lib.rs changes:**
```rust
// In setup():
if let Err(e) = tauri::async_runtime::block_on(commands::remote_desktop::remote_desktop_init()) {
    eprintln!("[azurepath] remote_desktop init warning: {e}");
}

// In generate_handler![]:
commands::remote_desktop::rd_list_sessions,
commands::remote_desktop::rd_create_session,
// ... all rd_* commands
```

**Cargo.toml additions:**
```toml
vnc-rs = "0.5"
rfb-encodings = "0.1"
base64 = "0.22"
image = "0.25"
```

---

### Task 6: Frontend — tauri.ts Wrappers

**Files:**
- Modify: `src/lib/tauri.ts`

Add TypeScript interfaces and invoke wrappers:

```typescript
// ── Types ──
export interface DesktopSession {
  id: string;
  name: string;
  protocol: 'rdp' | 'vnc';
  host: string;
  port: number;
  username: string;
  quality: number;
  createdAt: string;
  updatedAt: string;
}

export interface SessionInput {
  name: string;
  protocol: 'rdp' | 'vnc';
  host: string;
  port: number;
  username: string;
  quality?: number;
}

export interface DesktopFrame {
  sessionId: string;
  x: number;
  y: number;
  width: number;
  height: number;
  data: number[];  // base64 decoded JPEG bytes
  encoding: string;
}

export interface MouseEvent {
  x: number;
  y: number;
  button: number;
  pressed: boolean;
}

export interface KeyEvent {
  keyCode: number;
  pressed: boolean;
}

export interface Modifiers {
  ctrl: boolean;
  alt: boolean;
  shift: boolean;
  meta: boolean;
}

// ── Invoke wrappers ──
export const rdListSessions = () => invoke<DesktopSession[]>('rd_list_sessions');
export const rdCreateSession = (input: SessionInput, password: string) => 
  invoke<DesktopSession>('rd_create_session', { input, password });
export const rdUpdateSession = (id: string, input: SessionInput) =>
  invoke<DesktopSession>('rd_update_session', { id, input });
export const rdDeleteSession = (id: string) =>
  invoke<void>('rd_delete_session', { id });
export const rdConnect = (sessionId: string, password: string) =>
  invoke<void>('rd_connect', { sessionId, password });
export const rdDisconnect = (sessionId: string) =>
  invoke<void>('rd_disconnect', { sessionId });
export const rdResize = (sessionId: string, width: number, height: number) =>
  invoke<void>('rd_resize', { sessionId, width, height });
export const rdSendKey = (sessionId: string, event: KeyEvent) =>
  invoke<void>('rd_send_key', { sessionId, event });
export const rdSendMouse = (sessionId: string, event: MouseEvent) =>
  invoke<void>('rd_send_mouse', { sessionId, event });

// ── Event listener ──
export const onRdFrame = (cb: (frame: DesktopFrame) => void) =>
  listen<DesktopFrame>('rd:frame', (event) => cb(event.payload));
```

---

### Task 7: Frontend — Components (Canvas + SessionList + Toolbar + Dialog)

**Files:**
- Create: `src/components/remote-desktop/DesktopCanvas.vue`
- Create: `src/components/remote-desktop/SessionList.vue`
- Create: `src/components/remote-desktop/Toolbar.vue`
- Create: `src/components/remote-desktop/SessionDialog.vue`

**DesktopCanvas.vue** — Core rendering component:
```vue
<script setup lang="ts">
// Props: sessionId, width, height
// Emits: mouse-event, key-event
// On mounted: listen to 'rd:frame' events, draw to canvas
// On unmounted: remove listener
// Methods: clear(), resize(width, height)
// 
// Rendering logic:
// - On frame received: decode base64 JPEG → ImageBitmap → drawImage at (x, y, w, h)
// - CSS keeps canvas at 100% fill, internal resolution matches remote desktop
</script>
```

**SessionList.vue** — Lists saved sessions with connect/disconnect/delete actions.

**SessionDialog.vue** — Modal for creating/editing sessions (name, host, port, protocol, username, password, quality).

**Toolbar.vue** — Zoom percentage selector, fullscreen toggle, clipboard button.

---

### Task 8: Frontend — Pinia Store + Page + Router + Sidebar

**Files:**
- Create: `src/stores/remoteDesktop.ts`
- Create: `src/pages/remote-desktop/Page.vue`
- Modify: `src/router/index.ts`
- Modify: `src/components/layout/Sidebar.vue`

**Store** (`remoteDesktop.ts`):
```typescript
export const useRemoteDesktopStore = defineStore('remoteDesktop', () => {
  const sessions = ref<DesktopSession[]>([]);
  const activeConnections = ref<Record<string, { protocol: string; status: string; width: number; height: number }>>({});
  const isLoading = ref(false);
  const error = ref<string | null>(null);
  
  // Actions
  async function init() { ... }
  async function loadSessions() { ... }
  async function createSession(input: SessionInput, password: string) { ... }
  async function deleteSession(id: string) { ... }
  async function connect(sessionId: string, password: string) { ... }
  async function disconnect(sessionId: string) { ... }
  async function sendKey(sessionId: string, event: KeyEvent) { ... }
  async function sendMouse(sessionId: string, event: MouseEvent) { ... }
  
  return { sessions, activeConnections, isLoading, error, init, loadSessions, ... };
});
```

**Page.vue** — Main page layout: left sidebar (SessionList) + center (DesktopCanvas) + top toolbar.

**Router** — Add to `src/router/index.ts`:
```typescript
{ path: '/remote-desktop', name: 'remote-desktop', component: () => import('@/pages/remote-desktop/Page.vue') }
```

**Sidebar** — Add nav item:
```typescript
{ label: '远程桌面', name: 'remote-desktop', path: '/remote-desktop', icon: Monitor }
```

---

## Execution Order (Waves)

### Wave 1 (parallel — no deps):
- Task 1: Rust types
- Task 6: Frontend tauri.ts wrappers (rely on TypeScript interfaces, no Rust deps)

### Wave 2 (parallel — depends on Wave 1 types):
- Task 2: Core — DesktopClient trait + FrameEncoder
- Task 4: Core — Session Store
- Task 7: Frontend — Components (Canvas, SessionList, Toolbar, Dialog)
- Task 8: Frontend — Store + Page + Router + Sidebar

### Wave 3 (parallel — depends on Wave 2):
- Task 3: Core — VNC Client (depends on DesktopClient trait + FrameEncoder)
- (Task 8 components need tauri.ts wrappers from Wave 1)

### Wave 4:
- Task 5: Rust Commands (depends on all core)
- Cargo.toml dependency updates

### Wave 5:
- lib.rs registration
- Build & test
