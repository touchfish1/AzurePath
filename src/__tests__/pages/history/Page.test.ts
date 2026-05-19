import { describe, it, expect, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { createRouter, createWebHistory } from "vue-router";

vi.mock("@/lib/tauri", () => ({
  clipboardList: vi.fn(() => Promise.resolve([])),
  snifferList: vi.fn(() => Promise.resolve([])),
  discoveryPeers: vi.fn(() => Promise.resolve([])),
  clipboardDelete: vi.fn(() => Promise.resolve()),
  clipboardClear: vi.fn(() => Promise.resolve()),
}));

import HistoryPage from "@/pages/history/Page.vue";

const router = createRouter({
  history: createWebHistory(),
  routes: [{ path: "/", name: "history", component: { template: "<div />" } }],
});

describe("HistoryPage", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("renders the heading", () => {
    const wrapper = mount(HistoryPage, {
      global: { plugins: [router] },
    });
    expect(wrapper.text()).toContain("最近活动");
  });

  it("shows tab options", () => {
    const wrapper = mount(HistoryPage, {
      global: { plugins: [router] },
    });
    expect(wrapper.text()).toContain("全部活动");
    expect(wrapper.text()).toContain("收藏");
    expect(wrapper.text()).toContain("时间线");
  });

  it("shows loading state initially", () => {
    const wrapper = mount(HistoryPage, {
      global: { plugins: [router] },
    });
    expect(wrapper.text()).toContain("加载中");
  });
});
