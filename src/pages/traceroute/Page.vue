<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { Play, Square } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import {
  tracerouteStart,
  tracerouteStop,
  onTraceHop,
  onTraceComplete,
  type TraceHopPayload,
  type TraceCompletePayload,
} from "@/lib/tauri";
import type { UnlistenFn } from "@tauri-apps/api/event";

const target = ref("8.8.8.8");
const maxHops = ref(30);
const timeout = ref(3000);
const running = ref(false);
const error = ref("");
const currentTaskId = ref("");

interface HopResult {
  hop: number;
  addr: string | null;
  hostname: string | null;
  latencies: (number | null)[];
}

const hops = ref<HopResult[]>([]);
const completeInfo = ref<TraceCompletePayload | null>(null);

let unlistenHop: UnlistenFn | null = null;
let unlistenComplete: UnlistenFn | null = null;

async function startTrace() {
  if (!target.value.trim()) return;
  running.value = true;
  error.value = "";
  hops.value = [];
  completeInfo.value = null;

  try {
    const taskId = await tracerouteStart(target.value, {
      maxHops: maxHops.value,
      timeoutMs: timeout.value,
      probesPerHop: 3,
    });
    currentTaskId.value = taskId;
  } catch (e) {
    error.value = String(e);
    running.value = false;
  }
}

async function stopTrace() {
  if (!currentTaskId.value) return;
  try {
    await tracerouteStop(currentTaskId.value);
  } catch {
    // ignore
  }
  running.value = false;
}

function handleHop(payload: TraceHopPayload) {
  hops.value.push({
    hop: payload.hop,
    addr: payload.addr,
    hostname: payload.hostname,
    latencies: payload.latencies,
  });
}

function handleComplete(payload: TraceCompletePayload) {
  completeInfo.value = payload;
  running.value = false;
  currentTaskId.value = "";
}

function formatLatency(ms: number | null): string {
  if (ms === null) return "*";
  return `${ms.toFixed(1)} ms`;
}

function formatAddr(addr: string | null, hostname: string | null): string {
  if (!addr) return "*";
  if (hostname && hostname !== addr) return `${hostname} (${addr})`;
  return addr;
}

onMounted(async () => {
  unlistenHop = await onTraceHop(handleHop);
  unlistenComplete = await onTraceComplete(handleComplete);
});

onUnmounted(() => {
  if (running.value && currentTaskId.value) {
    tracerouteStop(currentTaskId.value).catch(() => {});
  }
  unlistenHop?.();
  unlistenComplete?.();
});
</script>

<template>
  <div class="flex h-full flex-col p-6 space-y-6 animate-view-fade">
    <!-- Header -->
    <div>
      <h1 class="text-2xl font-display font-bold text-ink">Traceroute</h1>
      <p class="mt-0.5 text-sm text-ink-faint">追踪数据包到达目标的路由路径</p>
    </div>

    <!-- Input card -->
    <div class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm">
      <div class="flex flex-wrap items-end gap-3">
        <div class="flex-1 min-w-[180px]">
          <label class="mb-1 block text-xs font-medium text-ink-soft">目标地址</label>
          <input
            v-model="target"
            placeholder="IP 地址或域名"
            :disabled="running"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink placeholder:text-ink-faint/50 outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20 disabled:opacity-50"
          />
        </div>
        <div class="w-24">
          <label class="mb-1 block text-xs font-medium text-ink-soft">最大跳数</label>
          <input
            v-model.number="maxHops"
            type="number"
            min="1"
            max="64"
            :disabled="running"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20 disabled:opacity-50"
          />
        </div>
        <div class="w-24">
          <label class="mb-1 block text-xs font-medium text-ink-soft">超时 (ms)</label>
          <input
            v-model.number="timeout"
            type="number"
            min="100"
            max="30000"
            step="100"
            :disabled="running"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20 disabled:opacity-50"
          />
        </div>
        <div class="flex gap-2">
          <Button :disabled="running" @click="startTrace">
            <Play class="mr-1.5 h-3.5 w-3.5" />
            开始
          </Button>
          <Button variant="danger" :disabled="!running" @click="stopTrace">
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

    <!-- Results card -->
    <div
      v-if="hops.length > 0"
      class="noise-bg rounded-xl border border-paper-deep/60 bg-paper shadow-sm overflow-hidden"
    >
      <div class="px-5 py-3 border-b border-paper-deep/50">
        <h2 class="text-sm font-semibold text-ink">路由节点</h2>
      </div>
      <div class="overflow-x-auto">
        <table class="w-full text-sm">
          <thead>
            <tr class="border-b border-paper-deep/30 text-xs text-ink-faint uppercase tracking-wider">
              <th class="px-5 py-3 text-left font-medium w-16">跳数</th>
              <th class="px-5 py-3 text-left font-medium">地址</th>
              <th class="px-5 py-3 text-right font-medium">延迟 1</th>
              <th class="px-5 py-3 text-right font-medium">延迟 2</th>
              <th class="px-5 py-3 text-right font-medium">延迟 3</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="h in hops"
              :key="h.hop"
              class="border-b border-paper-deep/20 last:border-0 animate-slide-up"
            >
              <td class="px-5 py-2.5 text-ink-soft font-mono">{{ h.hop }}</td>
              <td class="px-5 py-2.5 text-ink">
                {{ formatAddr(h.addr, h.hostname) }}
              </td>
              <td
                v-for="(lat, i) in h.latencies"
                :key="i"
                class="px-5 py-2.5 text-right font-mono text-ink-soft"
              >
                {{ formatLatency(lat) }}
              </td>
              <!-- Fill remaining cells if fewer than 3 latencies -->
              <td
                v-for="n in Math.max(0, 3 - h.latencies.length)"
                :key="'empty-' + n"
                class="px-5 py-2.5 text-right font-mono text-ink-faint"
              >
                *
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- Complete info -->
    <div
      v-if="completeInfo"
      class="rounded-xl border border-paper-deep/60 bg-paper-warm/50 px-5 py-3 text-sm text-ink-soft animate-fade-in"
    >
      追踪完成：目标 {{ completeInfo.target }}，经过 {{ completeInfo.hops.length }} 跳
    </div>
  </div>
</template>
