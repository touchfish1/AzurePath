import { describe, it, expect, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";

// Mock Tauri bridge so stores can import without errors
vi.mock("@/lib/tauri", () => ({
  portScanStart: vi.fn(),
  portScanStop: vi.fn(),
  onPortProgress: vi.fn(() => Promise.resolve(vi.fn())),
  onPortFound: vi.fn(() => Promise.resolve(vi.fn())),
  onPortComplete: vi.fn(() => Promise.resolve(vi.fn())),
  onPortError: vi.fn(() => Promise.resolve(vi.fn())),
}));

import PortScanPage from "@/pages/port-scan/Page.vue";
import { usePortScanStore } from "@/stores/portScan";

describe("PortScanPage", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("shows empty state guide when no ports scanned and not running", () => {
    const wrapper = mount(PortScanPage);
    expect(wrapper.text()).toContain("输入目标 IP 或域名开始端口扫描");
  });

  it('shows "未发现开放端口" after scan completes with no results', () => {
    const store = usePortScanStore();
    store.completeInfo = {
      target: "127.0.0.1",
      port_start: 1,
      port_end: 1024,
      open: 0,
      scanned: 1024,
      duration_ms: 5000,
    };

    const wrapper = mount(PortScanPage);
    expect(wrapper.text()).toContain("未发现开放端口");
    expect(wrapper.text()).not.toContain("输入目标 IP 或域名开始端口扫描");
  });

  it("hides empty state when there are found ports", () => {
    const store = usePortScanStore();
    store.foundPorts = [
      { port: 80, service: "http" },
    ];
    store.completeInfo = {
      target: "127.0.0.1",
      port_start: 1,
      port_end: 1024,
      open: 1,
      scanned: 1024,
      duration_ms: 3000,
    };

    const wrapper = mount(PortScanPage);
    expect(wrapper.text()).toContain("80");
    expect(wrapper.text()).not.toContain("输入目标 IP 或域名开始端口扫描");
  });
});
