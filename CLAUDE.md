# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
# Backend (Rust)
cd src-tauri && cargo build
cd src-tauri && cargo test                    # all tests
cd src-tauri && cargo test -- <test_name>     # single test
cd src-tauri && cargo test -- --nocapture     # with stdout
cd src-tauri && cargo check                   # fast type-check
cd src-tauri && cargo clippy                  # lints

# Backend examples (standalone binaries, not #[cfg(test)])
cd src-tauri && cargo run --example ssh_test
cd src-tauri && cargo run --example session_store_test

# Frontend (Vue 3 + TypeScript) — Bun + Vite hybrid
bun install
bun run build                    # vite build (compiles Vue SFCs, Tailwind)
bun run dev                      # vite build + bun serve.ts on port 1420
bun run test                     # vitest run
bun run tauri dev                # full Tauri dev (Vite build + Bun server + Rust backend)
bun run tauri build              # production build
```

## Architecture

**Tauri 2.0** desktop app (Rust backend + Vue 3/TypeScript frontend). Project: AzurePath — an intranet operations toolbox (网络运维工具箱). 40+ tool pages organized by function.

### Three-Layer Rust Structure

Every feature follows this pattern:

- **`src-tauri/src/types/<feature>.rs`** — Serializable models (`#[derive(Serialize, Deserialize)]`, `#[serde(rename_all = "camelCase")]` for TS interop). Some features use submodules (e.g. `types/remote_shell/`).
- **`src-tauri/src/core/<feature>/`** — Business logic (parsing, async I/O with tokio, SSH, protocol handling). Each feature is a module directory with `mod.rs`.
- **`src-tauri/src/commands/<feature>.rs`** — `#[tauri::command]` fn wrappers registered in `lib.rs` `generate_handler![]`.

Module registration chain: `lib.rs` → `core/mod.rs`, `commands/mod.rs`, `types/mod.rs` — each declares submodules. When adding a new feature, all three files must be updated.

### Cancellation Pattern

Long-running commands use `CancelRegistry` (`core/cancel.rs`):
- `CANCEL_REGISTRY.register(task_id)` returns a `CancelToken(Arc<AtomicBool>)`
- The running task polls `token.is_cancelled()` in its loop
- The stop command calls `CANCEL_REGISTRY.cancel(task_id)`, which sets the flag

Older features (ping, traceroute, port_scan, network_sniffer) use a legacy `CANCEL_TOKENS: LazyLock<Mutex<HashMap<String, bool>>>` instead — same concept, different implementation.

### Event-Driven Progress

Async commands stream progress to the frontend via Tauri events: `app.emit("feature:event", payload)`. The frontend subscribes with `listen()` from `@tauri-apps/api/event`. Unlisten fns are stored per-feature in Pinia stores.

### Frontend Architecture

- **Vue 3** — `<script setup>` + Composition API throughout
- **Pinia stores** in `src/stores/` — one per feature, wrap `invoke()` calls and manage event listeners
- **`src/lib/tauri.ts`** — ~1800-line file with typed `invoke()` wrappers and event `listen()` helpers. TS interfaces mirror Rust `#[derive(Serialize)]` types with camelCase
- **`src/router/index.ts`** — Vue Router with lazy-loaded `() => import(...)` routes. All pages under `/src/pages/<feature>/Page.vue`
- **`src/components/`** — Shared UI: `layout/` (AppShell, Sidebar, TitleBar), `ui/` (Button), plus per-feature component dirs (`remote-shell/`, `remote-desktop/`, etc.)
- **Tailwind CSS v4** via `@tailwindcss/vite`
- **Dev server**: `http://localhost:1420`, Tauri WebView loads this URL in dev mode
- **Window**: undecorated, 1180x760 default, min 900x620
- **`src/App.vue`** mounts to `#root` div in `index.html` (transparent background)
- **`src/composables/`** — shared composables (`useKeyboardShortcuts`, `useFileTransfer`, `useNotification`, `useUpdateChecker`)
- **Build**: Vite (`@vitejs/plugin-vue` for Vue SFCs, `@tailwindcss/vite` for Tailwind), served by Bun's `serve.ts`
- **Dev server**: Bun HTTP server (`serve.ts`) serves `dist/` on port 1420; `vite build` must be run before `serve.ts`

### Features Overview

| Area | Pages | Backend |
|------|-------|---------|
| Network Tools | ping, traceroute, mtr, dns, whois, subnetcalc | System ping/trace commands, DNS resolution |
| Port & Scan | port-scan, network-sniffer | TCP connect scan, concurrent host scan with OS fingerprinting/banner grab |
| Security | ssl-check, http-check, mac-lookup | SSL cert validation, HTTP status check, MAC vendor lookup |
| Remote Access | remote-shell (SSH/Telnet + SFTP + DB), remote-desktop (RDP/VNC) | ssh2, tokio-telnet, ironrdp, sqlx (mysql/postgres), redis |
| LAN Tools | chat, file-transfer, discovery | Custom TCP protocol (`core/connection/protocol.rs`) |
| Monitoring | monitor, bandwidth, mdns, snmp, topology | ICMP/tokio timers, sys probe, mdns-sd, SNMP v2c |
| System | clipboard, history, logs, backup/restore | SQLite via rusqlite (bundled) |
| Developer Tools | dev-tools (17 tools), api-test, speedtest, bookmarks | JSON/cron/hash/codec utils, reqwest, iperf3 |
| Config | settings, target-groups, presets, wol | JSON file storage, Wake-on-LAN magic packet |

### Notable Rust Dependencies

- **ssh2** (libssh2) for SSH client — PTY shell with worker thread pattern (`core/remote_shell/ssh.rs`)
- **ironrdp** for RDP client (`core/remote_desktop/`)
- **sqlx** (mysql, postgres) + **redis** for database connections via remote-shell
- **rusqlite** (bundled) for local persistence — clipboard history, chat history, remote shell sessions (5 tables: remote_sessions, remote_session_secrets, environments, db_connections, db_connection_secrets)
- **snmp2** for SNMP polling
- **reqwest** for HTTP checks and API testing
- **tokio-tungstenite** for WebSocket support

### Common Gotchas

- **Module registration**: When adding a new Rust feature, you MUST register it in THREE places: `types/mod.rs`, `core/mod.rs`, and `commands/mod.rs`, plus `lib.rs` `generate_handler![]`. Missing any one causes compile errors.
- **Router duplicates**: `src/router/index.ts` has a known duplicate `/remote-shell` route entry — if adding new routes, check for existing duplicates.
- **Global shortcut**: `Ctrl+Alt+A` is registered for show/focus. A "HotKey already registered" error is caught and logged (not fatal) — happens when the previous process didn't clean up the OS-level hotkey.
- **`cargo test --lib` crash on Windows**: `STATUS_ENTRYPOINT_NOT_FOUND (0xc0000139)` is a pre-existing issue with sqlx-postgres native DLL — not caused by code changes. Run `cargo test --tests` to exclude lib tests.
- **Remote shell SSH tests**: require a real SSH server; credentials read from env vars (`SSH_PASS`, `MYSQL_PASS`)
- **Frontend deps**: `@xterm/xterm` is used by the remote shell terminal component
- **Build pipeline**: Bun can't compile Vue SFCs — Vite (`@vitejs/plugin-vue`) handles this. `bun run build` = `vite build`. `bun run dev` = `vite build && bun run serve.ts`.
- **WebView2 + crossorigin**: WebView2 on Windows fails to execute `<script type="module" crossorigin>`. Vite config sets `build.crossOrigin: ""` to remove the attribute. If you see a white screen and the page loads OK in a browser, check if `crossorigin` reappeared in `dist/index.html`.
- **Dev server**: Vite 7 dev server does NOT respond to HTTP requests on Node.js 24. The workaround (Bun HTTP server + `vite build`) is the default setup.
