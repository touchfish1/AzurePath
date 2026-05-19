import { describe, it, expect, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { createRouter, createWebHistory } from "vue-router";
import { createPinia, setActivePinia } from "pinia";

vi.mock("@/lib/tauri", () => ({
  discoveryPeers: vi.fn(() => Promise.resolve([])),
  onPeerList: vi.fn(() => Promise.resolve(vi.fn())),
  onPeerOffline: vi.fn(() => Promise.resolve(vi.fn())),
  onTopologyProgress: vi.fn(() => Promise.resolve(vi.fn())),
  onTopologyResult: vi.fn(() => Promise.resolve(vi.fn())),
  onTopologyError: vi.fn(() => Promise.resolve(vi.fn())),
}));

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: "/", name: "topology", component: { template: "<div />" } },
    { path: "/chat", name: "chat", component: { template: "<div />" } },
  ],
});

import TopologyPage from "@/pages/topology/Page.vue";

describe("TopologyPage", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("renders the heading", () => {
    const wrapper = mount(TopologyPage, {
      global: { plugins: [router] },
    });
    expect(wrapper.text()).toContain("网络拓扑");
  });

  it("shows empty state when no peers", () => {
    const wrapper = mount(TopologyPage, {
      global: { plugins: [router] },
    });
    expect(wrapper.text()).toContain("未发现网络设备");
  });

  it("shows zoom controls", () => {
    const wrapper = mount(TopologyPage, {
      global: { plugins: [router] },
    });
    expect(wrapper.text()).toContain("重置");
  });
});
