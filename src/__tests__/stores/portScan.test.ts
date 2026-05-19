import { describe, it, expect, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";

vi.mock("@/lib/tauri", () => ({
  portScanStart: vi.fn(() => Promise.resolve("task-456")),
  portScanStop: vi.fn(() => Promise.resolve()),
  onPortProgress: vi.fn(() => Promise.resolve(vi.fn())),
  onPortFound: vi.fn(() => Promise.resolve(vi.fn())),
  onPortComplete: vi.fn(() => Promise.resolve(vi.fn())),
  onPortError: vi.fn(() => Promise.resolve(vi.fn())),
}));

import { usePortScanStore } from "@/stores/portScan";

describe("portScan store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("has default values", () => {
    const store = usePortScanStore();
    expect(store.target).toBe("127.0.0.1");
    expect(store.portStart).toBe(1);
    expect(store.portEnd).toBe(1024);
    expect(store.running).toBe(false);
    expect(store.foundPorts).toEqual([]);
    expect(store.completeInfo).toBeNull();
    expect(store.progressPercent).toBe(0);
  });

  it("progressPercent is 0 when no progress", () => {
    const store = usePortScanStore();
    expect(store.progressPercent).toBe(0);
  });

  it("progressPercent calculates correctly", () => {
    const store = usePortScanStore();
    store.progress = { task_id: "t1", scanned: 50, total: 100, open: 5 };
    expect(store.progressPercent).toBe(50);
  });

  it("reset clears all data", () => {
    const store = usePortScanStore();
    store.progress = { task_id: "t1", scanned: 50, total: 100, open: 5 };
    store.foundPorts = [{ port: 80, service: "http", task_id: "t1" }];
    store.completeInfo = { task_id: "t1", target: "127.0.0.1", open_ports: [{ port: 80, service: "http" }] };
    store.error = "err";

    store.reset();
    expect(store.progress).toBeNull();
    expect(store.foundPorts).toEqual([]);
    expect(store.completeInfo).toBeNull();
    expect(store.error).toBe("");
  });

  it("start sets running and clears old data", async () => {
    const store = usePortScanStore();
    store.foundPorts = [{ port: 80, service: "http", task_id: "t1" }];

    await store.start();
    expect(store.running).toBe(true);
    expect(store.foundPorts).toEqual([]);
    expect(store.currentTaskId).toBe("task-456");
  });

  it("start rejects invalid port range", async () => {
    const store = usePortScanStore();
    store.portStart = 70000;
    store.portEnd = 80000;

    await store.start();
    expect(store.running).toBe(false);
    expect(store.error).toBe("端口范围无效");
  });

  it("stop clears running state", async () => {
    const store = usePortScanStore();
    store.running = true;
    store.currentTaskId = "task-456";

    await store.stop();
    expect(store.running).toBe(false);
    expect(store.currentTaskId).toBe("");
  });
});
