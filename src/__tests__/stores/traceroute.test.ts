import { describe, it, expect, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";

vi.mock("@/lib/tauri", () => ({
  tracerouteStart: vi.fn(() => Promise.resolve("task-789")),
  tracerouteStop: vi.fn(() => Promise.resolve()),
  onTraceHop: vi.fn(() => Promise.resolve(vi.fn())),
  onTraceComplete: vi.fn(() => Promise.resolve(vi.fn())),
  onTraceError: vi.fn(() => Promise.resolve(vi.fn())),
}));

import { useTracerouteStore } from "@/stores/traceroute";

describe("traceroute store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("has default values", () => {
    const store = useTracerouteStore();
    expect(store.target).toBe("8.8.8.8");
    expect(store.maxHops).toBe(30);
    expect(store.timeout).toBe(3000);
    expect(store.running).toBe(false);
    expect(store.hops).toEqual([]);
    expect(store.completeInfo).toBeNull();
  });

  it("reset clears hops, completeInfo, and error", () => {
    const store = useTracerouteStore();
    store.hops = [{ hop: 1, addr: "192.168.1.1", hostname: null, latencies: [1.2] }];
    store.completeInfo = { task_id: "t1", target: "8.8.8.8", hops: [] };
    store.error = "error";

    store.reset();
    expect(store.hops).toEqual([]);
    expect(store.completeInfo).toBeNull();
    expect(store.error).toBe("");
  });

  it("start sets running and clears old data", async () => {
    const store = useTracerouteStore();
    store.hops = [{ hop: 1, addr: "192.168.1.1", hostname: null, latencies: [1.2] }];

    await store.start();
    expect(store.running).toBe(true);
    expect(store.hops).toEqual([]);
    expect(store.currentTaskId).toBe("task-789");
  });

  it("start does nothing when target is empty", async () => {
    const store = useTracerouteStore();
    store.target = "";
    await store.start();
    expect(store.running).toBe(false);
  });

  it("stop clears running state", async () => {
    const store = useTracerouteStore();
    store.running = true;
    store.currentTaskId = "task-789";

    await store.stop();
    expect(store.running).toBe(false);
    expect(store.currentTaskId).toBe("");
  });
});
