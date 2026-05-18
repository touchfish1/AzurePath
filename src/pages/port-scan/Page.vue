<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { Play, Square } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import {
  portScanStart,
  portScanStop,
  onPortProgress,
  onPortFound,
  onPortComplete,
  type PortProgressPayload,
  type PortFoundPayload,
  type PortCompletePayload,
} from "@/lib/tauri";
import type { UnlistenFn } from "@tauri-apps/api/event";

const target = ref("127.0.0.1");
const portStart = ref(1);
const portEnd = ref(1024);
const concurrency = ref(100);
const timeout = ref(2000);
const running = ref(false);
const error = ref("");
const currentTaskId = ref("");

const progress = ref<PortProgressPayload | null>(null);
const foundPorts = ref<PortFoundPayload[]>([]);
const completeInfo = ref<PortCompletePayload | null>(null);

const progressPercent = computed(() => {
  if (!progress.value || progress.value.total === 0) return 0;
  return Math.round((progress.value.scanned / progress.value.total) * 100);
});

let unlistenProgress: UnlistenFn | null = null;
let unlistenFound: UnlistenFn | null = null;
let unlistenComplete: UnlistenFn | null = null;

async function startScan() {
  if (!target.value.trim()) return;
  if (portStart.value < 1 || portEnd.value > 65535 || portStart.value > portEnd.value) {
    error.value = "端口范围无效，请确保起始端口 ≥ 1，结束端口 ≤ 65535，且起始 ≤ 结束";
    return;
  }

  running.value = true;
  error.value = "";
  progress.value = null;
  foundPorts.value = [];
  completeInfo.value = null;

  try {
    const taskId = await portScanStart(
      target.value,
      { start: portStart.value, end: portEnd.value },
      { concurrency: concurrency.value, timeoutMs: timeout.value },
    );
    currentTaskId.value = taskId;
  } catch (e) {
    error.value = String(e);
    running.value = false;
  }
}

async function stopScan() {
  if (!currentTaskId.value) return;
  try {
    await portScanStop(currentTaskId.value);
  } catch {
    // ignore
  }
  running.value = false;
}

function handleProgress(payload: PortProgressPayload) {
  progress.value = payload;
}

function handleFound(payload: PortFoundPayload) {
  foundPorts.value.push(payload);
}

function handleComplete(payload: PortCompletePayload) {
  completeInfo.value = payload;
  progress.value = null;
  running.value = false;
  currentTaskId.value = "";
}

onMounted(async () => {
  unlistenProgress = await onPortProgress(handleProgress);
  unlistenFound = await onPortFound(handleFound);
  unlistenComplete = await onPortComplete(handleComplete);
});

onUnmounted(() => {
  if (running.value && currentTaskId.value) {
    portScanStop(currentTaskId.value).catch(() => {});
  }
  unlistenProgress?.();
  unlistenFound?.();
  unlistenComplete?.();
});
</script>

<template>
  <div class="flex h-full flex-col p-6 space-y-6 animate-view-fade">
    <!-- Header -->
    <div>
      <h1 class="text-2xl font-display font-bold text-ink">端口扫描</h1>
      <p class="mt-0.5 text-sm text-ink-faint">扫描目标主机的开放端口</p>
    </div>

    <!-- Input card -->
    <div class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm">
      <div class="flex flex-wrap items-end gap-3">
        <div class="flex-1 min-w-[160px]">
          <label class="mb-1 block text-xs font-medium text-ink-soft">目标地址</label>
          <input
            v-model="target"
            placeholder="IP 地址或域名"
            :disabled="running"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink placeholder:text-ink-faint/50 outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20 disabled:opacity-50"
          />
        </div>
        <div class="w-20">
          <label class="mb-1 block text-xs font-medium text-ink-soft">起始端口</label>
          <input
            v-model.number="portStart"
            type="number"
            min="1"
            max="65535"
            :disabled="running"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20 disabled:opacity-50"
          />
        </div>
        <div class="w-20">
          <label class="mb-1 block text-xs font-medium text-ink-soft">结束端口</label>
          <input
            v-model.number="portEnd"
            type="number"
            min="1"
            max="65535"
            :disabled="running"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20 disabled:opacity-50"
          />
        </div>
        <div class="flex gap-2">
          <Button :disabled="running" @click="startScan">
            <Play class="mr-1.5 h-3.5 w-3.5" />
            开始
          </Button>
          <Button variant="danger" :disabled="!running" @click="stopScan">
            <Square class="mr-1.5 h-3.5 w-3.5" />
            停止
          </Button>
        </div>
      </div>
    </div>

    <!-- Error banner -->
    <div
      v-if="error"
      class="rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700 dark:border-red-800/30 dark:bg-red-900/10 dark:text-red-400"
    >
      {{ error }}
    </div>

    <!-- Progress bar -->
    <div
      v-if="progress"
      class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm"
    >
      <div class="flex items-center justify-between mb-3">
        <span class="text-sm font-medium text-ink">扫描进度</span>
        <span class="text-xs text-ink-faint font-mono">
          {{ progress.scanned }} / {{ progress.total }}
          ({{ progressPercent }}%)
        </span>
      </div>
      <div class="h-2 rounded-full bg-paper-deep overflow-hidden">
        <div
          class="h-full rounded-full bg-bamboo transition-all duration-300 ease-out"
          :style="{ width: progressPercent + '%' }"
        />
      </div>
      <p class="mt-2 text-xs text-ink-faint">
        已发现 <span class="font-semibold text-bamboo">{{ progress.open }}</span> 个开放端口
      </p>
    </div>

    <!-- Found ports -->
    <div
      v-if="foundPorts.length > 0"
      class="noise-bg rounded-xl border border-paper-deep/60 bg-paper shadow-sm overflow-hidden"
    >
      <div class="px-5 py-3 border-b border-paper-deep/50">
        <h2 class="text-sm font-semibold text-ink">
          开放端口 ({{ foundPorts.length }})
        </h2>
      </div>
      <div class="overflow-x-auto">
        <table class="w-full text-sm">
          <thead>
            <tr class="border-b border-paper-deep/30 text-xs text-ink-faint uppercase tracking-wider">
              <th class="px-5 py-3 text-left font-medium">端口</th>
              <th class="px-5 py-3 text-left font-medium">服务</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="fp in foundPorts"
              :key="fp.port"
              class="border-b border-paper-deep/20 last:border-0 animate-slide-up"
            >
              <td class="px-5 py-2.5 font-mono text-ink">{{ fp.port }}</td>
              <td class="px-5 py-2.5 text-ink-soft">
                {{ fp.service || "未知" }}
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- No ports found -->
    <div
      v-if="!running && completeInfo && foundPorts.length === 0"
      class="rounded-xl border border-paper-deep/60 bg-paper-warm/50 px-5 py-4 text-center text-sm text-ink-faint animate-fade-in"
    >
      未发现开放端口
    </div>
  </div>
</template>
