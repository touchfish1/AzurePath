import { invoke } from "@tauri-apps/api/core";

/**
 * Try to save content to a file. Uses Tauri invoke if available,
 * falls back to browser download.
 */
async function saveFile(filename: string, content: string): Promise<void> {
  try {
    // Try using Tauri save_file command
    const homeDir = await invoke<string>("get_home_dir").catch(() => "");
    const defaultPath = homeDir
      ? `${homeDir}/Desktop/${filename}`
      : `/tmp/${filename}`;
    await invoke("save_file", { path: defaultPath, content });
    return;
  } catch {
    // Fallback to browser download
    const blob = new Blob([content], { type: "application/octet-stream" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  }
}

/**
 * Convert an array of rows to CSV format and trigger a download.
 * The first row's keys are used as headers.
 */
export function exportAsCsv(rows: Record<string, any>[], filename: string): void {
  if (rows.length === 0) {
    console.warn("No data to export as CSV");
    return;
  }

  const headers = Object.keys(rows[0]);
  const csvLines: string[] = [];

  // Header row
  csvLines.push(headers.map(escapeCsvField).join(","));

  // Data rows
  for (const row of rows) {
    csvLines.push(
      headers.map((h) => escapeCsvField(String(row[h] ?? ""))).join(","),
    );
  }

  const content = csvLines.join("\r\n");
  const safeName = filename.endsWith(".csv") ? filename : `${filename}.csv`;
  saveFile(safeName, content);
}

/**
 * Generate an HTML table report and trigger a download.
 */
export function exportAsHtml(
  rows: Record<string, any>[],
  columns: string[],
  title: string,
  filename?: string,
): void {
  if (rows.length === 0) {
    console.warn("No data to export as HTML");
    return;
  }

  const headers = columns.length > 0 ? columns : Object.keys(rows[0]);

  let html = `<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<title>${escapeHtml(title)}</title>
<style>
  body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; max-width: 1200px; margin: 0 auto; padding: 20px; background: #f5f5f5; }
  h1 { color: #333; font-size: 1.5rem; }
  table { width: 100%; border-collapse: collapse; background: #fff; box-shadow: 0 1px 3px rgba(0,0,0,0.1); border-radius: 8px; overflow: hidden; }
  th { background: #f0f0f0; color: #555; font-weight: 600; padding: 10px 12px; text-align: left; font-size: 0.85rem; text-transform: uppercase; letter-spacing: 0.5px; }
  td { padding: 8px 12px; border-top: 1px solid #eee; color: #333; font-size: 0.9rem; }
  tr:hover td { background: #f9f9f9; }
  .meta { color: #888; font-size: 0.8rem; margin-bottom: 20px; }
</style>
</head>
<body>
<h1>${escapeHtml(title)}</h1>
<p class="meta">导出时间: ${new Date().toLocaleString("zh-CN")} | 共 ${rows.length} 条记录</p>
<table>
<thead><tr>`;

  for (const h of headers) {
    html += `<th>${escapeHtml(h)}</th>`;
  }
  html += "</tr></thead><tbody>";

  for (const row of rows) {
    html += "<tr>";
    for (const h of headers) {
      html += `<td>${escapeHtml(String(row[h] ?? ""))}</td>`;
    }
    html += "</tr>";
  }

  html += `</tbody></table>
<p class="meta">由 AzurePath 生成</p>
</body>
</html>`;

  const safeName = (filename || "export").endsWith(".html") ? (filename || "export") : `${filename || "export"}.html`;
  saveFile(safeName, html);
}

/**
 * Export data as a JSON file.
 */
export function exportAsJson(data: any, filename: string): void {
  const content = JSON.stringify(data, null, 2);
  const safeName = filename.endsWith(".json") ? filename : `${filename}.json`;
  saveFile(safeName, content);
}

/**
 * Export plain text as a .txt file.
 */
export function exportAsTxt(text: string, filename: string): void {
  const safeName = filename.endsWith(".txt") ? filename : `${filename}.txt`;
  saveFile(safeName, text);
}

// ── Helpers ──────────────────────────────────────────────────────

function escapeCsvField(value: string): string {
  if (value.includes(",") || value.includes('"') || value.includes("\n")) {
    return `"${value.replace(/"/g, '""')}"`;
  }
  return value;
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}
