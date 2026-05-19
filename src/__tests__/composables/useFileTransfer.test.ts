import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { defineComponent, nextTick, ref } from "vue";

vi.mock("@/lib/tauri", () => ({
  onFileRequest: vi.fn(() => Promise.resolve(vi.fn())),
  onFileProgress: vi.fn(() => Promise.resolve(vi.fn())),
  onFileComplete: vi.fn(() => Promise.resolve(vi.fn())),
  onFileError: vi.fn(() => Promise.resolve(vi.fn())),
}));

import { useFileTransferListeners } from "@/composables/useFileTransfer";
import type { FileTransfer } from "@/lib/tauri";

function createTestComponent() {
  const transfers = ref<FileTransfer[]>([]);
  const incomingRequest = ref<{ fileId: string; filename: string; size: number; from: string } | null>(null);

  return defineComponent({
    setup() {
      const { setup, teardown } = useFileTransferListeners(transfers, incomingRequest);

      return { transfers, incomingRequest, setup, teardown };
    },
    template: '<div><div data-testid="count">{{ transfers.length }}</div><div data-testid="request">{{ incomingRequest ? incomingRequest.filename : "none" }}</div></div>',
  });
}

describe("useFileTransferListeners", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("is a function", () => {
    expect(typeof useFileTransferListeners).toBe("function");
  });

  it("setup registers listeners", async () => {
    const wrapper = mount(createTestComponent());
    await nextTick();

    const { setup } = wrapper.vm as any;
    await setup();

    const { onFileRequest, onFileProgress, onFileComplete, onFileError } = await import("@/lib/tauri");
    expect(onFileRequest).toHaveBeenCalled();
    expect(onFileProgress).toHaveBeenCalled();
    expect(onFileComplete).toHaveBeenCalled();
    expect(onFileError).toHaveBeenCalled();
  });

  it("teardown cleans up listeners", async () => {
    const unlistenMock = vi.fn();
    const mockTauri = await import("@/lib/tauri");
    (mockTauri.onFileRequest as any).mockResolvedValue(unlistenMock);
    (mockTauri.onFileProgress as any).mockResolvedValue(unlistenMock);
    (mockTauri.onFileComplete as any).mockResolvedValue(unlistenMock);
    (mockTauri.onFileError as any).mockResolvedValue(unlistenMock);

    const wrapper = mount(createTestComponent());
    await nextTick();

    const { setup, teardown } = wrapper.vm as any;
    await setup();
    teardown();

    // Each listener should have been unregistered
    expect(unlistenMock).toHaveBeenCalledTimes(4);
  });
});
