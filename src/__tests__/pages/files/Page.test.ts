import { describe, it, expect, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";

vi.mock("@/lib/tauri", () => ({
  lanInit: vi.fn(() => Promise.resolve()),
  fileList: vi.fn(() => Promise.resolve([])),
  fileAccept: vi.fn(() => Promise.resolve()),
  fileReject: vi.fn(() => Promise.resolve()),
  getFileDownloadUrl: vi.fn(() => Promise.resolve("http://example.com/file")),
  discoveryPeers: vi.fn(() => Promise.resolve([])),
}));

vi.mock("@/composables/useFileTransfer", () => ({
  useFileTransferListeners: () => ({ setup: vi.fn(), teardown: vi.fn() }),
}));

vi.mock("@/composables/useNotification", () => ({
  sendSystemNotification: vi.fn(),
}));

import FilesPage from "@/pages/files/Page.vue";

describe("FilesPage", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("renders the heading", () => {
    const wrapper = mount(FilesPage);
    expect(wrapper.text()).toContain("文件传输");
  });

  it("shows loading state initially", () => {
    const wrapper = mount(FilesPage);
    expect(wrapper.text()).toContain("加载中");
  });

  it("shows transfer records header", () => {
    const wrapper = mount(FilesPage);
    expect(wrapper.text()).toContain("传输记录");
  });
});
