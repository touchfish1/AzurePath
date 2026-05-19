import { describe, it, expect, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";

vi.mock("@/lib/tauri", () => ({
  pingStart: vi.fn(() => Promise.resolve("task-123")),
  pingStop: vi.fn(() => Promise.resolve()),
  onPingProgress: vi.fn(() => Promise.resolve(vi.fn())),
  onPingComplete: vi.fn(() => Promise.resolve(vi.fn())),
  onPingError: vi.fn(() => Promise.resolve(vi.fn())),
}));

import { usePingStore } from "@/stores/ping";

describe("ping store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("has default values", () => {
    const store = usePingStore();
    expect(store.target).toBe("8.8.8.8");
    expect(store.count).toBe(4);
    expect(store.timeout).toBe(3000);
    expect(store.running).toBe(false);
    expect(store.results).toEqual([]);
    expect(store.stats).toBeNull();
    expect(store.error).toBe("");
  });

  it("reset clears results, stats, and error", () => {
    const store = usePingStore();
    store.results = [{ seq: 1, ttl: 64, latencyMs: 10, status: "success" }];
    store.stats = { task_id: "t1", sent: 1, received: 1, loss_percent: 0, min_ms: 10, avg_ms: 10, max_ms: 10 };
    store.error = "some error";

    store.reset();
    expect(store.results).toEqual([]);
    expect(store.stats).toBeNull();
    expect(store.error).toBe("");
  });

  it("start sets running and clears old data", async () => {
    const store = usePingStore();
    store.results = [{ seq: 1, ttl: 64, latencyMs: 10, status: "success" }];
    store.error = "old error";

    await store.start();
    expect(store.running).toBe(true);
    expect(store.results).toEqual([]);
    expect(store.error).toBe("");
    expect(store.currentTaskId).toBe("task-123");
  });

  it("start does nothing when target is empty", async () => {
    const store = usePingStore();
    store.target = "";
    await store.start();
    expect(store.running).toBe(false);
  });

  it("stop clears running state", async () => {
    const store = usePingStore();
    store.running = true;
    store.currentTaskId = "task-123";

    await store.stop();
    expect(store.running).toBe(false);
    expect(store.currentTaskId).toBe("");
  });

  it("stop does nothing when no currentTaskId", async () => {
    const store = usePingStore();
    store.running = true;
    await store.stop();
    expect(store.running).toBe(true); // unchanged because no taskId
  });
});
