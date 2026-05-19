import { describe, it, expect, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";

vi.mock("@/lib/tauri", () => ({
  lanInit: vi.fn(() => Promise.resolve()),
  chatSend: vi.fn(() => Promise.resolve({ id: "1", peer_id: "p1", peer_name: "Test", peer_ip: "192.168.1.2", content: "hi", is_broadcast: false, is_incoming: false, file_ref: null, created_at: "2025-01-01T00:00:00Z" })),
  chatBroadcast: vi.fn(() => Promise.resolve({ id: "1", peer_id: "*", peer_name: "Me", peer_ip: "0.0.0.0", content: "broadcast", is_broadcast: true, is_incoming: false, file_ref: null, created_at: "2025-01-01T00:00:00Z" })),
  chatMessages: vi.fn(() => Promise.resolve([])),
  chatSearch: vi.fn(() => Promise.resolve([])),
  chatDelete: vi.fn(() => Promise.resolve()),
  chatClear: vi.fn(() => Promise.resolve()),
  fileSend: vi.fn(() => Promise.resolve({ file_id: "f1", file_size: 100 })),
  fileBroadcast: vi.fn(() => Promise.resolve({ file_id: "f2", file_size: 200 })),
  fileAccept: vi.fn(() => Promise.resolve()),
  fileReject: vi.fn(() => Promise.resolve()),
  fileList: vi.fn(() => Promise.resolve([])),
  discoveryPeers: vi.fn(() => Promise.resolve([])),
  onChatMessage: vi.fn(() => Promise.resolve(vi.fn())),
  onPeerList: vi.fn(() => Promise.resolve(vi.fn())),
  onPeerOffline: vi.fn(() => Promise.resolve(vi.fn())),
}));

vi.mock("@/composables/useFileTransfer", () => ({
  useFileTransferListeners: () => ({ setup: vi.fn(), teardown: vi.fn() }),
}));

vi.mock("@/composables/useNotification", () => ({
  sendSystemNotification: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(() => Promise.resolve(null)),
}));

import ChatPage from "@/pages/chat/Page.vue";

describe("ChatPage", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("renders the heading", () => {
    const wrapper = mount(ChatPage);
    expect(wrapper.text()).toContain("聊天");
  });

  it("shows empty state with no peers message", () => {
    const wrapper = mount(ChatPage);
    expect(wrapper.text()).toContain("未发现局域网设备");
  });

  it("shows online count", () => {
    const wrapper = mount(ChatPage);
    expect(wrapper.text()).toContain("在线");
  });
});
