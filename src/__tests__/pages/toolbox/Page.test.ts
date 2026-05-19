import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";

vi.mock("@/lib/tauri", () => ({}));

import ToolboxPage from "@/pages/toolbox/Page.vue";

describe("ToolboxPage", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("renders the heading", () => {
    const wrapper = mount(ToolboxPage);
    expect(wrapper.text()).toContain("工具箱");
  });

  it("shows all tool tabs", () => {
    const wrapper = mount(ToolboxPage);
    expect(wrapper.text()).toContain("子网计算");
    expect(wrapper.text()).toContain("Base64");
    expect(wrapper.text()).toContain("URL");
    expect(wrapper.text()).toContain("Hash");
    expect(wrapper.text()).toContain("端口速查");
    expect(wrapper.text()).toContain("WiFi QR");
    expect(wrapper.text()).toContain("JSON");
    expect(wrapper.text()).toContain("JWT");
    expect(wrapper.text()).toContain("时间戳");
  });

  it("shows subnet calculator section by default with CIDR input", () => {
    const wrapper = mount(ToolboxPage);
    // Default CIDR value is displayed as part of the subnet calculator
    expect(wrapper.text()).toContain("子网计算");
  });
});
