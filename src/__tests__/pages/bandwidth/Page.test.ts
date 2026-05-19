import { describe, it, expect, beforeEach, vi } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";

vi.mock("@/lib/tauri", () => ({}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(() => Promise.resolve([])),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(vi.fn())),
}));

// ResizeObserver must be a proper constructor (not vi.fn()) for `new ResizeObserver(...)` to work
class MockResizeObserver {
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
}
vi.stubGlobal("ResizeObserver", MockResizeObserver);

import BandwidthPage from "@/pages/bandwidth/Page.vue";

describe("BandwidthPage", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("renders the heading", () => {
    const wrapper = mount(BandwidthPage);
    expect(wrapper.text()).toContain("带宽监控");
  });

  it("shows start monitoring button", () => {
    const wrapper = mount(BandwidthPage);
    expect(wrapper.text()).toContain("开始监控");
  });

  it("shows interface selector", () => {
    const wrapper = mount(BandwidthPage);
    expect(wrapper.text()).toContain("网络接口");
    expect(wrapper.text()).toContain("全部接口");
  });

  it("shows current speed stats section", () => {
    const wrapper = mount(BandwidthPage);
    expect(wrapper.text()).toContain("下载");
    expect(wrapper.text()).toContain("上传");
  });
});
