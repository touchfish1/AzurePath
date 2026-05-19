# 剩余功能补全计划

## 概述

根据现有 docs/ 中的设计文档和实现计划，分析出 4 项尚未完成的功能。这 4 项相互独立，适合并行执行。

## 任务

### Task A: 历史记录页面 — 替换为"活动概览"

- **文件**: `src/pages/history/Page.vue`
- **描述**: 当前页面是空壳占位（"功能开发中"）。改为显示近期剪贴板条目、扫描统计等概览信息。作为轻量级的 dashboard 补充页面。
- **依赖**: 仅前端改动，无后端需求
- **标签**: `frontend`, `independent`

### Task B: DNS 自定义服务器地址

- **文件**: `src-tauri/src/core/dns/mod.rs`, `src-tauri/src/commands/dns.rs`, `src-tauri/src/types/dns.rs`, `src/lib/tauri.ts`, `src/pages/dns/Page.vue`
- **描述**: 当前 DNS 硬编码为 `8.8.8.8:53`（`core/dns/mod.rs:4`）。设计文档要求支持指定 DNS 服务器地址。
  - Rust: `resolve()` 添加可选 `dns_server: Option<String>` 参数，默认 `8.8.8.8:53`
  - Command: `dns_lookup` 接受可选 `dnsServer` 参数
  - Frontend: 添加 DNS 服务器输入框（默认 `8.8.8.8`），支持端口号
- **依赖**: 无（修改文件不与其他任务冲突）
- **标签**: `backend`, `frontend`, `independent`

### Task C: 剪贴板图片缩略图预览

- **文件**: `src/pages/clipboard/Page.vue`
- **描述**: 当前 clipbaord 页面对于 `content_type === "image"` 只显示文件名。需要使用 Tauri 2 `convertFileSrc` 将本地图片路径转为 webview 可加载的 URL，显示缩略图预览。
  - 使用 `@tauri-apps/api/core` 中的 `convertFileSrc`（Tauri 2 的 asset protocol）
  - 图片条目显示 100x100 缩略图，点击可大图预览
- **依赖**: 无
- **标签**: `frontend`, `independent`

### Task D: 独立文件传输管理页面

- **文件**: `src/pages/files/Page.vue`（新建）, `src/router/index.ts`, `src/components/layout/Sidebar.vue`
- **描述**: 当前 `/files` 路由 redirect 到 `/chat`。创建独立的文件传输管理页面，显示：
  - 文件传输历史列表（收发双向）
  - 传输进度条
  - 下载链接
  - 状态标签（传输中/已完成/已拒绝/失败）
  - 使用现有 `fileList()`, `fileAccept()`, `fileReject()` 等 API
- **依赖**: 无
- **标签**: `frontend`, `independent`

## 执行计划

4 个任务完全独立（修改不同文件），并行执行：

```
Task A ──→ src/pages/history/Page.vue
Task B ──→ src-tauri/src/core/dns/ + commands/ + types/ + frontend
Task C ──→ src/pages/clipboard/Page.vue
Task D ──→ src/pages/files/ + router/ + sidebar/

并行执行 → 验证编译 → 测试 → 更新 README
```
