import { describe, it, expect, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";

// Mock Tauri bridge so stores can import without errors
vi.mock("@/lib/tauri", () => ({
  tracerouteStart: vi.fn(),
  tracerouteStop: vi.fn(),
  onTraceHop: vi.fn(() => Promise.resolve(vi.fn())),
  onTraceComplete: vi.fn(() => Promise.resolve(vi.fn())),
  onTraceError: vi.fn(() => Promise.resolve(vi.fn())),
}));

import TraceroutePage from "@/pages/traceroute/Page.vue";
import { useTracerouteStore } from "@/stores/traceroute";

describe("TraceroutePage", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("shows empty state guide when no hops and not running", () => {
    const wrapper = mount(TraceroutePage);
    expect(wrapper.text()).toContain("输入目标 IP 或域名开始路由追踪");
  });

  it("hides empty state when there are results", () => {
    const store = useTracerouteStore();
    store.hops = [
      { hop: 1, addr: "192.168.1.1", hostname: null, latencies: [1.2, 1.5, 1.3] },
    ];

    const wrapper = mount(TraceroutePage);
    expect(wrapper.text()).not.toContain("输入目标 IP 或域名开始路由追踪");
    expect(wrapper.text()).toContain("192.168.1.1");
  });
});
