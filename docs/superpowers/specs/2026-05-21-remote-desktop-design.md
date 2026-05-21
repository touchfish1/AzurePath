# AzurePath 远程桌面 — 设计文档

## 概述

在现有远程 Shell（SSH/Telnet）基础上，新增**远程桌面**功能，支持 **RDP** 和 **VNC** 两种协议。用户可通过 AzurePath 直接连接远程 Windows（RDP）和 Linux/其他（VNC）桌面，无需额外客户端。

## 技术选型

| 协议 | 库 | 理由 |
|------|-----|------|
| **RDP** | [IronRDP](https://github.com/Devolutions/IronRDP) | 纯 Rust，模块化架构，活跃维护（Devolutions 公司产品），支持 RemoteFX 图形解码、TLS/CredSSP、虚拟通道 |
| **VNC** | [vnc-rs](https://crates.io/crates/vnc-rs) | 纯 Rust 异步 VNC 客户端，RFB 协议完整实现，~100 万下载 |
| **RFB 编码** | [rfb-encodings](https://lib.rs/crates/rfb-encodings) | 纯 Rust RFB 编解码，支持 Tight/ZRLE/ZYWRLE/TightPng 等编码 |
| **前端渲染** | HTML5 Canvas (Vue 3) | 直接使用 WebView Canvas 渲染桌面帧，无需额外依赖 |

### 参考项目

- **[JumpServer Client](https://github.com/jumpserver/client)** — Tauri 2 + Vue 3 堡垒机客户端，已支持 RDP/VNC/SSH，架构参考价值最高
- **[MirrorX](https://github.com/qitiandashengsunwukong/MirrorX)** — Tauri + egui 远程桌面，Rust 全栈方案，渲染层思路可借鉴

## 架构设计

```
┌─────────────────────────────────────────────────────────────┐
│                    Tauri 桌面窗口                            │
│                                                             │
│  ┌────────────────────────┐  ┌───────────────────────────┐  │
│  │  前端 (Vue 3)           │  │  Rust 后端                 │  │
│  │                         │  │                           │  │
│  │  Page.vue               │  │  commands/remote_desktop  │  │
│  │  ├─ 连接侧边栏          │◄─┤  ├─ connect/disconnect    │  │
│  │  ├─ 桌面 Canvas         │  │  ├─ send_input            │  │
│  │  ├─ 缩放/全屏控件       │  │  ├─ resize (窗口变化)     │  │
│  │  └─ 剪贴板/文件传输面板 │  │  └─ clipboard             │  │
│  │                         │  │                           │  │
│  │  ┌──────────────────┐   │  │  core/remote_desktop      │  │
│  │  │ 桌面帧渲染管道     │   │  │  ├─ rdp.rs (IronRDP)    │  │
│  │  │ ← Tauri Event     │   │  │  ├─ vnc.rs (vnc-rs)     │  │
│  │  │ ← Canvas draw     │   │  │  └─ session_store.rs     │  │
│  │  └──────────────────┘   │  │                           │  │
│  └────────────────────────┘  └───────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### 关键设计决策

1. **帧传输：Tauri Events 推送** — Rust 后端启动帧轮询循环，将桌面帧编码为 JPEG 通过 `app.emit("rd:frame", payload)` 推送。前端 `listen("rd:frame")` 接收后在 Canvas 增量渲染。避免前端轮询降低延迟。
2. **输入转发** — 前端通过 `invoke` 发送键盘/鼠标事件到 Rust 后端，后端协议层转换为对应 RDP/VNC 输入 PDU。
3. **会话持久化** — 复用现有 `SessionStore`（SQLite）模式，保存 RDP/VNC 连接配置和密码。
4. **增量编码** — 后端缓存上一帧 bitmap，与当前帧做像素级 diff，只传输变化区域的 JPEG 编码，大幅降低带宽。

## Rust 三层结构

### types/remote_desktop/

```
types/remote_desktop/
├── mod.rs
├── session.rs      — Protocol enum (Rdp, Vnc), DesktopSession, SessionInput, SessionSummary
├── frame.rs        — DesktopFrame (x, y, width, height, data, encoding)
├── input.rs        — MouseEvent, KeyEvent, ModifierState
└── clipboard.rs    — ClipboardData
```

### core/remote_desktop/

```
core/remote_desktop/
├── mod.rs
├── session_store.rs  — SQLite 持久化（复用/扩展 remote_shell 的 session_store）
├── rdp.rs            — IronRDP 客户端封装
│   └── struct RdpClient: connect/decode_frame/send_input/resize/disconnect
├── vnc.rs            — vnc-rs 客户端封装
│   └── struct VncClient: connect/read_frame/send_input/resize/disconnect
├── desktop_client.rs — DesktopClient trait（统一 RDP/VNC 接口）
│   └── enum DesktopClientType { Rdp(RdpClient), Vnc(VncClient) }
└── frame_encoder.rs  — 帧编码（原始 bitmap → JPEG/PNG 压缩，增量变化检测）
```

#### DesktopClient Trait

```rust
#[async_trait]
pub trait DesktopClient: Send {
    async fn connect(&mut self, session: &DesktopSession, password: &str) -> Result<(), String>;
    async fn poll_frame(&mut self) -> Result<Option<DesktopFrame>, String>;
    async fn send_key_event(&mut self, event: KeyEvent) -> Result<(), String>;
    async fn send_mouse_event(&mut self, event: MouseEvent) -> Result<(), String>;
    async fn resize(&mut self, width: u16, height: u16) -> Result<(), String>;
    async fn disconnect(&mut self) -> Result<(), String>;
}
```

#### Frame 流水线

```
远程桌面 (RDP/VNC) ──→ 协议解码 ──→ 增量差异检测 ──→ JPEG编码 ──→ Tauri Event ──→ 前端 Canvas
                           │              │              │
                     原始 bitmap     只发送变化块    压缩减少带宽
```

### commands/remote_desktop.rs

```rust
// ── 会话管理 ──
#[tauri::command] async fn rd_list_sessions(state) -> Result<Vec<DesktopSession>, String>;
#[tauri::command] async fn rd_create_session(state, input, password) -> Result<DesktopSession, String>;
#[tauri::command] async fn rd_update_session(state, id, input) -> Result<DesktopSession, String>;
#[tauri::command] async fn rd_delete_session(state, id) -> Result<(), String>;

// ── 连接控制 ──
#[tauri::command] async fn rd_connect(app, state, session_id) -> Result<(), String>;
#[tauri::command] async fn rd_disconnect(state, session_id) -> Result<(), String>;
#[tauri::command] async fn rd_resize(state, session_id, width, height) -> Result<(), String>;

// ── 输入 ──
#[tauri::command] async fn rd_send_key(state, session_id, event) -> Result<(), String>;
#[tauri::command] async fn rd_send_mouse(state, session_id, event) -> Result<(), String>;

// ── 剪贴板 ──
#[tauri::command] async fn rd_push_clipboard(state, session_id, data) -> Result<(), String>;
```

## 数据流

### 连接流程

```
1. 用户填写 RDP/VNC 连接信息 → invoke("rd_create_session")
2. 用户点击连接 → invoke("rd_connect", { sessionId })
3. Rust 后端根据 protocol 创建 RdpClient 或 VncClient
4. 后端启动帧轮询循环 (0x20ms 间隔, 约 50fps):
   a. 协议层读取帧数据
   b. 帧编码器检测增量变化区域
   c. 将变化区域编码为 JPEG
   d. 通过 Tauri Event "rd:frame" 发送到前端
5. 前端 listen("rd:frame", cb) → Canvas.drawImage() 增量渲染

```

### 输入流程

```
1. 用户在前端 Canvas 上移动鼠标/按键
2. 前端捕获事件坐标 → invoke("rd_send_mouse", { sessionId, x, y, button, pressed })
3. Rust 后端转换为协议输入 PDU → 发送到远程桌面
```

## 前端设计

### 页面布局

```
┌──────────────────────────────────────────────────────┐
│  [← 返回]  远程桌面                         [⚙︎ 设置] │
├────────┬─────────────────────────────────────────────┤
│ 连接列表│                                             │
│ ┌────┐ │             桌面区域 (Canvas)                │
│ │Win  │ │                                             │
│ │Svr  │ │      ┌─────────────────────────────┐       │
│ │Ubnt │ │      │                             │       │
│ │ ... │ │      │   远程桌面画面（自适应缩放）   │       │
│ │     │ │      │                             │       │
│ │     │ │      └─────────────────────────────┘       │
│ │ [+] │ │        [100%] [全屏] [剪贴板] [文件]       │
│ └────┘ │                                             │
└────────┴─────────────────────────────────────────────┘
```

### Pinia Store

```typescript
// stores/remoteDesktop.ts
interface RemoteDesktopState {
  sessions: DesktopSession[];
  activeConnections: Record<string, {
    protocol: 'rdp' | 'vnc';
    status: 'connecting' | 'connected' | 'disconnected';
    width: number;
    height: number;
  }>;
  isLoading: boolean;
  error: string | null;
}
```

### Canvas 渲染策略

- 使用单个 `<canvas>` 元素，CSS 自适应缩放
- 监听 Tauri Event 接收 `DesktopFrame` 数据，只重绘 `{x, y, width, height}` 区域
- 全屏模式下隐藏窗口装饰，Canvas 铺满整个屏幕
- Ctrl+Alt+Del 等特殊按键通过下拉菜单或快捷键发送

## 剪贴板与文件传输

- **剪贴板**：监听系统剪贴板变化，通过 RDP 虚拟通道（IronRDP clipboard）同步到远程桌面
- **文件传输**：利用现有 SFTP 面板模式，为 VNC 实现文件传输扩展；RDP 使用 `rdpdr` 设备重定向虚拟通道

## 实现阶段

### Phase 1 — 基础框架（目标：可连接并看到桌面）
1. 创建 `types/remote_desktop/` 数据模型
2. 实现 `DesktopClient` trait 和 `desktop_client.rs`
3. 实现 VNC 客户端（vnc-rs 封装）
4. 实现帧轮询 + 增量编码
5. 前端 Canvas 渲染管道
6. 连接管理 UI（侧边栏 + 会话 CRUD）

### Phase 2 — RDP 支持
1. 集成 IronRDP 客户端
2. RDP 帧解码 + 输入转发
3. 统一 VNC/RDP 连接管理 UI

### Phase 3 — 进阶功能
1. 剪贴板同步
2. 文件传输
3. 全屏模式
4. 多显示器支持
5. 连接凭据加密存储

## 注意事项

- **IronRDP 依赖较重** — 仅按需加载，不影响启动速度
- **帧率控制** — 根据网络质量动态调整帧率（15-60fps），低带宽自动降低 JPEG 质量
- **鼠标捕获** — 全屏模式下需捕获鼠标指针，使用 Tauri `window.setCursorGrab()` + `setCursorVisible()`
- **RDP 授权** — 需处理 NLA（Network Level Authentication）和 CredSSP
