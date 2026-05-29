# AGENTS.md — AzurePath

Tauri 2.0 desktop app: intranet operations toolbox. Rust backend + Vue 3/TypeScript frontend.

## Build & Test

```bash
# Frontend — Bun + Vite hybrid (Bun cannot compile Vue SFCs)
bun install                           # install deps (dev)
npm ci                                # install deps (CI only)
bun run build                         # vite build → dist/
bun run dev                           # vite build && bun serve.ts (port 1420)
bun run test                          # vitest run (jsdom env)

# Backend
cd src-tauri && cargo check           # fast type-check
cd src-tauri && cargo clippy          # lints
cd src-tauri && cargo test --tests    # OK; --lib crashes on Windows (sqlx-postgres DLL)
cd src-tauri && cargo test -- <name>  # single test
cd src-tauri && cargo run --example ssh_test  # standalone binary

# Full Tauri
bun run tauri dev                     # vite build + bun serve + Rust backend
bun run tauri build                   # production (NSIS/MSI on Windows)
```

## Three-Layer Rust (every feature)

| Layer | Location | Role |
|-------|----------|------|
| Types | `src-tauri/src/types/<feature>.rs` | `#[derive(Serialize, Deserialize)]` models, `#[serde(rename_all = "camelCase")]` |
| Core | `src-tauri/src/core/<feature>/mod.rs` | Business logic, no Tauri dependency, testable in isolation |
| Commands | `src-tauri/src/commands/<feature>.rs` | `#[tauri::command]` wrappers, validation, event emission |

**New feature MUST register in 4 files:**
1. `types/mod.rs` — `pub mod <feature>;`
2. `core/mod.rs` — `pub mod <feature>;`
3. `commands/mod.rs` — `pub mod <feature>;`
4. `lib.rs` — add to `tauri::generate_handler![...]`

Missing any one → compile error.

## Cancellation (long-running tasks)

- **New pattern**: `CANCEL_REGISTRY` (`core/cancel.rs`) — `register(task_id)` returns `CancelToken(Arc<AtomicBool>)`, task polls `token.is_cancelled()`, stop command calls `CANCEL_REGISTRY.cancel(task_id)`
- **Legacy pattern** (ping, traceroute, port_scan, network_sniffer): `CANCEL_TOKENS: LazyLock<Mutex<HashMap<String, bool>>>`
- Both live in `core/`. Unregister/cleanup after task completes.

## Event-Driven Progress

Async commands emit progress via `app.emit("feature:event", payload)`.
Frontend subscribes with `listen("feature:event", callback)` from `@tauri-apps/api/event`.
Unlisten fns stored per-feature in Pinia stores.

**Store listener lifecycle pattern** (ping, mtr, portScan, traceroute, apiTest stores):
```
attachListeners() → detach previous, await onEvent(cb) for each event
detachListeners() → unlisten?.() for each, set to null
```
Called on start/stop of the backing command. Stores without events use `invoke()` only (theme, toast, bookmark, settings, remoteShell, remoteDesktop, targetGroup, database, preset, commandPalette).

## Frontend Testing (vitest + jsdom)

All frontend tests in `src/__tests__/` mirror the `src/` structure. Common patterns:

```typescript
// Store test: mock @/lib/tauri, create Pinia in beforeEach
vi.mock("@/lib/tauri", () => ({
  pingStart: vi.fn(() => Promise.resolve("task-123")),
  onPingProgress: vi.fn(() => Promise.resolve(vi.fn())),
}));
beforeEach(() => { setActivePinia(createPinia()); });

// Page test: mock both @/lib/tauri and @tauri-apps/api/core
vi.mock("@/lib/tauri", () => ({}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(() => Promise.resolve([])) }));
```

## Frontend Architecture

| Directory | Content |
|-----------|---------|
| `src/pages/<feature>/Page.vue` | One per feature, lazy-loaded by router |
| `src/stores/<feature>.ts` | Pinia stores wrapping `invoke()` calls + event listeners |
| `src/lib/tauri.ts` | ~1800 line typed `invoke()` wrappers (197 fns) + 44 `onFeatureEvent()` listeners; TS interfaces mirror Rust camelCase |
| `src/components/layout/` | AppShell, Sidebar, TitleBar |
| `src/components/<feature>/` | Feature-specific components (remote-shell/, remote-desktop/, network-sniffer/, etc.) |
| `src/components/ui/` | Generic UI primitives (button/Button.vue uses cva + cn) |
| `src/composables/` | useKeyboardShortcuts, useFileTransfer, useNotification, useUpdateChecker |
| `src/lib/` | `tauri.ts` (invoke wrappers), `format.ts` (locale-aware formatTime/formatSize), `utils.ts` (cn()), `export.ts` (CSV/file save) |
| `src/router/index.ts` | Vue Router with `() => import(...)` lazy routes; **known duplicate `/remote-shell` entry at lines 26-29 and 151-154** |
| `src/__tests__/` | 41 frontend test files (vitest + jsdom), each page/store/composable/lib has a `.test.ts` |

## Rust Module Structure

- **186 `#[tauri::command]` functions** across 38 command files
- ~37 core features, each in `core/<feature>/` (65 .rs files total)
- Complex features use submodules: `core/remote_shell/{ssh,telnet,session_store}`, `core/remote_desktop/{rdp,vnc,session_store,frame_encoder,desktop_client}`, `core/network_sniffer/{discovery,port_scanner,banner,fingerprint,os_detect}`
- Tests are inline `#[cfg(test)]` mod blocks (not a separate `tests/` dir), found in ~68 files across core + commands
- 4 standalone example binaries: `src-tauri/examples/{ssh_test,mysql_test,session_store_test,db_check}.rs`

## Design System (CSS Custom Properties)

Custom theme via Tailwind v4 `@theme` directive in `src/style.css`. All colors use semantic names, not Tailwind defaults:

| Variable | Light | Dark | Usage |
|----------|-------|------|-------|
| `--color-paper` | `#f6f3ec` | `#222120` | Page background |
| `--color-ink` | `#1a1a18` | `#e5e1da` | Text color |
| `--color-bamboo` | `#2d5a3d` | `#4faa70` | Accent/primary |
| `--color-stone` | `#6b6b62` | `#8a8880` | Secondary text |

Dark theme toggled via `data-theme="dark"` on `<html>`. Theme transition class `.theme-transition` is applied for smooth switching. 6 custom keyframe animations defined (fade-up, fade-in, scale-in, slide-up, view-fade, status-fade).

## UI Components & Styling

- **Icons**: `lucide-vue-next` — used in 60+ components
- **Accessible primitives**: `radix-vue` — dialogs, dropdowns, selects, modals (60+ import sites)
- **Button pattern**: `components/ui/button/Button.vue` uses `class-variance-authority` (cva) + `tailwind-merge` (cn) + `clsx`. Variants: default, secondary, ghost, outline, danger. Sizes: default, sm, lg, icon. New UI components should follow this cva + cn() pattern.
- **Custom utility classes** in `style.css`: `.noise-bg` (subtle texture overlay), `.scrollbar-hidden` (hide scrollbar), `.ease-out-expo` easing curve
- **Animation classes**: `.animate-fade-up` (0.55s), `.animate-fade-in` (0.25s), `.animate-scale-in` (0.45s), `.animate-slide-up` (0.3s), `.animate-view-fade` (0.72s), `.animate-status-fade` (0.25s)

## Data flow pattern

Rust `#[derive(Serialize, Deserialize)]` + `#[serde(rename_all = "camelCase")]` → TS interfaces (snake_case event payloads, camelCase frontend) → `invoke()` command → Pinia store → Vue component.

## Tooling & Config

- **TypeScript**: strict mode, `noUnusedLocals`, `noUnusedParameters`
- **Biome**: linter + formatter, 2-space indent, double quotes, semicolons
- **Tailwind CSS v4**: via `@tailwindcss/vite` plugin
- **Vue 3**: `<script setup>` + Composition API throughout
- **Window**: undecorated, 1180×760 default, min 900×620, mounts to `<div id="root">`
- **Styling background**: `html, body, #root { background: transparent }`

## CI Pipeline (`.github/workflows/ci.yml`)

- **Frontend** (ubuntu): `npm ci` → `npx vue-tsc --noEmit` → `npx vitest run`
- **Rust** (windows-latest): `cargo check` → `cargo test`

## Gotchas

- **`cargo test --lib` crashes on Windows**: `STATUS_ENTRYPOINT_NOT_FOUND (0xc0000139)` from sqlx-postgres native DLL — pre-existing, not your changes. Use `cargo test --tests` to skip lib tests.
- **WebView2 + crossorigin**: WebView2 on Windows fails on `<script type="module" crossorigin>`. Vite config has a custom `removeCrossOrigin()` plugin to strip it. If page loads in browser but white screen in Tauri, check `dist/index.html` for stray `crossorigin`.
- **Dev server**: Vite 7 dev server does NOT work on Node.js ≥ 24. Workaround is Bun HTTP server (`serve.ts`) serving the Vite build output. Always `vite build` before `bun run serve.ts`.
- **Global shortcut**: Ctrl+Alt+A registered at startup. "HotKey already registered" error is logged but non-fatal — previous process didn't clean up the OS-level hotkey.
- **Remote shell SSH tests**: require real SSH server. Credentials read from env vars `SSH_PASS`, `MYSQL_PASS`.
- **Build `npm run dev`**: runs `vite build && bun run serve.ts` — it does NOT use Vite's dev server. `bun run build` = `vite build` only.
- **Tracing/logging**: Configured in `src-tauri/src/main.rs` — env filter via `AZUREPATH_LOG` env var (default `info`), emits to stdout + in-app `LogLayer` for the log viewer.
