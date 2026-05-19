import { describe, it, expect, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { createRouter, createWebHistory } from "vue-router";

vi.mock("@/lib/tauri", () => ({}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(() => Promise.resolve([])),
}));

import DashboardPage from "@/pages/dashboard/Page.vue";

const router = createRouter({
  history: createWebHistory(),
  routes: [{ path: "/", name: "dashboard", component: { template: "<div />" } }],
});

describe("DashboardPage", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    localStorage.clear();
  });

  it("renders the heading", async () => {
    const wrapper = mount(DashboardPage, {
      global: { plugins: [router] },
    });
    expect(wrapper.text()).toContain("网络工具集");
  });

  it("shows tool cards", async () => {
    const wrapper = mount(DashboardPage, {
      global: { plugins: [router] },
    });
    expect(wrapper.text()).toContain("Ping");
    expect(wrapper.text()).toContain("Traceroute");
    expect(wrapper.text()).toContain("端口扫描");
    expect(wrapper.text()).toContain("DNS 查询");
  });

  it("shows empty chart state when no data", async () => {
    const wrapper = mount(DashboardPage, {
      global: { plugins: [router] },
    });
    expect(wrapper.text()).toContain("暂无图表数据");
  });
});
