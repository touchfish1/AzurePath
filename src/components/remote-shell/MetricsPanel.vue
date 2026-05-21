<script setup lang="ts">
import { ref, watch, onUnmounted, computed } from "vue";
import { Cpu, MemoryStick, HardDrive, RefreshCw, Activity } from "lucide-vue-next";
import { remoteShellGetMetrics, type HostMetrics } from "@/lib/tauri";
import { formatSize } from "@/lib/format";

interface Props {
  sessionId: string | null;
}

const props = defineProps<Props>();

const metrics = ref<HostMetrics | null>(null);
const loading = ref(false);
const error = ref("");
const autoRefresh = ref(false);
let refreshTimer: ReturnType<typeof setInterval> | null = null;

async function loadMetrics() {
  if (!props.sessionId) return;
  loading.value = true;
  error.value = "";
  try {
    metrics.value = await remoteShellGetMetrics(props.sessionId);
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

function toggleAutoRefresh() {
  autoRefresh.value = !autoRefresh.value;
}

watch(autoRefresh, (val) => {
  if (refreshTimer) {
    clearInterval(refreshTimer);
    refreshTimer = null;
  }
  if (val && props.sessionId) {
    loadMetrics();
    refreshTimer = setInterval(loadMetrics, 3000);
  }
});

watch(
  () => props.sessionId,
  (id) => {
    if (refreshTimer) {
      clearInterval(refreshTimer);
      refreshTimer = null;
      autoRefresh.value = false;
    }
    if (id) {
      loadMetrics();
    } else {
      metrics.value = null;
      error.value = "";
    }
  },
  { immediate: true },
);

onUnmounted(() => {
  if (refreshTimer) {
    clearInterval(refreshTimer);
  }
});

const cpuPercent = computed(() => metrics.value?.cpuPercent ?? 0);
const memPercent = computed(() => metrics.value?.memoryPercent ?? 0);
const diskPercent = computed(() => metrics.value?.diskPercent ?? 0);

function barColor(pct: number) {
  if (pct >= 90) return "bg-red-500";
  if (pct >= 70) return "bg-yellow-500";
  return "bg-bamboo";
}
</script>

<template>
  <div class="flex h-full flex-col">
    <!-- Header -->
    <div class="flex items-center justify-between px-3 pb-2">
      <div class="flex items-center gap-1.5">
        <Activity class="h-3.5 w-3.5 text-ink-faint" />
        <span class="text-xs font-medium text-ink">主机指标</span>
      </div>
      <div class="flex items-center gap-1">
        <button
          class="rounded p-1 text-xs transition-colors"
          :class="
            autoRefresh
              ? 'bg-bamboo/10 text-bamboo'
              : 'text-ink-faint hover:text-ink hover:bg-paper-deep/50'
          "
          title="自动刷新 (3s)"
          @click="toggleAutoRefresh"
        >
          <RefreshCw class="h-3 w-3" :class="{ 'animate-spin': loading && autoRefresh }" />
        </button>
      </div>
    </div>

    <!-- Body -->
    <div class="flex-1 overflow-y-auto px-3 pb-3 space-y-3">
      <!-- Empty state -->
      <div
        v-if="!sessionId"
        class="flex flex-col items-center justify-center py-8 text-center"
      >
        <Activity class="h-6 w-6 text-ink-faint/40 mb-2" />
        <p class="text-xs text-ink-faint/60">选择一个已连接的会话</p>
      </div>

      <!-- Error -->
      <div
        v-else-if="error"
        class="rounded-lg border border-red-200 bg-red-50 px-2.5 py-1.5 text-[11px] text-red-700 dark:border-red-800/30 dark:bg-red-900/10 dark:text-red-400"
      >
        {{ error }}
      </div>

      <!-- Metrics cards -->
      <template v-else-if="metrics">
        <!-- CPU -->
        <div class="rounded-lg border border-paper-deep/50 bg-paper-warm/30 p-2.5">
          <div class="flex items-center justify-between mb-1.5">
            <div class="flex items-center gap-1.5">
              <Cpu class="h-3.5 w-3.5 text-ink-faint" />
              <span class="text-[11px] font-medium text-ink">CPU</span>
            </div>
            <span class="text-xs font-mono font-semibold" :class="cpuPercent >= 90 ? 'text-red-500' : cpuPercent >= 70 ? 'text-yellow-500' : 'text-bamboo'">
              {{ cpuPercent.toFixed(1) }}%
            </span>
          </div>
          <div class="h-1.5 w-full overflow-hidden rounded-full bg-paper-deep/50">
            <div
              class="h-full rounded-full transition-all duration-500"
              :class="barColor(cpuPercent)"
              :style="{ width: cpuPercent + '%' }"
            />
          </div>
        </div>

        <!-- Memory -->
        <div class="rounded-lg border border-paper-deep/50 bg-paper-warm/30 p-2.5">
          <div class="flex items-center justify-between mb-1.5">
            <div class="flex items-center gap-1.5">
              <MemoryStick class="h-3.5 w-3.5 text-ink-faint" />
              <span class="text-[11px] font-medium text-ink">内存</span>
            </div>
            <span class="text-xs font-mono font-semibold" :class="memPercent >= 90 ? 'text-red-500' : memPercent >= 70 ? 'text-yellow-500' : 'text-bamboo'">
              {{ memPercent.toFixed(1) }}%
            </span>
          </div>
          <div class="h-1.5 w-full overflow-hidden rounded-full bg-paper-deep/50">
            <div
              class="h-full rounded-full transition-all duration-500"
              :class="barColor(memPercent)"
              :style="{ width: memPercent + '%' }"
            />
          </div>
          <div class="mt-1 text-[10px] text-ink-faint">
            {{ formatSize(metrics.memoryUsedBytes) }} / {{ formatSize(metrics.memoryTotalBytes) }}
          </div>
        </div>

        <!-- Disk -->
        <div class="rounded-lg border border-paper-deep/50 bg-paper-warm/30 p-2.5">
          <div class="flex items-center justify-between mb-1.5">
            <div class="flex items-center gap-1.5">
              <HardDrive class="h-3.5 w-3.5 text-ink-faint" />
              <span class="text-[11px] font-medium text-ink">磁盘</span>
            </div>
            <span class="text-xs font-mono font-semibold" :class="diskPercent >= 90 ? 'text-red-500' : diskPercent >= 70 ? 'text-yellow-500' : 'text-bamboo'">
              {{ diskPercent.toFixed(1) }}%
            </span>
          </div>
          <div class="h-1.5 w-full overflow-hidden rounded-full bg-paper-deep/50">
            <div
              class="h-full rounded-full transition-all duration-500"
              :class="barColor(diskPercent)"
              :style="{ width: diskPercent + '%' }"
            />
          </div>
          <div class="mt-1 text-[10px] text-ink-faint">
            {{ formatSize(metrics.diskUsedBytes) }} / {{ formatSize(metrics.diskTotalBytes) }}
          </div>
        </div>

        <!-- Timestamp -->
        <div class="text-center text-[10px] text-ink-faint/60">
          更新于 {{ new Date(metrics.collectedAt).toLocaleTimeString("zh-CN") }}
        </div>
      </template>

      <!-- Loading skeleton -->
      <div v-else class="space-y-3">
        <div v-for="i in 3" :key="i" class="rounded-lg border border-paper-deep/50 bg-paper-warm/30 p-2.5 animate-pulse">
          <div class="flex items-center justify-between mb-2">
            <div class="h-3 w-12 rounded bg-paper-deep/60" />
            <div class="h-3 w-10 rounded bg-paper-deep/60" />
          </div>
          <div class="h-1.5 w-full rounded-full bg-paper-deep/60" />
        </div>
      </div>
    </div>
  </div>
</template>
