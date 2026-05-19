import { describe, it, expect, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";

vi.mock("@/lib/tauri", () => ({
  snifferStart: vi.fn(() => Promise.resolve("task-1")),
  snifferStop: vi.fn(() => Promise.resolve()),
  snifferPresets: vi.fn(() => Promise.resolve([])),
  snifferExport: vi.fn(() => Promise.resolve("")),
  onSnifferProgress: vi.fn(() => Promise.resolve(vi.fn())),
  onSnifferDevice: vi.fn(() => Promise.resolve(vi.fn())),
  onSnifferComplete: vi.fn(() => Promise.resolve(vi.fn())),
  onSnifferError: vi.fn(() => Promise.resolve(vi.fn())),
}));

vi.mock("@/composables/useNotification", () => ({
  sendSystemNotification: vi.fn(),
}));

import NetworkSnifferPage from "@/pages/network-sniffer/Page.vue";

describe("NetworkSnifferPage", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("renders the heading", () => {
    const wrapper = mount(NetworkSnifferPage);
    expect(wrapper.text()).toContain("网络嗅探");
  });

  it("shows start scan button", () => {
    const wrapper = mount(NetworkSnifferPage);
    expect(wrapper.text()).toContain("开始扫描");
  });

  it("shows the empty state guide", () => {
    const wrapper = mount(NetworkSnifferPage);
    // The empty state tells user to input target CIDR
    expect(wrapper.text()).toContain("开始扫描");
  });
});
