/**
 * Format an ISO date string into a human-friendly display.
 * - Today: "HH:mm"
 * - Yesterday: "昨天 HH:mm"
 * - This week: "周X HH:mm"
 * - Older: "MM/DD HH:mm"
 */
export function formatTime(iso: string): string {
  try {
    const d = new Date(iso);
    const now = new Date();
    const todayStart = new Date(now.getFullYear(), now.getMonth(), now.getDate());
    const yesterdayStart = new Date(todayStart.getTime() - 86400000);
    const weekDayNames = ["日", "一", "二", "三", "四", "五", "六"];
    const timeStr = d.toLocaleTimeString("zh-CN", {
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    });

    if (d >= todayStart) {
      return timeStr;
    }
    if (d >= yesterdayStart) {
      return `昨天 ${timeStr}`;
    }
    if (d >= new Date(todayStart.getTime() - 6 * 86400000)) {
      return `周${weekDayNames[d.getDay()]} ${timeStr}`;
    }
    return `${(d.getMonth() + 1).toString().padStart(2, "0")}/${d.getDate().toString().padStart(2, "0")} ${timeStr}`;
  } catch {
    return iso;
  }
}

/**
 * Format a byte count into a human-readable size string.
 * E.g., 1536 → "1.5 KB"
 */
export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

/**
 * Truncate a string to a maximum length, appending "..." if truncated.
 */
export function truncate(text: string, len: number): string {
  return text.length > len ? text.slice(0, len) + "..." : text;
}

/**
 * Calculate the percentage of received data out of total.
 */
export function progressPercent(received: number, total: number): number {
  if (total === 0) return 0;
  return Math.round((received / total) * 100);
}
