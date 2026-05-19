# AzurePath 下一阶段方案 — 功能打磨 + 工程基建

> 基于 2026-05-19 代码库全面审查结果。已完成：主题持久化、Rust 日志系统、前端测试基础设施、History 页面增强、自动更新机制、补充测试。

---

## 一、国际化 (i18n) — 延后（后续完善项目后再做）

### 1. i18n 基础设施
- **文件**: `src/i18n/index.ts`, `src/i18n/locales/zh-CN.json`, `src/i18n/locales/en.json`
- **新 Store**: `src/stores/locale.ts`（持久化语言偏好）
- **改**: `src/main.ts` 注册 vue-i18n, `index.html` 动态 lang
- 估计: ~2.5h

### 2. i18n 文本提取
- 逐个页面将硬编码中文替换为 `$t("key")`
- 涉及 12+ 文件：所有 10 个页面 + Sidebar + TitleBar + PortModal
- 翻译 key 按页面/组件组织，如 `sidebar.dashboard`, `ping.targetPlaceholder`
- 挑战：Sidebar 的 `navItems` 静态数组需要改成 computed 以响应语言切换
- 估计: ~5h

---

## 二、全局快捷键 — 待实现

### 3. 全局键盘快捷键
- **新文件**: `src/composables/useKeyboardShortcuts.ts`
- **改**: `AppShell.vue` 注册全局监听
- 快捷键规划: `Ctrl+1~9` 导航前 9 个页面, `Ctrl+T` 切换主题, `Ctrl+D` 仪表盘, `Ctrl+F` 文件传输, `Escape` 关闭面板/取消操作
- 需要监听 `keydown` 事件，匹配 Ctrl+[0-9] 时调用 `router.push()` 对应路由
- 排除输入框/文本框中的快捷键冲突（`event.target` 判断）
- 估计: ~2.5h

---

## 三、安全加固 — 新发现

### 5. 文件接收器超时机制 (高优先级)
- **文件**: `src-tauri/src/core/file_transfer/receiver.rs`
- 问题: `receive_file()` 在 `read_string`, `read_u64`, `read_exact` 上均无超时，恶意对端可以 1 字节/秒的速度占用接收槽
- 修复: 对每次网络读取操作加 `tokio::time::timeout()`，建议单次读取超时 30s，总传输超时视文件大小而定
- 估计: ~1h

### 6. 文件接收器磁盘空间检查
- **文件**: `src-tauri/src/core/file_transfer/receiver.rs`
- 问题: 不对 `total_size` 做合理性检查（无磁盘空间预检），恶意对端可声称 TB 级文件导致磁盘耗尽
- 修复: 在开始接收前检查 `total_size` <= 可用磁盘空间，超过则拒绝
- 估计: ~0.5h

### 7. 移除 `panic!()` 在 Sniffer 中
- **文件**: `src-tauri/src/commands/network_sniffer.rs:337`
- 问题: `Arc::try_unwrap(devices).unwrap_or_else(|_| panic!(...))` — 如果某个 spawn 任务持有引用，整个扫描崩溃
- 修复: 替换为 `Arc::into_inner()` 或优雅的 `try_unwrap` 降级处理
- 估计: ~0.5h

### 8. 配置 Content Security Policy (CSP)
- **文件**: `src-tauri/tauri.conf.json`
- 问题: `"security": { "csp": null }` — 无 CSP 策略，缺少 XSS 纵深防御
- 修复: 定义严格 CSP，如 `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'`
- 估计: ~0.5h

### 9. 修复 `resolve_hostname()` 空实现
- **文件**: `src-tauri/src/core/network_sniffer/discovery.rs:114-125`
- 问题: `resolve_hostname()` 始终返回 `None`，主体逻辑为空
- 修复: 实现实际的 hostname 解析或移除该函数并清理调用处
- 估计: ~0.5h

---

## 四、性能优化 — 新发现

### 10. Heartbeat 循环中避免重复分配
- **文件**: `src-tauri/src/core/discovery/mod.rs:167`
- 问题: `std::env::consts::OS.to_string()` 在每 5 秒的 heartbeat 循环中调用，OS 是编译期常量，无需每次都 to_string
- 修复: 提到循环外部初始化一次
- 估计: ~0.3h

### 11. 广播发送改为并发
- **文件**: `src-tauri/src/core/connection/mod.rs:273-285`
- 问题: `broadcast()` 顺序遍历 peers，慢 peer 阻塞所有其他人的消息
- 修复: 使用 `tokio::spawn` + `join_all` 并发发送
- 估计: ~1h

### 12. Sniffer 结果内存泄漏
- **文件**: `src-tauri/src/commands/network_sniffer.rs`
- 问题: `SCAN_RESULTS: HashMap<String, Vec<DeviceResult>>` 无限增长，无清理机制
- 修复: 添加 eviction 策略（限制最多保留 N 次扫描结果）或提供清理命令
- 估计: ~0.5h

---

## 五、代码质量 — 新发现

### 13. 移除 `#![allow(dead_code)]`
- **文件**: `src-tauri/src/lib.rs:1`
- 问题: 全局允许 dead code 隐藏了所有未使用代码的警告
- 修复: 删除该属性，逐个标记真正需要的 `#[allow(dead_code)]`
- 注意: `core/port_scan/mod.rs` 的 `scan_ports()` 函数除了测试外未被使用，会被标记
- 估计: ~0.5h

### 14. 提取重复的 `home_dir()` 函数
- **文件**: `receiver.rs`, `chat/store.rs`, `clipboard/store.rs`
- 问题: 三个模块各有完全相同的 `home_dir()` 实现
- 修复: 提取到 `core/utils.rs` 或 `core/mod.rs` 共享
- 估计: ~0.5h

### 15. 提取重复的编码转换逻辑
- **文件**: `ping/mod.rs`, `traceroute/mod.rs`, `commands/traceroute.rs`
- 问题: 三处重复的 GBK/UTF-8 编码转换 fallback
- 修复: 提取到共享工具函数
- 估计: ~0.5h

### 16. 统一端口-服务名映射
- **文件**: `port_scan/mod.rs` + `network_sniffer/fingerprint.rs`
- 问题: 两份几乎相同的 `match port { ... }` 映射表，部分条目不一致（如端口 587 一个叫 "SMTP Submission" 一个叫 "SMTP"）
- 修复: 合并到一处共享的 `core/mod.rs` 或独立的 `service_db.rs`
- 估计: ~1h

### 17. 使用 `unknown` 替代 `any`
- **文件**: `src/pages/network-sniffer/Page.vue:168` 等
- 问题: `catch (e: any)` 应使用 `catch (e: unknown)` 并做类型收窄
- 估计: ~0.5h

### 18. `lib/tauri.ts` 移除 `as any` 类型断言
- **文件**: `src/lib/tauri.ts:379,383,387,391`
- 问题: 文件传输事件监听缺少泛型参数，使用 `as any`
- 修复: 添加正确的 `listen<PayloadType>(...)` 泛型
- 估计: ~0.5h

---

## 六、前端 UI/UX 打磨 — 新发现

### 19. 完善缺失的空状态/引导页
- **文件**: Ping/Traceroute/PortScan 三个页面的 Page.vue
- 问题: 首次打开时仅显示输入卡片，无引导提示。DNS 页面做得最好（有空状态和错误提示），应参考其模式
- 修复: 在结果区域添加 `v-if` / `v-else` 展示引导文案或使用说明
- 估计: ~1h

### 20. 提取公共组件减少代码重复
- Ping/Traceroute/PortScan 三个输入卡片高度相似（目标输入、数量设定、开始/停止按钮、错误横幅、结果表格）
- Chat/Files 共享文件传输监听器注册和清理逻辑
- 多个页面各有自己的 `formatTime`, `truncate`, `formatSize`, `progressPercent` 工具函数
- 估计: ~2h

### 21. Sniffer 监听器泄漏修复
- **文件**: `src/pages/network-sniffer/Page.vue:142-164`
- 问题: 每次 `startScan()` 注册新监听器但不清除旧监听器。停止后重新扫描会累积监听器
- 修复: 在 `startScan()` 开头调用 `cleanup()` 清除之前的监听器
- 估计: ~0.5h

### 22. 修复 Sniffer 空监听器
- **文件**: `src/pages/network-sniffer/Page.vue:152-153`
- 问题: `onSnifferPort(() => {})` 注册了空操作的监听器，浪费资源
- 修复: 删除该行
- 估计: ~0.2h

### 23. 添加基础 ARIA 可访问性
- 问题: 全项目零 ARIA 属性，所有交互元素缺少无障碍标签
- 最低要求：为图标按钮添加 `aria-label`，为输入框绑定 `for`/`id`，为展开面板添加 `aria-expanded`
- 估计: ~2h

### 24. 网络嗅探器深色模式修复
- **文件**: `src/pages/network-sniffer/Page.vue:556-588`
- 问题: 服务标签使用硬编码十六进制颜色（如 `#d4e8f7`），深色模式下不可见
- 修复: 使用 CSS 变量替代
- 估计: ~0.5h

---

## 七、工程基建 — 新发现

### 25. 配置 GitHub Actions CI
- 问题: 无任何 CI 配置
- 添加 PR 检查 workflow：`npm ci` → `vue-tsc --noEmit` → `vitest run` → `cargo check` → `cargo test`
- 建议: 使用 `tauri-apps/tauri-action` 进行构建和发布
- 估计: ~2h

### 26. 添加代码格式化/Lint 工具
- 建议: 引入 Biome (替代 ESLint + Prettier)
- 配置: biome.json，集成 pre-commit hook
- 估计: ~1.5h

### 27. 完善 `.gitignore`
- 缺失: `*.log`, `.env`, `.env.*`, `.DS_Store`, `*.local`, `.vscode/settings.json`, `src-tauri/gen/`
- 估计: ~0.2h

### 28. 限制 `tokio` features
- **文件**: `src-tauri/Cargo.toml`
- 问题: `features = ["full"]` 引入大量可能未使用的特性
- 修复: 按需选择（rt, rt-multi-thread, macros, sync, net, io-util, process, fs, signal）
- 估计: ~0.5h

### 29. 添加 `rust-toolchain.toml`
- 固定 Rust 版本，避免 CI 和本地环境不一致
- 估计: ~0.2h

### 30. 更新 Updater 配置
- **文件**: `src-tauri/tauri.conf.json`
- 问题: `pubkey` 和 `endpoints` 为空
- 修复: 生成签名密钥对，搭建更新服务器或配置 GitHub Releases 作为更新源
- 估计: ~1h

---

## 八、DNS TCP 回退 — 新发现

### 31. DNS 模块 TCP 回退
- **文件**: `src-tauri/src/core/dns/mod.rs`
- 问题: 响应超过 4096 字节（TC 标志）时直接截断，DNS 协议要求回退到 TCP
- 修复: 检测 TC 标志后使用 TCP 连接重发请求
- 估计: ~2h

---

## 状态标记说明

- ✅ **已完成** — 已实现并合入
- 🔜 **待实现** — 用户确认要做，未开始
- ⏸️ **延后** — 后续阶段再做

---

## 执行顺序建议

```
第一阶段（性能 + 快捷键）:
  全局快捷键 → Heartbeat 优化 → 广播并发 → Sniffer 结果清理

第二阶段（安全加固）:
  文件接收器超时 → 磁盘空间检查 → Sniffer panic 修复 → CSP 配置 → 空函数修复

第三阶段（代码质量）:
  移除 dead_code → 提取重复代码(home_dir/编码转换/端口映射) → 广播并发
  catch any 修复 → 移除 as any 断言

第四阶段（UI 打磨）:
  空状态引导页 → 公共组件提取 → 深色模式修复 → 监听器泄漏/空监听器 → ARIA

第五阶段（工程基建）:
  CI 配置 → Biome 集成 → rust-toolchain → gitignore → tokio features → Updater

第六阶段（扩展功能）:
  DNS TCP 回退

---

## 九、新功能方向（基于行业调研 2026）

### 32. 系统通知 (OS Notification) 🔜 待实现
- **用途**: 文件传输完成、新聊天消息、嗅探扫描完成时推送系统原生通知
- **实现**: `tauri-plugin-notification` — Rust 端注册，前端触发
- 无需后台常驻，应用在前台时推送
- 估计: ~1.5h

### 33. 系统托盘 (System Tray) 🔜 待实现
- **用途**: 最小化到系统托盘，后台继续接收文件/消息
- **实现**: Tauri 2.0 的 `TrayIconBuilder` + 右键菜单（显示/退出）
- 点击托盘恢复窗口，有新消息时托盘图标闪烁
- 估计: ~2h

### 34. 开发者工具箱页面 🔜 待实现
- **用途**: 聚合常用开发/网络小工具到一个页面
- **内容建议**:
  - 子网计算器（IP/CIDR/子网掩码换算）
  - Base64 编解码
  - URL 编解码
  - Hash 生成器（MD5/SHA1/SHA256）
  - 端口号速查
- 纯前端实现，无 Rust 依赖
- 估计: ~3h

### 35. 虚拟滚动优化 🔜 待实现
- **用途**: 历史记录/剪贴板列表过长时性能优化
- **实现**: `vue-virtual-scroller` 或 `@vueuse/core` 的 `useVirtualList`
- 优先应用于 History 页面和 Clipboard 页面
- 10,000 条记录只渲染 ~30 个 DOM 节点
- 估计: ~2h

### 36. 原生全局快捷键 (Tauri Plugin) 🔜 待实现
- **用途**: 应用最小化时也能通过快捷键唤起
- **实现**: `tauri-plugin-global-shortcut`
- 与前端快捷键不冲突（不同场景使用）
- 估计: ~1h

---

## 十、功能完善 — 更多网络工具

### 37. WHOIS 查询 🔜 待实现
- **用途**: 查询域名/IP 的注册信息（注册商、注册日期、到期日等）
- **实现**: Rust TCP 连接 whois 服务器（端口 43），发送查询，解析响应
- 需要内置常见 whois 服务器列表（whois.verisign-grs.com 等）
- 纯后端实现，前端简单输入+结果展示
- 估计: ~1.5h

### 38. HTTP 状态检查 🔜 待实现
- **用途**: 快速检查目标 URL 是否存活，返回状态码、响应头、响应时间
- **实现**: Rust 使用 `reqwest` 或 `hyper` 发送 GET/HEAD 请求
- 最好避免新增依赖，可以用 `tokio::net::TcpStream` 手动发送 HTTP 请求
- 结果显示状态码、Content-Type、服务器、响应时间
- 估计: ~1h

### 39. SSL/TLS 证书检查 🔜 待实现
- **用途**: 连接目标服务器获取 TLS 证书，解析证书链
- **实现**: Rust `tokio-native-tls` 或 `rustls` 连接目标端口 443
- 显示：颁发者、主题、有效期（开始/到期）、是否自签名、是否过期
- 估计: ~2h

### 40. WiFi QR 生成器 🔜 待实现
- **用途**: 输入 SSID + 密码 + 加密类型，生成 WiFi 配置二维码
- **实现**: 纯前端，使用 `qrcode` npm 包
- 支持 WPA/WPA2/WPA3/无加密
- 二维码可直接用手机扫描连接
- 估计: ~1h

### 41. MAC 地址厂商查询 🔜 待实现
- **用途**: 输入 MAC 地址，返回设备厂商名称
- **实现**: 前端内置常用 OUI 数据库（或 Rust 端维护精简列表）
- 支持格式: 00:11:22:33:44:55 / 00-11-22-33-44-55 / 001122334455
- 估计: ~0.5h

---

## 十一、数据管理

### 42. 聊天历史管理 🔜 待实现
- **文件**: `src-tauri/src/core/chat/store.rs` + `src/pages/chat/Page.vue`
- 聊天页添加"历史记录"入口/弹窗
- 支持按日期、关键词搜索历史消息
- 单条删除 + 清空全部
- Rust 端新增：`search_messages(keyword: &str)`, `delete_messages(ids: &[i64])`, `clear_history()`
- 调用 `app.emit("chat:history", ...)` 推送结果
- 估计: ~2h

### 43. 剪贴板数据增强 🔜 待实现
- **文件**: `src/pages/clipboard/Page.vue`
- 批量选择（checkbox）+ 批量删除
- 按应用来源筛选（从哪个应用复制的）
- 按内容类型筛选（文本/图片）
- 数据导出为 JSON/CSV
- 存储上限设置（保留最近 N 条，超过自动清理）
- 估计: ~2h

### 44. 活动日志持久化 🔜 待实现
- **文件**: 新建 `src-tauri/src/core/history/store.rs` + 更新 `src/pages/history/Page.vue`
- 当前活动概览是纯内存聚合，重启丢失
- 新建 SQLite 表记录所有操作（扫描/传输/聊天等）
- 每次操作（ping 完成、文件传输等）写入一条记录
- 前端历史页面改为从持久化存储读取
- 支持搜索、过滤、删除、导出
- 估计: ~2.5h

### 45. 数据导入导出 🔜 待实现
- 聊天记录导出为 JSON/TXT
- 剪贴板数据导出
- 设置/偏好导出（跨设备迁移或备份）
- 估计: ~2h

---

## 十二、用户体验打磨

### 46. 操作 Toast 提示 🔜 待实现
- **用途**: 复制成功、发送成功、删除完成等轻量反馈
- **实现**: 新建 `src/components/Toast.vue` + `src/stores/toast.ts`
- 右下角出现，2 秒后自动消失
- 支持 success / error / info 三种类型
- 集成到：复制按钮、文件发送、设置保存等操作
- 估计: ~1h

### 47. 侧栏可折叠 🔜 待实现
- **用途**: 窄模式下只显示图标，鼠标悬停展开
- **实现**: 修改 `Sidebar.vue`，添加 collapsed/expanded 状态
- 切换按钮在侧栏底部
- 折叠时只显示图标 + tooltip
- 估计: ~1.5h

### 48. 结果复制按钮 🔜 待实现
- **用途**: 所有结果表格行添加一键复制按钮
- **涉及页面**: Ping、Traceroute、端口扫描、DNS、嗅探器、剪贴板
- 复制内容根据上下文: IP、域名、端口号、整行数据
- 复制后触发 Toast 提示
- 估计: ~1h

### 49. 列表排序筛选 🔜 待实现
- **用途**: 扫描/结果表格支持按列排序
- **涉及页面**: 端口扫描、嗅探器、剪贴板、历史
- 点击表头排序，支持升序/降序切换
- 估计: ~1.5h

### 50. 统一设置面板 🔜 待实现
- **用途**: 在一个页面管理所有应用设置
- **新建**: `src/pages/settings/Page.vue` + 路由 + 侧栏入口
- **包含设置项**:
  - 主题（已有，迁移过来）
  - 剪贴板监控间隔（1s/3s/5s/10s）
  - 扫描默认参数（Ping 次数、超时等）
  - 文件下载目录
  - 数据保留策略（保留最近 N 天）
  - 通知开关
- Rust 端：持久化设置到 JSON 或 SQLite
- 估计: ~2h

### 51. 文件传输进度条统一样式 🔜 待实现
- **用途**: 统一 Chat 页和 Files 页的进度条视觉风格
- 使用 Tailwind 自定义颜色，与主题一致
- 显示百分比 + 已传输/总大小
- 估计: ~0.5h

---

## 执行顺序建议

```
第一阶段（当前 agents 正在做）:
  系统通知 + 系统托盘 + 原生快捷键 + 工具箱 + 虚拟滚动

第二阶段（数据管理，高 Impact）:
  操作 Toast 提示 → 结果复制按钮 → 剪贴板批量操作 → 聊天历史管理

第三阶段（功能补齐）:
  WHOIS → HTTP 状态检查 → SSL 检查 → WiFi QR → MAC 查询

第四阶段（体验提升）:
  统一设置面板 → 侧栏折叠 → 列表排序筛选 → 进度条统一

第五阶段（数据深化）:
  活动日志持久化 → 数据导入导出

第六阶段（新功能 + 质量冲刺）:
  Speedtest 局域网测速 → 网络拓扑可视化 → 保存预设配置 → 首次使用向导
  修复 Rust dead code 警告 → 响应式布局 → 统一错误提示 → 二进制体积优化
```

---

## 十三、新功能扩展

### 52. Speedtest 局域网测速 🔜 待实现
- **用途**: 测量局域网内两台设备之间的带宽、延迟、抖动
- **实现**: Rust 后端
  - 客户端向服务端发送测试数据包（类似 iperf 模型）
  - 测量 TCP 吞吐量（发送固定大小数据，计算耗时）
  - 测量延迟和抖动（发送小包，计算 RTT 变化）
- **类型**: `SpeedtestResult { download_mbps, upload_mbps, latency_ms, jitter_ms }`
- 复用已有的 `connection` 模块的 peer 发现机制
- 前端：简单启动/停止按钮 + 实时结果展示
- 估计: ~3h

### 53. 网络拓扑可视化 🔜 待实现
- **用途**: 将局域网发现的设备绘制为拓扑图
- **实现**: 前端 Canvas/SVG 渲染（使用 `@vueuse/core` 或原生 Canvas）
- 节点：发现的设备（IP、hostname、OS）
- 连线：设备之间的连接关系（基于 traceroute 路径或 discovery 结果）
- 交互：点击节点查看详情，拖拽移动节点
- 放在 Dashboard 页面作为一个独立面板，或嗅探器页面的补充视图
- 估计: ~4h

### 54. 保存预设配置 🔜 待实现
- **用途**: 将 Ping/端口扫描/嗅探的参数保存为预设，快速复用
- **实现**: Rust 端 JSON 文件存储 + 前端 UI
- 预设包含：目标、端口范围、并发数、超时等参数
- 前端下拉选择预设，自动填充参数
- 命名 + 保存 + 删除预设
- 估计: ~2h

### 55. 首次使用向导 🔜 待实现
- **用途**: 第一次启动时引导用户完成基础配置
- **实现**: 前端多步向导页面
  - 步骤 1: 欢迎 + 选择主题
  - 步骤 2: 配置剪贴板监控
  - 步骤 3: 配置下载目录
  - 步骤 4: 通知权限
  - 步骤 5: 完成
- 存储在 `localStorage` 中标记是否已完成向导
- 估计: ~2h

---

## 十四、质量冲刺

### 56. 修复 Rust dead code 警告 🔜 待实现
- **现状**: 移除了 `#![allow(dead_code)]` 后存在 34 个 dead code 警告
- **处理方案**:
  - 对仅供测试使用的函数标记 `#[cfg(test)]`
  - 对为将来预留的函数标记 `#[allow(dead_code)]` 加注释说明
  - 对确实无用且不在任何路径中使用的函数直接删除
- 涉及模块: ping, traceroute, port_scan, network_sniffer, dns
- 估计: ~1h

### 57. 响应式布局适配 🔜 待实现
- **现状**: 各页面在某些窗口尺寸下布局异常
- **修复范围**:
  - Chat 页侧栏（`w-52` 固定宽度）
  - Sniffer 配置区（flex-wrap 堆叠）
  - PortModal（`w-[640px]` 固定宽度溢出）
  - 各页面 `p-6` 在小窗口减少为 `p-3`
- 估计: ~1.5h

### 58. 统一错误提示 🔜 待实现
- **现状**: 各页面错误处理风格不一致（有的 console.error、有的 alert、有的无反馈）
- **修复**: 全部改为 Toast 提示（复用已有的 Toast 组件）
- 覆盖所有 Tauri invoke catch 块
- 估计: ~1h

### 59. 二进制体积优化 🔜 待实现
- **现状**: 默认 debug 构建较大
- **措施**:
  - `Cargo.toml` 添加 `[profile.release]` 优化配置
  - `opt-level = "z"`, `lto = true`, `strip = true`, `codegen-units = 1`
  - 检查 `tauri.conf.json` 中 bundle 配置
- 估计: ~0.5h

---

## 最终执行顺序

```
第一波（已完成）:
  Toast/复制/排序/侧栏/设置/聊天历史/剪贴板批量/进度条/WiFiQR
  + Rust: store/WHOIS/HTTP/SSL/MAC/活动日志/导出/设置持久化

第二波（已完成）:
  Speedtest → 预设配置 → 首次向导 → 网络拓扑可视化

第三波（已完成）:
  修复 dead code 警告 + 响应式布局 + 统一错误提示 + 二进制优化

第四波（Phase 5，当前 agents 正在做）:
  SSH 终端 + WOL + 窗口状态/自启动 + 工具箱开发者工具
  + mDNS 发现 + 带宽监控 + 仪表盘图表 + 性能历史
  + 报告导出 + 数据导出增强
```

---

## 十五、Phase 5 — 新功能扩展

> 基于 2026-05-19 调研讨论确认。覆盖系统集成、网络发现增强、SSH 终端、仪表盘增强等方向。

### 第一梯队：高价值低工作量

#### 60. Wake-on-LAN (WOL) 🔜 待实现
- **用途**: 远程唤醒局域网内设备
- **Rust**: `tokio::net::UdpSocket` 广播魔术包（6 字节 0xFF + MAC 重复 16 次）到 UDP 端口 9
- **前端**: 新建 `src/pages/wol/Page.vue` — 输入 MAC 地址 + 目标 IP/广播地址，保存历史记录
- **联动**: 与设备发现列表集成，已发现的设备直接点 WOL 按钮
- **文件**: 新建 `src-tauri/src/types/wol.rs`, `src-tauri/src/core/wol/mod.rs`, `src-tauri/src/commands/wol.rs`
- 路由 `/wol`，侧栏入口
- 估计: ~1.5h

#### 61. 窗口状态持久化 🔜 待实现
- **用途**: 记住窗口位置、大小、是否最大化，重启恢复
- **实现**: 添加 `tauri-plugin-window-state` 到 `Cargo.toml`，在 `lib.rs` 注册 `.plugin(tauri_plugin_window_state::Builder::new().build())`
- 几乎零代码，纯配置
- 估计: ~0.3h

#### 62. 自启动 + 单实例 🔜 待实现
- **用途**: 开机自启 + 防止重复启动
- **自启动**: `tauri-plugin-autostart` — 设置页添加开关
- **单实例**: Tauri 2.0 内置 `.single_instance()` — 监听第二次启动事件，唤出已有窗口
- **前端**: 设置页面增加 "开机自启" 开关，调用 `tauri-plugin-autostart` API
- 估计: ~0.5h

#### 63. 报告导出 (HTML/PDF) 🔜 待实现
- **用途**: Ping/端口扫描/嗅探/DNS 结果导出为格式化报告
- **实现**: 前端纯实现
  - 通用报告模板渲染函数，输入 `{ title, columns, rows, timestamp }` 输出完整 HTML
  - HTML 自带内联样式，可直接打印为 PDF（`window.print()`）
  - 保存通过 Tauri dialog 选路径
- **页面集成**: Ping/端口扫描/嗅探/DNS 结果工具栏加 "导出报告" 按钮
- 估计: ~2h

#### 64. mDNS/Bonjour 服务发现 🔜 待实现
- **用途**: 发现局域网内 HTTP/SMB/SSH/FTP 等服务
- **Rust**: 发送 UDP 多播到 `224.0.0.251:5353`，解析 mDNS 响应（与 DNS 报文格式兼容）
  - 查询 `_http._tcp.local`, `_smb._tcp.local`, `_ssh._tcp.local`, `_ftp._tcp.local` 等常见服务
  - 解析结果：服务类型、主机名、端口、TXT 记录
- **前端**: 嗅探器页面或独立页面展示发现的服务列表
- **复用**: DNS 解析模块的报文解析逻辑
- 估计: ~2.5h

### 第二梯队：核心功能加强

#### 65. SSH 终端 🔜 待实现
- **用途**: 内嵌 SSH 客户端，多标签会话管理
- **Rust**: 使用 `russh` 或 `ssh2` crate 建立 SSH 连接
  - `src-tauri/src/core/ssh/` 模块 — Session 管理，Channel 读写
  - `src-tauri/src/commands/ssh.rs` — `ssh_connect`, `ssh_exec`, `ssh_resize`, `ssh_disconnect`
  - 通过 Tauri event `ssh:data` 推送终端输出，接收 `ssh:input` 事件发送输入
- **前端**: xterm.js + fit-addon
  - 新建 `src/pages/terminal/Page.vue`
  - 多标签切换，连接管理侧栏（保存的会话列表）
  - 支持密码和密钥认证
- 注意: `russh` 是纯 Rust 实现，无需依赖 OpenSSL；`ssh2` 依赖 libssh2，功能更成熟
- 路由 `/terminal`，侧栏入口
- 估计: ~4-6h

#### 66. 实时带宽监控 🔜 待实现
- **用途**: 实时显示各网卡的上传/下载速率
- **Rust**: Windows 上读取 `GetIfEntry2` / `GetAdapterAddresses` API（或 `libc::getifaddrs` on Unix）
  - 1 秒轮询间隔，计算差值得到瞬时速率
  - 返回 `{ interface: String, download_bps: u64, upload_bps: u64, total_rx: u64, total_tx: u64 }`
- **前端**: 折线图实时展示（Chart.js 或纯 Canvas）
  - 可选网卡，显示最近 60 秒趋势
  - 仪表盘集成小部件或独立页面
- 文件: 新建 `src-tauri/src/core/bandwidth/mod.rs`, `src-tauri/src/commands/bandwidth.rs`, `src-tauri/src/types/bandwidth.rs`
- 估计: ~3h

#### 67. 仪表盘图表增强 🔜 待实现
- **用途**: Ping 延迟历史、端口扫描热力图、流量趋势
- **Ping 历史图**:
  - 每次 Ping 完成后将结果存 `localStorage`（保留最近 100 次）
  - 仪表盘展示延迟折线图，Y 轴延迟 ms，X 轴时间点
  - 不同目标用不同颜色系列
- **端口扫描热力图**:
  - X 轴端口号，Y 轴目标 IP，颜色表示状态（绿=开放，灰=关闭，红=过滤）
  - 复用 Tailwind 颜色
- **实现**: 纯前端，使用简单的 Canvas 或 SVG（避免新增依赖）
- 估计: ~2.5h

#### 68. 工具箱新增开发者工具 🔜 待实现
- **用途**: JSON 格式化/JWT 解码/时间戳转换
- 工具箱新增 tab："开发者工具"
- **JSON 格式化**: textarea 输入 → 格式化/压缩切换，语法校验报错
- **JWT 解码**: 输入 JWT token → 解码 Header + Payload（Base64），展示 JSON 结构
- **时间戳转换**: Unix 秒/毫秒 ↔ 可读日期，时区选择
- 全部纯前端实现
- 估计: ~1.5h

#### 69. 数据导出增强 🔜 待实现
- **用途**: 现有导出基础上增加 CSV 和 HTML 报告格式
- **新增格式**:
  - CSV: 通用表格导出，可用 Excel 打开
  - HTML: 带样式的报告，可直接打印为 PDF
- **涉及页面**: 聊天历史、剪贴板、活动历史
- **实现**: 前端格式化函数，Tauri dialog 保存文件
- 估计: ~1h

### 第三梯队：旗舰功能

#### 70. 网络性能历史（Smokeping 风格）🔜 待实现
- **用途**: 定时 Ping 目标列表，持久化历史，趋势可视化
- **Rust**: 后台定时任务（tokio interval）
  - 从配置读取目标列表，每 5/30/60 分钟 Ping 一次
  - 结果写入 SQLite（`ping_history` 表）：`{ target, timestamp, latency_ms, loss_rate }`
  - 命令: `start_monitoring`, `stop_monitoring`, `get_history(target, range)`
- **前端**: 仪表盘展示延迟趋势图（日/周/月视图）
  - 类 Smokeping 风格：最近 24 小时延迟折线图 + 丢包率标注
  - 目标管理：添加/删除监控目标
- 依赖活动日志持久化基础设施
- 估计: ~4h

#### 71. VNC 远程桌面 🔜 待实现
- **用途**: 内嵌 VNC 客户端查看远程桌面
- **Rust**: `rfb` 协议实现（或使用已有 crate）
  - TCP 连接 VNC 服务器（默认 5900 端口）
  - RFB 握手 → 认证 → FramebufferUpdate 接收
  - 处理编码：Raw、Hextile、TRLE
  - 转发鼠标/键盘事件到服务器
- **前端**: Canvas 渲染帧缓冲
  - 缩放、全屏
  - 连接管理（保存的 VNC 服务器列表）
- **工作量大**: 建议先实现基本可用的 Raw 编码 + 简单认证
- 估计: ~8h+

#### 72. 批量命令执行 🔜 待实现
- **用途**: 对多台远程设备并发执行命令
- **Rust**: 复用 SSH 终端基础设施
  - 从设备列表或手动输入多个目标
  - 并发 SSH 连接，执行命令，聚合输出
  - 返回 `{ host, exit_code, stdout, stderr }`
- **前端**: 多选设备 + 命令输入 + 结果对比表格
  - 绿色/红色标记成功/失败
- **依赖**: SSH 终端功能完成后实现
- 估计: ~3h

---

## Phase 5 执行顺序

```
第一波（4 个 agent 并发，可独立部署）:
  Agent A: WOL + 窗口状态 + 自启动/单实例 + 工具箱开发者工具（1.5+0.3+0.5+1.5 ≈ 4h）
  Agent B: mDNS/Bonjour + 实时带宽监控 + 报告导出（2.5+3+2 ≈ 7.5h）
  Agent C: SSH 终端（4-6h）
  Agent D: 仪表盘图表 + 数据导出增强 + 网络性能历史（2.5+1+4 ≈ 7.5h）

第二波（依赖 SSH 基础设施）:
  批量命令执行 + VNC 远程桌面（依赖 SSH/RFB 模块）
```

## 状态标记说明

- ✅ **已完成** — 已实现并合入
- 🔜 **待实现** — 用户确认要做，未开始
- 🆕 **新提议** — 刚加入，待讨论
- ⏸️ **延后** — 后续阶段再做
- 每个步骤完成后 `cargo check` / `npm run build` 确保编译通过
- 前端测试: `vitest run`
- Rust 测试: `cargo test`
- 修改 UI 的步骤需要 Tauri dev 手动验证
