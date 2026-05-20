import { createRouter, createWebHistory } from "vue-router";
import type { RouteRecordRaw } from "vue-router";

const routes: RouteRecordRaw[] = [
  {
    path: "/",
    name: "dashboard",
    component: () => import("@/pages/dashboard/Page.vue"),
  },
  {
    path: "/ping",
    name: "ping",
    component: () => import("@/pages/ping/Page.vue"),
  },
  {
    path: "/traceroute",
    name: "traceroute",
    component: () => import("@/pages/traceroute/Page.vue"),
  },
  {
    path: "/mtr",
    name: "mtr",
    component: () => import("@/pages/mtr/index.vue"),
  },
  {
    path: "/port-scan",
    name: "port-scan",
    component: () => import("@/pages/port-scan/Page.vue"),
  },
  {
    path: "/dns",
    name: "dns",
    component: () => import("@/pages/dns/Page.vue"),
  },
  {
    path: "/history",
    name: "history",
    component: () => import("@/pages/history/Page.vue"),
  },
  {
    path: "/chat",
    name: "chat",
    component: () => import("@/pages/chat/Page.vue"),
  },
  {
    path: "/clipboard",
    name: "clipboard",
    component: () => import("@/pages/clipboard/Page.vue"),
  },
  {
    path: "/network-sniffer",
    name: "network-sniffer",
    component: () => import("@/pages/network-sniffer/Page.vue"),
  },
  {
    path: "/files",
    name: "files",
    component: () => import("@/pages/files/Page.vue"),
  },
  {
    path: "/toolbox",
    name: "toolbox",
    component: () => import("@/pages/toolbox/Page.vue"),
  },
  {
    path: "/speedtest",
    name: "speedtest",
    component: () => import("@/pages/speedtest/Page.vue"),
  },
  {
    path: "/topology",
    name: "topology",
    component: () => import("@/pages/topology/Page.vue"),
  },
  {
    path: "/wol",
    name: "wol",
    component: () => import("@/pages/wol/Page.vue"),
  },
{
    path: "/monitor",
    name: "monitor",
    component: () => import("@/pages/monitor/Page.vue"),
  },
  {
    path: "/mdns",
    name: "mdns",
    component: () => import("@/pages/mdns/Page.vue"),
  },
  {
    path: "/bandwidth",
    name: "bandwidth",
    component: () => import("@/pages/bandwidth/Page.vue"),
  },
  {
    path: "/api-test",
    name: "api-test",
    component: () => import("@/pages/api-test/Page.vue"),
  },
  {
    path: "/bookmarks",
    name: "bookmarks",
    component: () => import("@/pages/bookmarks/Page.vue"),
  },
  {
    path: "/target-groups",
    name: "target-groups",
    component: () => import("@/pages/target-groups/Page.vue"),
  },
  {
    path: "/settings",
    name: "settings",
    component: () => import("@/pages/settings/Page.vue"),
  },
  {
    path: "/logs",
    name: "logs",
    component: () => import("@/pages/logs/Page.vue"),
  },
  {
    path: "/backup",
    name: "backup",
    component: () => import("@/pages/backup/Page.vue"),
  },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

export default router;
