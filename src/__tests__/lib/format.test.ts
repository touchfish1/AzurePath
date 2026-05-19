import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { formatTime, formatSize, truncate, progressPercent } from "@/lib/format";

describe("formatTime", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    // Jan 15, 2025 12:00 local — this is a Wednesday
    vi.setSystemTime(new Date(2025, 0, 15, 12, 0, 0));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("returns only time for today", () => {
    const today = new Date(2025, 0, 15, 10, 30, 0);
    expect(formatTime(today.toISOString())).toBe("10:30");
  });

  it('returns "昨天" prefix for yesterday', () => {
    const yesterday = new Date(2025, 0, 14, 10, 30, 0);
    expect(formatTime(yesterday.toISOString())).toBe("昨天 10:30");
  });

  it("returns weekday prefix for earlier this week", () => {
    // Jan 12, 2025 is a Sunday
    const date = new Date(2025, 0, 12, 10, 30, 0);
    const result = formatTime(date.toISOString());
    expect(result).toMatch(/^周日 \d{2}:\d{2}$/);
  });

  it("returns MM/DD prefix for dates older than a week", () => {
    const old = new Date(2024, 11, 25, 10, 30, 0); // Dec 25, 2024
    expect(formatTime(old.toISOString())).toBe("12/25 10:30");
  });

  it("returns NaN/NaN for truly invalid date objects", () => {
    // Date constructor doesn't throw for invalid strings — produces Invalid Date
    expect(formatTime("not-a-date")).toContain("NaN");
  });
});

describe("formatSize", () => {
  it('formats 0 bytes', () => {
    expect(formatSize(0)).toBe("0 B");
  });

  it('formats 1024 bytes as 1.0 KB', () => {
    expect(formatSize(1024)).toBe("1.0 KB");
  });

  it('formats 1048576 bytes as 1.0 MB', () => {
    expect(formatSize(1048576)).toBe("1.0 MB");
  });

  it('formats 1073741824 bytes as 1.00 GB', () => {
    expect(formatSize(1073741824)).toBe("1.00 GB");
  });
});

describe("truncate", () => {
  it("returns the original string when shorter than maxLen", () => {
    expect(truncate("hello", 10)).toBe("hello");
  });

  it("returns the original string when equal to maxLen", () => {
    expect(truncate("exact", 5)).toBe("exact");
  });

  it("truncates and appends ellipsis when longer than maxLen", () => {
    expect(truncate("hello world", 5)).toBe("hello...");
  });

  it("supports custom maxLen", () => {
    expect(truncate("a very long string indeed", 7)).toBe("a very ...");
  });
});

describe("progressPercent", () => {
  it("returns 0 when total is 0", () => {
    expect(progressPercent(0, 0)).toBe(0);
  });

  it("returns 0 for 0 out of 100", () => {
    expect(progressPercent(0, 100)).toBe(0);
  });

  it("returns 50 for 50 out of 100", () => {
    expect(progressPercent(50, 100)).toBe(50);
  });

  it("returns 100 for 100 out of 100", () => {
    expect(progressPercent(100, 100)).toBe(100);
  });

  it("rounds to the nearest integer", () => {
    expect(progressPercent(1, 3)).toBe(33);
  });
});
