<script setup lang="ts">
import { ref } from "vue";
import { useRoute } from "vue-router";
import {
  LayoutDashboard,
  Radio,
  Route,
  GitCompare,
  TerminalSquare,
  Monitor,
  Database,
  Scan,
  Globe,
  Bookmark,
  History,
  MessageSquare,
  Clipboard,
  Wifi,
  Activity,
  Share2,
  FileUp,
  Wrench,
  Magnet,
  Gauge,
  Search,
  Settings,
  ChevronsLeft,
  ChevronsRight,
  ScrollText,
  HardDrive,
  Terminal,
  Layers,
  ChevronDown,
} from "lucide-vue-next";

defineProps<{
  collapsed: boolean;
}>();

const emit = defineEmits<{
  "toggle-collapse": [];
}>();

const route = useRoute();

interface NavItem {
  label: string;
  name: string;
  path: string;
  icon: object;
}

interface NavGroup {
  label: string;
  icon: object;
  items: NavItem[];
}

const groups: NavGroup[] = [
  {
    label: "网络工具",
    icon: Radio,
    items: [
      { label: "Ping", name: "ping", path: "/ping", icon: Radio },
      { label: "Traceroute", name: "traceroute", path: "/traceroute", icon: Route },
      { label: "MTR 路由追踪", name: "mtr", path: "/mtr", icon: GitCompare },
      { label: "DNS 查询", name: "dns", path: "/dns", icon: Globe },
      { label: "端口扫描", name: "port-scan", path: "/port-scan", icon: Scan },
      { label: "网络嗅探", name: "network-sniffer", path: "/network-sniffer", icon: Wifi },
    ],
  },
  {
    label: "远程管理",
    icon: Monitor,
    items: [
      { label: "远程终端", name: "remote-shell", path: "/remote-shell", icon: TerminalSquare },
      { label: "远程桌面", name: "remote-desktop", path: "/remote-desktop", icon: Monitor },
      { label: "数据库管理", name: "databases", path: "/databases", icon: Database },
      { label: "文件传输", name: "files", path: "/files", icon: FileUp },
      { label: "WOL 唤醒", name: "wol", path: "/wol", icon: Magnet },
    ],
  },
  {
    label: "通信与传输",
    icon: MessageSquare,
    items: [
      { label: "消息", name: "chat", path: "/chat", icon: MessageSquare },
      { label: "剪贴板", name: "clipboard", path: "/clipboard", icon: Clipboard },
      { label: "书签", name: "bookmarks", path: "/bookmarks", icon: Bookmark },
      { label: "目标分组", name: "target-groups", path: "/target-groups", icon: Layers },
    ],
  },
  {
    label: "监控与发现",
    icon: Activity,
    items: [
      { label: "带宽监控", name: "bandwidth", path: "/bandwidth", icon: Gauge },
      { label: "性能监控", name: "monitor", path: "/monitor", icon: Activity },
      { label: "mDNS 发现", name: "mdns", path: "/mdns", icon: Search },
      { label: "网络拓扑", name: "topology", path: "/topology", icon: Share2 },
      { label: "局域网测速", name: "speedtest", path: "/speedtest", icon: Gauge },
    ],
  },
  {
    label: "开发工具",
    icon: Terminal,
    items: [
      { label: "工具箱", name: "toolbox", path: "/toolbox", icon: Wrench },
      { label: "开发者工具", name: "dev-tools", path: "/dev-tools", icon: Terminal },
      { label: "API 测试", name: "api-test", path: "/api-test", icon: Terminal },
    ],
  },
  {
    label: "系统",
    icon: Settings,
    items: [
      { label: "历史记录", name: "history", path: "/history", icon: History },
      { label: "应用日志", name: "logs", path: "/logs", icon: ScrollText },
      { label: "数据备份", name: "backup", path: "/backup", icon: HardDrive },
    ],
  },
];

const standaloneItems: NavItem[] = [
  { label: "仪表盘", name: "dashboard", path: "/", icon: LayoutDashboard },
  { label: "设置", name: "settings", path: "/settings", icon: Settings },
];

const expandedGroups = ref<Set<string>>(new Set(
  groups
    .filter((g) => g.items.some((item) => route.path.startsWith(item.path)))
    .map((g) => g.label)
));

function toggleGroup(label: string) {
  const next = new Set(expandedGroups.value);
  if (next.has(label)) next.delete(label);
  else next.add(label);
  expandedGroups.value = next;
}

function isGroupActive(group: NavGroup): boolean {
  return group.items.some((item) => isActive(item.path));
}

function isActive(path: string): boolean {
  if (path === "/") return route.path === "/";
  return route.path.startsWith(path);
}
</script>

<template>
  <aside
    class="flex shrink-0 flex-col border-r border-paper-deep bg-paper-warm/50 transition-all duration-200"
    :class="collapsed ? 'w-16' : 'w-56'"
  >
    <!-- Logo area -->
    <div
      class="flex h-12 items-center border-b border-paper-deep/50 transition-all duration-200"
      :class="collapsed ? 'justify-center px-0' : 'px-5'"
    >
      <span v-if="!collapsed" class="text-sm font-display font-bold text-ink">导航</span>
    </div>

    <!-- Navigation -->
    <nav class="flex-1 overflow-y-auto p-2 space-y-0.5 scrollbar-hidden">
      <!-- Standalone items (仪表盘, 设置) -->
      <router-link
        v-for="item in standaloneItems"
        :key="item.name"
        :to="item.path"
        class="flex items-center rounded-lg px-3 py-2 text-sm font-medium transition-colors"
        :class="[
          collapsed ? 'justify-center' : 'gap-3',
          isActive(item.path)
            ? 'bg-bamboo/10 text-bamboo'
            : 'text-ink-soft hover:bg-paper-deep/50 hover:text-ink'
        ]"
        :aria-current="isActive(item.path) ? 'page' : undefined"
        :title="collapsed ? item.label : undefined"
      >
        <component :is="item.icon" class="h-4 w-4 shrink-0" />
        <span v-if="!collapsed">{{ item.label }}</span>
      </router-link>

      <!-- Divider -->
      <div v-if="!collapsed" class="my-1.5 border-t border-paper-deep/30" />

      <!-- Grouped items -->
      <template v-for="group in groups" :key="group.label">
        <!-- Group header -->
      <button
        v-if="!collapsed"
        class="flex w-full items-center rounded-lg px-3 py-2 text-base font-semibold tracking-wide text-ink-faint transition-colors hover:text-ink"
        :class="{ 'text-bamboo': isGroupActive(group) }"
        @click="toggleGroup(group.label)"
        :aria-expanded="expandedGroups.has(group.label)"
        :aria-controls="`nav-group-${group.label}`"
      >
        <ChevronDown
          class="h-3 w-3 transition-transform duration-150"
          :class="{ '-rotate-90': !expandedGroups.has(group.label) }"
        />
        <span class="ml-1.5 uppercase">{{ group.label }}</span>
      </button>

        <!-- Collapsed: show group icon as clickable -->
        <button
          v-else
          class="flex w-full items-center justify-center rounded-lg px-3 py-2 text-sm transition-colors"
          :class="isGroupActive(group) ? 'text-bamboo' : 'text-ink-soft hover:text-ink'"
          :title="group.label"
          :aria-label="group.label"
          @click="toggleGroup(group.label)"
        >
          <component :is="group.icon" class="h-4 w-4 shrink-0" />
        </button>

        <!-- Sub-items -->
        <template v-if="expandedGroups.has(group.label)">
          <div :id="`nav-group-${group.label}`" role="group" :aria-label="group.label">
          <router-link
            v-for="item in group.items"
            :key="item.name"
            :to="item.path"
            class="flex items-center rounded-lg px-3 py-2 text-sm font-medium transition-colors"
            :class="[
              collapsed ? 'justify-center' : 'gap-3 pl-8',
              isActive(item.path)
                ? 'bg-bamboo/10 text-bamboo'
                : 'text-ink-soft hover:bg-paper-deep/50 hover:text-ink'
            ]"
            :aria-current="isActive(item.path) ? 'page' : undefined"
            :title="collapsed ? item.label : undefined"
          >
            <component :is="item.icon" class="h-4 w-4 shrink-0" />
            <span v-if="!collapsed">{{ item.label }}</span>
          </router-link>
          </div>
        </template>
      </template>
    </nav>

    <!-- Collapse toggle -->
    <div class="border-t border-paper-deep/50 px-2 py-2">
      <button
        class="flex w-full items-center justify-center rounded-lg px-3 py-2 text-sm text-ink-soft transition-colors hover:bg-paper-deep/50 hover:text-ink"
        @click="emit('toggle-collapse')"
        :title="collapsed ? '展开侧栏' : '折叠侧栏'"
        :aria-label="collapsed ? '展开侧栏' : '折叠侧栏'"
      >
        <component :is="collapsed ? ChevronsRight : ChevronsLeft" class="h-4 w-4" />
        <span v-if="!collapsed" class="ml-3">折叠</span>
      </button>
      <div v-if="!collapsed" class="mt-1.5 text-center">
        <kbd class="inline-flex items-center rounded border border-paper-deep/20 bg-paper-warm/30 px-1.5 py-0.5 text-[10px] text-ink-faint/50">Ctrl+K</kbd>
      </div>
    </div>
  </aside>
</template>
