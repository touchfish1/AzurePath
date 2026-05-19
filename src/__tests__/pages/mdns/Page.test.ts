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

import MdnsPage from "@/pages/mdns/Page.vue";

describe("MdnsPage", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("renders the heading", () => {
    const wrapper = mount(MdnsPage);
    expect(wrapper.text()).toContain("mDNS 服务发现");
  });

  it("shows scan button", () => {
    const wrapper = mount(MdnsPage);
    expect(wrapper.text()).toContain("扫描");
  });

  it("has initial empty state", () => {
    const wrapper = mount(MdnsPage);
    expect(wrapper.text()).not.toContain("未发现 mDNS 服务");
  });
});
