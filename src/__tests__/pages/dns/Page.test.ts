import { describe, it, expect, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";

vi.mock("@/lib/tauri", () => ({
  dnsLookup: vi.fn(() => Promise.resolve([])),
  onDnsResult: vi.fn(() => Promise.resolve(vi.fn())),
  onDnsError: vi.fn(() => Promise.resolve(vi.fn())),
}));

import DnsPage from "@/pages/dns/Page.vue";

describe("DnsPage", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("renders the heading", () => {
    const wrapper = mount(DnsPage);
    expect(wrapper.text()).toContain("DNS 查询");
  });

  it("shows record type options", () => {
    const wrapper = mount(DnsPage);
    expect(wrapper.text()).toContain("A");
    expect(wrapper.text()).toContain("AAAA");
    expect(wrapper.text()).toContain("CNAME");
    expect(wrapper.text()).toContain("MX");
  });

  it("has empty initial state with no errors", () => {
    const wrapper = mount(DnsPage);
    expect(wrapper.text()).not.toContain("查询中");
  });
});
