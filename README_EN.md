# AzurePath

[![CI](https://github.com/chengccn/AzurePath/actions/workflows/ci.yml/badge.svg)](https://github.com/chengccn/AzurePath/actions/workflows/ci.yml)
![Rust](https://img.shields.io/badge/rust-stable-orange)
![Tauri](https://img.shields.io/badge/Tauri-2.0-purple)
![Vue](https://img.shields.io/badge/Vue-3.5-brightgreen)
![License](https://img.shields.io/badge/license-MIT-blue)

**AzurePath** is a cross-platform desktop intranet operations toolkit built with Tauri 2.0. It integrates network diagnostics, file transfer, instant messaging, clipboard management, device discovery, and more into a single application.

---

## Features

### 🔧 Network Diagnostics
- **📡 Ping** — ICMP ping with customizable count, interval, and timeout; cross-platform output parsing
- **🗺️ Traceroute** — Route tracing with per-hop latency measurement; Windows/Unix output parsing
- **🔍 Port Scan** — Concurrent TCP port scanning with configurable range and concurrency
- **🌐 DNS Lookup** — Supports A / AAAA / CNAME / MX / NS / SOA / TXT / ALL record types with custom DNS server
- **📡 Network Sniffer** — LAN device discovery, port scanning, service banner grabbing, OS fingerprinting, concurrent host scanning with NAT detection

### 💬 Communication & Collaboration
- **💬 LAN Chat** — Peer-to-peer instant messaging over LAN with automatic device discovery and OS notification support
- **📁 File Transfer** — Peer-to-peer file transfer within local network with dedicated management page and drag-and-drop support
- **📋 Clipboard Manager** — Clipboard history with persistent SQLite storage, search, favorites, image thumbnail preview, and LAN sync

### 🧰 Toolbox
- **Subnet Calculator** — IP/CIDR/subnet mask conversions
- **Base64 Encoder/Decoder** — Text to Base64 and back
- **URL Encoder/Decoder** — Percent-encoding with UTF-8 support
- **Hash Generator** — MD5/SHA1/SHA256/SHA512 via Web Crypto API
- **Port Lookup** — Quick reference for common TCP/UDP ports

### 📊 Dashboard
- **Activity Overview** — Aggregated view of recent clipboard, device discovery, and active nodes
- **History** — Three views (All / Favorites / Timeline) with search filtering and batch operations

### ⚙️ System Features
- **System Tray** — Minimize to system tray for background operation
- **OS Notifications** — Native notifications for file transfer completion, messages, and scan results
- **Global Shortcut** — `Ctrl+Alt+A` to bring window to front from anywhere
- **Keyboard Navigation** — `Ctrl+1~9` for page switching, `Ctrl+T` for theme toggling
- **Theme Switching** — Light / Dark / System with persistent preference
- **Auto Updater** — Powered by tauri-plugin-updater

---

## Tech Stack

| Layer | Technology |
|-------|------------|
| Desktop Framework | Tauri 2.0 |
| Backend | Rust (tokio async runtime) |
| Frontend | Vue 3 + TypeScript + Composition API |
| UI Components | shadcn-vue + Tailwind CSS v4 |
| State Management | Pinia |
| Routing | Vue Router |
| Database | SQLite (rusqlite, bundled) |
| Logging | tracing + tracing-subscriber |
| Testing | Rust: built-in test harness / Frontend: vitest + @vue/test-utils |

---

## Quick Start

### Prerequisites

- [Rust](https://www.rust-lang.org/) (stable)
- [Node.js](https://nodejs.org/) >= 18
- [Tauri 2.0 system dependencies](https://v2.tauri.app/start/prerequisites/)

### Development

```bash
# Install frontend dependencies
npm install

# Start Tauri dev mode (hot-reload for both frontend and backend)
npm run tauri dev
```

### Build

```bash
npm run tauri build
```

### Testing

```bash
# Frontend tests (52 test cases)
npm test

# Rust backend tests
cd src-tauri && cargo test

# Rust compilation check
cd src-tauri && cargo check

# TypeScript type checking
npx vue-tsc --noEmit
```

---

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+1` ~ `Ctrl+9` | Navigate to pages |
| `Ctrl+T` | Toggle light/dark theme |
| `Ctrl+D` | Go to Dashboard |
| `Ctrl+F` | Go to File Transfer |
| `Ctrl+Alt+A` | Bring window to front (global) |
| `Escape` | Close modal / cancel |

---

## Architecture

The project follows a classic three-layer architecture:

```
src/                          # Frontend source (Vue 3 + TypeScript)
├── components/               # Shared UI components
│   ├── layout/               # Layout (AppShell, TitleBar, Sidebar)
│   └── ui/                   # Base UI primitives
├── composables/              # Reusable composition functions
├── pages/                    # Feature pages
│   ├── dashboard/
│   ├── ping/
│   ├── traceroute/
│   ├── port-scan/
│   ├── dns/
│   ├── chat/
│   ├── clipboard/
│   ├── files/
│   ├── network-sniffer/
│   ├── history/
│   └── toolbox/
├── lib/                      # Utilities and Tauri bindings
│   ├── tauri.ts              # Tauri invoke/event wrappers
│   └── format.ts             # Formatting helpers
├── router/                   # Route configuration
└── stores/                   # Pinia state management

src-tauri/                    # Backend source (Rust)
├── src/
│   ├── commands/             # #[tauri::command] handlers
│   ├── core/                 # Business logic
│   │   ├── ping/
│   │   ├── traceroute/
│   │   ├── port_scan/
│   │   ├── dns/
│   │   ├── chat/
│   │   ├── clipboard/
│   │   ├── connection/
│   │   ├── discovery/
│   │   ├── file_transfer/
│   │   ├── file_server/
│   │   ├── network_sniffer/
│   │   ├── utils.rs
│   │   └── settings.rs
│   └── types/                # Data models
├── capabilities/             # Tauri v2 permissions
└── Cargo.toml

docs/                         # Design documents
```

### Layer Responsibilities

1. **`types/`** — Serializable data models with `#[serde(rename_all = "camelCase")]` for consistent naming between Rust and TypeScript
2. **`core/`** — Pure business logic: computation, parsing, async I/O. No Tauri API dependency
3. **`commands/`** — `#[tauri::command]` wrappers handling validation, calling core, and emitting Tauri events

---

## License

MIT
