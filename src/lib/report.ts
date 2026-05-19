export interface ReportColumn {
  key: string;
  label: string;
}

export interface ReportData {
  title: string;
  columns: ReportColumn[];
  rows: Record<string, unknown>[];
  timestamp: string;
}

/**
 * Generate a complete HTML report document with inline styles.
 * Supports dark/light theme via CSS custom properties.
 * Printable via @media print.
 */
export function generateHtmlReport(data: ReportData): string {
  return `<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>${escapeHtml(data.title)}</title>
<style>
  :root {
    --bg: #ffffff;
    --bg-card: #f8f9fa;
    --text: #1a1a2e;
    --text-soft: #6b7280;
    --border: #e5e7eb;
    --accent: #10b981;
    --row-even: #f3f4f6;
    --row-hover: #e5e7eb;
  }

  @media (prefers-color-scheme: dark) {
    :root {
      --bg: #1a1a2e;
      --bg-card: #16213e;
      --text: #e5e7eb;
      --text-soft: #9ca3af;
      --border: #374151;
      --accent: #34d399;
      --row-even: #1f2937;
      --row-hover: #374151;
    }
  }

  * { margin: 0; padding: 0; box-sizing: border-box; }

  body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    background: var(--bg);
    color: var(--text);
    padding: 2rem;
    line-height: 1.6;
  }

  .header {
    margin-bottom: 2rem;
    padding-bottom: 1rem;
    border-bottom: 2px solid var(--accent);
  }

  .header h1 {
    font-size: 1.75rem;
    font-weight: 700;
    color: var(--text);
    margin-bottom: 0.5rem;
  }

  .header .meta {
    color: var(--text-soft);
    font-size: 0.875rem;
  }

  .summary {
    background: var(--bg-card);
    border-radius: 0.5rem;
    padding: 1rem;
    margin-bottom: 1.5rem;
    border: 1px solid var(--border);
    font-size: 0.875rem;
    color: var(--text-soft);
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.875rem;
  }

  thead th {
    background: var(--accent);
    color: #ffffff;
    text-align: left;
    padding: 0.75rem 1rem;
    font-weight: 600;
    white-space: nowrap;
  }

  thead th:first-child { border-radius: 0.5rem 0 0 0; }
  thead th:last-child { border-radius: 0 0.5rem 0 0; }

  tbody td {
    padding: 0.625rem 1rem;
    border-bottom: 1px solid var(--border);
    vertical-align: top;
  }

  tbody tr:nth-child(even) { background: var(--row-even); }
  tbody tr:hover { background: var(--row-hover); }

  .footer {
    margin-top: 2rem;
    padding-top: 1rem;
    border-top: 1px solid var(--border);
    text-align: center;
    color: var(--text-soft);
    font-size: 0.75rem;
  }

  @media print {
    body { padding: 0; }
    thead th { background: var(--accent) !important; color: white !important; -webkit-print-color-adjust: exact; print-color-adjust: exact; }
    tbody tr:nth-child(even) { background: var(--row-even) !important; -webkit-print-color-adjust: exact; print-color-adjust: exact; }
  }
</style>
</head>
<body>
<div class="header">
  <h1>${escapeHtml(data.title)}</h1>
  <div class="meta">
    生成时间: ${escapeHtml(data.timestamp)} &nbsp;|&nbsp;
    记录数: ${data.rows.length}
  </div>
</div>

<div class="summary">
  共 ${data.columns.length} 列, ${data.rows.length} 条记录
</div>

<table>
  <thead>
    <tr>
      ${data.columns.map(col => `<th>${escapeHtml(col.label)}</th>`).join('')}
    </tr>
  </thead>
  <tbody>
    ${data.rows.map(row => {
      const cells = data.columns.map(col => {
        const value = row[col.key];
        const display = value == null ? '' : String(value);
        return `<td>${escapeHtml(display)}</td>`;
      }).join('');
      return `<tr>${cells}</tr>`;
    }).join('')}
  </tbody>
</table>

<div class="footer">
  AzurePath 报告导出 &mdash; ${escapeHtml(data.timestamp)}
</div>
</body>
</html>`;
}

function escapeHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}
