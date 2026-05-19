<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { RouterLink } from "vue-router";
import { Radio, Route, Scan, Globe, ArrowRight, Activity, BarChart3, Grid3X3, TrendingUp } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import { invoke } from "@tauri-apps/api/core";

interface ToolCard {
  label: string;
  path: string;
  icon: object;
  description: string;
}

const tools: ToolCard[] = [
  {
    label: "Ping",
    path: "/ping",
    icon: Radio,
    description: "测试网络连通性，测量目标主机的响应延迟与丢包率",
  },
  {
    label: "Traceroute",
    path: "/traceroute",
    icon: Route,
    description: "追踪数据包到达目标所经过的路由节点与每一跳的延迟",
  },
  {
    label: "端口扫描",
    path: "/port-scan",
    icon: Scan,
    description: "扫描目标主机的开放端口，识别正在运行的服务",
  },
  {
    label: "DNS 查询",
    path: "/dns",
    icon: Globe,
    description: "查询域名解析记录，支持 A / AAAA / CNAME / MX 等多种记录类型",
  },
];

// ─── Ping History Chart (localStorage) ──────────────────────────

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
    if (!groups[entry.target]) {
      groups[entry.target] = [];
    }
    groups[entry.target].push({ latency: entry.latency, timestamp: entry.timestamp });
  }
  // Limit to last 50 per target and sort by time
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

interface ChartPoint {
  x: number;
  y: number;
  target: string;
  latency: number;
}

const chartLines = computed(() => {
  const groups = groupedPingData.value;
  const targets = Object.keys(groups);
  if (targets.length === 0) return [];

  const allPoints: { target: string; latency: number; time: number }[] = [];
  for (const t of targets) {
    for (const p of groups[t]) {
      allPoints.push({ target: t, latency: p.latency, time: new Date(p.timestamp).getTime() });
    }
  }

  if (allPoints.length === 0) return [];

  const minTime = Math.min(...allPoints.map((p) => p.time));
  const maxTime = Math.max(...allPoints.map(p => p.time));
  const timeRange = Math.max(maxTime - minTime, 1);
  const latencies = allPoints.map((p) => p.latency);
  const minLat = Math.min(...latencies);
  const maxLat = Math.max(...latencies);
  const latRange = Math.max(maxLat - minLat, 1);

  const result: { target: string; color: string; path: string; points: ChartPoint[] }[] = [];

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
    for (let j = 1; j < pts.length; j++) {
      path += ` L ${pts[j].x} ${pts[j].y}`;
    }

    result.push({
      target: t,
      color: TARGET_COLORS[i % TARGET_COLORS.length],
      path,
      points: pts,
    });
  }

  return result;
});

const yAxisLabels = computed(() => {
  const allPoints: { target: string; latency: number }[] = [];
  for (const t of Object.keys(groupedPingData.value)) {
    for (const p of groupedPingData.value[t]) {
      allPoints.push({ target: t, latency: p.latency });
    }
  }
  if (allPoints.length === 0) return [];
  const latencies = allPoints.map((p) => p.latency);
  const minLat = Math.min(...latencies);
  const maxLat = Math.max(...latencies);
  const range = Math.max(maxLat - minLat, 1);
  const labels = [];
  for (let i = 0; i <= 4; i++) {
    const val = maxLat - (i * range) / 4;
    labels.push({ y: padTop + (i * chartH) / 4, label: val.toFixed(0) });
  }
  return labels;
});

// ─── Port Scan Heatmap ──────────────────────────────────────────

const heatmapTargets = computed(() => {
  const targets = new Set<string>();
  for (const p of portData.value) {
    targets.add(p.target);
  }
  return Array.from(targets);
});

const heatmapPorts = computed(() => {
  const ports = new Set<number>();
  for (const p of portData.value) {
    ports.add(p.port);
  }
  return Array.from(ports).sort((a, b) => a - b).slice(0, 30);
});

function portState(target: string, port: number): string {
  const entry = portData.value.find((p) => p.target === target && p.port === port);
  return entry?.state || "unknown";
}

// ─── Monitor Sparkline Data (from SQLite backend) ──────────────

interface MonitorPingRecord {
  id: number;
  targetId: string;
  targetHost: string;
  timestamp: string;
  latencyMs: number | null;
  lossRate: number;
}

const monitorData = ref<MonitorPingRecord[]>([]);
const monitorLoading = ref(false);

async function loadMonitorSparkline() {
  monitorLoading.value = true;
  try {
    monitorData.value = await invoke<MonitorPingRecord[]>("monitor_get_all_recent_history", {
      sinceDays: 1,
    });
  } catch {
    // Backend not available yet
    monitorData.value = [];
  } finally {
    monitorLoading.value = false;
  }
}

const monitorSparklines = computed(() => {
  const groups: Record<string, MonitorPingRecord[]> = {};
  for (const r of monitorData.value) {
    if (!groups[r.targetId]) {
      groups[r.targetId] = [];
    }
    groups[r.targetId].push(r);
  }

  const result: { targetHost: string; path: string; color: string }[] = [];
  const colors = ["#22c55e", "#3b82f6", "#f59e0b"];

  let idx = 0;
  for (const [, records] of Object.entries(groups)) {
    const withLatency = records.filter((r): r is MonitorPingRecord & { latencyMs: number } => r.latencyMs !== null);
    if (withLatency.length < 2) {
      idx++;
      continue;
    }

    const w = 300;
    const h = 60;
    const pL = 5;
    const pR = 5;
    const pT = 5;
    const pB = 5;
    const cW = w - pL - pR;
    const cH = h - pT - pB;

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
    for (let i = 1; i < pts.length; i++) {
      path += ` L ${pts[i].x} ${pts[i].y}`;
    }

    const latestRecord = withLatency[withLatency.length - 1];
    const hostName = latestRecord.targetHost || "";
    const msLabel = `${latestRecord.latencyMs.toFixed(0)} ms`;
    result.push({
      targetHost: hostName ? `${hostName} — ${msLabel}` : msLabel,
      path,
      color: colors[idx % colors.length],
    });
    idx++;
  }

  return result;
});

const chartLinesWithPoints = computed(() =>
  chartLines.value.filter((l) => l.points.length > 0),
);

const hasChartData = computed(() => uniqueTargets.value.length > 0);
const hasHeatmapData = computed(() => portData.value.length > 0);
const hasSparklineData = computed(() => monitorSparklines.value.length > 0);
const hasAnyData = computed(() => hasChartData.value || hasHeatmapData.value || hasSparklineData.value);

// ─── Init ────────────────────────────────────────────────────────

onMounted(() => {
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
  <div class="flex h-full flex-col p-4 md:p-6 lg:p-8 animate-view-fade overflow-y-auto space-y-6">
    <!-- Header -->
    <div>
      <h1 class="text-2xl font-display font-bold text-ink">网络工具集</h1>
      <p class="mt-1 text-sm text-ink-faint">选择一项工具开始诊断与分析</p>
    </div>

    <!-- Tool cards grid -->
    <div class="grid grid-cols-1 gap-5 sm:grid-cols-2">
      <RouterLink
        v-for="tool in tools"
        :key="tool.path"
        :to="tool.path"
        class="group noise-bg rounded-xl border border-paper-deep/60 bg-paper p-6 shadow-sm transition-all hover:border-bamboo/25 hover:shadow-md hover:-translate-y-0.5"
      >
        <div class="flex items-start justify-between">
          <div
            class="flex h-10 w-10 items-center justify-center rounded-lg bg-bamboo/10 text-bamboo transition-colors group-hover:bg-bamboo/15"
          >
            <component :is="tool.icon" class="h-5 w-5" />
          </div>
          <ArrowRight
            class="h-4 w-4 text-ink-ghost transition-all group-hover:text-bamboo group-hover:translate-x-0.5"
          />
        </div>
        <h2 class="mt-4 text-base font-display font-semibold text-ink">
          {{ tool.label }}
        </h2>
        <p class="mt-1.5 text-sm leading-relaxed text-ink-faint">
          {{ tool.description }}
        </p>
        <div class="mt-4">
          <Button variant="ghost" size="sm" class="group-hover:text-bamboo">
            打开工具
          </Button>
        </div>
      </RouterLink>
    </div>

    <!-- Charts section -->
    <div v-if="!hasAnyData && !monitorLoading" class="mt-4 flex items-center justify-center rounded-xl border border-dashed border-paper-deep/30 bg-paper-warm/20 py-12 text-sm text-ink-faint">
      <div class="text-center max-w-sm">
        <Activity class="mx-auto h-10 w-10 mb-3 opacity-30" />
        <p class="font-medium text-ink-soft">暂无图表数据</p>
        <p class="mt-2 text-xs opacity-60 leading-relaxed">
          运行 Ping 或端口扫描后将在此显示图表
          <br />
          也可在性能监控页面配置持续监控
        </p>
      </div>
    </div>

    <template v-if="hasAnyData">
      <!-- Ping Latency History Chart -->
      <div
        v-if="hasChartData"
        class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm"
      >
        <div class="flex items-center gap-2 mb-4">
          <BarChart3 class="h-5 w-5 text-bamboo" />
          <h2 class="text-base font-display font-semibold text-ink">Ping 延迟历史</h2>
          <span class="ml-auto text-xs text-ink-faint">最近 50 个采样点/目标</span>
        </div>

        <!-- Legend -->
        <div class="flex flex-wrap gap-4 mb-3 text-xs">
          <div
            v-for="(target, i) in uniqueTargets"
            :key="'leg'+target"
            class="flex items-center gap-1.5"
          >
            <span
              class="inline-block h-2.5 w-2.5 rounded-full"
              :style="{ backgroundColor: TARGET_COLORS[i % TARGET_COLORS.length] }"
            />
            <span class="text-ink-soft">{{ target }}</span>
          </div>
        </div>

        <!-- SVG Chart -->
        <div class="w-full overflow-x-auto">
          <svg
            :viewBox="`0 0 ${chartWidth} ${chartHeight}`"
            class="w-full min-h-[200px]"
            preserveAspectRatio="xMidYMid meet"
            style="max-width: 100%; height: auto;"
          >
            <!-- Grid lines -->
            <line
              v-for="i in 4"
              :key="'g'+i"
              :x1="padLeft"
              :y1="padTop + (i * chartH) / 4"
              :x2="padLeft + chartW"
              :y2="padTop + (i * chartH) / 4"
              stroke="currentColor"
              stroke-opacity="0.1"
              stroke-dasharray="4,4"
            />
            <!-- Y-axis labels -->
            <text
              v-for="(label, i) in yAxisLabels"
              :key="'yl'+i"
              :x="padLeft - 5"
              :y="label.y + 4"
              text-anchor="end"
              class="fill-ink-faint"
              font-size="10"
            >
              {{ label.label }}
            </text>
            <!-- Y-axis label -->
            <text
              :x="10"
              :y="padTop + chartH / 2"
              text-anchor="middle"
              class="fill-ink-faint"
              font-size="10"
              transform="rotate(-90, 10, 110)"
            >
              ms
            </text>
            <!-- Data lines -->
            <path
              v-for="(line, i) in chartLines"
              :key="'ln'+i"
              :d="line.path"
              fill="none"
              :stroke="line.color"
              stroke-width="2"
              stroke-linejoin="round"
              stroke-linecap="round"
            />
            <!-- Data points (last point per line) -->
            <circle
              v-for="(line, i) in chartLinesWithPoints"
              :key="'lp'+i"
              :cx="line.points[line.points.length - 1].x"
              :cy="line.points[line.points.length - 1].y"
              :r="3"
              :fill="line.color"
              stroke="white"
              stroke-width="1.5"
            />
          </svg>
        </div>
      </div>

      <!-- Port Scan Heatmap -->
      <div
        v-if="hasHeatmapData"
        class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm"
      >
        <div class="flex items-center gap-2 mb-4">
          <Grid3X3 class="h-5 w-5 text-bamboo" />
          <h2 class="text-base font-display font-semibold text-ink">端口扫描热力图</h2>
          <span class="ml-auto text-xs text-ink-faint">绿色=开放</span>
        </div>

        <div class="overflow-x-auto">
          <table class="w-full text-xs">
            <thead>
              <tr>
                <th class="px-2 py-1 text-left text-ink-faint font-medium">IP / 端口</th>
                <th
                  v-for="port in heatmapPorts"
                  :key="'h'+port"
                  class="px-2 py-1 text-center text-ink-faint font-medium"
                >
                  {{ port }}
                </th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="target in heatmapTargets" :key="'ht'+target">
                <td class="px-2 py-1.5 text-ink-soft font-mono text-xs whitespace-nowrap">{{ target }}</td>
                <td
                  v-for="port in heatmapPorts"
                  :key="'hc'+target+port"
                  class="px-1 py-1.5 text-center"
                >
                  <span
                    class="inline-block h-5 w-5 rounded"
                    :class="{
                      'bg-green-500': portState(target, port) === 'open',
                      'bg-gray-200 dark:bg-gray-700': portState(target, port) === 'unknown',
                      'bg-red-400': portState(target, port) === 'filtered',
                    }"
                    :title="`${target}:${port} - ${portState(target, port)}`"
                  />
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- Monitor Sparkline Widget -->
      <div
        v-if="hasSparklineData"
        class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm"
      >
        <div class="flex items-center gap-2 mb-4">
          <TrendingUp class="h-5 w-5 text-bamboo" />
          <h2 class="text-base font-display font-semibold text-ink">最近 24 小时延迟趋势</h2>
          <RouterLink to="/monitor" class="ml-auto text-xs text-bamboo hover:underline">
            查看详情
          </RouterLink>
        </div>

        <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          <div
            v-for="(spark, i) in monitorSparklines"
            :key="'sp'+i"
            class="rounded-lg bg-paper-deep/15 p-3"
          >
            <p class="text-xs text-ink-faint mb-1">目标 {{ i + 1 }}</p>
            <p class="text-sm font-mono font-semibold text-ink">{{ spark.targetHost }}</p>
            <svg :viewBox="'0 0 300 60'" class="w-full h-12 mt-2">
              <path
                :d="spark.path"
                fill="none"
                :stroke="spark.color"
                stroke-width="2"
                stroke-linejoin="round"
                stroke-linecap="round"
              />
            </svg>
          </div>
        </div>

        <div v-if="!hasSparklineData && !monitorLoading" class="text-center py-4 text-xs text-ink-faint">
          暂无监控数据 — 前往<RouterLink to="/monitor" class="text-bamboo hover:underline"> 性能监控</RouterLink> 页面配置
        </div>
      </div>
    </template>
  </div>
</template>
