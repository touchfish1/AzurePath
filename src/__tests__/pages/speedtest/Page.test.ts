import { describe, it, expect, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";

vi.mock("@/lib/tauri", () => ({
  startSpeedtest: vi.fn(() => Promise.resolve("task-1")),
  onSpeedtestProgress: vi.fn(() => Promise.resolve(vi.fn())),
  onSpeedtestComplete: vi.fn(() => Promise.resolve(vi.fn())),
}));

import SpeedtestPage from "@/pages/speedtest/Page.vue";

describe("SpeedtestPage", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("renders the heading", () => {
    const wrapper = mount(SpeedtestPage);
    expect(wrapper.text()).toContain("局域网测速");
  });

  it("shows input fields", () => {
    const wrapper = mount(SpeedtestPage);
    expect(wrapper.text()).toContain("对等节点 IP");
    expect(wrapper.text()).toContain("端口");
    expect(wrapper.text()).toContain("时长");
    expect(wrapper.text()).toContain("模式");
  });

  it("shows start button", () => {
    const wrapper = mount(SpeedtestPage);
    expect(wrapper.text()).toContain("开始");
  });

  it("shows empty state initially", () => {
    const wrapper = mount(SpeedtestPage);
    expect(wrapper.text()).toContain("配置对等节点信息开始测速");
  });
});
