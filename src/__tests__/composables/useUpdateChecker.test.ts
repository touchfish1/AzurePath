import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { defineComponent, nextTick } from "vue";

const mockCheck = vi.hoisted(() => vi.fn());
const mockDownloadAndInstall = vi.hoisted(() => vi.fn());
mockCheck.mockResolvedValue(null);

vi.mock("@tauri-apps/plugin-updater", () => ({
  check: mockCheck,
}));

import { useUpdateChecker } from "@/composables/useUpdateChecker";

function createTestComponent() {
  return defineComponent({
    setup() {
      return useUpdateChecker();
    },
    template: '<div><div data-testid="available">{{ updateAvailable }}</div></div>',
  });
}

describe("useUpdateChecker", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockCheck.mockResolvedValue(null);
  });

  it("returns reactive state", () => {
    const wrapper = mount(createTestComponent());
    const vm = wrapper.vm as any;
    expect(typeof vm.updateAvailable).toBe("boolean");
    expect(typeof vm.checking).toBe("boolean");
    expect(typeof vm.checkForUpdate).toBe("function");
    expect(typeof vm.dismiss).toBe("function");
  });

  it("checkForUpdate sets updateAvailable when update exists", async () => {
    mockCheck.mockResolvedValue({
      available: true,
      version: "2.0.0",
      date: "2025-06-01",
      body: "New features",
      downloadAndInstall: mockDownloadAndInstall,
    });

    const wrapper = mount(createTestComponent());
    const vm = wrapper.vm as any;
    await vm.checkForUpdate();
    expect(vm.updateAvailable).toBe(true);
    expect(vm.updateInfo?.version).toBe("2.0.0");
  });

  it("dismiss clears update state", () => {
    const wrapper = mount(createTestComponent());
    const vm = wrapper.vm as any;

    vm.updateAvailable = true;
    vm.updateInfo = { version: "2.0.0", date: "2025-06-01", body: "New features" };

    vm.dismiss();
    expect(vm.updateAvailable).toBe(false);
    expect(vm.updateInfo).toBeNull();
  });

  it("checkForUpdate handles check failure gracefully", async () => {
    mockCheck.mockRejectedValue(new Error("Network error"));

    const wrapper = mount(createTestComponent());
    const vm = wrapper.vm as any;
    await vm.checkForUpdate();
    expect(vm.checking).toBe(false);
    expect(vm.updateAvailable).toBe(false);
  });

  it("installUpdate calls downloadAndInstall", async () => {
    mockDownloadAndInstall.mockResolvedValue(undefined);
    mockCheck.mockResolvedValue({
      available: true,
      version: "2.0.0",
      date: "2025-06-01",
      body: "New features",
      downloadAndInstall: mockDownloadAndInstall,
    });

    const wrapper = mount(createTestComponent());
    const vm = wrapper.vm as any;

    await vm.installUpdate();
    expect(mockDownloadAndInstall).toHaveBeenCalled();
  });
});
