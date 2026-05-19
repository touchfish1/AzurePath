import { describe, it, expect, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";

const mockInvoke = vi.hoisted(() => vi.fn().mockResolvedValue({}));
vi.mock("@tauri-apps/api/core", () => ({
  invoke: mockInvoke,
}));

vi.mock("@tauri-apps/plugin-autostart", () => ({
  isEnabled: vi.fn(() => Promise.resolve(false)),
  enable: vi.fn(),
  disable: vi.fn(),
}));

import SettingsPage from "@/pages/settings/Page.vue";

describe("SettingsPage", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
  });

  it("renders the heading", () => {
    const wrapper = mount(SettingsPage);
    expect(wrapper.text()).toContain("设置");
  });

  it("shows appearance options (light/dark/system)", () => {
    const wrapper = mount(SettingsPage);
    expect(wrapper.text()).toContain("外观");
    expect(wrapper.text()).toContain("浅色");
    expect(wrapper.text()).toContain("深色");
    expect(wrapper.text()).toContain("跟随系统");
  });

  it("shows clipboard settings", () => {
    const wrapper = mount(SettingsPage);
    expect(wrapper.text()).toContain("剪贴板");
  });

  it("shows save button", () => {
    const wrapper = mount(SettingsPage);
    expect(wrapper.text()).toContain("保存设置");
  });
});
