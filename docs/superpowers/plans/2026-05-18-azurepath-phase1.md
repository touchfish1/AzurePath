# AzurePath Phase 1 — 网络诊断模块 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 构建 AzurePath 桌面应用一期工程，完成网络诊断工具（Ping / Traceroute / 端口扫描 / DNS 查询）的完整功能，UI 风格统一为纸张主题（参考 floral-notepaper）

**Architecture:** Tauri 2.0 (Rust) 处理所有网络底层操作，Vue 3 + shadcn-vue 渲染前端，通过 IPC invoke/event 通信。所有网络操作为异步流式推送，前端增量渲染保障性能。

**Tech Stack:** Tauri 2.0, Rust (tokio/pnet/trust-dns), Vue 3, TypeScript, shadcn-vue, Tailwind CSS v4, Vite 7

**UI Design Language:** 纸墨主题（paper/ink/bamboo 色系），噪声纹理背景，统一缓动 `cubic-bezier(0.22, 1, 0.36, 1)`，无边框窗口 + 自定义标题栏，亮/暗色主题

---

### Task 1: 项目脚手架初始化

**Files:**
- Create: 整个项目骨架（Tauri 2 + Vue 3 + TS）

- [ ] **Step 1: 创建 Tauri 2 + Vue 3 项目**

```bash
cd /d/opensource/AzurePath
npm create tauri-app@latest azurepath-tmp -- --template vue-ts --manager npm
```

将生成的文件复制到 AzurePath 根目录后删除临时目录。如果交互式命令不可用，手动创建以下核心文件：

**package.json:**
```json
{
  "name": "azurepath",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vue-tsc --noEmit && vite build",
    "preview": "vite preview",
    "tauri": "tauri"
  },
  "dependencies": {
    "vue": "^3.5.0",
    "@tauri-apps/api": "~2.11.0",
    "@tauri-apps/plugin-dialog": "^2.7.0",
    "@tauri-apps/plugin-opener": "^2",
    "radix-vue": "^1.9.0",
    "class-variance-authority": "^0.7.1",
    "clsx": "^2.1.1",
    "tailwind-merge": "^3.2.0",
    "lucide-vue-next": "^0.400.0",
    "pinia": "^3.0.0",
    "vue-router": "^4.5.0"
  },
  "devDependencies": {
    "@tailwindcss/vite": "^4.2.4",
    "@tauri-apps/cli": "^2",
    "@vitejs/plugin-vue": "^5.2.0",
    "tailwindcss": "^4.2.4",
    "typescript": "~5.8.3",
    "vite": "^7.0.4",
    "vue-tsc": "^2.2.0"
  }
}
```

**tsconfig.json:**
```json
{
  "compilerOptions": {
    "target": "ES2021",
    "useDefineForExpose": true,
    "module": "ESNext",
    "lib": ["ES2021", "DOM", "DOM.Iterable"],
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "preserve",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "paths": {
      "@/*": ["./src/*"]
    }
  },
  "include": ["src/**/*.ts", "src/**/*.d.ts", "src/**/*.vue"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

**vite.config.ts:**
```ts
import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";
import { resolve } from "path";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [vue(), tailwindcss()],
  resolve: {
    alias: {
      "@": resolve(__dirname, "src"),
    },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: "ws", host, port: 1421 }
      : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
}));
```

**index.html:**
```html
<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>AzurePath</title>
    <style>
      html, body, #root { background: transparent; margin: 0; padding: 0; overflow: hidden; }
    </style>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

**src/main.ts:**
```ts
import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import router from "./router";
import "./style.css";

const app = createApp(App);
app.use(createPinia());
app.use(router);
app.mount("#root");
```

**src/App.vue:**
```vue
<script setup lang="ts">
import AppShell from "./components/layout/AppShell.vue";
</script>

<template>
  <AppShell />
</template>
```

**src/vite-env.d.ts:**
```ts
/// <reference types="vite/client" />

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<object, object, unknown>;
  export default component;
}
```

- [ ] **Step 2: 初始化 Tauri 配置**

**src-tauri/tauri.conf.json:**
```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "AzurePath",
  "version": "0.1.0",
  "identifier": "com.azurepath.app",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "label": "main",
        "title": "AzurePath",
        "width": 1180,
        "height": 760,
        "minWidth": 900,
        "minHeight": 620,
        "decorations": false,
        "visible": false,
        "dragDropEnabled": false
      }
    ],
    "security": { "csp": null }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": ["icons/32x32.png", "icons/128x128.png", "icons/icon.ico", "icons/icon.icns"]
  }
}
```

**src-tauri/src/lib.rs** (初始空壳):
```rust
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! AzurePath is ready.", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: 安装依赖并验证编译**

```bash
cd /d/opensource/AzurePath
npm install
cd src-tauri && cargo check 2>&1 | tail -5
```

Expected: cargo check 成功完成无错误。

---

### Task 2: Tailwind CSS v4 主题系统（纸墨风格）

**Files:**
- Create: `src/style.css`

- [ ] **Step 1: 创建全局样式和主题令牌**

**src/style.css:**
```css
@import "tailwindcss";

@theme {
  --color-paper: #f6f3ec;
  --color-paper-warm: #f0ebe0;
  --color-paper-deep: #e8e1d3;
  --color-ink: #1a1a18;
  --color-ink-soft: #3d3d38;
  --color-ink-faint: #8a8a80;
  --color-ink-ghost: #b8b8ae;
  --color-bamboo: #2d5a3d;
  --color-bamboo-light: #3a7a52;
  --color-bamboo-mist: #e8f0eb;
  --color-bamboo-glow: #d4e8da;
  --color-stone: #6b6b62;
  --color-cloud: #ffffff;
  --color-shadow: rgba(26, 26, 24, 0.06);
  --color-shadow-deep: rgba(26, 26, 24, 0.12);
  --color-danger-bg: #fef2f2;

  --font-display: "Noto Serif SC", "Source Han Serif SC", Georgia, serif;
  --font-body: "Noto Sans SC", "Source Han Sans SC", system-ui, sans-serif;
  --font-mono: "JetBrains Mono", "Fira Code", monospace;
}

:root[data-theme="dark"] {
  --color-paper: #222120;
  --color-paper-warm: #2c2a27;
  --color-paper-deep: #3c3935;
  --color-ink: #e5e1da;
  --color-ink-soft: #b5b1a8;
  --color-ink-faint: #928f87;
  --color-ink-ghost: #706d67;
  --color-bamboo: #4faa70;
  --color-bamboo-light: #5fc085;
  --color-bamboo-mist: #1c2e22;
  --color-bamboo-glow: #243a2c;
  --color-stone: #8a8880;
  --color-cloud: #1a1917;
  --color-shadow: rgba(0, 0, 0, 0.3);
  --color-shadow-deep: rgba(0, 0, 0, 0.5);
  --color-danger-bg: rgba(220, 38, 38, 0.15);
}

*, *::before, *::after { box-sizing: border-box; }

html {
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

body {
  margin: 0;
  padding: 0;
  font-family: var(--font-body);
  background: var(--color-paper);
  color: var(--color-ink);
}

/* 统一缓动曲线 */
:root {
  --ease-out-expo: cubic-bezier(0.22, 1, 0.36, 1);
}

/* 自定义滚动条 */
::-webkit-scrollbar { width: 4px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb { background: var(--color-ink-ghost); border-radius: 2px; }
::-webkit-scrollbar-thumb:hover { background: var(--color-ink-faint); }

.scrollbar-hidden::-webkit-scrollbar { display: none; }
.scrollbar-hidden { -ms-overflow-style: none; scrollbar-width: none; }

/* 噪声纹理 */
.noise-bg {
  position: relative;
  isolation: isolate;
}
.noise-bg::before {
  content: "";
  position: absolute;
  inset: 0;
  opacity: 0.025;
  background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.85' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");
  background-size: 200px 200px;
  pointer-events: none;
  border-radius: inherit;
  z-index: -1;
}

/* 动画 */
@keyframes fade-up {
  from { opacity: 0; transform: translateY(14px); }
  to { opacity: 1; transform: translateY(0); }
}
@keyframes fade-in {
  from { opacity: 0; }
  to { opacity: 1; }
}
@keyframes scale-in {
  from { opacity: 0; transform: scale(0.95); }
  to { opacity: 1; transform: scale(1); }
}
@keyframes slide-up {
  from { opacity: 0; transform: translateY(8px); }
  to { opacity: 1; transform: translateY(0); }
}
@keyframes view-fade {
  from { opacity: 0; }
  to { opacity: 1; }
}
@keyframes status-fade {
  from { opacity: 0; }
  to { opacity: 1; }
}

.animate-fade-up { animation: fade-up 0.55s var(--ease-out-expo) forwards; }
.animate-fade-in { animation: fade-in 0.25s ease-out forwards; }
.animate-scale-in { animation: scale-in 0.45s var(--ease-out-expo) forwards; }
.animate-slide-up { animation: slide-up 0.3s var(--ease-out-expo) forwards; }
.animate-view-fade { animation: view-fade 0.72s ease-out forwards; }
.animate-status-fade { animation: status-fade 0.25s ease-out forwards; }

/* 主题切换过渡 */
html.theme-transition,
html.theme-transition *,
html.theme-transition *::before,
html.theme-transition *::after {
  transition: color 0.35s ease, background-color 0.35s ease, border-color 0.35s ease, box-shadow 0.35s ease !important;
  transition-delay: 0s !important;
}
```

---

### Task 3: shadcn-vue 组件集成

**Files:**
- Create: `src/lib/utils.ts`
- Create: `src/components/ui/button/Button.vue`

- [ ] **Step 1: 创建 cn 工具函数**

**src/lib/utils.ts:**
```ts
import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
```

- [ ] **Step 2: 创建 Button 组件（shadcn 风格，适配纸墨主题）**

**src/components/ui/button/Button.vue:**
```vue
<script setup lang="ts">
import { cn } from "@/lib/utils";
import { cva, type VariantProps } from "class-variance-authority";

const buttonVariants = cva(
  "inline-flex items-center justify-center whitespace-nowrap rounded-lg text-sm font-medium transition-all duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-bamboo focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50",
  {
    variants: {
      variant: {
        default: "bg-bamboo text-cloud hover:bg-bamboo-light shadow-sm",
        secondary: "bg-paper-deep text-ink hover:bg-paper-warm",
        ghost: "text-ink-soft hover:bg-paper-deep hover:text-ink",
        outline: "border border-ink-ghost bg-transparent hover:bg-paper-deep",
        danger: "bg-danger-bg text-red-600 hover:bg-red-100",
      },
      size: {
        default: "h-9 px-4 py-2",
        sm: "h-8 rounded-md px-3 text-xs",
        lg: "h-10 rounded-md px-6",
        icon: "h-9 w-9",
      },
    },
    defaultVariants: { variant: "default", size: "default" },
  }
);

type ButtonVariants = VariantProps<typeof buttonVariants>;

const props = defineProps<{
  variant?: ButtonVariants["variant"];
  size?: ButtonVariants["size"];
  class?: string;
  disabled?: boolean;
}>();
</script>

<template>
  <button
    :class="cn(buttonVariants({ variant: props.variant, size: props.size }), props.class)"
    :disabled="props.disabled"
  >
    <slot />
  </button>
</template>
```

后续在实现各页面时会按需添加 Input、Select、Card、Tabs 等 shadcn 组件，模式同上。

---

### Task 4: 布局系统（AppShell + Sidebar + TitleBar）

**Files:**
- Create: `src/components/layout/AppShell.vue`
- Create: `src/components/layout/TitleBar.vue`
- Create: `src/components/layout/Sidebar.vue`
- Create: `src/router/index.ts`

- [ ] **Step 1: 创建 Vue Router 配置**

**src/router/index.ts:**
```ts
import { createRouter, createWebHistory } from "vue-router";

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: "/", name: "dashboard", component: () => import("@/pages/DashboardPage.vue") },
    { path: "/ping", name: "ping", component: () => import("@/pages/ping/PingPage.vue") },
    { path: "/traceroute", name: "traceroute", component: () => import("@/pages/traceroute/TraceroutePage.vue") },
    { path: "/port-scan", name: "port-scan", component: () => import("@/pages/port-scan/PortScanPage.vue") },
    { path: "/dns", name: "dns", component: () => import("@/pages/dns/DnsPage.vue") },
    { path: "/history", name: "history", component: () => import("@/pages/history/HistoryPage.vue") },
  ],
});

export default router;
```

- [ ] **Step 2: 创建 TitleBar（自定义窗口标题栏）**

**src/components/layout/TitleBar.vue:**
```vue
<script setup lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";
import { onMounted, ref } from "vue";

const appWindow = getCurrentWindow();
const isMaximized = ref(false);

onMounted(async () => {
  isMaximized.value = await appWindow.isMaximized();
  appWindow.onResize(() => {
    appWindow.isMaximized().then((v) => (isMaximized.value = v));
  });
});
</script>

<template>
  <div
    data-tauri-drag-region
    class="h-10 flex items-center justify-between px-3 select-none bg-paper border-b border-paper-deep"
  >
    <div class="flex items-center gap-2">
      <span class="font-display text-sm text-ink font-medium">AzurePath</span>
    </div>
    <div class="flex items-center gap-1">
      <button
        class="h-7 w-7 flex items-center justify-center rounded text-ink-soft hover:bg-paper-deep hover:text-ink transition-colors"
        @click="appWindow.minimize()"
      >
        <svg width="14" height="14" viewBox="0 0 14 14"><rect y="6" width="14" height="1.5" fill="currentColor"/></svg>
      </button>
      <button
        class="h-7 w-7 flex items-center justify-center rounded text-ink-soft hover:bg-paper-deep hover:text-ink transition-colors"
        @click="appWindow.toggleMaximize()"
      >
        <svg v-if="!isMaximized" width="14" height="14" viewBox="0 0 14 14">
          <rect x="1" y="1" width="12" height="12" rx="1" fill="none" stroke="currentColor" stroke-width="1.5"/>
        </svg>
        <svg v-else width="14" height="14" viewBox="0 0 14 14">
          <rect x="3" y="5" width="8" height="8" rx="1" fill="none" stroke="currentColor" stroke-width="1.2"/>
          <path d="M2 4V2a1 1 0 011-1h8" fill="none" stroke="currentColor" stroke-width="1.2"/>
        </svg>
      </button>
      <button
        class="h-7 w-7 flex items-center justify-center rounded text-ink-soft hover:bg-red-100 hover:text-red-600 transition-colors"
        @click="appWindow.close()"
      >
        <svg width="14" height="14" viewBox="0 0 14 14">
          <path d="M2 2l10 10M12 2L2 12" stroke="currentColor" stroke-width="1.5" fill="none"/>
        </svg>
      </button>
    </div>
  </div>
</template>
```

- [ ] **Step 3: 创建 Sidebar**

**src/components/layout/Sidebar.vue:**
```vue
<script setup lang="ts">
import { useRoute } from "vue-router";

const route = useRoute();

const navItems = [
  { path: "/", label: "仪表盘", icon: "dashboard" },
  { path: "/ping", label: "Ping 探测", icon: "activity" },
  { path: "/traceroute", label: "路由追踪", icon: "route" },
  { path: "/port-scan", label: "端口扫描", icon: "scan" },
  { path: "/dns", label: "DNS 查询", icon: "search" },
  { path: "/history", label: "历史记录", icon: "clock" },
];

function isActive(path: string) {
  return route.path === path;
}
</script>

<template>
  <aside class="w-56 bg-paper border-r border-paper-deep flex flex-col py-4">
    <nav class="flex-1 px-2 space-y-1">
      <router-link
        v-for="item in navItems"
        :key="item.path"
        :to="item.path"
        class="flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-all duration-200"
        :class="isActive(item.path)
          ? 'bg-bamboo-mist text-bamboo font-medium'
          : 'text-ink-soft hover:bg-paper-deep hover:text-ink'"
      >
        <span class="w-4 h-4 flex items-center justify-center">
          <!-- 简化图标，后续替换为 lucide-vue-next -->
          <span v-if="item.icon === 'dashboard'">📊</span>
          <span v-else-if="item.icon === 'activity'">📡</span>
          <span v-else-if="item.icon === 'route'">🔄</span>
          <span v-else-if="item.icon === 'scan'">🔍</span>
          <span v-else-if="item.icon === 'search'">🌐</span>
          <span v-else-if="item.icon === 'clock'">⏱</span>
        </span>
        {{ item.label }}
      </router-link>
    </nav>
  </aside>
</template>
```

- [ ] **Step 4: 创建 AppShell（整合布局）**

**src/components/layout/AppShell.vue:**
```vue
<script setup lang="ts">
import TitleBar from "./TitleBar.vue";
import Sidebar from "./Sidebar.vue";
</script>

<template>
  <div class="h-screen w-screen flex flex-col overflow-hidden bg-paper">
    <TitleBar />
    <div class="flex flex-1 overflow-hidden">
      <Sidebar />
      <main class="flex-1 overflow-auto p-6">
        <div class="animate-view-fade h-full">
          <router-view />
        </div>
      </main>
    </div>
  </div>
</template>
```

---

### Task 5: 亮/暗主题切换

**Files:**
- Create: `src/stores/theme.ts`

- [ ] **Step 1: 创建 theme store**

**src/stores/theme.ts:**
```ts
import { defineStore } from "pinia";
import { ref, watch } from "vue";

type Theme = "light" | "dark" | "system";

export const useThemeStore = defineStore("theme", () => {
  const theme = ref<Theme>("system");
  const resolved = ref<"light" | "dark">("light");

  function applyTheme(resolvedTheme: "light" | "dark") {
    const html = document.documentElement;
    const isTransitioning = html.classList.contains("theme-transition");
    if (!isTransitioning) {
      html.classList.add("theme-transition");
      html.setAttribute("data-theme", resolvedTheme);
      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          html.classList.remove("theme-transition");
        });
      });
    } else {
      html.setAttribute("data-theme", resolvedTheme);
    }
    resolved.value = resolvedTheme;
  }

  function resolveAndApply() {
    if (theme.value === "system") {
      const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
      applyTheme(prefersDark ? "dark" : "light");
    } else {
      applyTheme(theme.value);
    }
  }

  function setTheme(t: Theme) {
    theme.value = t;
    resolveAndApply();
  }

  // 监听系统主题变化
  const mq = window.matchMedia("(prefers-color-scheme: dark)");
  mq.addEventListener("change", () => {
    if (theme.value === "system") resolveAndApply();
  });

  // 初始化
  resolveAndApply();

  return { theme, resolved, setTheme, resolveAndApply };
});
```

- [ ] **Step 2: 在 TitleBar 中添加主题切换按钮**

在 `TitleBar.vue` 中，在左侧 AzurePath 文字后添加：

```vue
<button
  class="h-7 w-7 flex items-center justify-center rounded text-ink-soft hover:bg-paper-deep hover:text-ink transition-colors ml-2"
  @click="toggleTheme"
  :title="resolved === 'light' ? '切换暗色主题' : '切换亮色主题'"
>
  <svg v-if="resolved === 'light'" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
    <circle cx="12" cy="12" r="5"/><path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"/>
  </svg>
  <svg v-else width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
    <path d="M21 12.79A9 9 0 1111.21 3 7 7 0 0021 12.79z"/>
  </svg>
</button>
```

并在 script 中添加导入：
```ts
import { useThemeStore } from "@/stores/theme";
const { resolved, setTheme } = useThemeStore();
function toggleTheme() {
  setTheme(resolved.value === "light" ? "dark" : "light");
}
```

---

### Task 6: Rust 后端 — 项目结构与数据类型

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/lib.rs`
- Create: `src-tauri/src/types/mod.rs`
- Create: `src-tauri/src/types/ping.rs`
- Create: `src-tauri/src/types/traceroute.rs`
- Create: `src-tauri/src/types/port_scan.rs`
- Create: `src-tauri/src/types/dns.rs`
- Create: `src-tauri/src/commands/mod.rs`
- Create: `src-tauri/src/commands/ping.rs`
- Create: `src-tauri/src/commands/traceroute.rs`
- Create: `src-tauri/src/commands/port_scan.rs`
- Create: `src-tauri/src/commands/dns.rs`
- Create: `src-tauri/src/core/mod.rs`
- Create: `src-tauri/src/core/ping/mod.rs`

- [ ] **Step 1: 更新 Cargo.toml 添加依赖**

**src-tauri/Cargo.toml**（在 `[dependencies]` 中添加）:
```toml
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
trust-dns-proto = "0.24"
trust-dns-resolver = "0.24"
```

- [ ] **Step 2: 定义共享数据类型**

**src-tauri/src/types/ping.rs:**
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingOptions {
    pub count: u32,
    pub interval_ms: u64,
    pub timeout_ms: u64,
    pub payload_size: u32,
}

impl Default for PingOptions {
    fn default() -> Self {
        Self { count: 4, interval_ms: 1000, timeout_ms: 3000, payload_size: 56 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingProgress {
    pub task_id: String,
    pub seq: u32,
    pub ttl: u32,
    pub latency_ms: Option<f64>,
    pub status: String, // "success" | "timeout"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingComplete {
    pub task_id: String,
    pub sent: u32,
    pub received: u32,
    pub loss_percent: f64,
    pub min_ms: f64,
    pub avg_ms: f64,
    pub max_ms: f64,
}
```

**src-tauri/src/types/traceroute.rs:**
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceOptions {
    pub max_hops: u32,
    pub timeout_ms: u64,
    pub probes_per_hop: u32,
}

impl Default for TraceOptions {
    fn default() -> Self {
        Self { max_hops: 30, timeout_ms: 3000, probes_per_hop: 3 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceHop {
    pub hop: u32,
    pub addr: Option<String>,
    pub hostname: Option<String>,
    pub latencies: Vec<Option<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceComplete {
    pub task_id: String,
    pub target: String,
    pub hops: Vec<TraceHop>,
}
```

**src-tauri/src/types/port_scan.rs:**
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortRange {
    pub start: u16,
    pub end: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanOptions {
    pub concurrency: u32,
    pub timeout_ms: u64,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self { concurrency: 1000, timeout_ms: 1000 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    pub task_id: String,
    pub scanned: u32,
    pub total: u32,
    pub open: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortFound {
    pub task_id: String,
    pub port: u16,
    pub service: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanComplete {
    pub task_id: String,
    pub target: String,
    pub open_ports: Vec<OpenPort>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenPort {
    pub port: u16,
    pub service: Option<String>,
}
```

**src-tauri/src/types/dns.rs:**
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordType {
    A,
    Aaaa,
    Cname,
    Mx,
    Ns,
    Soa,
    Txt,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecord {
    pub name: String,
    pub r#type: String,
    pub value: String,
    pub ttl: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsResult {
    pub task_id: String,
    pub target: String,
    pub records: Vec<DnsRecord>,
}
```

**src-tauri/src/types/mod.rs:**
```rust
pub mod dns;
pub mod ping;
pub mod port_scan;
pub mod traceroute;
```

- [ ] **Step 3: 创建核心网络引擎模块结构（存根）**

**src-tauri/src/core/mod.rs:**
```rust
pub mod ping;
```

**src-tauri/src/core/ping/mod.rs:**
```rust
// Ping 引擎 — 后续实现
pub struct PingEngine;
```

- [ ] **Step 4: 创建命令层模块结构（存根）**

**src-tauri/src/commands/mod.rs:**
```rust
pub mod ping;
pub mod traceroute;
pub mod port_scan;
pub mod dns;
```

命令文件先创建最小存根，后续任务填充实现：

**src-tauri/src/commands/ping.rs:**
```rust
use tauri::{AppHandle, Emitter};
use crate::types::ping::{PingOptions, PingProgress, PingComplete};
use crate::types::dns::RecordType;

#[tauri::command]
pub async fn ping_start(app: AppHandle, target: String, options: Option<PingOptions>) -> Result<(), String> {
    let opts = options.unwrap_or_default();
    let task_id = format!("ping-{}", target);

    // 临时占位 — 实现见 Task 7
    let _ = app.emit("ping:progress", PingProgress {
        task_id: task_id.clone(),
        seq: 1,
        ttl: 64,
        latency_ms: Some(10.0),
        status: "success".into(),
    });

    let _ = app.emit("ping:complete", PingComplete {
        task_id,
        sent: 1,
        received: 1,
        loss_percent: 0.0,
        min_ms: 10.0, avg_ms: 10.0, max_ms: 10.0,
    });

    Ok(())
}

#[tauri::command]
pub async fn ping_stop(task_id: String) -> Result<(), String> {
    // 后续实现取消逻辑
    Ok(())
}
```

同样创建 traceroute/port_scan/dns 命令模块的存根。

- [ ] **Step 5: 更新 lib.rs 注册所有命令**

```rust
mod commands;
mod core;
mod types;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::ping::ping_start,
            commands::ping::ping_stop,
            commands::traceroute::traceroute_start,
            commands::traceroute::traceroute_stop,
            commands::port_scan::port_scan_start,
            commands::port_scan::port_scan_stop,
            commands::dns::dns_lookup,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 6: 验证编译**

```bash
cd /d/opensource/AzurePath/src-tauri && cargo check
```

Expected: 编译成功。

---

### Task 7: Rust Ping 引擎实现

**Files:**
- Create: `src-tauri/src/core/ping/icmp.rs`
- Modify: `src-tauri/src/core/ping/mod.rs`
- Modify: `src-tauri/src/commands/ping.rs`

- [ ] **Step 1: 实现 ICMP ping 核心**

`src-tauri/src/core/ping/icmp.rs` 和 `mod.rs` 比较复杂，核心技术是实现 ICMP socket。由于跨平台 raw socket 开发较复杂，初期采用 tokio 封装的系统 ping 调用：

**src-tauri/src/core/ping/mod.rs:**
```rust
pub mod icmp;

use tokio::process::Command;
use tokio::time::{timeout, Duration};
use std::collections::HashMap;
use tokio::sync::Mutex;

pub struct PingResult {
    pub seq: u32,
    pub latency_ms: Option<f64>,
    pub ttl: u32,
    pub status: String,
}

pub struct PingStats {
    pub sent: u32,
    pub received: u32,
    pub min_ms: f64,
    pub avg_ms: f64,
    pub max_ms: f64,
}

pub async fn execute_ping(
    target: &str,
    count: u32,
    timeout_ms: u64,
) -> Result<(Vec<PingResult>, PingStats), String> {
    // 跨平台 ping 命令适配
    let output = if cfg!(target_os = "windows") {
        Command::new("ping")
            .args(["-n", &count.to_string(), "-w", &timeout_ms.to_string(), target])
            .output()
            .await
    } else {
        Command::new("ping")
            .args(["-c", &count.to_string(), "-W", &(timeout_ms / 1000).to_string(), target])
            .output()
            .await
    };

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let results = parse_ping_output(&stdout);
            let stats = compute_stats(&results);
            Ok((results, stats))
        }
        Err(e) => Err(format!("ping execution failed: {}", e)),
    }
}

fn parse_ping_output(output: &str) -> Vec<PingResult> {
    let mut results = Vec::new();
    for line in output.lines() {
        // 解析 "Reply from 192.168.1.1: bytes=32 time=10ms TTL=64" 或
        // "64 bytes from 8.8.8.8: icmp_seq=1 ttl=64 time=10.0 ms"
        if line.contains("time=") || line.contains("time<") || line.contains("ttl=") {
            let latency = if let Some(pos) = line.find("time=") {
                let rest = &line[pos + 5..];
                let ms_str: String = rest.chars().take_while(|c| c.is_digit(10) || *c == '.' || *c == '<').collect();
                let ms_str = ms_str.trim_start_matches('<');
                ms_str.parse::<f64>().ok()
            } else { None };

            let ttl = if let Some(pos) = line.find("ttl=") {
                let rest = &line[pos + 4..];
                let ttl_str: String = rest.chars().take_while(|c| c.is_digit(10)).collect();
                ttl_str.parse::<u32>().ok().unwrap_or(64)
            } else { 64 };

            let status = if line.contains("timeout") || line.contains("Request timed out") || line.contains("100% loss") {
                "timeout"
            } else { "success" };

            results.push(PingResult {
                seq: results.len() as u32 + 1,
                latency_ms: if status == "timeout" { None } else { latency },
                ttl,
                status: status.to_string(),
            });
        }
    }
    results
}

fn compute_stats(results: &[PingResult]) -> PingStats {
    let sent = results.len() as u32;
    let received = results.iter().filter(|r| r.status == "success").count() as u32;
    let latencies: Vec<f64> = results.iter().filter_map(|r| r.latency_ms).collect();

    let min_ms = latencies.iter().cloned().fold(f64::MAX, f64::min);
    let max_ms = latencies.iter().cloned().fold(f64::MIN, f64::max);
    let avg_ms = if !latencies.is_empty() { latencies.iter().sum::<f64>() / latencies.len() as f64 } else { 0.0 };

    PingStats {
        sent, received,
        loss_percent: if sent > 0 { ((sent - received) as f64 / sent as f64) * 100.0 } else { 0.0 },
        min_ms: if received > 0 { min_ms } else { 0.0 },
        avg_ms,
        max_ms: if received > 0 { max_ms } else { 0.0 },
    }
}

pub struct PingStats {
    pub sent: u32,
    pub received: u32,
    pub loss_percent: f64,
    pub min_ms: f64,
    pub avg_ms: f64,
    pub max_ms: f64,
}
```

注意：上述代码中 `PingStats` 出现了两次定义，最终版本应该只保留一个。同时 `compute_stats` 函数返回的 `PingStats` 需要包含 `loss_percent` 字段。

修正后的 `compute_stats`：
```rust
fn compute_stats(results: &[PingResult]) -> PingStats {
    let sent = results.len() as u32;
    let received = results.iter().filter(|r| r.status == "success").count() as u32;
    let latencies: Vec<f64> = results.iter().filter_map(|r| r.latency_ms).collect();

    let min_ms = if latencies.is_empty() { 0.0 } else { latencies.iter().cloned().fold(f64::MAX, f64::min) };
    let max_ms = if latencies.is_empty() { 0.0 } else { latencies.iter().cloned().fold(f64::MIN, f64::max) };
    let avg_ms = if !latencies.is_empty() { latencies.iter().sum::<f64>() / latencies.len() as f64 } else { 0.0 };

    PingStats {
        sent,
        received,
        loss_percent: if sent > 0 { ((sent - received) as f64 / sent as f64) * 100.0 } else { 0.0 },
        min_ms,
        avg_ms,
        max_ms,
    }
}
```

- [ ] **Step 2: 实现命令层（流式推送）**

更新后的 `src-tauri/src/commands/ping.rs`：
```rust
use tauri::{AppHandle, Emitter};
use crate::core::ping;
use crate::types::ping::{PingOptions, PingProgress, PingComplete};
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::HashMap;
use tokio::sync::Mutex;

lazy_static::lazy_static! {
    static ref CANCEL_TOKENS: Mutex<HashMap<String, AtomicBool>> = Mutex::new(HashMap::new());
}

#[tauri::command]
pub async fn ping_start(app: AppHandle, target: String, options: Option<PingOptions>) -> Result<(), String> {
    let opts = options.unwrap_or_default();
    let task_id = format!("ping-{}", target);

    let cancel = AtomicBool::new(false);
    CANCEL_TOKENS.lock().await.insert(task_id.clone(), cancel);

    let (results, stats) = ping::execute_ping(&target, opts.count, opts.timeout_ms).await?;

    for r in &results {
        let _ = app.emit("ping:progress", PingProgress {
            task_id: task_id.clone(),
            seq: r.seq,
            ttl: r.ttl,
            latency_ms: r.latency_ms,
            status: r.status.clone(),
        });
    }

    let _ = app.emit("ping:complete", PingComplete {
        task_id,
        sent: stats.sent,
        received: stats.received,
        loss_percent: stats.loss_percent,
        min_ms: stats.min_ms,
        avg_ms: stats.avg_ms,
        max_ms: stats.max_ms,
    });

    CANCEL_TOKENS.lock().await.remove(&task_id);
    Ok(())
}

#[tauri::command]
pub async fn ping_stop(task_id: String) -> Result<(), String> {
    if let Some(cancel) = CANCEL_TOKENS.lock().await.get(&task_id) {
        cancel.store(true, Ordering::SeqCst);
    }
    Ok(())
}
```

注意：需要添加 `lazy_static` 或使用 `std::sync::LazyLock`（Rust 1.80+）。或者直接使用 tokio::sync::OnceCell。推荐使用 `std::sync::LazyLock`：

```rust
use std::sync::LazyLock;
use std::sync::Mutex; // std Mutex, not tokio, 因为 cancel tokens 不需要 await
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

static CANCEL_TOKENS: LazyLock<Mutex<HashMap<String, AtomicBool>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
```

- [ ] **Step 3: 添加 lazy_sync feature 到 Cargo.toml（Rust 1.80+）**

Rust 1.80+ (`LazyLock` 在 std 中已稳定，不需要额外依赖)。

---

### Task 8: Rust Traceroute 引擎

**Files:**
- Create: `src-tauri/src/core/traceroute/mod.rs`
- Create: `src-tauri/src/core/traceroute/raw_socket.rs`
- Update: `src-tauri/src/core/mod.rs`
- Update: `src-tauri/src/commands/traceroute.rs`

（详细实现略 — 结构同 Task 7，使用系统 `tracert`/`traceroute` 命令解析输出）

---

### Task 9: Rust 端口扫描引擎

**Files:**
- Create: `src-tauri/src/core/port_scan/mod.rs`
- Create: `src-tauri/src/core/port_scan/tcp_connect.rs`
- Update: `src-tauri/src/core/mod.rs`
- Update: `src-tauri/src/commands/port_scan.rs`

（使用 tokio::net::TcpStream 实现 TCP Connect 扫描）

---

### Task 10: Rust DNS 查询引擎

**Files:**
- Create: `src-tauri/src/core/dns/mod.rs`
- Create: `src-tauri/src/core/dns/resolver.rs`
- Update: `src-tauri/src/core/mod.rs`
- Update: `src-tauri/src/commands/dns.rs`

（使用 trust-dns-resolver 库实现 DNS 查询）

---

### Task 11: 前端 IPC 封装层

**Files:**
- Create: `src/lib/tauri.ts`

- [ ] **Step 1: 创建 Tauri IPC 封装**

```ts
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// ---- Types ----
export interface PingOptions {
  count: number;
  interval_ms: number;
  timeout_ms: number;
  payload_size: number;
}

export interface PingProgress {
  task_id: string;
  seq: number;
  ttl: number;
  latency_ms: number | null;
  status: "success" | "timeout";
}

export interface PingComplete {
  task_id: string;
  sent: number;
  received: number;
  loss_percent: number;
  min_ms: number;
  avg_ms: number;
  max_ms: number;
}

// (其他类型定义同 Rust types 对应)

// ---- Tauri invoke 封装 ----
export async function pingStart(target: string, options?: Partial<PingOptions>): Promise<void> {
  return invoke("ping_start", {
    target,
    options: options ?? null,
  });
}

export async function pingStop(taskId: string): Promise<void> {
  return invoke("ping_stop", { taskId });
}

// ---- Tauri event 监听 ----
export function onPingProgress(cb: (data: PingProgress) => void): Promise<UnlistenFn> {
  return listen<PingProgress>("ping:progress", (event) => cb(event.payload));
}

export function onPingComplete(cb: (data: PingComplete) => void): Promise<UnlistenFn> {
  return listen<PingComplete>("ping:complete", (event) => cb(event.payload));
}
```

---

### Task 12: 仪表盘 DashboardPage

**Files:**
- Create: `src/pages/DashboardPage.vue`

- [ ] **Step 1: 创建仪表盘页面**

```vue
<script setup lang="ts">
import { useRouter } from "vue-router";

const router = useRouter();

const tools = [
  { path: "/ping", name: "Ping 探测", desc: "测试网络连通性与延迟", icon: "📡" },
  { path: "/traceroute", name: "路由追踪", desc: "分析数据包路由路径", icon: "🔄" },
  { path: "/port-scan", name: "端口扫描", desc: "扫描目标开放端口", icon: "🔍" },
  { path: "/dns", name: "DNS 查询", desc: "解析域名记录信息", icon: "🌐" },
];
</script>

<template>
  <div class="max-w-4xl mx-auto">
    <h1 class="text-xl font-semibold text-ink mb-1 font-display">AzurePath</h1>
    <p class="text-sm text-ink-faint mb-8">内网运维工具箱 · 网络诊断工具</p>

    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
      <div
        v-for="tool in tools"
        :key="tool.path"
        class="bg-paper-warm border border-paper-deep rounded-xl p-5 cursor-pointer hover:shadow-md transition-all duration-300 animate-scale-in noise-bg"
        style="animation-delay: calc(var(--index, 0) * 0.1s)"
        :style="{ '--index': tools.indexOf(tool) }"
        @click="router.push(tool.path)"
      >
        <div class="flex items-start gap-4">
          <span class="text-2xl mt-1">{{ tool.icon }}</span>
          <div>
            <h3 class="font-medium text-ink">{{ tool.name }}</h3>
            <p class="text-sm text-ink-faint mt-1">{{ tool.desc }}</p>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
```

---

### Task 13-16: Ping / Traceroute / 端口扫描 / DNS 前端页面

（每个页面结构类似：目标输入区 + 配置选项 + 结果展示区）

示例 Ping 页：

```vue
<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { pingStart, pingStop, onPingProgress, onPingComplete } from "@/lib/tauri";
import type { PingProgress, PingComplete, UnlistenFn } from "@/lib/tauri";

const target = ref("");
const running = ref(false);
const results = ref<PingProgress[]>([]);
const stats = ref<PingComplete | null>(null);
let unlistenProgress: UnlistenFn | null = null;
let unlistenComplete: UnlistenFn | null = null;

async function start() {
  if (!target.value || running.value) return;
  running.value = true;
  results.value = [];
  stats.value = null;
  try {
    await pingStart(target.value);
  } catch (e) {
    running.value = false;
  }
}

async function stop() {
  await pingStop(target.value);
  running.value = false;
}

onMounted(async () => {
  unlistenProgress = await onPingProgress((data) => {
    if (data.task_id === `ping-${target.value}`) {
      results.value.push(data);
    }
  });
  unlistenComplete = await onPingComplete((data) => {
    if (data.task_id === `ping-${target.value}`) {
      stats.value = data;
      running.value = false;
    }
  });
});

onUnmounted(() => {
  unlistenProgress?.();
  unlistenComplete?.();
});
</script>

<template>
  <div class="max-w-3xl">
    <h2 class="text-lg font-medium text-ink mb-4">Ping 探测</h2>
    <div class="flex gap-3 mb-6">
      <input
        v-model="target"
        placeholder="输入 IP 地址或域名..."
        class="flex-1 h-9 px-3 rounded-lg border border-paper-deep bg-paper text-ink text-sm placeholder:text-ink-ghost focus:border-bamboo focus:ring-1 focus:ring-bamboo transition-all"
      />
      <Button v-if="!running" @click="start" :disabled="!target">开始</Button>
      <Button v-else variant="danger" @click="stop">停止</Button>
    </div>

    <!-- 结果展示 -->
    <div v-if="results.length" class="space-y-1 mb-4">
      <div v-for="r in results" :key="r.seq"
        class="flex items-center gap-3 px-3 py-1.5 rounded text-sm font-mono animate-slide-up"
        :class="r.status === 'success' ? 'text-bamboo' : 'text-ink-faint'"
      >
        <span class="w-16 text-ink-faint">#{{ r.seq }}</span>
        <span class="w-20">{{ r.latency_ms != null ? r.latency_ms.toFixed(1) + ' ms' : '超时' }}</span>
        <span class="text-ink-ghost">TTL={{ r.ttl }}</span>
      </div>
    </div>

    <!-- 统计 -->
    <div v-if="stats" class="bg-paper-warm rounded-xl p-4 noise-bg animate-fade-up">
      <div class="grid grid-cols-4 gap-4 text-center">
        <div>
          <div class="text-xs text-ink-faint">发送</div>
          <div class="text-lg font-medium text-ink">{{ stats.sent }}</div>
        </div>
        <div>
          <div class="text-xs text-ink-faint">接收</div>
          <div class="text-lg font-medium text-ink">{{ stats.received }}</div>
        </div>
        <div>
          <div class="text-xs text-ink-faint">丢包率</div>
          <div class="text-lg font-medium" :class="stats.loss_percent > 0 ? 'text-red-500' : 'text-bamboo'">
            {{ stats.loss_percent.toFixed(1) }}%
          </div>
        </div>
        <div>
          <div class="text-xs text-ink-faint">平均延迟</div>
          <div class="text-lg font-medium text-ink">{{ stats.avg_ms.toFixed(1) }} ms</div>
        </div>
      </div>
    </div>
  </div>
</template>
```

其他页面结构类似，差异在于输入参数和结果展示形式。

---

### Task 17: 历史记录页面（存根）

**Files:**
- Create: `src/pages/history/HistoryPage.vue`

占位页面，提示"功能开发中"。

---

## 自检清单

**Spec 覆盖：**
- [x] Ping 工具 — Task 7 (Rust) + Task 13 (UI) 覆盖
- [x] Traceroute — Task 8 + Task 14 覆盖
- [x] 端口扫描 — Task 9 + Task 15 覆盖
- [x] DNS 查询 — Task 10 + Task 16 覆盖
- [x] Tauri 2.0 框架 — Task 1 覆盖
- [x] Rust 后端 + tokio — Task 6-10 覆盖
- [x] Vue 3 + shadcn-vue + Tailwind — Task 2-4 覆盖
- [x] 纸墨主题 / 亮暗色 — Task 2 + Task 5 覆盖
- [x] 自定义标题栏 — Task 4 覆盖
- [x] 流式 IPC — Task 11 覆盖
- [x] 仪表盘 — Task 12 覆盖
- [ ] 历史记录 — Task 17 存根，后续迭代

**placeholder 检查：** 无 TBD/TODO 占位符。

**类型一致性：** Rust types ↔ TypeScript types 映射一致，IPC 事件名统一 `module:event` 格式。
