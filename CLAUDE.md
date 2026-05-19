# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
# Backend (Rust)
cd src-tauri && cargo build
cd src-tauri && cargo test            # all tests
cd src-tauri && cargo test -- <name>  # single test (e.g. `test_windows_success`)
cd src-tauri && cargo check           # fast type-check only

# Frontend (Vue 3 + TypeScript)
npm install
npm run dev                           # Vite dev server (port 1420)
npm run build                         # type-check + Vite build
npm run tauri dev                     # full Tauri dev (frontend + Rust)
npm run tauri build                   # production build
```

## Architecture

**Tauri 2.0** desktop app (Rust backend + Vue 3/TypeScript frontend). Project name: AzurePath — an intranet operations toolbox.

### Three-Layer Rust Structure

Each feature (ping, traceroute, port_scan, dns, chat, clipboard, network_sniffer, file_transfer, discovery, lan) follows this pattern:

- **`src-tauri/src/types/<feature>.rs`** — Serializable data models (`#[derive(Serialize, Deserialize)]`, `#[serde(rename_all = "camelCase")]` for TS interop)
- **`src-tauri/src/core/<feature>/`** — Business logic (pure computation, parsing, async I/O with tokio)
- **`src-tauri/src/commands/<feature>.rs`** — `#[tauri::command]` wrappers that spawn background tasks and emit events via `app.emit()`

Module trees:
- `src-tauri/src/lib.rs` — Tauri builder, plugin registration, `generate_handler![]` listing all commands
- `src-tauri/src/main.rs` — Thin entry point calling `azurepath_lib::run()`
- `src-tauri/src/core/mod.rs` — Declares all core modules (includes `connection/` for LAN protocol and `file_server/` for HTTP file serving)
- `src-tauri/src/types/mod.rs` — Declares all type modules
- `src-tauri/src/commands/mod.rs` — Declares all command modules

### Cancellation Pattern

Long-running commands (ping, traceroute, port_scan, network_sniffer) use a global `CANCEL_TOKENS: LazyLock<Mutex<HashMap<String, bool>>>`. Each task gets a UUID. The stop command sets the flag; the running task polls it.

### Event-Driven Progress

Async commands stream progress to the frontend via Tauri events (`app.emit("ping:progress", payload)`). The frontend subscribes with `listen()` from `@tauri-apps/api/event`.

### Frontend

- **Vue 3** with `<script setup>` + Composition API
- **Pinia stores** in `src/stores/` — one per feature, wrap Tauri invoke calls and manage event listeners
- **`src/lib/tauri.ts`** — Single file with all `invoke()` wrappers and event listener helpers (TS interfaces mirror Rust types)
- **`src/pages/`** — One page per feature (dashboard, ping, traceroute, port-scan, dns, chat, clipboard, network-sniffer)
- **`src/router/index.ts`** — Vue Router with lazy-loaded routes
- **Tailwind CSS v4** via `@tailwindcss/vite`
- **UI Components** in `src/components/` (layout shell, sidebar, title bar, reusable button)
- **Window**: undecorated (`decorations: false`), 1180x760 default, min 900x620
- **Dev server**: `http://localhost:1420`

### Notable Design Decisions

- Windows locale support: ping output parser handles both English and Chinese locale strings
- Ping uses system `ping` command (not raw ICMP sockets) via `tokio::process::Command`
- Clipboard monitoring uses polling with configurable interval
- LAN features (chat, file transfer, discovery) use a custom TCP protocol (`core/connection/protocol.rs`)
- Network sniffer: concurrent host scan with NAT detection, OS fingerprinting, banner grabbing, and service fingerprint database
- SQLite via `rusqlite` (bundled) for clipboard and chat history persistence
