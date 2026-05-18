# AzurePath Phase 2 — 局域网发现 + 聊天 + 文件传输 设计文档

## 概述

在 AzurePath 中新增局域网自动发现、实时聊天和点对点文件传输能力。采用 P2P 全连接网格架构，无中心节点，每个 host 本地 SQLite 持久化。

## 架构

### 整体架构

```
┌─────────────────────────────────────────────────────────────┐
│  Host A (my-pc-a3f1)                                        │
│                                                             │
│  ┌──────────┐  ┌──────────────┐  ┌──────────────────────┐   │
│  │ Discovery │  │ TCP ConnPool │  │ SQLite (本地持久化)    │   │
│  │ (UDP 广播) │◄─┤              │  │ - messages           │   │
│  │ :42069    │  │ :42070       │  │ - peers              │   │
│  └──────────┘  └──────┬───────┘  └──────────────────────┘   │
│                        │                                     │
│              ┌─────────┼─────────┐                           │
│              │ Chat    │ File    │                           │
│              │ (JSON帧) │ (独立TCP)│                          │
│              └─────────┴─────────┘                           │
└─────────────────────────┬───────────────────────────────────┘
                          │ TCP
┌─────────────────────────▼───────────────────────────────────┐
│  Host B (other-pc-b2e4)  (same structure)                   │
└─────────────────────────────────────────────────────────────┘
```

## 1. 网络发现协议

### 机制

- 每个 Host 启动时生成唯一 ID: `hostname-{random_hex(4)}`
- 监听 UDP 端口 `42069` 接收广播
- 每 5 秒向 `255.255.255.255:42069` 发送 UDP 广播包
- 收到新 peer → 记录到本地 peers 表
- 30 秒未收到 peer 心跳 → 标记为 offline

### 广播包格式

```json
{
  "id": "my-pc-a3f1",
  "hostname": "my-pc",
  "ip": "192.168.1.10",
  "os": "Windows 11",
  "listen_port": 42070
}
```

### 离线检测

不依赖主动断开，采用超时机制：last_seen > 30s → offline。IP 变更时对端发更新广播覆盖旧记录。

## 2. 聊天协议

### 传输层

长度前缀的 JSON 帧：

```
[4字节 网络字节序长度] [JSON 内容]
```

所有消息类型共用同一帧格式，通过 `type` 字段区分。

### TCP 连接管理

- 发现新 peer 后 → 主动连接 `peer.ip:42070`
- 连接建立后 → 双向发送身份包 `{ type: "hello", id: "my-id" }`
- 心跳 → 每 15 秒发送空帧
- 连接断开 → 标记 peer offline，30 秒后清理
- 所有聊天消息写入本地 SQLite

### 消息类型

| type | 方向 | 说明 |
|------|------|------|
| `chat` | 单播/广播 | 聊天消息，`to: "*"` 为广播 |
| `system` | 广播 | 上下线通知 |
| `file_request` | 单播 | 文件传输请求 |
| `file_response` | 单播 | 接受/拒绝文件 |
| `file_progress` | 单播 | 传输进度通知 |

### Rust 并发模型

- 1× UdpSocket task → 发现/心跳
- 1× TcpListener task → 接受新连接
- N× TcpStream task → 每个已建立的连接一个独立读写 task

### 持久化 Schema

```sql
CREATE TABLE messages (
  id          TEXT PRIMARY KEY,
  peer_id     TEXT NOT NULL,
  peer_name   TEXT NOT NULL,     -- 显示用
  peer_ip     TEXT NOT NULL,
  peer_os     TEXT,
  content     TEXT NOT NULL,
  is_broadcast BOOLEAN DEFAULT false,
  is_incoming  BOOLEAN DEFAULT true,
  file_ref    TEXT,              -- 关联文件传输
  created_at  TEXT NOT NULL      -- ISO 8601
);

CREATE TABLE peers (
  id          TEXT PRIMARY KEY,
  hostname    TEXT NOT NULL,
  ip          TEXT NOT NULL,
  os          TEXT,
  last_seen   TEXT NOT NULL,
  status      TEXT DEFAULT 'online'
);
```

## 3. 文件传输协议

### 流程

```
Sender                            Receiver
  │── file_request (聊天连接) ──→│  发起请求
  │←── file_response ─────────────│  接受，附带数据端口
  │── 独立 TCP 连接 :动态端口 ──→│  数据通道
  │── [4B长度][文件块] ─────────→│  流式写入
  │── [4B长度][文件块] ─────────→│
  │── file_complete ────────────→│
  │←── file_ack ─────────────────│
  连接关闭
```

- **独立数据通道** — 文件传输使用独立 TCP 连接，不阻塞聊天消息
- **流式写入** — 不分块编号，末尾 complete 信号
- **进度推送** — 通过聊天连接推送 `file_progress` 事件
- **自动拒绝** — Receiver 不在线或 30 秒未响应，取消传输
- **存储路径** — `~/AzurePath/downloads/{filename}`

## 4. Rust 模块结构

```
src-tauri/src/
├── core/
│   ├── discovery/           # UDP 广播发现
│   │   ├── mod.rs
│   │   └── peer_table.rs
│   ├── connection/          # TCP 连接管理
│   │   ├── mod.rs
│   │   └── protocol.rs      # 帧编解码
│   ├── chat/
│   │   ├── mod.rs           # 消息路由 + 广播
│   │   └── store.rs         # SQLite 持久化
│   └── file_transfer/
│       ├── mod.rs
│       ├── sender.rs
│       └── receiver.rs
├── commands/                # Tauri IPC 命令
│   ├── discovery.rs
│   ├── chat.rs
│   └── file_transfer.rs
├── types/
│   ├── discovery.rs
│   ├── chat.rs
│   └── file_transfer.rs
```

## 5. 前端新增页面

| 页面 | 路由 | 说明 |
|------|------|------|
| 聊天 | `/chat` | peer 列表 + 消息区 + 输入框 |
| 文件传输 | `/files` | 传输列表 + 进度条 |

Sidebar 新增 "聊天" 和 "文件传输" 导航项。页面风格延续纸墨主题。

## 6. Tauri IPC 接口（草案）

```typescript
// 发现
'discovery:start'   () => void          // 启动发现服务
'discovery:peers'   () => Peer[]        // 获取 peer 列表
'discovery:stop'    () => void

// 事件
'peer:online'   => { id, hostname, ip, os }
'peer:offline'  => { id }

// 聊天
'chat:send'     (target: string, content: string) => void
'chat:broadcast'(content: string) => void
'chat:messages' (peerId?: string) => Message[]
'chat:history'  (limit?: number) => Message[]

// 事件
'chat:message'  => { id, from, fromName, fromIp, fromOs, content, isBroadcast, timestamp }

// 文件
'file:send'     (target: string, path: string) => void
'file:accept'   (fileId: string) => void
'file:reject'   (fileId: string) => void
'file:list'     () => FileTransfer[]

// 事件
'file:request'  => { fileId, filename, size, from }
'file:progress' => { fileId, received, total, speed }
'file:complete' => { fileId, path }
'file:error'    => { fileId, error }
```

---

*设计版本 v1.0 — 2026-05-18*
