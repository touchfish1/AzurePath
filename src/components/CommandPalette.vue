<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, nextTick } from "vue";
import type { Component } from "vue";
import { useRouter } from "vue-router";
import {
  Search,
  SunMoon,
  LayoutDashboard,
  Radio,
  Route,
  GitCompare,
  Scan,
  Globe,
  Bookmark,
  Layers,
  History,
  MessageSquare,
  Clipboard,
  Wifi,
  Terminal,
  Gauge,
  Activity,
  Share2,
  FileUp,
  Wrench,
  Magnet,
  ScrollText,
  HardDrive,
  Settings,
} from "lucide-vue-next";
import { useCommandPaletteStore } from "@/stores/commandPalette";
import type { Command } from "@/stores/commandPalette";
import { useThemeStore } from "@/stores/theme";

const store = useCommandPaletteStore();
const router = useRouter();
const themeStore = useThemeStore();

const inputRef = ref<HTMLInputElement | null>(null);
const selectedIndex = ref(0);
const listRef = ref<HTMLDivElement | null>(null);

// Route metadata mapping (name -> { label, icon })
const routeMeta: Record<string, { label: string; icon: Component }> = {
  dashboard: { label: "仪表盘", icon: LayoutDashboard },
  ping: { label: "Ping", icon: Radio },
  traceroute: { label: "Traceroute", icon: Route },
  mtr: { label: "MTR 路由追踪", icon: GitCompare },
  "port-scan": { label: "端口扫描", icon: Scan },
  dns: { label: "DNS 查询", icon: Globe },
  bookmarks: { label: "书签", icon: Bookmark },
  "target-groups": { label: "目标分组", icon: Layers },
  history: { label: "历史记录", icon: History },
  chat: { label: "消息", icon: MessageSquare },
  clipboard: { label: "剪贴板", icon: Clipboard },
  "network-sniffer": { label: "网络嗅探", icon: Wifi },
  "api-test": { label: "API 测试", icon: Terminal },
  mdns: { label: "mDNS 发现", icon: Search },
  bandwidth: { label: "带宽监控", icon: Gauge },
  monitor: { label: "性能监控", icon: Activity },
  topology: { label: "网络拓扑", icon: Share2 },
  speedtest: { label: "局域网测速", icon: Activity },
  files: { label: "文件传输", icon: FileUp },
  toolbox: { label: "工具箱", icon: Wrench },
  wol: { label: "WOL 唤醒", icon: Magnet },
  logs: { label: "应用日志", icon: ScrollText },
  backup: { label: "数据备份", icon: HardDrive },
  settings: { label: "设置", icon: Settings },
};

const categoryLabels: Record<string, string> = {
  navigation: "导航",
  action: "操作",
  tool: "工具",
};

watch(
  () => store.isOpen,
  (open) => {
    if (open) {
      selectedIndex.value = 0;
      nextTick(() => inputRef.value?.focus());
    }
  },
);

watch(
  () => store.query,
  () => {
    selectedIndex.value = 0;
  },
);

function onKeydown(e: KeyboardEvent) {
  const commands = store.filteredCommands;
  if (e.key === "ArrowDown") {
    e.preventDefault();
    selectedIndex.value = Math.min(selectedIndex.value + 1, commands.length - 1);
    scrollToSelected();
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    selectedIndex.value = Math.max(selectedIndex.value - 1, 0);
    scrollToSelected();
  } else if (e.key === "Enter") {
    e.preventDefault();
    const cmd = commands[selectedIndex.value];
    if (cmd) executeCommand(cmd);
  } else if (e.key === "Escape") {
    store.close();
  }
}

function scrollToSelected() {
  if (!listRef.value) return;
  const items = listRef.value.querySelectorAll<HTMLElement>("[data-command-index]");
  const selected = items[selectedIndex.value];
  selected?.scrollIntoView({ block: "nearest" });
}

function executeCommand(cmd: Command) {
  store.close();
  cmd.action();
}

function setupCommands() {
  const routes = router.getRoutes();
  const navCommands: Command[] = routes
    .filter((r): r is typeof r & { name: string } => !!r.name && typeof r.name === "string")
    .map((r) => {
      const name = r.name;
      const meta = routeMeta[name];
      return {
        id: `nav-${name}`,
        label: meta?.label ?? name,
        description: `导航到 ${r.path}`,
        icon: meta?.icon,
        category: "navigation" as const,
        action: () => { router.push({ name }); },
      };
    });

  const actions: Command[] = [
    {
      id: "toggle-theme",
      label: "切换主题",
      description: "在明暗主题之间切换",
      icon: SunMoon,
      category: "action",
      action: () => themeStore.toggleTheme(),
      keywords: ["theme", "dark", "light", "黑暗", "明亮"],
    },
  ];

  store.registerCommands([...navCommands, ...actions]);
}

function globalKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
    e.preventDefault();
    store.toggle();
  }
}

onMounted(() => {
  setupCommands();
  window.addEventListener("keydown", globalKeydown);
});

onUnmounted(() => {
  window.removeEventListener("keydown", globalKeydown);
});
</script>

<template>
  <Teleport to="body">
    <Transition
      enter-active-class="transition duration-200 ease-out"
      enter-from-class="opacity-0 scale-95"
      leave-active-class="transition duration-150 ease-in"
      leave-to-class="opacity-0 scale-95"
    >
      <div
        v-if="store.isOpen"
        class="fixed inset-0 z-[999]"
      >
        <!-- Backdrop -->
        <div
          class="absolute inset-0 bg-black/30 backdrop-blur-sm"
          @click="store.close"
        />

        <!-- Palette -->
        <div class="relative mx-auto mt-[15vh] w-full max-w-xl px-4">
          <div class="noise-bg rounded-xl border border-paper-deep/60 bg-paper shadow-2xl shadow-black/20 overflow-hidden">
            <!-- Search input -->
            <div class="flex items-center gap-3 border-b border-paper-deep/30 px-4 py-3">
              <Search class="h-5 w-5 shrink-0 text-ink-faint" />
              <input
                ref="inputRef"
                v-model="store.query"
                placeholder="搜索命令..."
                class="flex-1 bg-transparent text-sm text-ink outline-none placeholder:text-ink-faint/40"
                @keydown="onKeydown"
              />
              <kbd class="hidden sm:inline-flex items-center rounded-md border border-paper-deep/30 bg-paper-warm/50 px-1.5 py-0.5 text-xs text-ink-faint">ESC</kbd>
            </div>

            <!-- Results list -->
            <div
              v-if="store.filteredCommands.length > 0"
              ref="listRef"
              class="max-h-80 overflow-y-auto p-2"
            >
              <div
                v-for="(cmd, index) in store.filteredCommands"
                :key="cmd.id"
                :data-command-index="index"
                class="flex items-center gap-3 rounded-lg px-3 py-2.5 cursor-pointer transition-colors"
                :class="
                  index === selectedIndex
                    ? 'bg-bamboo/10 text-bamboo'
                    : 'text-ink hover:bg-paper-deep/30'
                "
                @click="executeCommand(cmd)"
                @mouseenter="selectedIndex = index"
              >
                <component
                  :is="cmd.icon"
                  v-if="cmd.icon"
                  class="h-4 w-4 shrink-0 opacity-60"
                />
                <div class="flex-1 min-w-0">
                  <div class="text-sm font-medium">
                    {{ cmd.label }}
                  </div>
                  <div
                    v-if="cmd.description"
                    class="truncate text-xs text-ink-faint"
                  >
                    {{ cmd.description }}
                  </div>
                </div>
                <span class="rounded-md bg-paper-deep/30 px-1.5 py-0.5 text-xs text-ink-faint">
                  {{ categoryLabels[cmd.category] || cmd.category }}
                </span>
              </div>
            </div>

            <!-- Empty state (query entered, no matches) -->
            <div
              v-else-if="store.query"
              class="flex items-center justify-center py-12 text-sm text-ink-faint"
            >
              没有找到匹配的命令
            </div>

            <!-- Initial hint (no query entered) -->
            <div
              v-else
              class="flex items-center justify-center py-12 text-sm text-ink-faint"
            >
              输入关键词搜索命令
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
