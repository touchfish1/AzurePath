<script setup lang="ts">
import { onMounted, onUnmounted, ref, computed } from "vue";
import { Play, Square, Route, Copy, BarChart3, Table2 } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import BookmarkButton from "@/components/BookmarkButton.vue";
import { useMtrStore } from "@/stores/mtr";
import { useToastStore } from "@/stores/toast";

const toast = useToastStore();
const store = useMtrStore();

const activeTab = ref<"real-time" | "report">("real-time");

// Switch to report tab when MTR completes
const prevRunning = ref(store.isRunning);
import { watch } from "vue";
watch(
  () => store.isRunning,
  (running) => {
    if (prevRunning.value && !running && !store.error) {
      activeTab.value = "report";
    }
    prevRunning.value = running;
  },
);

// ─── Computed stats for summary ─────────────────────────────────

const totalLossRate = computed(() => {
  if (store.hops.length === 0) return 0;
  const totalSent = store.hops.reduce((s, h) => s + h.sent, 0);
  const totalReceived = store.hops.reduce((s, h) => s + h.received, 0);
  if (totalSent === 0) return 0;
  return ((totalSent - totalReceived) / totalSent) * 100;
});

const avgLatency = computed(() => {
  const withData = store.hops.filter((h) => h.received > 0);
  if (withData.length === 0) return 0;
  return withData.reduce((s, h) => s + h.avgMs, 0) / withData.length;
});

// ─── Helpers ────────────────────────────────────────────────────

function formatLatency(ms: number | null): string {
  if (ms === null) return "---";
  return `${ms.toFixed(1)}`;
}

function formatLossClass(lossPercent: number): string {
  if (lossPercent <= 0) return "";
  if (lossPercent < 10) return "text-yellow-600 dark:text-yellow-400";
  return "text-red-600 dark:text-red-400 font-semibold";
}

function formatAddr(addr: string | null, hostname: string | null): string {
  if (!addr) return "*";
  if (hostname && hostname !== addr) return `${hostname} (${addr})`;
  return addr;
}

function copyAddr(addr: string | null) {
  if (!addr) return;
  navigator.clipboard.writeText(addr).then(() => {
    toast.add("success", "已复制");
  });
}

async function handleStart() {
  await store.start();
}

function handleStop() {
  store.stop();
}

onMounted(async () => {
  if (store.currentTaskId) {
    await store.attachListeners();
  }
});

onUnmounted(() => {
  store.detachListeners();
});
</script>

<template>
  <div class="flex h-full flex-col p-4 md:p-6 space-y-4 md:space-y-6 animate-view-fade">
    <!-- Header -->
    <div>
      <h1 class="text-2xl font-display font-bold text-ink">MTR 路由追踪</h1>
      <p class="mt-0.5 text-sm text-ink-faint">持续探测路由路径每跳的延迟与丢包率，实时监控网络质量</p>
    </div>

    <!-- Input card -->
    <div class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm">
      <div class="flex flex-wrap items-end gap-3">
        <div class="flex-1 min-w-[180px]">
          <label class="mb-1 block text-xs font-medium text-ink-soft">目标地址</label>
          <div class="flex items-center gap-1">
            <input
              v-model="store.target"
              placeholder="IP 地址或域名"
              :disabled="store.isRunning"
              class="flex-1 rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink placeholder:text-ink-faint/50 outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20 disabled:opacity-50"
            />
            <BookmarkButton :target="store.target" @select="store.target = $event" />
          </div>
        </div>
        <div class="w-20">
          <label class="mb-1 block text-xs font-medium text-ink-soft">最大跳数</label>
          <input
            v-model.number="store.maxHops"
            type="number"
            min="1"
            max="64"
            :disabled="store.isRunning"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20 disabled:opacity-50"
          />
        </div>
        <div class="w-20">
          <label class="mb-1 block text-xs font-medium text-ink-soft">间隔 (ms)</label>
          <input
            v-model.number="store.intervalMs"
            type="number"
            min="100"
            max="60000"
            step="100"
            :disabled="store.isRunning"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20 disabled:opacity-50"
          />
        </div>
        <div class="w-20">
          <label class="mb-1 block text-xs font-medium text-ink-soft">超时 (ms)</label>
          <input
            v-model.number="store.timeoutMs"
            type="number"
            min="100"
            max="30000"
            step="100"
            :disabled="store.isRunning"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20 disabled:opacity-50"
          />
        </div>
        <div class="flex gap-2">
          <Button :disabled="store.isRunning" @click="handleStart">
            <Play class="mr-1.5 h-3.5 w-3.5" />
            开始
          </Button>
          <Button variant="danger" :disabled="!store.isRunning" @click="handleStop">
            <Square class="mr-1.5 h-3.5 w-3.5" />
            停止
          </Button>
        </div>
      </div>
    </div>

    <!-- Running indicator -->
    <div
      v-if="store.isRunning"
      class="rounded-xl border border-bamboo/20 bg-bamboo/5 px-4 py-3 text-sm text-bamboo animate-pulse"
    >
      正在探测：{{ store.target }}，已采集 {{ store.totalRounds }} 轮数据
    </div>

    <!-- Error banner -->
    <div
      v-if="store.error"
      class="rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700 dark:border-red-800/30 dark:bg-red-900/10 dark:text-red-400"
    >
      {{ store.error }}
    </div>

    <!-- Tab switch + Summary stats -->
    <div
      v-if="store.hops.length > 0"
      class="noise-bg rounded-xl border border-paper-deep/60 bg-paper shadow-sm overflow-hidden"
    >
      <!-- Tab bar -->
      <div class="flex items-center border-b border-paper-deep/50 px-5">
        <button
          class="flex items-center gap-1.5 px-3 py-3 text-sm font-medium transition-colors border-b-2 -mb-px"
          :class="
            activeTab === 'real-time'
              ? 'border-bamboo text-bamboo'
              : 'border-transparent text-ink-soft hover:text-ink'
          "
          @click="activeTab = 'real-time'"
        >
          <Table2 class="h-4 w-4" />
          实时数据
        </button>
        <button
          class="flex items-center gap-1.5 px-3 py-3 text-sm font-medium transition-colors border-b-2 -mb-px"
          :class="
            activeTab === 'report'
              ? 'border-bamboo text-bamboo'
              : 'border-transparent text-ink-soft hover:text-ink'
          "
          @click="activeTab = 'report'"
        >
          <BarChart3 class="h-4 w-4" />
          分析报告
        </button>

        <!-- Summary inline -->
        <div class="ml-auto flex items-center gap-4 text-xs text-ink-faint">
          <span>总轮次: <strong class="text-ink-soft">{{ store.totalRounds }}</strong></span>
          <span>总丢失: <strong :class="totalLossRate > 5 ? 'text-red-500' : 'text-ink-soft'">{{ totalLossRate.toFixed(1) }}%</strong></span>
          <span>平均延迟: <strong class="text-ink-soft">{{ avgLatency.toFixed(1) }} ms</strong></span>
        </div>
      </div>

      <!-- Real-time view -->
      <div v-if="activeTab === 'real-time'" class="overflow-x-auto">
        <table class="w-full text-sm">
          <thead>
            <tr class="border-b border-paper-deep/30 text-xs text-ink-faint uppercase tracking-wider">
              <th class="px-4 py-3 text-left font-medium w-12">#</th>
              <th class="px-4 py-3 text-left font-medium">地址</th>
              <th class="px-4 py-3 text-right font-medium">Loss%</th>
              <th class="px-4 py-3 text-right font-medium">Snt</th>
              <th class="px-4 py-3 text-right font-medium">上次</th>
              <th class="px-4 py-3 text-right font-medium">平均</th>
              <th class="px-4 py-3 text-right font-medium">最佳</th>
              <th class="px-4 py-3 text-right font-medium">最差</th>
              <th class="px-4 py-3 text-right font-medium">抖动</th>
              <th class="px-4 py-3 text-left font-medium w-10">操作</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="h in store.hops"
              :key="h.hop"
              class="border-b border-paper-deep/20 last:border-0 animate-slide-up transition-colors"
              :class="h.lossPercent > 0 ? 'bg-red-50/30 dark:bg-red-900/5' : ''"
            >
              <td class="px-4 py-2.5 text-ink-soft font-mono">{{ h.hop }}</td>
              <td class="px-4 py-2.5 text-ink font-mono text-xs">
                <span v-if="h.addr" class="flex items-center gap-1">
                  {{ h.addr }}
                  <button
                    class="inline-flex p-0.5 text-ink-faint transition-colors hover:text-bamboo"
                    title="复制地址"
                    @click="copyAddr(h.addr)"
                  >
                    <Copy class="h-3 w-3" />
                  </button>
                </span>
                <span v-else class="text-ink-faint">*</span>
              </td>
              <td class="px-4 py-2.5 text-right font-mono" :class="formatLossClass(h.lossPercent)">
                {{ h.lossPercent.toFixed(1) }}
              </td>
              <td class="px-4 py-2.5 text-right font-mono text-ink-soft">{{ h.sent }}</td>
              <td class="px-4 py-2.5 text-right font-mono text-ink-soft">
                {{ formatLatency(h.lastMs) }}
              </td>
              <td class="px-4 py-2.5 text-right font-mono text-ink-soft">
                {{ h.received > 0 ? formatLatency(h.avgMs) : "---" }}
              </td>
              <td class="px-4 py-2.5 text-right font-mono text-bamboo">
                {{ h.received > 0 ? formatLatency(h.minMs) : "---" }}
              </td>
              <td class="px-4 py-2.5 text-right font-mono text-red-500">
                {{ h.received > 0 ? formatLatency(h.maxMs) : "---" }}
              </td>
              <td class="px-4 py-2.5 text-right font-mono text-ink-soft">
                {{ h.received > 0 ? formatLatency(h.jitterMs) : "---" }}
              </td>
              <td class="px-4 py-2.5">
                <button
                  class="rounded-lg p-1 text-ink-faint transition-colors hover:text-bamboo hover:bg-bamboo/5"
                  title="复制地址"
                  :disabled="!h.addr"
                  @click="copyAddr(h.addr)"
                >
                  <Copy class="h-3.5 w-3.5" />
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <!-- Report view -->
      <div v-else class="p-5 space-y-4">
        <div class="grid grid-cols-2 sm:grid-cols-4 gap-4 pb-4 border-b border-paper-deep/30">
          <div>
            <p class="text-xs text-ink-faint">目标</p>
            <p class="mt-0.5 text-sm font-mono font-semibold text-ink">{{ store.target }}</p>
          </div>
          <div>
            <p class="text-xs text-ink-faint">总轮次</p>
            <p class="mt-0.5 text-sm font-mono font-semibold text-ink">{{ store.totalRounds }}</p>
          </div>
          <div>
            <p class="text-xs text-ink-faint">总丢包率</p>
            <p class="mt-0.5 text-sm font-mono font-semibold" :class="totalLossRate > 5 ? 'text-red-500' : 'text-bamboo'">
              {{ totalLossRate.toFixed(1) }}%
            </p>
          </div>
          <div>
            <p class="text-xs text-ink-faint">平均延迟 (所有跳)</p>
            <p class="mt-0.5 text-sm font-mono font-semibold text-ink">{{ avgLatency.toFixed(1) }} ms</p>
          </div>
        </div>

        <table class="w-full text-sm">
          <thead>
            <tr class="border-b border-paper-deep/30 text-xs text-ink-faint uppercase tracking-wider">
              <th class="px-3 py-2 text-left font-medium">#</th>
              <th class="px-3 py-2 text-left font-medium">地址</th>
              <th class="px-3 py-2 text-right font-medium">Loss%</th>
              <th class="px-3 py-2 text-right font-medium">Snt</th>
              <th class="px-3 py-2 text-right font-medium">平均</th>
              <th class="px-3 py-2 text-right font-medium">最佳</th>
              <th class="px-3 py-2 text-right font-medium">最差</th>
              <th class="px-3 py-2 text-right font-medium">抖动</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="h in store.hops"
              :key="h.hop"
              class="border-b border-paper-deep/20 last:border-0"
              :class="h.lossPercent > 0 ? 'bg-red-50/30 dark:bg-red-900/5' : ''"
            >
              <td class="px-3 py-2 text-ink-soft font-mono">{{ h.hop }}</td>
              <td class="px-3 py-2 text-ink font-mono text-xs">{{ formatAddr(h.addr, h.hostname) }}</td>
              <td class="px-3 py-2 text-right font-mono" :class="formatLossClass(h.lossPercent)">
                {{ h.lossPercent.toFixed(1) }}
              </td>
              <td class="px-3 py-2 text-right font-mono text-ink-soft">{{ h.sent }}</td>
              <td class="px-3 py-2 text-right font-mono text-ink-soft">
                {{ h.received > 0 ? formatLatency(h.avgMs) : "---" }}
              </td>
              <td class="px-3 py-2 text-right font-mono text-bamboo">
                {{ h.received > 0 ? formatLatency(h.minMs) : "---" }}
              </td>
              <td class="px-3 py-2 text-right font-mono text-red-500">
                {{ h.received > 0 ? formatLatency(h.maxMs) : "---" }}
              </td>
              <td class="px-3 py-2 text-right font-mono text-ink-soft">
                {{ h.received > 0 ? formatLatency(h.jitterMs) : "---" }}
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- Empty state guide -->
    <div
      v-else-if="!store.isRunning && !store.error"
      class="flex items-center justify-center rounded-xl border border-dashed border-paper-deep/30 bg-paper-warm/20 py-16 text-sm text-ink-faint"
    >
      <div class="text-center max-w-sm">
        <Route class="mx-auto h-10 w-10 mb-3 opacity-30" />
        <p class="font-medium text-ink-soft">输入目标 IP 或域名开始 MTR 路由追踪</p>
        <p class="mt-2 text-xs opacity-60 leading-relaxed">
          先探测路由路径，再持续对每跳进行 Ping 探测
          <br />
          实时监控每跳的延迟、丢包和抖动指标
        </p>
      </div>
    </div>
  </div>
</template>
