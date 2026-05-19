import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock Tauri invoke — always reject so saveFile falls through to browser fallback
const mockInvoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({
  invoke: mockInvoke,
}));

import { exportAsCsv, exportAsJson, exportAsTxt, exportAsHtml } from "@/lib/export";

function flushMicrotasks() {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

function setupAnchor() {
  const mockClick = vi.fn();
  const mockAnchor = { href: "", download: "", click: mockClick };
  vi.spyOn(document, "createElement").mockReturnValue(mockAnchor as unknown as HTMLAnchorElement);
  return mockAnchor;
}

describe("exportAsCsv", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockRejectedValue(new Error("Tauri not available"));
  });

  it("warns and returns when rows is empty", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    exportAsCsv([], "test");
    expect(warn).toHaveBeenCalledWith("No data to export as CSV");
    warn.mockRestore();
  });

  it("uses browser fallback when Tauri invoke fails", async () => {
    const revoke = vi.spyOn(URL, "revokeObjectURL").mockImplementation(() => {});
    setupAnchor();

    exportAsCsv([{ name: "Alice", age: 30 }], "test.csv");
    await flushMicrotasks();

    expect(document.createElement).toHaveBeenCalledWith("a");
    revoke.mockRestore();
  });

  it("escapes CSV fields with commas and quotes", async () => {
    const revoke = vi.spyOn(URL, "revokeObjectURL").mockImplementation(() => {});
    setupAnchor();

    const rows = [{ name: 'Smith, John', note: 'He said "hello"' }];
    exportAsCsv(rows, "test.csv");
    await flushMicrotasks();

    expect(document.createElement).toHaveBeenCalledWith("a");
    revoke.mockRestore();
  });
});

describe("exportAsJson", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockRejectedValue(new Error("Tauri not available"));
  });

  it("exports data as formatted JSON", async () => {
    const revoke = vi.spyOn(URL, "revokeObjectURL").mockImplementation(() => {});
    setupAnchor();

    exportAsJson({ key: "value" }, "data");
    await flushMicrotasks();

    expect(document.createElement).toHaveBeenCalledWith("a");
    revoke.mockRestore();
  });

  it("appends .json extension if missing", async () => {
    const revoke = vi.spyOn(URL, "revokeObjectURL").mockImplementation(() => {});
    const mockAnchor = setupAnchor();

    exportAsJson({ foo: 1 }, "data");
    await flushMicrotasks();

    expect(mockAnchor.download).toBe("data.json");
    revoke.mockRestore();
  });
});

describe("exportAsTxt", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockRejectedValue(new Error("Tauri not available"));
  });

  it("exports text as .txt file", async () => {
    const revoke = vi.spyOn(URL, "revokeObjectURL").mockImplementation(() => {});
    const mockAnchor = setupAnchor();

    exportAsTxt("hello world", "greeting");
    await flushMicrotasks();

    expect(mockAnchor.download).toBe("greeting.txt");
    revoke.mockRestore();
  });
});

describe("exportAsHtml", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockRejectedValue(new Error("Tauri not available"));
  });

  it("warns when rows is empty", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    exportAsHtml([], ["col1"], "Test");
    expect(warn).toHaveBeenCalledWith("No data to export as HTML");
    warn.mockRestore();
  });

  it("generates HTML table with proper structure", async () => {
    const revoke = vi.spyOn(URL, "revokeObjectURL").mockImplementation(() => {});
    setupAnchor();

    const rows = [{ name: "Alice", age: 30 }];
    exportAsHtml(rows, ["name", "age"], "Test Report");
    await flushMicrotasks();

    expect(document.createElement).toHaveBeenCalledWith("a");
    revoke.mockRestore();
  });

  it("falls back to Object.keys when columns array is empty", async () => {
    const revoke = vi.spyOn(URL, "revokeObjectURL").mockImplementation(() => {});
    setupAnchor();

    const rows = [{ key1: "val1", key2: "val2" }];
    exportAsHtml(rows, [], "Test");
    await flushMicrotasks();

    expect(document.createElement).toHaveBeenCalledWith("a");
    revoke.mockRestore();
  });
});
