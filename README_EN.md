# AzurePath

AzurePath is a cross-platform desktop intranet operations toolkit built with Tauri 2.0.

## Features

- **📡 Ping** — ICMP ping with customizable count, interval, and timeout
- **🗺️ Traceroute** — Route tracing with per-hop latency measurement
- **🔍 Port Scan** — Concurrent TCP port scanning with configurable range and concurrency
- **🌐 DNS Lookup** — Supports A / AAAA / CNAME / MX / NS / SOA / TXT / ALL record types
- **💬 LAN Chat** — Peer discovery and instant messaging over LAN
- **📁 File Transfer** — Peer-to-peer file transfer within local network
- **📋 Clipboard Manager** — Clipboard history with persistent storage, search, favorites, and LAN sync
- **📡 Network Sniffer** — LAN device discovery, port scanning, and service version detection (in development)

## Tech Stack

| Layer | Technology |
|-------|------------|
| Desktop Framework | Tauri 2.0 |
| Backend | Rust (tokio async runtime) |
| Frontend | Vue 3 + TypeScript |
| UI Components | shadcn-vue + Tailwind CSS |
| State Management | Pinia |
| Routing | Vue Router |
| Database | SQLite (rusqlite) |

## Development

### Prerequisites

- [Rust](https://www.rust-lang.org/) (stable)
- [Node.js](https://nodejs.org/) >= 18
- System dependencies as specified in Tauri 2.0 documentation

### Getting Started

```bash
# Install frontend dependencies
npm install

# Start development server
npm run tauri dev
```

### Build

```bash
npm run tauri build
```

## Project Structure

```
azurepath/
├── src/                          # Frontend source (Vue 3 + TS)
│   ├── components/               # Shared UI components
│   ├── pages/                    # Page components
│   ├── lib/                      # Utilities and Tauri bindings
│   ├── router/                   # Route configuration
│   ├── stores/                   # Pinia state management
│   └── App.vue                   # Root component
├── src-tauri/                    # Backend source (Rust)
│   ├── src/
│   │   ├── commands/             # Tauri command handlers
│   │   ├── core/                 # Core network engine
│   │   └── types/                # Data models
│   └── Cargo.toml
└── docs/                         # Design documents and plans
```

## License

MIT
