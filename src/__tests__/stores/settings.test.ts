import { describe, it, expect, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";

const mockInvoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({
  invoke: mockInvoke,
}));

import { useSettingsStore } from "@/stores/settings";

describe("settings store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
  });

  it("has default values before loading", () => {
    const store = useSettingsStore();
    expect(store.loaded).toBe(false);
    expect(store.saving).toBe(false);
    expect(store.settings.theme).toBe("system");
    expect(store.settings.clipboardInterval).toBe(1000);
    expect(store.settings.clipboardMaxItems).toBe(500);
    expect(store.settings.pingCount).toBe(4);
    expect(store.settings.retentionDays).toBe(30);
  });

  it("load uses defaults when backend is unavailable", async () => {
    mockInvoke.mockRejectedValueOnce(new Error("Backend not available"));

    const store = useSettingsStore();
    await store.load();
    expect(store.loaded).toBe(true);
    expect(store.settings.theme).toBe("system");
    expect(store.settings.clipboardInterval).toBe(1000);
  });

  it("load merges backend settings with defaults", async () => {
    mockInvoke.mockResolvedValueOnce({
      theme: "dark",
      clipboardInterval: 2000,
    });

    const store = useSettingsStore();
    await store.load();
    expect(store.loaded).toBe(true);
    expect(store.settings.theme).toBe("dark");
    expect(store.settings.clipboardInterval).toBe(2000);
    // Defaults preserved
    expect(store.settings.clipboardMaxItems).toBe(500);
    expect(store.settings.pingCount).toBe(4);
  });

  it("update changes a specific setting", () => {
    const store = useSettingsStore();
    store.update("theme", "dark");
    expect(store.settings.theme).toBe("dark");
    store.update("pingCount", 10);
    expect(store.settings.pingCount).toBe(10);
  });

  it("save calls invoke with settings", async () => {
    mockInvoke.mockResolvedValueOnce(undefined);

    const store = useSettingsStore();
    store.update("theme", "light");
    await store.save();
    expect(mockInvoke).toHaveBeenCalledWith("save_settings", {
      settings: expect.objectContaining({ theme: "light" }),
    });
  });

  it("save sets saving flag", async () => {
    mockInvoke.mockResolvedValueOnce(undefined);

    const store = useSettingsStore();
    const savePromise = store.save();
    expect(store.saving).toBe(true);
    await savePromise;
    expect(store.saving).toBe(false);
  });
});
