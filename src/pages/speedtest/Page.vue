<script setup lang="ts">
import { ref, onUnmounted } from "vue";
import { Play, Square, Activity } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import {
  startSpeedtest,
  onSpeedtestProgress,
  onSpeedtestComplete,
  type SpeedtestProgress,
  type SpeedtestResult,
} from "@/lib/tauri";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { useToastStore } from "@/stores/toast";

const toast = useToastStore();

const peerIp = ref("127.0.0.1");
const port = ref(5201);
const duration = ref(5);
const mode = ref<"client" | "server">("client");
const running = ref(false);
const progress = ref<SpeedtestProgress | null>(null);
const result = ref<SpeedtestResult | null>(null);

let unlistenProgress: UnlistenFn | null = null;
let unlistenComplete: UnlistenFn | null = null;

async function attachListeners() {
  detachListeners();
  unlistenProgress = await onSpeedtestProgress(handleProgress);
  unlistenComplete = await onSpeedtestComplete(handleComplete);
}

function detachListeners() {
  unlistenProgress?.();
  unlistenComplete?.();
  unlistenProgress = null;
  unlistenComplete = null;
}

function handleProgress(payload: SpeedtestProgress) {
  progress.value = payload;
}

function handleComplete(payload: SpeedtestResult) {
  result.value = payload;
  running.value = false;
  progress.value = null;
  toast.add("success", "测速完成");
}

async function start() {
  if (!peerIp.value.trim()) return;
  running.value = true;
  result.value = null;
  progress.value = null;

  try {
    await startSpeedtest(peerIp.value, port.value, duration.value, mode.value);
    await attachListeners();
  } catch (e) {
    running.value = false;
    toast.add("error", String(e));
  }
}

function stop() {
  // Speedtest cannot be stopped mid-way, but switching mode resets
  running.value = false;
  progress.value = null;
  detachListeners();
}

function phaseLabel(phase: string): string {
  const map: Record<string, string> = {
    latency: "延迟测试",
    download: "下载测速",
    upload: "上传测速",
    complete: "完成",
  };
  return map[phase] || phase;
}

onUnmounted(() => {
  detachListeners();
});
</script>

<template>
  <div class="flex h-full flex-col p-6 space-y-6 animate-view-fade">
    <!-- Header -->
    <div>
      <h1 class="text-2xl font-display font-bold text-ink">局域网测速</h1>
      <p class="mt-0.5 text-sm text-ink-faint">测量与 LAN 对等节点的带宽、延迟与抖动</p>
    </div>

    <!-- Input card -->
    <div class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm">
      <div class="flex flex-wrap items-end gap-3">
        <div class="flex-1 min-w-[160px]">
          <label class="mb-1 block text-xs font-medium text-ink-soft">对等节点 IP</label>
          <input
            v-model="peerIp"
            placeholder="192.168.1.x"
            :disabled="running"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink placeholder:text-ink-faint/50 outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20 disabled:opacity-50"
          />
        </div>
        <div class="w-24">
          <label class="mb-1 block text-xs font-medium text-ink-soft">端口</label>
          <input
            v-model.number="port"
            type="number"
            min="1024"
            max="65535"
            :disabled="running"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20 disabled:opacity-50"
          />
        </div>
        <div class="w-24">
          <label class="mb-1 block text-xs font-medium text-ink-soft">时长(s)</label>
          <input
            v-model.number="duration"
            type="number"
            min="1"
            max="30"
            :disabled="running"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20 disabled:opacity-50"
          />
        </div>
        <div class="w-28">
          <label class="mb-1 block text-xs font-medium text-ink-soft">模式</label>
          <select
            v-model="mode"
            :disabled="running"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20 disabled:opacity-50"
          >
            <option value="client">客户端</option>
            <option value="server">服务端</option>
          </select>
        </div>
        <div class="flex gap-2">
          <Button :disabled="running" @click="start">
            <Play class="mr-1.5 h-3.5 w-3.5" />
            开始
          </Button>
          <Button variant="danger" :disabled="!running" @click="stop">
            <Square class="mr-1.5 h-3.5 w-3.5" />
            重置
          </Button>
        </div>
      </div>
      <p v-if="mode === 'server'" class="mt-3 text-xs text-ink-faint">
        服务端模式：监听指定端口，等待客户端连接进行测速
      </p>
      <p v-else class="mt-3 text-xs text-ink-faint">
        客户端模式：连接到指定对等节点执行完整测速
      </p>
    </div>

    <!-- Progress card -->
    <div
      v-if="progress"
      class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm"
    >
      <div class="flex items-center justify-between mb-3">
        <span class="text-sm font-medium text-ink">
          {{ phaseLabel(progress.phase) }}
        </span>
        <span class="text-xs text-ink-faint font-mono">
          {{ progress.percent.toFixed(0) }}%
        </span>
      </div>
      <div class="h-2 rounded-full bg-paper-deep overflow-hidden">
        <div
          class="h-full rounded-full bg-bamboo transition-all duration-300 ease-out"
          :style="{ width: progress.percent + '%' }"
        />
      </div>
      <p v-if="progress.currentValue > 0" class="mt-2 text-xs text-ink-faint">
        当前速度: {{ progress.currentValue.toFixed(2) }} Mbps
      </p>
    </div>

    <!-- Result card -->
    <div
      v-if="result"
      class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm animate-scale-in"
    >
      <h2 class="text-sm font-semibold text-ink mb-4">测速结果</h2>
      <div class="grid grid-cols-2 gap-4 sm:grid-cols-4">
        <div>
          <p class="text-xs text-ink-faint">下载速度</p>
          <p class="mt-0.5 text-lg font-mono font-semibold text-bamboo">
            {{ result.downloadMbps.toFixed(2) }} Mbps
          </p>
        </div>
        <div>
          <p class="text-xs text-ink-faint">上传速度</p>
          <p class="mt-0.5 text-lg font-mono font-semibold text-bamboo">
            {{ result.uploadMbps.toFixed(2) }} Mbps
          </p>
        </div>
        <div>
          <p class="text-xs text-ink-faint">延迟</p>
          <p class="mt-0.5 text-lg font-mono font-semibold text-ink">
            {{ result.latencyMs.toFixed(2) }} ms
          </p>
        </div>
        <div>
          <p class="text-xs text-ink-faint">抖动</p>
          <p class="mt-0.5 text-lg font-mono font-semibold text-ink">
            {{ result.jitterMs.toFixed(2) }} ms
          </p>
        </div>
      </div>
      <p class="mt-4 text-xs text-ink-faint">
        对等节点: {{ result.peerIp }}
      </p>
    </div>

    <!-- Empty state -->
    <div
      v-if="!running && !progress && !result"
      class="flex items-center justify-center rounded-xl border border-dashed border-paper-deep/30 bg-paper-warm/20 py-16 text-sm text-ink-faint"
    >
      <div class="text-center max-w-sm">
        <Activity class="mx-auto h-10 w-10 mb-3 opacity-30" />
        <p class="font-medium text-ink-soft">配置对等节点信息开始测速</p>
        <p class="mt-2 text-xs opacity-60 leading-relaxed">
          在目标机器上启动服务端模式，在本机使用客户端模式连接
          <br />
          默认端口 5201，测试时长 5 秒
        </p>
      </div>
    </div>
  </div>
</template>
