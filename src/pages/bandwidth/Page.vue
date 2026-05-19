<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch, nextTick } from "vue";
import { Play, Square, Activity } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

interface InterfaceInfo {
  name: string;
  friendlyName: string;
  ip: string;
}

interface BandwidthSample {
  interface: string;
  downloadBps: number;
  uploadBps: number;
  totalRx: number;
  totalTx: number;
  timestamp: string;
}

const interfaces = ref<InterfaceInfo[]>([]);
const selectedInterface = ref<string>("*");
const running = ref(false);
const error = ref("");

// Data points: keep 60 seconds of history
const maxDataPoints = 60;
const downloadHistory = ref<number[]>([]);
const uploadHistory = ref<number[]>([]);
const labels = ref<string[]>([]);

// Canvas ref
const canvasRef = ref<HTMLCanvasElement | null>(null);

let unlistenData: UnlistenFn | null = null;
let unlistenError: UnlistenFn | null = null;

// Format bytes per second to human-readable
function formatBps(bps: number): string {
  if (bps === 0) return "0 b/s";
  if (bps < 1024) return `${bps.toFixed(0)} b/s`;
  if (bps < 1024 * 1024) return `${(bps / 1024).toFixed(1)} Kb/s`;
  return `${(bps / (1024 * 1024)).toFixed(2)} Mb/s`;
}

const currentDownload = computed(() => {
  const last = downloadHistory.value[downloadHistory.value.length - 1];
  return last ?? 0;
});

const currentUpload = computed(() => {
  const last = uploadHistory.value[uploadHistory.value.length - 1];
  return last ?? 0;
});

async function loadInterfaces() {
  try {
    const result = await invoke<InterfaceInfo[]>("get_interfaces");
    interfaces.value = result;
  } catch (e) {
    error.value = String(e);
  }
}

async function startMonitor() {
  running.value = true;
  error.value = "";
  downloadHistory.value = [];
  uploadHistory.value = [];
  labels.value = [];

  try {
    // First load interfaces
    await loadInterfaces();
    // Then start the monitor
    await invoke("start_bandwidth_monitor");
  } catch (e) {
    error.value = String(e);
    running.value = false;
  }
}

async function stopMonitor() {
  try {
    await invoke("stop_bandwidth_monitor");
  } catch (e) {
    // Ignore errors on stop
  }
  running.value = false;
}

function handleData(samples: BandwidthSample[]) {
  // Clear any previous error when new data arrives
  if (samples.length > 0) error.value = "";

  const sample = samples.find(s => s.interface === selectedInterface.value)
    || (selectedInterface.value === "*" ? samples.find(s => s.interface === "*") : null);

  if (sample) {
    const now = new Date().toLocaleTimeString("zh-CN");
    downloadHistory.value.push(sample.downloadBps);
    uploadHistory.value.push(sample.uploadBps);
    labels.value.push(now);

    // Keep only last N data points
    if (downloadHistory.value.length > maxDataPoints) {
      downloadHistory.value.shift();
      uploadHistory.value.shift();
      labels.value.shift();
    }

    nextTick(() => drawChart());
  }
}

// Chart drawing
function drawChart() {
  const canvas = canvasRef.value;
  if (!canvas) return;

  const ctx = canvas.getContext("2d");
  if (!ctx) return;

  const dpr = window.devicePixelRatio || 1;
  const rect = canvas.getBoundingClientRect();
  canvas.width = rect.width * dpr;
  canvas.height = rect.height * dpr;
  ctx.scale(dpr, dpr);

  const w = rect.width;
  const h = rect.height;
  const padding = { top: 20, right: 20, bottom: 30, left: 60 };
  const plotW = w - padding.left - padding.right;
  const plotH = h - padding.top - padding.bottom;

  // Clear
  ctx.clearRect(0, 0, w, h);

  // Background
  ctx.fillStyle = getComputedStyle(canvas).getPropertyValue("--bg-chart").trim() || "transparent";
  ctx.fillRect(0, 0, w, h);

  if (downloadHistory.value.length < 2) {
    // Draw "Waiting for data..." text
    ctx.fillStyle = "#9ca3af";
    ctx.font = "14px sans-serif";
    ctx.textAlign = "center";
    ctx.fillText("等待数据...", w / 2, h / 2);
    return;
  }

  // Find max value for scaling
  const allValues = [...downloadHistory.value, ...uploadHistory.value];
  const maxVal = Math.max(...allValues, 1); // Avoid division by zero
  const yMax = Math.ceil(maxVal * 1.1);

  // Y-axis labels
  ctx.fillStyle = "#9ca3af";
  ctx.font = "10px sans-serif";
  ctx.textAlign = "right";
  const ySteps = 4;
  for (let i = 0; i <= ySteps; i++) {
    const val = (yMax / ySteps) * i;
    const y = padding.top + plotH - (i / ySteps) * plotH;
    ctx.fillText(formatBps(val), padding.left - 8, y + 3);

    // Grid line
    ctx.strokeStyle = "rgba(156, 163, 175, 0.15)";
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(padding.left, y);
    ctx.lineTo(padding.left + plotW, y);
    ctx.stroke();
  }

  const count = downloadHistory.value.length;

  // Helper: draw a filled path
  function drawLinePath(c: CanvasRenderingContext2D, data: number[], color: string) {
    c.beginPath();
    data.forEach((val, i) => {
      const x = padding.left + (count > 1 ? (i / (count - 1)) * plotW : plotW / 2);
      const y = padding.top + plotH - (val / yMax) * plotH;
      if (i === 0) c.moveTo(x, y);
      else c.lineTo(x, y);
    });

    // Line
    c.strokeStyle = color;
    c.lineWidth = 2;
    c.stroke();

    // Fill area under the line
    const lastX = padding.left + plotW;
    const bottomY = padding.top + plotH;
    c.lineTo(lastX, bottomY);
    c.lineTo(padding.left, bottomY);
    c.closePath();

    const grad = c.createLinearGradient(0, padding.top, 0, padding.top + plotH);
    grad.addColorStop(0, color + "40");
    grad.addColorStop(1, color + "05");
    c.fillStyle = grad;
    c.fill();
  }

  // Draw download (green)
  drawLinePath(ctx, downloadHistory.value, "#10b981");
  // Draw upload (blue)
  drawLinePath(ctx, uploadHistory.value, "#3b82f6");

  // X-axis labels (show every ~10 data points)
  ctx.fillStyle = "#9ca3af";
  ctx.textAlign = "center";
  const labelStep = Math.max(1, Math.floor(count / 6));
  for (let i = 0; i < count; i += labelStep) {
    const x = padding.left + (count > 1 ? (i / (count - 1)) * plotW : plotW / 2);
    ctx.fillText(labels.value[i] || "", x, h - 5);
  }

  // Legend
  const legendX = padding.left + 8;
  const legendY = padding.top + 8;
  ctx.font = "11px sans-serif";
  ctx.textAlign = "left";

  // Download legend
  ctx.fillStyle = "#10b981";
  ctx.fillRect(legendX, legendY, 12, 3);
  ctx.fillStyle = "#9ca3af";
  ctx.fillText("下载", legendX + 18, legendY + 4);

  // Upload legend
  ctx.fillStyle = "#3b82f6";
  ctx.fillRect(legendX + 60, legendY, 12, 3);
  ctx.fillStyle = "#9ca3af";
  ctx.fillText("上传", legendX + 78, legendY + 4);
}

// Watch interface change - canvas update
watch(selectedInterface, () => {
  if (running.value) {
    // Reset history and redraw
    downloadHistory.value = [];
    uploadHistory.value = [];
    labels.value = [];
  }
});

let resizeObserver: ResizeObserver | null = null;

onMounted(async () => {
  await loadInterfaces();

  unlistenData = await listen<BandwidthSample[]>("bandwidth:data", (event) => {
    handleData(event.payload);
  });

  unlistenError = await listen<{ error: string }>("bandwidth:error", (event) => {
    error.value = event.payload.error;
    running.value = false;
  });

  // Resize observer for canvas
  const canvas = canvasRef.value;
  if (canvas) {
    resizeObserver = new ResizeObserver(() => {
      if (downloadHistory.value.length > 0) drawChart();
    });
    resizeObserver.observe(canvas.parentElement!);
  }
});

onUnmounted(() => {
  if (running.value) {
    invoke("stop_bandwidth_monitor").catch(() => {});
  }
  unlistenData?.();
  unlistenError?.();
  resizeObserver?.disconnect();
});
</script>

<template>
  <div class="flex h-full flex-col p-4 md:p-6 space-y-4 md:space-y-6 animate-view-fade">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-display font-bold text-ink">带宽监控</h1>
        <p class="mt-0.5 text-sm text-ink-faint">实时网络接口流量监控</p>
      </div>
      <div class="flex items-center gap-2">
        <Button
          :variant="running ? 'danger' : 'default'"
          @click="running ? stopMonitor() : startMonitor()"
        >
          <component :is="running ? Square : Play" class="mr-1.5 h-3.5 w-3.5" />
          {{ running ? "停止" : "开始监控" }}
        </Button>
      </div>
    </div>

    <!-- Error -->
    <div
      v-if="error"
      class="rounded-lg border border-red-200 bg-red-50 px-4 py-2 text-sm text-red-600 dark:border-red-900/30 dark:bg-red-900/20 dark:text-red-400"
    >
      {{ error }}
    </div>

    <!-- Controls and current stats -->
    <div class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm">
      <div class="flex flex-wrap items-end gap-4">
        <!-- Interface selector -->
        <div class="min-w-[200px]">
          <label class="mb-1 block text-xs font-medium text-ink-soft">网络接口</label>
          <select
            v-model="selectedInterface"
            :disabled="running"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20 disabled:opacity-50"
          >
            <option value="*">全部接口</option>
            <option
              v-for="iface in interfaces"
              :key="iface.name"
              :value="iface.name"
            >
              {{ iface.friendlyName || iface.name }}
            </option>
          </select>
        </div>

        <!-- Current speed stats -->
        <div class="flex gap-4 ml-auto">
          <div class="text-right">
            <div class="text-xs text-ink-faint">下载</div>
            <div class="text-lg font-mono font-bold text-green-500">
              {{ formatBps(currentDownload) }}
            </div>
          </div>
          <div class="text-right">
            <div class="text-xs text-ink-faint">上传</div>
            <div class="text-lg font-mono font-bold text-blue-500">
              {{ formatBps(currentUpload) }}
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Chart -->
    <div
      class="relative flex-1 min-h-[250px] overflow-hidden rounded-xl border border-paper-deep/60 bg-paper shadow-sm"
      :class="{ 'opacity-50': !running }"
    >
      <div class="absolute inset-0 p-4">
        <canvas
          ref="canvasRef"
          class="h-full w-full"
          :style="{
            '--bg-chart': 'transparent',
          }"
        ></canvas>
      </div>
      <!-- Overlay when not running -->
      <div
        v-if="!running"
        class="absolute inset-0 flex items-center justify-center bg-paper/60 backdrop-blur-[1px]"
      >
        <div class="text-center text-ink-faint">
          <Activity class="mx-auto mb-2 h-8 w-8" />
          <p class="text-sm">点击"开始监控"查看实时带宽</p>
        </div>
      </div>
    </div>
  </div>
</template>
