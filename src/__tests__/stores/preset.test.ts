import { describe, it, expect, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";

vi.mock("@/lib/tauri", () => ({
  savePreset: vi.fn((name: string, feature: string, params: string) =>
    Promise.resolve({ id: "p1", name, feature, params: JSON.parse(params), createdAt: "2025-01-01", updatedAt: "2025-01-01" }),
  ),
  loadPresets: vi.fn((feature?: string) => {
    const all = [
      { id: "p1", name: "Web Scan", feature: "portScan", params: { ports: [80, 443] }, createdAt: "2025-01-01", updatedAt: "2025-01-01" },
    ];
    return Promise.resolve(feature ? all.filter((p) => p.feature === feature) : all);
  }),
  deletePreset: vi.fn(() => Promise.resolve()),
}));

import { usePresetStore } from "@/stores/preset";

describe("preset store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("starts with empty presets", () => {
    const store = usePresetStore();
    expect(store.presets).toEqual([]);
    expect(store.loading).toBe(false);
  });

  it("load populates presets", async () => {
    const store = usePresetStore();
    await store.load();
    expect(store.presets.length).toBeGreaterThan(0);
    expect(store.presets[0].name).toBe("Web Scan");
  });

  it("load with feature filters presets", async () => {
    const store = usePresetStore();
    await store.load("portScan");
    expect(store.presets.length).toBeGreaterThan(0);
  });

  it("load handles errors gracefully", async () => {
    const { loadPresets } = await import("@/lib/tauri");
    (loadPresets as any).mockRejectedValueOnce(new Error("fail"));

    const store = usePresetStore();
    await store.load();
    expect(store.presets).toEqual([]);
    expect(store.loading).toBe(false);
  });

  it("save adds a preset to the list", async () => {
    const store = usePresetStore();
    await store.save("New Preset", "ping", { count: 5 });
    expect(store.presets.length).toBeGreaterThan(0);
    const saved = store.presets.find((p) => p.name === "New Preset");
    expect(saved).toBeDefined();
  });

  it("remove deletes a preset", async () => {
    const store = usePresetStore();
    await store.load();
    const count = store.presets.length;

    await store.remove("p1");
    expect(store.presets.length).toBe(count - 1);
  });
});
