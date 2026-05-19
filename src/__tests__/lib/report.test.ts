import { describe, it, expect } from "vitest";
import { generateHtmlReport } from "@/lib/report";

const sampleData = {
  title: "Ping Test Report",
  columns: [
    { key: "target", label: "目标" },
    { key: "latency", label: "延迟 (ms)" },
    { key: "loss", label: "丢包率" },
  ],
  rows: [
    { target: "8.8.8.8", latency: 10.5, loss: 0 },
    { target: "1.1.1.1", latency: 15.2, loss: 0 },
  ],
  timestamp: "2025-01-15T12:00:00.000Z",
};

describe("generateHtmlReport", () => {
  it("returns a string starting with DOCTYPE html", () => {
    const html = generateHtmlReport(sampleData);
    expect(html).toMatch(/^<!DOCTYPE html>/);
  });

  it("contains the report title", () => {
    const html = generateHtmlReport(sampleData);
    expect(html).toContain("Ping Test Report");
  });

  it("contains the timestamp", () => {
    const html = generateHtmlReport(sampleData);
    expect(html).toContain("2025-01-15T12:00:00.000Z");
  });

  it("contains data rows", () => {
    const html = generateHtmlReport(sampleData);
    expect(html).toContain("8.8.8.8");
    expect(html).toContain("1.1.1.1");
    expect(html).toContain("10.5");
    expect(html).toContain("15.2");
  });

  it("contains column labels", () => {
    const html = generateHtmlReport(sampleData);
    expect(html).toContain("目标");
    expect(html).toContain("延迟 (ms)");
    expect(html).toContain("丢包率");
  });

  it("handles empty rows", () => {
    const emptyData = {
      title: "Empty Report",
      columns: [{ key: "col1", label: "Column 1" }],
      rows: [],
      timestamp: "2025-01-01T00:00:00.000Z",
    };
    const html = generateHtmlReport(emptyData);
    expect(html).toContain("Empty Report");
    expect(html).toContain("0 条记录");
  });

  it("includes dark theme media query", () => {
    const html = generateHtmlReport(sampleData);
    expect(html).toContain("prefers-color-scheme: dark");
  });

  it("escapes HTML in data values", () => {
    const maliciousData = {
      title: "Test",
      columns: [{ key: "val", label: "Value" }],
      rows: [{ val: '<script>alert("xss")</script>' }],
      timestamp: "2025-01-01T00:00:00.000Z",
    };
    const html = generateHtmlReport(maliciousData);
    expect(html).toContain("&lt;script&gt;");
    expect(html).not.toContain("<script>");
  });

  it("handles null values in rows", () => {
    const dataWithNull = {
      title: "Null Test",
      columns: [{ key: "val", label: "Value" }],
      rows: [{ val: null }],
      timestamp: "2025-01-01T00:00:00.000Z",
    };
    const html = generateHtmlReport(dataWithNull);
    expect(html).toContain("<td></td>");
  });

  it("includes printable styles", () => {
    const html = generateHtmlReport(sampleData);
    expect(html).toContain("@media print");
    expect(html).toContain("print-color-adjust");
  });
});
