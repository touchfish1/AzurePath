<script setup lang="ts">
import { useRoute } from "vue-router";
import {
  LayoutDashboard,
  Radio,
  Route,
  Scan,
  Globe,
  History,
  MessageSquare,
  Clipboard,
  Wifi,
  Activity,
  Share2,
  FileUp,
  Wrench,
  Settings,
  ChevronsLeft,
  ChevronsRight,
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

const navItems: NavItem[] = [
  { label: "仪表盘", name: "dashboard", path: "/", icon: LayoutDashboard },
  { label: "Ping", name: "ping", path: "/ping", icon: Radio },
  { label: "Traceroute", name: "traceroute", path: "/traceroute", icon: Route },
  { label: "端口扫描", name: "port-scan", path: "/port-scan", icon: Scan },
  { label: "DNS 查询", name: "dns", path: "/dns", icon: Globe },
  { label: "历史记录", name: "history", path: "/history", icon: History },
  { label: "消息", name: "chat", path: "/chat", icon: MessageSquare },
  { label: "剪贴板", name: "clipboard", path: "/clipboard", icon: Clipboard },
  { label: "网络嗅探", name: "network-sniffer", path: "/network-sniffer", icon: Wifi },
  { label: "网络拓扑", name: "topology", path: "/topology", icon: Share2 },
  { label: "局域网测速", name: "speedtest", path: "/speedtest", icon: Activity },
  { label: "文件传输", name: "files", path: "/files", icon: FileUp },
  { label: "工具箱", name: "toolbox", path: "/toolbox", icon: Wrench },
  { label: "设置", name: "settings", path: "/settings", icon: Settings },
];

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
      <router-link
        v-for="item in navItems"
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
        <component
          :is="item.icon"
          class="h-4 w-4 shrink-0"
        />
        <span v-if="!collapsed">{{ item.label }}</span>
      </router-link>
    </nav>

    <!-- Collapse toggle -->
    <div class="border-t border-paper-deep/50 px-2 py-2">
      <button
        class="flex w-full items-center justify-center rounded-lg px-3 py-2 text-sm text-ink-soft transition-colors hover:bg-paper-deep/50 hover:text-ink"
        @click="emit('toggle-collapse')"
        :title="collapsed ? '展开侧栏' : '折叠侧栏'"
      >
        <component
          :is="collapsed ? ChevronsRight : ChevronsLeft"
          class="h-4 w-4"
        />
        <span v-if="!collapsed" class="ml-3">折叠</span>
      </button>
    </div>
  </aside>
</template>
