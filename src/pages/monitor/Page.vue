<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted, computed } from "vue";
import { Play, Square, Plus, Trash2, Activity, Clock, Radio, BarChart3 } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import { useToastStore } from "@/stores/toast";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

const toast = useToastStore();

// ─── Types (mirrors Rust) ──────────────────────────────────────────

interface MonitorTarget {
  id: string;
  host: string;
  label: string;
  intervalSecs: number;
  enabled: boolean;
}

interface PingRecord {
  id: number;
  targetId: string;
  targetHost: string;
  timestamp: string;
  latencyMs: number | null;
  lossRate: number;
}

interface MonitorUpdate {
  targetId: string;
  targetHost: string;
  label: string;
  timestamp: string;
  latencyMs: number | null;
  lossRate: number;
  minMs: number;
  avgMs: number;
  maxMs: number;
  sent: number;
  received: number;
}

// ─── State ─────────────────────────────────────────────────────────

const targets = ref<MonitorTarget[]>([]);
const running = ref(false);
const selectedTargetId = ref<string>("");
const historyData = ref<PingRecord[]>([]);
const liveUpdates = ref<Record<string, MonitorUpdate>>({});
const timeRange = ref(1); // hours

// New target form
const newHost = ref("");
const newLabel = ref("");
const newInterval = ref(300);

let unlistenUpdate: UnlistenFn | null = null;
let unlistenStatus: UnlistenFn | null = null;

// ─── Computed ──────────────────────────────────────────────────────

const chartData = computed(() => {
  const records = historyData.value;
  if (records.length === 0) return [];

  const rangeMs = timeRange.value * 60 * 60 * 1000;
  const cutoff = Date.now() - rangeMs;
  const filtered = records.filter((r) => new Date(r.timestamp).getTime() >= cutoff);

  return filtered.map((r) => ({
    timestamp: r.timestamp,
    latencyMs: r.latencyMs,
    lossRate: r.lossRate,
  }));
});

const chartSvgPath = computed(() => {
  const data = chartData.value;
  if (data.length < 2) return "";

  // Filter points with valid latency
  const validPoints = data.filter((d) => d.latencyMs !== null && d.latencyMs !== undefined) as {
    timestamp: string;
    latencyMs: number;
    lossRate: number;
  }[];

  if (validPoints.length < 2) return "";

  const width = 700;
  const height = 200;
  const padTop = 20;
  const padBottom = 30;
  const padLeft = 60;
  const padRight = 20;
  const chartW = width - padLeft - padRight;
  const chartH = height - padTop - padBottom;

  const latencies = validPoints.map((p) => p.latencyMs);
  const minLat = Math.min(...latencies);
  const maxLat = Math.max(...latencies);
  const range = Math.max(maxLat - minLat, 1);

  const times = validPoints.map((p) => new Date(p.timestamp).getTime());
  const minTime = Math.min(...times);
  const maxTime = Math.max(...times);
  const timeRange = Math.max(maxTime - minTime, 1);

  const points = validPoints.map((p) => {
    const x = padLeft + ((new Date(p.timestamp).getTime() - minTime) / timeRange) * chartW;
    const y = padTop + chartH - ((p.latencyMs - minLat) / range) * chartH;
    return { x, y, lossRate: p.lossRate };
  });

  // Build line path
  let path = `M ${points[0].x} ${points[0].y}`;
  for (let i = 1; i < points.length; i++) {
    path += ` L ${points[i].x} ${points[i].y}`;
  }

  return path;
});

const selectedTarget = computed(() =>
  targets.value.find((t) => t.id === selectedTargetId.value),
);

const chartSvgWidth = computed(() => 700);
const chartSvgHeight = computed(() => 200);

const chartPoints = computed(() => {
  const data = chartData.value.filter((d) => d.latencyMs !== null && d.latencyMs !== undefined) as {
    timestamp: string;
    latencyMs: number;
    lossRate: number;
  }[];
  if (data.length < 2) return [];
  const width = 700, height = 200, padTop = 20, padBottom = 30, padLeft = 60, padRight = 20;
  const chartW = width - padLeft - padRight, chartH = height - padTop - padBottom;
  const latencies = data.map((p) => p.latencyMs);
  const minLat = Math.min(...latencies), maxLat = Math.max(...latencies), range = Math.max(maxLat - minLat, 1);
  const times = data.map((p) => new Date(p.timestamp).getTime());
  const minTime = Math.min(...times), maxTime = Math.max(...times), tRange = Math.max(maxTime - minTime, 1);
  return data.map((p) => ({
    x: padLeft + ((new Date(p.timestamp).getTime() - minTime) / tRange) * chartW,
    y: padTop + chartH - ((p.latencyMs - minLat) / range) * chartH,
    lossRate: p.lossRate,
  }));
});

// ─── Methods ───────────────────────────────────────────────────────

async function loadTargets() {
  try {
    targets.value = await invoke<MonitorTarget[]>("monitor_list_targets");
    if (targets.value.length > 0 && !selectedTargetId.value) {
      selectedTargetId.value = targets.value[0].id;
    }
  } catch (e) {
    toast.error(`加载监控目标失败: ${e}`);
  }
}

async function loadHistory() {
  if (!selectedTargetId.value) return;
  try {
    historyData.value = await invoke<PingRecord[]>("monitor_get_history", {
      targetId: selectedTargetId.value,
      sinceDays: 7,
    });
  } catch (e) {
    toast.error(`加载历史数据失败: ${e}`);
  }
}

async function addTarget() {
  if (!newHost.value.trim() || !newLabel.value.trim()) {
    toast.error("请填写主机地址和标签");
    return;
  }
  try {
    await invoke<MonitorTarget>("monitor_add_target", {
      host: newHost.value.trim(),
      label: newLabel.value.trim(),
      intervalSecs: newInterval.value,
    });
    newHost.value = "";
    newLabel.value = "";
    newInterval.value = 300;
    await loadTargets();
    toast.add("success", "目标已添加");
  } catch (e) {
    toast.error(`添加目标失败: ${e}`);
  }
}

async function deleteTarget(id: string) {
  try {
    await invoke("monitor_delete_target", { id });
    if (selectedTargetId.value === id) {
      selectedTargetId.value = "";
    }
    await loadTargets();
    toast.add("success", "目标已删除");
  } catch (e) {
    toast.error(`删除目标失败: ${e}`);
  }
}

async function startMonitor() {
  try {
    await invoke("monitor_start");
    running.value = true;
    toast.add("success", "监控已启动");
  } catch (e) {
    toast.error(`启动监控失败: ${e}`);
  }
}

async function stopMonitor() {
  try {
    await invoke("monitor_stop");
    running.value = false;
    toast.add("success", "监控已停止");
  } catch (e) {
    toast.error(`停止监控失败: ${e}`);
  }
}

// ─── Lifecycle ─────────────────────────────────────────────────────

onMounted(async () => {
  await loadTargets();
  if (selectedTargetId.value) {
    await loadHistory();
  }

  // Check current status
  try {
    running.value = await invoke<boolean>("monitor_status");
  } catch {
    // ignore
  }

  // Listen for updates
  unlistenUpdate = await listen<MonitorUpdate>("monitor:update", (event) => {
    const update = event.payload;
    liveUpdates.value[update.targetId] = update;
    // Auto-refresh chart for selected target (deduplicate by timestamp)
    if (update.targetId === selectedTargetId.value) {
      const exists = historyData.value.some((r) => r.timestamp === update.timestamp && r.targetId === update.targetId);
      if (!exists) {
        historyData.value.push({
          id: 0,
          targetId: update.targetId,
          targetHost: update.targetHost,
          timestamp: update.timestamp,
          latencyMs: update.latencyMs,
          lossRate: update.lossRate,
        });
        // Keep only last 1000 points visible
        if (historyData.value.length > 1000) {
          historyData.value = historyData.value.slice(-1000);
        }
      }
    }
  });

  unlistenStatus = await listen<{ running: boolean }>("monitor:status", (event) => {
    running.value = event.payload.running;
  });
});

onUnmounted(() => {
  unlistenUpdate?.();
  unlistenStatus?.();
});

// Watch target change to reload history
watch(selectedTargetId, async (id) => {
  historyData.value = [];
  if (id) {
    await loadHistory();
  }
});
</script>

<template>
  <div class="flex h-full flex-col p-4 md:p-6 space-y-4 md:space-y-6 animate-view-fade">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-display font-bold text-ink">网络性能监控</h1>
        <p class="mt-0.5 text-sm text-ink-faint">定时 Ping 目标主机，记录延迟趋势（Smokeping 风格）</p>
      </div>
      <Button
        :variant="running ? 'danger' : 'default'"
        @click="running ? stopMonitor() : startMonitor()"
      >
        <Square v-if="running" class="mr-1.5 h-3.5 w-3.5" />
        <Play v-else class="mr-1.5 h-3.5 w-3.5" />
        {{ running ? '停止监控' : '开始监控' }}
      </Button>
    </div>

    <!-- Status indicator -->
    <div
      class="flex items-center gap-3 rounded-xl border px-4 py-3 text-sm"
      :class="running ? 'border-bamboo/30 bg-bamboo/5 text-bamboo' : 'border-paper-deep/30 bg-paper-warm/30 text-ink-faint'"
    >
      <Activity class="h-4 w-4" />
      <span class="font-medium">{{ running ? '监控运行中' : '监控未启动' }}</span>
      <span v-if="running" class="text-xs opacity-70">
        · {{ targets.length }} 个目标
      </span>
    </div>

    <!-- Add target form -->
    <div class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm">
      <h2 class="text-sm font-semibold text-ink mb-3">添加监控目标</h2>
      <div class="flex flex-wrap items-end gap-3">
        <div class="flex-1 min-w-[160px]">
          <label class="mb-1 block text-xs font-medium text-ink-soft">主机地址</label>
          <input
            v-model="newHost"
            placeholder="IP 或域名"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink placeholder:text-ink-faint/50 outline-none focus:border-bamboo/50"
          />
        </div>
        <div class="flex-1 min-w-[120px]">
          <label class="mb-1 block text-xs font-medium text-ink-soft">标签</label>
          <input
            v-model="newLabel"
            placeholder="例如: 公司网关"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink placeholder:text-ink-faint/50 outline-none focus:border-bamboo/50"
          />
        </div>
        <div class="w-28">
          <label class="mb-1 block text-xs font-medium text-ink-soft">间隔(秒)</label>
          <input
            v-model.number="newInterval"
            type="number"
            min="30"
            max="86400"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none focus:border-bamboo/50"
          />
        </div>
        <Button @click="addTarget">
          <Plus class="mr-1.5 h-3.5 w-3.5" />
          添加
        </Button>
      </div>
    </div>

    <!-- Targets list + Chart area -->
    <div class="flex flex-1 gap-4 min-h-0">
      <!-- Targets sidebar -->
      <div class="w-56 shrink-0 noise-bg rounded-xl border border-paper-deep/60 bg-paper shadow-sm overflow-y-auto">
        <div class="px-4 py-3 border-b border-paper-deep/50">
          <h2 class="text-sm font-semibold text-ink">监控目标</h2>
        </div>
        <div v-if="targets.length === 0" class="px-4 py-8 text-center text-sm text-ink-faint">
          <Radio class="mx-auto h-6 w-6 mb-2 opacity-40" />
          <p>暂无目标</p>
        </div>
        <div v-for="t in targets" :key="t.id" class="border-b border-paper-deep/20 last:border-0">
          <button
            class="flex w-full items-center gap-3 px-4 py-3 text-left transition-colors hover:bg-paper-deep/20"
            :class="selectedTargetId === t.id ? 'bg-bamboo/10 text-bamboo' : 'text-ink-soft'"
            @click="selectedTargetId = t.id"
          >
            <div class="flex-1 min-w-0">
              <div class="text-sm font-medium truncate">{{ t.label }}</div>
              <div class="text-xs text-ink-faint truncate">{{ t.host }}</div>
              <div class="text-xs text-ink-faint/60">{{ t.intervalSecs }}s 间隔</div>
            </div>
            <button
              class="shrink-0 rounded-lg p-1.5 text-ink-faint transition-colors hover:text-red-500 hover:bg-red-500/10"
              title="删除"
              @click.stop="deleteTarget(t.id)"
            >
              <Trash2 class="h-3.5 w-3.5" />
            </button>
          </button>
          <!-- Latest update indicator -->
          <div v-if="liveUpdates[t.id]" class="px-4 pb-2 text-xs">
            <span
              class="inline-block rounded-full px-2 py-0.5"
              :class="(liveUpdates[t.id]?.lossRate ?? 0) > 0 ? 'bg-red-100 text-red-600' : 'bg-bamboo/10 text-bamboo'"
            >
              {{ liveUpdates[t.id]?.latencyMs != null ? liveUpdates[t.id]!.latencyMs!.toFixed(1) + ' ms' : '超时' }}
            </span>
          </div>
        </div>
      </div>

      <!-- Chart area -->
      <div class="flex-1 noise-bg rounded-xl border border-paper-deep/60 bg-paper shadow-sm p-5 flex flex-col min-h-0">
        <div v-if="!selectedTargetId" class="flex items-center justify-center h-full text-sm text-ink-faint">
          <div class="text-center">
            <BarChart3 class="mx-auto h-8 w-8 mb-2 opacity-40" />
            <p>请选择一个监控目标</p>
          </div>
        </div>

        <template v-else>
          <!-- Target info + time range -->
          <div class="flex items-center justify-between mb-4">
            <div>
              <h2 class="text-base font-semibold text-ink">{{ selectedTarget?.label }}</h2>
              <p class="text-xs text-ink-faint">{{ selectedTarget?.host }} · {{ historyData.length }} 条记录</p>
            </div>
            <div class="flex gap-1">
              <button
                v-for="h in [1, 6, 24, 168]"
                :key="h"
                class="rounded-lg px-3 py-1 text-xs font-medium transition-colors"
                :class="timeRange === h ? 'bg-bamboo/15 text-bamboo ring-1 ring-bamboo/30' : 'text-ink-soft hover:bg-paper-deep/30'"
                @click="timeRange = h"
              >
                {{ h === 168 ? '7天' : h + '小时' }}
              </button>
            </div>
          </div>

          <!-- SVG Chart -->
          <div class="flex-1 min-h-0 flex flex-col">
            <div v-if="chartData.length < 2" class="flex items-center justify-center flex-1 text-sm text-ink-faint">
              <div class="text-center">
                <Clock class="mx-auto h-6 w-6 mb-2 opacity-40" />
                <p>数据不足，等待更多样本</p>
              </div>
            </div>
            <svg
              v-else
              :viewBox="`0 0 ${chartSvgWidth} ${chartSvgHeight}`"
              class="w-full h-full max-h-[300px]"
              preserveAspectRatio="xMidYMid meet"
            >
              <!-- Grid lines (5 positions matching chart: padTop=20, chartH=150) -->
              <line
                v-for="i in 5"
                :key="'g'+i"
                :x1="60"
                :y1="20 + (i-1) * 37.5"
                :x2="680"
                :y2="20 + (i-1) * 37.5"
                stroke="currentColor"
                stroke-opacity="0.1"
                stroke-dasharray="4,4"
              />
              <!-- Y-axis labels -->
              <text
                v-for="i in 5"
                :key="'yl'+i"
                :x="55"
                :y="24 + (i-1) * 37.5"
                text-anchor="end"
                class="text-[10px] fill-ink-faint"
              >
                {{ (() => {
                  const data = chartData.filter(d => d.latencyMs !== null).map(d => d.latencyMs as number);
                  if (data.length === 0) return '';
                  const min = Math.min(...data);
                  const max = Math.max(...data);
                  const r = Math.max(max - min, 1);
                  return (max - (i-1) * r / 4).toFixed(0);
                })() }}
              </text>
              <!-- Line path -->
              <path
                :d="chartSvgPath"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                class="text-bamboo"
              />
              <!-- Data points -->
              <circle
                v-for="(pt, i) in chartPoints"
                :key="'pt'+i"
                :cx="pt.x"
                :cy="pt.y"
                :r="pt.lossRate > 0 ? 4 : 3"
                :fill="pt.lossRate > 0 ? '#ef4444' : 'currentColor'"
                class="text-bamboo"
                stroke="white"
                stroke-width="1"
              />
            </svg>
          </div>

          <!-- Stats summary -->
          <div v-if="chartData.length > 0" class="mt-4 grid grid-cols-4 gap-3 text-center">
            <div class="rounded-lg bg-paper-deep/20 p-2">
              <p class="text-xs text-ink-faint">样本数</p>
              <p class="text-lg font-mono font-semibold text-ink">{{ chartData.length }}</p>
            </div>
            <div class="rounded-lg bg-paper-deep/20 p-2">
              <p class="text-xs text-ink-faint">平均延迟</p>
              <p class="text-lg font-mono font-semibold text-ink">
                {{ (() => {
                  const vals = chartData.filter(d => d.latencyMs !== null).map(d => d.latencyMs as number);
                  return vals.length ? (vals.reduce((a, b) => a + b, 0) / vals.length).toFixed(1) : '---';
                })() }} ms
              </p>
            </div>
            <div class="rounded-lg bg-paper-deep/20 p-2">
              <p class="text-xs text-ink-faint">最小 / 最大</p>
              <p class="text-lg font-mono font-semibold text-ink">
                {{ (() => {
                  const vals = chartData.filter(d => d.latencyMs !== null).map(d => d.latencyMs as number);
                  return vals.length ? `${Math.min(...vals).toFixed(0)} / ${Math.max(...vals).toFixed(0)}` : '---';
                })() }} ms
              </p>
            </div>
            <div class="rounded-lg bg-paper-deep/20 p-2">
              <p class="text-xs text-ink-faint">丢包率</p>
              <p class="text-lg font-mono font-semibold" :class="chartData.some(d => d.lossRate > 0) ? 'text-red-500' : 'text-bamboo'">
                {{ (() => {
                  const losses = chartData.filter(d => d.lossRate > 0).length;
                  return chartData.length ? (losses / chartData.length * 100).toFixed(1) : '0.0';
                })() }}%
              </p>
            </div>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>
