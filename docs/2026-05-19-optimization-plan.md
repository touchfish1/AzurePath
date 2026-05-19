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

## 状态标记说明

- ✅ **已完成** — 已实现并合入
- 🔜 **待实现** — 用户确认要做，未开始
- 🆕 **新提议** — 刚加入，待讨论
- ⏸️ **延后** — 后续阶段再做
- 每个步骤完成后 `cargo check` / `npm run build` 确保编译通过
- 前端测试: `vitest run`
- Rust 测试: `cargo test`
- 修改 UI 的步骤需要 Tauri dev 手动验证
