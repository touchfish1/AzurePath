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
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

export default router;
