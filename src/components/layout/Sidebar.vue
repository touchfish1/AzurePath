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
} from "lucide-vue-next";

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
];

function isActive(path: string): boolean {
  if (path === "/") return route.path === "/";
  return route.path.startsWith(path);
}
</script>

<template>
  <aside
    class="flex w-56 shrink-0 flex-col border-r border-paper-deep bg-paper-warm/50"
  >
    <!-- Logo area -->
    <div class="flex h-12 items-center px-5 border-b border-paper-deep/50">
      <span class="text-sm font-display font-bold text-ink">导航</span>
    </div>

    <!-- Navigation -->
    <nav class="flex-1 overflow-y-auto p-2 space-y-0.5 scrollbar-hidden">
      <router-link
        v-for="item in navItems"
        :key="item.name"
        :to="item.path"
        class="flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors"
        :class="
          isActive(item.path)
            ? 'bg-bamboo/10 text-bamboo'
            : 'text-ink-soft hover:bg-paper-deep/50 hover:text-ink'
        "
      >
        <component
          :is="item.icon"
          class="h-4 w-4 shrink-0"
        />
        <span>{{ item.label }}</span>
      </router-link>
    </nav>

    <!-- Footer -->
    <div class="border-t border-paper-deep/50 px-5 py-3">
      <span class="text-xs text-ink-faint">AzurePath v0.1.0</span>
    </div>
  </aside>
</template>
