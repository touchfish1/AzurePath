<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { RouterLink, useRouter } from "vue-router";
import {
  Radio, Route, Scan, Globe, Activity, BarChart3,
  Grid3X3, TrendingUp, Wifi, ExternalLink, Terminal,
  Star, Clock, Search,
} from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import { getLocalNetworkInfo, type LocalNetworkInfo } from "@/lib/tauri";
import { invoke } from "@tauri-apps/api/core";

const router = useRouter();

// ─── Local Network Info ────────────────────────────────────

const networkInfo = ref<LocalNetworkInfo | null>(null);

async function loadNetworkInfo() {
  try {
    networkInfo.value = await getLocalNetworkInfo();
  } catch { /* ignore */ }
}

// ─── Quick Tool Cards ──────────────────────────────────────

interface ToolCard {
  label: string;
  path: string;
  icon: object;
  description: string;
  color: string;
}

const tools: ToolCard[] = [
  {
    label: "Ping", path: "/ping", icon: Radio,
    description: "连通性检测与延迟测量",
    color: "text-green-500 bg-green-50 dark:bg-green-900/20",
  },
  {
    label: "Traceroute", path: "/traceroute", icon: Route,
    description: "路由追踪与路径分析",
    color: "text-blue-500 bg-blue-50 dark:bg-blue-900/20",
  },
  {
    label: "端口扫描", path: "/port-scan", icon: Scan,
    description: "主机端口开放探测",
    color: "text-orange-500 bg-orange-50 dark:bg-orange-900/20",
  },
  {
    label: "DNS 查询", path: "/dns", icon: Globe,
    description: "域名解析记录查询",
    color: "text-purple-500 bg-purple-50 dark:bg-purple-900/20",
  },
  {
    label: "远程终端", path: "/remote-shell", icon: Terminal,
    description: "SSH/Telnet 远程连接",
    color: "text-cyan-500 bg-cyan-50 dark:bg-cyan-900/20",
  },
  {
    label: "MTR", path: "/mtr", icon: TrendingUp,
    description: "持续路由与性能监测",
    color: "text-pink-500 bg-pink-50 dark:bg-pink-900/20",
  },
];

// ─── Quick Ping ────────────────────────────────────────────

const quickTarget = ref("");
const quickResult = ref<string | null>(null);
const quickRunning = ref(false);

async function doQuickPing() {
  const target = quickTarget.value.trim();
  if (!target || quickRunning.value) return;

  quickRunning.value = true;
  quickResult.value = null;

  try {
    const result = await invoke<string>("ping_start", {
      options: { count: 4, intervalMs: 1000, timeoutMs: 3000, payloadSize: 56 },
      target,
    });
    quickResult.value = result;
  } catch {
    quickResult.value = "请求超时或无响应";
  } finally {
    quickRunning.value = false;
  }
}

function openTool(path: string) {
  if (quickTarget.value.trim()) {
    router.push({ path, query: { target: quickTarget.value.trim() } });
  } else {
    router.push(path);
  }
}

// ─── Recently pinged targets (from localStorage) ───────────

interface PingEntry {
  target: string;
  latency: number;
  timestamp: string;
}

const recentTargets = ref<string[]>([]);

function loadRecentTargets() {
  try {
    const raw = localStorage.getItem("ping_history");
    if (raw) {
      const entries: PingEntry[] = JSON.parse(raw);
      const seen = new Set<string>();
      recentTargets.value = [];
      // Get last 8 unique targets in reverse order
      for (let i = entries.length - 1; i >= 0 && recentTargets.value.length < 8; i--) {
        if (!seen.has(entries[i].target)) {
          seen.add(entries[i].target);
          recentTargets.value.push(entries[i].target);
        }
      }
    }
  } catch { /* ignore */ }
}

// ─── Bookmarks ─────────────────────────────────────────────

interface Bookmark {
  id: string;
  label: string;
  target: string;
  tool: string;
}

const bookmarks = ref<Bookmark[]>([]);

async function loadBookmarks() {
  try {
    bookmarks.value = await invoke<Bookmark[]>("list_bookmarks");
  } catch { /* ignore */ }
}

// ─── Ping History Chart ────────────────────────────────────

interface PingHistoryEntry {
  target: string;
  latency: number;
  timestamp: string;
}

interface PortScanEntry {
  target: string;
  port: number;
  service: string | null;
  state: string;
  timestamp: string;
}

const PING_KEY = "ping_history";
const PORT_KEY = "port_scan_history";

const pingData = ref<PingHistoryEntry[]>([]);
const portData = ref<PortScanEntry[]>([]);

const groupedPingData = computed(() => {
  const groups: Record<string, { latency: number; timestamp: string }[]> = {};
  for (const entry of pingData.value) {
    if (!groups[entry.target]) groups[entry.target] = [];
    groups[entry.target].push({ latency: entry.latency, timestamp: entry.timestamp });
  }
  for (const key of Object.keys(groups)) {
    groups[key].sort((a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime());
    groups[key] = groups[key].slice(-50);
  }
  return groups;
});

const uniqueTargets = computed(() => Object.keys(groupedPingData.value));

const TARGET_COLORS = ["#22c55e", "#3b82f6", "#f59e0b", "#ef4444", "#8b5cf6", "#ec4899"];

const chartWidth = 700;
const chartHeight = 220;
const padLeft = 55;
const padRight = 15;
const padTop = 15;
const padBottom = 25;
const chartW = chartWidth - padLeft - padRight;
const chartH = chartHeight - padTop - padBottom;

const chartData = computed(() => {
  const groups = groupedPingData.value;
  const targets = Object.keys(groups);
  if (targets.length === 0) return { lines: [], yLabels: [] };

  const allPoints: { target: string; latency: number; time: number }[] = [];
  for (const t of targets) {
    for (const p of groups[t]) {
      allPoints.push({ target: t, latency: p.latency, time: new Date(p.timestamp).getTime() });
    }
  }

  if (allPoints.length === 0) return { lines: [], yLabels: [] };

  const minTime = Math.min(...allPoints.map((p) => p.time));
  const maxTime = Math.max(...allPoints.map(p => p.time));
  const timeRange = Math.max(maxTime - minTime, 1);
  const latencies = allPoints.map((p) => p.latency);
  const minLat = Math.min(...latencies);
  const maxLat = Math.max(...latencies);
  const latRange = Math.max(maxLat - minLat, 1);

  const lines: { target: string; color: string; path: string; points: { x: number; y: number; target: string; latency: number }[] }[] = [];
  for (let i = 0; i < targets.length; i++) {
    const t = targets[i];
    const pts = groups[t]
      .map((p) => ({
        x: padLeft + ((new Date(p.timestamp).getTime() - minTime) / timeRange) * chartW,
        y: padTop + chartH - ((p.latency - minLat) / latRange) * chartH,
        target: t,
        latency: p.latency,
      }))
      .sort((a, b) => a.x - b.x);

    if (pts.length < 2) continue;

    let path = `M ${pts[0].x} ${pts[0].y}`;
    for (let j = 1; j < pts.length; j++) path += ` L ${pts[j].x} ${pts[j].y}`;

    lines.push({ target: t, color: TARGET_COLORS[i % TARGET_COLORS.length], path, points: pts });
  }

  const yLabels: { y: number; label: string }[] = [];
  for (let i = 0; i <= 4; i++) {
    const val = maxLat - (i * latRange) / 4;
    yLabels.push({ y: padTop + (i * chartH) / 4, label: val.toFixed(0) });
  }

  return { lines, yLabels };
});

const chartLines = computed(() => chartData.value.lines);
const yAxisLabels = computed(() => chartData.value.yLabels);

const heatmapTargets = computed(() => {
  const targets = new Set<string>();
  for (const p of portData.value) targets.add(p.target);
  return Array.from(targets);
});

const heatmapPorts = computed(() => {
  const ports = new Set<number>();
  for (const p of portData.value) ports.add(p.port);
  return Array.from(ports).sort((a, b) => a - b).slice(0, 30);
});

const portStateMap = computed(() => {
  const map = new Map<string, string>();
  for (const p of portData.value) map.set(`${p.target}:${p.port}`, p.state);
  return map;
});

function portState(target: string, port: number): string {
  return portStateMap.value.get(`${target}:${port}`) || "unknown";
}

// ─── Monitor Sparklines ────────────────────────────────────

interface MonitorPingRecord {
  id: number; targetId: string; targetHost: string; timestamp: string; latencyMs: number | null; lossRate: number;
}

const monitorData = ref<MonitorPingRecord[]>([]);
const monitorLoading = ref(false);

async function loadMonitorSparkline() {
  monitorLoading.value = true;
  try {
    monitorData.value = await invoke<MonitorPingRecord[]>("monitor_get_all_recent_history", { sinceDays: 1 });
  } catch { monitorData.value = []; }
  finally { monitorLoading.value = false; }
}

const monitorSparklines = computed(() => {
  const groups: Record<string, MonitorPingRecord[]> = {};
  for (const r of monitorData.value) {
    if (!groups[r.targetId]) groups[r.targetId] = [];
    groups[r.targetId].push(r);
  }

  const result: { targetHost: string; path: string; color: string }[] = [];
  const colors = ["#22c55e", "#3b82f6", "#f59e0b"];
  let idx = 0;

  for (const [, records] of Object.entries(groups)) {
    const withLatency = records.filter((r): r is MonitorPingRecord & { latencyMs: number } => r.latencyMs !== null);
    if (withLatency.length < 2) { idx++; continue; }

    const w = 300, h = 60, pL = 5, pR = 5, pT = 5, pB = 5;
    const cW = w - pL - pR, cH = h - pT - pB;
    const latencies = withLatency.map((r) => r.latencyMs);
    const minLat = Math.min(...latencies);
    const maxLat = Math.max(...latencies);
    const lRange = Math.max(maxLat - minLat, 1);
    const times = withLatency.map((r) => new Date(r.timestamp).getTime());
    const minTime = Math.min(...times);
    const maxTime = Math.max(...times);
    const tRange = Math.max(maxTime - minTime, 1);

    const pts = withLatency.map((r) => ({
      x: pL + ((new Date(r.timestamp).getTime() - minTime) / tRange) * cW,
      y: pT + cH - ((r.latencyMs - minLat) / lRange) * cH,
    }));

    let path = `M ${pts[0].x} ${pts[0].y}`;
    for (let i = 1; i < pts.length; i++) path += ` L ${pts[i].x} ${pts[i].y}`;

    const latestRecord = withLatency[withLatency.length - 1];
    const hostName = latestRecord.targetHost || "";
    const msLabel = `${latestRecord.latencyMs.toFixed(0)} ms`;
    result.push({ targetHost: hostName ? `${hostName} — ${msLabel}` : msLabel, path, color: colors[idx % colors.length] });
    idx++;
  }
  return result;
});

const chartLinesWithPoints = computed(() => chartLines.value.filter((l) => l.points.length > 0));
const hasChartData = computed(() => uniqueTargets.value.length > 0);
const hasHeatmapData = computed(() => portData.value.length > 0);
const hasSparklineData = computed(() => monitorSparklines.value.length > 0);

// ─── Init ──────────────────────────────────────────────────

onMounted(() => {
  loadNetworkInfo();
  loadRecentTargets();
  loadBookmarks();

  try {
    const raw = localStorage.getItem(PING_KEY);
    if (raw) pingData.value = JSON.parse(raw);
  } catch { /* ignore */ }

  try {
    const raw = localStorage.getItem(PORT_KEY);
    if (raw) portData.value = JSON.parse(raw);
  } catch { /* ignore */ }

  loadMonitorSparkline();
});
</script>

<template>
  <div class="flex h-full flex-col p-4 md:p-6 lg:p-8 animate-view-fade overflow-y-auto space-y-5">
    <!-- Header + Network Info -->
    <div class="flex items-start justify-between">
      <div>
        <h1 class="text-2xl font-display font-bold text-ink">仪表盘</h1>
        <p class="mt-1 text-sm text-ink-faint">快速访问与状态概览</p>
      </div>
      <div
        v-if="networkInfo"
        class="hidden sm:flex items-center gap-3 rounded-xl border border-paper-deep/40 bg-paper-warm/30 px-4 py-2.5 text-xs"
      >
        <Wifi class="h-4 w-4 text-bamboo" />
        <div class="text-ink-faint">
          <span class="text-ink-soft font-medium">{{ networkInfo.hostname }}</span>
          <span v-for="ip in networkInfo.ipv4" :key="ip" class="ml-2 font-mono text-ink-soft">
            {{ ip }}
          </span>
        </div>
      </div>
    </div>

    <!-- Quick Action Bar -->
    <div class="rounded-xl border border-paper-deep/50 bg-paper p-4 shadow-sm">
      <div class="flex flex-wrap items-center gap-3">
        <div class="relative flex-1 min-w-[200px]">
          <Search class="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-ink-faint" />
          <input
            v-model="quickTarget"
            type="text"
            placeholder="输入目标 IP 或域名..."
            class="w-full rounded-lg border border-paper-deep/40 bg-paper-warm/50 py-2 pl-9 pr-3 text-sm outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/50 focus:bg-paper-warm"
            @keydown.enter="doQuickPing"
          />
        </div>
        <Button variant="default" size="sm" :disabled="quickRunning || !quickTarget.trim()" @click="doQuickPing">
          <Radio class="mr-1.5 h-3.5 w-3.5" /> Ping
        </Button>
        <Button variant="secondary" size="sm" :disabled="!quickTarget.trim()" @click="openTool('/port-scan')">
          <Scan class="mr-1.5 h-3.5 w-3.5" /> 端口扫描
        </Button>
        <Button variant="secondary" size="sm" :disabled="!quickTarget.trim()" @click="openTool('/traceroute')">
          <Route class="mr-1.5 h-3.5 w-3.5" /> Traceroute
        </Button>
      </div>

      <!-- Quick result -->
      <div
        v-if="quickResult"
        class="mt-3 rounded-lg border border-paper-deep/30 bg-paper-warm/30 px-4 py-2.5"
        :class="quickResult?.includes('超时') || quickResult?.includes('无响应') ? 'border-red-200 dark:border-red-900/30' : ''"
      >
        <pre class="text-xs font-mono text-ink-soft whitespace-pre-wrap">{{ quickResult }}</pre>
      </div>

      <!-- Recent targets -->
      <div v-if="recentTargets.length > 0" class="mt-3 flex flex-wrap items-center gap-1.5">
        <Clock class="h-3.5 w-3.5 text-ink-faint/50 shrink-0" />
        <span
          v-for="target in recentTargets"
          :key="target"
          class="cursor-pointer rounded-md border border-paper-deep/20 bg-paper-warm/40 px-2.5 py-1 text-xs font-mono text-ink-faint transition-colors hover:border-bamboo/30 hover:text-bamboo"
          @click="quickTarget = target"
        >
          {{ target }}
        </span>
      </div>
    </div>

    <!-- Tool Grid -->
    <div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-3">
      <RouterLink
        v-for="tool in tools"
        :key="tool.path"
        :to="tool.path"
        class="group flex flex-col items-center gap-2 rounded-xl border border-paper-deep/40 bg-paper p-4 text-center shadow-sm transition-all hover:border-bamboo/25 hover:shadow-md hover:-translate-y-0.5"
      >
        <div
          class="flex h-10 w-10 items-center justify-center rounded-lg transition-colors"
          :class="tool.color"
        >
          <component :is="tool.icon" class="h-5 w-5" />
        </div>
        <span class="text-sm font-semibold text-ink">{{ tool.label }}</span>
        <span class="text-[11px] leading-tight text-ink-faint/70">{{ tool.description }}</span>
      </RouterLink>
    </div>

    <!-- Bookmarks row -->
    <div v-if="bookmarks.length > 0" class="flex flex-wrap items-center gap-2 rounded-xl border border-paper-deep/30 bg-paper-warm/20 px-4 py-3">
      <Star class="h-4 w-4 text-amber-400 shrink-0" />
      <span class="text-xs text-ink-faint mr-1">书签:</span>
      <RouterLink
        v-for="bm in bookmarks.slice(0, 8)"
        :key="bm.id"
        :to="`/ping?target=${encodeURIComponent(bm.target)}`"
        class="inline-flex items-center gap-1 rounded-md bg-paper-deep/20 px-2.5 py-1 text-xs text-ink-soft transition-colors hover:bg-bamboo/10 hover:text-bamboo"
      >
        <ExternalLink class="h-3 w-3" />
        {{ bm.label }}
      </RouterLink>
    </div>

    <!-- Charts section -->
    <template v-if="hasChartData || hasHeatmapData || hasSparklineData">
      <div class="border-t border-paper-deep/20 pt-4">
        <h2 class="text-base font-semibold text-ink flex items-center gap-2 mb-4">
          <Activity class="h-4 w-4 text-bamboo" />
          活动数据
        </h2>
      </div>

      <!-- Ping Chart -->
      <div v-if="hasChartData" class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm">
        <div class="flex items-center gap-2 mb-4">
          <BarChart3 class="h-5 w-5 text-bamboo" />
          <h3 class="text-base font-display font-semibold text-ink">Ping 延迟历史</h3>
          <span class="ml-auto text-xs text-ink-faint">最近 50 个采样点/目标</span>
        </div>
        <div class="flex flex-wrap gap-4 mb-3 text-xs">
          <div v-for="(target, i) in uniqueTargets" :key="'leg'+target" class="flex items-center gap-1.5">
            <span class="inline-block h-2.5 w-2.5 rounded-full" :style="{ backgroundColor: TARGET_COLORS[i % TARGET_COLORS.length] }" />
            <span class="text-ink-soft">{{ target }}</span>
          </div>
        </div>
        <div class="w-full overflow-x-auto">
          <svg :viewBox="`0 0 ${chartWidth} ${chartHeight}`" class="w-full min-h-[200px]" preserveAspectRatio="xMidYMid meet" style="max-width: 100%; height: auto;">
            <line v-for="i in 4" :key="'g'+i" :x1="padLeft" :y1="padTop + (i * chartH) / 4" :x2="padLeft + chartW" :y2="padTop + (i * chartH) / 4" stroke="currentColor" stroke-opacity="0.1" stroke-dasharray="4,4" />
            <text v-for="(label, i) in yAxisLabels" :key="'yl'+i" :x="padLeft - 5" :y="label.y + 4" text-anchor="end" class="fill-ink-faint" font-size="10">{{ label.label }}</text>
            <text :x="10" :y="padTop + chartH / 2" text-anchor="middle" class="fill-ink-faint" font-size="10" transform="rotate(-90, 10, 110)">ms</text>
            <path v-for="(line, i) in chartLines" :key="'ln'+i" :d="line.path" fill="none" :stroke="line.color" stroke-width="2" stroke-linejoin="round" stroke-linecap="round" />
            <circle v-for="(line, i) in chartLinesWithPoints" :key="'lp'+i" :cx="line.points[line.points.length - 1].x" :cy="line.points[line.points.length - 1].y" :r="3" :fill="line.color" stroke="white" stroke-width="1.5" />
          </svg>
        </div>
      </div>

      <!-- Port Scan Heatmap -->
      <div v-if="hasHeatmapData" class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm">
        <div class="flex items-center gap-2 mb-4">
          <Grid3X3 class="h-5 w-5 text-bamboo" />
          <h3 class="text-base font-display font-semibold text-ink">端口扫描热力图</h3>
          <span class="ml-auto text-xs text-ink-faint">绿色=开放</span>
        </div>
        <div class="overflow-x-auto">
          <table class="w-full text-xs">
            <thead>
              <tr>
                <th class="px-2 py-1 text-left text-ink-faint font-medium">IP / 端口</th>
                <th v-for="port in heatmapPorts" :key="'h'+port" class="px-2 py-1 text-center text-ink-faint font-medium">{{ port }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="target in heatmapTargets" :key="'ht'+target">
                <td class="px-2 py-1.5 text-ink-soft font-mono text-xs whitespace-nowrap">{{ target }}</td>
                <td v-for="port in heatmapPorts" :key="'hc'+target+port" class="px-1 py-1.5 text-center">
                  <span class="inline-block h-5 w-5 rounded" :class="{ 'bg-green-500': portState(target, port) === 'open', 'bg-gray-200 dark:bg-gray-700': portState(target, port) === 'unknown', 'bg-red-400': portState(target, port) === 'filtered' }" :title="`${target}:${port} - ${portState(target, port)}`" />
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- Sparklines -->
      <div v-if="hasSparklineData" class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm">
        <div class="flex items-center gap-2 mb-4">
          <TrendingUp class="h-5 w-5 text-bamboo" />
          <h3 class="text-base font-display font-semibold text-ink">最近 24 小时延迟趋势</h3>
          <RouterLink to="/monitor" class="ml-auto text-xs text-bamboo hover:underline">查看详情</RouterLink>
        </div>
        <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          <div v-for="(spark, i) in monitorSparklines" :key="'sp'+i" class="rounded-lg bg-paper-deep/15 p-3">
            <p class="text-xs text-ink-faint mb-1">目标 {{ i + 1 }}</p>
            <p class="text-sm font-mono font-semibold text-ink">{{ spark.targetHost }}</p>
            <svg :viewBox="'0 0 300 60'" class="w-full h-12 mt-2">
              <path :d="spark.path" fill="none" :stroke="spark.color" stroke-width="2" stroke-linejoin="round" stroke-linecap="round" />
            </svg>
          </div>
        </div>
      </div>
    </template>

    <!-- Empty state when no data -->
    <div v-if="!hasChartData && !hasHeatmapData && !hasSparklineData && !monitorLoading" class="flex items-center justify-center rounded-xl border border-dashed border-paper-deep/30 bg-paper-warm/20 py-12 text-sm text-ink-faint">
      <div class="text-center max-w-sm">
        <Activity class="mx-auto h-10 w-10 mb-3 opacity-30" />
        <p class="font-medium text-ink-soft">暂无统计数据</p>
        <p class="mt-2 text-xs opacity-60 leading-relaxed">使用 Ping 或端口扫描后将在此显示图表分析</p>
      </div>
    </div>
  </div>
</template>
