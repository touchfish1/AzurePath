import { describe, it, expect, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";

vi.mock("@/lib/tauri", () => ({}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(() => Promise.resolve([])),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(vi.fn())),
}));

import MonitorPage from "@/pages/monitor/Page.vue";

describe("MonitorPage", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("renders the heading", () => {
    const wrapper = mount(MonitorPage);
    expect(wrapper.text()).toContain("网络性能监控");
  });

  it("shows start monitoring button", () => {
    const wrapper = mount(MonitorPage);
    expect(wrapper.text()).toContain("开始监控");
  });

  it("shows add target form", () => {
    const wrapper = mount(MonitorPage);
    expect(wrapper.text()).toContain("添加监控目标");
    expect(wrapper.text()).toContain("主机地址");
    expect(wrapper.text()).toContain("标签");
  });

  it("shows empty targets state", () => {
    const wrapper = mount(MonitorPage);
    expect(wrapper.text()).toContain("暂无目标");
  });

  it("shows monitoring not started state", () => {
    const wrapper = mount(MonitorPage);
    expect(wrapper.text()).toContain("监控未启动");
  });
});
