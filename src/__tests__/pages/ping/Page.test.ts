import { describe, it, expect, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";

// Mock Tauri bridge so stores can import without errors
vi.mock("@/lib/tauri", () => ({
  pingStart: vi.fn(),
  pingStop: vi.fn(),
  onPingProgress: vi.fn(() => Promise.resolve(vi.fn())),
  onPingComplete: vi.fn(() => Promise.resolve(vi.fn())),
  onPingError: vi.fn(() => Promise.resolve(vi.fn())),
}));

import PingPage from "@/pages/ping/Page.vue";
import { usePingStore } from "@/stores/ping";

describe("PingPage", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("shows empty state guide when no results and not running", () => {
    const wrapper = mount(PingPage);
    expect(wrapper.text()).toContain("输入目标 IP 或域名开始 Ping 测试");
  });

  it("hides empty state when there are results", () => {
    const store = usePingStore();
    store.results = [
      { seq: 1, ttl: 64, latencyMs: 10.5, status: "success" },
    ];

    const wrapper = mount(PingPage);
    expect(wrapper.text()).not.toContain("输入目标 IP 或域名开始 Ping 测试");
    expect(wrapper.text()).toContain("10.5");
  });
});
