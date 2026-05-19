import { describe, it, expect, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";

const mockInvoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({
  invoke: mockInvoke,
}));

vi.mock("@/lib/tauri", () => ({}));

import WolPage from "@/pages/wol/Page.vue";

describe("WolPage", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
    mockInvoke.mockResolvedValue([]);
  });

  it("renders the heading", () => {
    const wrapper = mount(WolPage);
    expect(wrapper.text()).toContain("Wake-on-LAN");
  });

  it("shows send form", () => {
    const wrapper = mount(WolPage);
    expect(wrapper.text()).toContain("发送魔术包");
    expect(wrapper.text()).toContain("MAC 地址");
    expect(wrapper.text()).toContain("广播地址");
    expect(wrapper.text()).toContain("端口");
  });

  it("shows empty saved devices state", () => {
    const wrapper = mount(WolPage);
    expect(wrapper.text()).toContain("暂无保存的设备");
  });

  it("shows save button", () => {
    const wrapper = mount(WolPage);
    expect(wrapper.text()).toContain("保存设备");
  });
});
