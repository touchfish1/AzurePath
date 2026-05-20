# Developer Toolbox — Design Spec

## Overview

Add a new "Developer Toolbox" page to AzurePath, complementing the existing Network Toolbox page. Contains 17 lightweight developer utility tools organized into 3 categories. All processing is done in the frontend (TypeScript) using native APIs and well-established JS libraries — no Rust backend required.

## Route & Layout

- **Route**: `/dev-tools` (lazy-loaded)
- **Layout**: Left sidebar with category/tool tree + right content area rendered by `<component :is="">` based on selected tool
- **State**: Pinia store tracks `selectedTool`; each tool managed by local reactive state (no per-tool stores needed)

## Tool Categories & Tools

### 1. Data Format Tools

| Tool | Dependencies | Description |
|------|-------------|-------------|
| JSON Formatter | None (native `JSON`) | Format, validate, minify JSON input with error display |
| YAML Formatter | `js-yaml` | Parse, validate, dump YAML |
| TOML Formatter | `smol-toml` | Parse, validate, stringify TOML |
| Format Converter | `js-yaml`, `smol-toml` | Convert between JSON ↔ YAML ↔ TOML (any direction) |
| Base64 Codec | None (native `btoa`/`atob`) | Encode/decode Base64; optional UTF-8 support via `TextEncoder` |
| Hex Codec | None (native) | Encode/decode hex strings |
| URL Codec | None (native) | URL encode/decode with component vs full URI mode toggle |
| HTML Entity Codec | None (native) | Encode/decode HTML entities (`&amp;`, `&lt;`, etc.) |

### 2. Development Tools

| Tool | Dependencies | Description |
|------|-------------|-------------|
| JWT Decoder | None (Base64 decode) | Decode JWT header + payload; display as formatted JSON |
| Cron Expression | `cronstrue`, `cron-parser` | Input cron → show human-readable description + next N fire times |
| UUID Generator | None (`crypto.randomUUID()`) | Generate v4 UUIDs; bulk mode (1-100), copy with one click |
| Timestamp Converter | None (native `Date`) | Convert between Unix seconds/milliseconds and human date; both directions |
| Hash Generator | `crypto-js` | MD5, SHA1, SHA224, SHA256, SHA384, SHA512; hex output |

### 3. Code Tools

| Tool | Dependencies | Description |
|------|-------------|-------------|
| Text Diff | `diff` | Side-by-side or unified diff of two text inputs; line-level highlighting |
| Naming Converter | None (regex) | Convert between camelCase, snake_case, kebab-case, PascalCase, UPPER_CASE |
| SQL Formatter | `sql-formatter` | Format SQL with configurable indentation and dialect |
| Regex Tester | None (native `RegExp`) | Test regex with input text; show match groups, flags toggle, error display |

## Component Structure

```
src/pages/dev-tools/
├── Page.vue              # Main layout: sidebar + tool area
├── components/
│   ├── sidebar/
│   │   └── DevToolSidebar.vue    # Category collapsible groups + tool list
│   ├── DataFormat/
│   │   ├── JsonFormatter.vue
│   │   ├── YamlFormatter.vue
│   │   ├── TomlFormatter.vue
│   │   ├── FormatConverter.vue
│   │   ├── Base64Codec.vue
│   │   ├── HexCodec.vue
│   │   ├── UrlCodec.vue
│   │   └── HtmlEntityCodec.vue
│   ├── DevTools/
│   │   ├── JwtDecoder.vue
│   │   ├── CronExpression.vue
│   │   ├── UuidGenerator.vue
│   │   ├── TimestampConverter.vue
│   │   └── HashGenerator.vue
│   └── CodeTools/
│       ├── TextDiff.vue
│       ├── NamingConverter.vue
│       ├── SqlFormatter.vue
│       └── RegexTester.vue
└── stores/
    └── index.ts           # DevToolStore: selectedTool, tool list metadata
```

## Data Flow

- `DevToolStore` holds `selectedTool: string` (the currently active tool ID) and a static `toolList` array with category/tool metadata
- `Page.vue` uses `selectedTool` to load the corresponding component via `<component :is="currentComponent" />`
- Each tool component is fully self-contained: input → reactive transform → output. No cross-tool state.
- Utility functions (shared codec helpers) live in `src/utils/dev-tools.ts`

## Dependencies

```json
{
  "js-yaml": "^4.1.3",
  "smol-toml": "^1.3.1",
  "cronstrue": "^2.50.0",
  "cron-parser": "^4.9.0",
  "diff": "^5.2.0",
  "sql-formatter": "^15.4.0",
  "crypto-js": "^4.2.0"
}
```

## Implementation Order

Phase 1 — Setup: route, store, sidebar layout, install deps
Phase 2 — Data Format (8 tools)
Phase 3 — Debug Tools (5 tools)
Phase 4 — Code Tools (4 tools)

Each tool is independent and can be implemented in parallel.

## UI Design Principles

- Clean, minimal: textarea input + formatted output area (side-by-side or top-bottom depending on tool)
- Real-time: results update on every input change (debounced 300ms for SQL formatter)
- Copy-to-clipboard button on all output areas
- Error handling: parse errors shown inline in red, not as alerts
- Dark/light theme consistent with the rest of AzurePath (Tailwind)
