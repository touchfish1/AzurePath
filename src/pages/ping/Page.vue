<script setup lang="ts">
import { onMounted, onUnmounted, watch, ref, nextTick } from "vue";
import { Play, Square, Radio, Copy } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import PresetDropdown from "@/components/preset/PresetDropdown.vue";
import ReportButton from "@/components/ReportButton.vue";
import { usePingStore } from "@/stores/ping";
import type { PingResultItem } from "@/stores/ping";
import { usePresetStore } from "@/stores/preset";
import { useToastStore } from "@/stores/toast";
import { useTargetGroupStore } from "@/stores/targetGroup";
import TargetGroupPicker from "@/components/target-group/TargetGroupPicker.vue";
import type { Preset, TargetGroup } from "@/lib/tauri";

// ─── localStorage ping history ───────────────────────────────────
const PING_HISTORY_KEY = "ping_history";

interface PingHistoryEntry {
  target: string;
  latency: number;
  timestamp: string;
}

function savePingResults(target: string, results: { latencyMs: number | null }[]) {
  try {
    const raw = localStorage.getItem(PING_HISTORY_KEY);
    const history: PingHistoryEntry[] = raw ? JSON.parse(raw) : [];
    const now = new Date().toISOString();
    for (const r of results) {
      if (r.latencyMs !== null) {
        history.push({ target, latency: r.latencyMs, timestamp: now });
      }
    }
    // Keep only last 500 entries total
    while (history.length > 500) {
      history.shift();
    }
    localStorage.setItem(PING_HISTORY_KEY, JSON.stringify(history));
  } catch {
    // Ignore localStorage errors
  }
}

const toast = useToastStore();
const presetStore = usePresetStore();

function copyIp(target: string) {
  navigator.clipboard.writeText(target).then(() => {
    toast.add("success", "已复制");
  });
}

const store = usePingStore();

// ─── Target group support ─────────────────────────────────────────
const targetGroupStore = useTargetGroupStore();
const selectedGroupId = ref<string | null>(null);
const selectedGroup = ref<TargetGroup | null>(null);
const checkedTargets = ref<Set<string>>(new Set());
const batchRunning = ref(false);
const currentBatchIdx = ref(-1);
const batchAllResults = ref<PingResultItem[]>([]);

watch(selectedGroupId, async (id) => {
  if (id) {
    await targetGroupStore.loadGroups();
    selectedGroup.value = targetGroupStore.groups.find((g) => g.id === id) || null;
    if (selectedGroup.value) {
      checkedTargets.value = new Set(selectedGroup.value.targets);
    }
  } else {
    selectedGroup.value = null;
    checkedTargets.value = new Set();
  }
});

function toggleTarget(target: string) {
  const next = new Set(checkedTargets.value);
  if (next.has(target)) {
    next.delete(target);
  } else {
    next.add(target);
  }
  checkedTargets.value = next;
}

function waitForPingIdle(): Promise<void> {
  return new Promise((resolve) => {
    if (!store.running) {
      resolve();
      return;
    }
    const stop = watch(
      () => store.running,
      (val) => {
        if (!val) {
          stop();
          resolve();
        }
      },
    );
  });
}

async function startBatch() {
  const targets = Array.from(checkedTargets.value);
  if (targets.length === 0) return;

  batchRunning.value = true;
  batchAllResults.value = [];
  store.reset();

  for (let i = 0; i < targets.length; i++) {
    if (!batchRunning.value) break;
    currentBatchIdx.value = i;
    store.target = targets[i];

    // Start ping for this target
    await store.start();
    await nextTick();
    await waitForPingIdle();

    // Accumulate results
    batchAllResults.value.push(...store.results);
  }

  // Show accumulated results
  store.results = batchAllResults.value;
  batchRunning.value = false;
  currentBatchIdx.value = -1;
}

function handleStart() {
  if (selectedGroup.value && checkedTargets.value.size > 0) {
    startBatch();
  } else {
    store.start();
  }
}

function handleStop() {
  batchRunning.value = false;
  store.stop();
}

function loadPreset(preset: Preset) {
  const params = preset.params as Record<string, unknown>;
  if (params.target) store.target = String(params.target);
  if (params.count) store.count = Number(params.count);
  if (params.timeout) store.timeout = Number(params.timeout);
}

function savePreset(name: string) {
  const params = {
    target: store.target,
    count: store.count,
    timeout: store.timeout,
  };
  presetStore.save(name, "ping", params);
}

onMounted(async () => {
  // Re-attach listeners if a task is still running from a previous visit
  if (store.currentTaskId) {
    await store.attachListeners();
  }
  await targetGroupStore.loadGroups();
});

onUnmounted(() => {
  // Only detach listeners — never stop a running task
  store.detachListeners();
});

// Watch for batch completion and save to localStorage
watch(
  () => store.stats,
  (stats) => {
    if (stats && store.results.length > 0 && !batchRunning.value) {
      savePingResults(store.target, store.results);
    }
  },
);
</script>

<template>
  <div class="flex h-full flex-col p-4 md:p-6 space-y-4 md:space-y-6 animate-view-fade">
    <!-- Header -->
    <div class="flex items-start justify-between">
      <div>
        <h1 class="text-2xl font-display font-bold text-ink">Ping</h1>
        <p class="mt-0.5 text-sm text-ink-faint">测试网络连通性与延迟</p>
      </div>
      <ReportButton
        v-if="store.results.length > 0"
        title="Ping 测试结果"
        :columns="[
          { key: 'seq', label: '序号' },
          { key: 'ttl', label: 'TTL' },
          { key: 'latencyMs', label: '延迟 (ms)' },
          { key: 'status', label: '状态' },
        ]"
        :rows="store.results.map(r => ({
          seq: r.seq,
          ttl: r.ttl,
          latencyMs: r.latencyMs !== null ? r.latencyMs.toFixed(1) : '---',
          status: r.status === 'success' ? '成功' : r.status,
        }))"
      />
    </div>

    <!-- Input card -->
    <div class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm">
      <!-- Target group picker -->
      <div class="mb-3">
        <TargetGroupPicker v-model="selectedGroupId" />
      </div>

      <div class="flex flex-wrap items-end gap-3">
        <div class="flex-1 min-w-[180px]">
          <label class="mb-1 block text-xs font-medium text-ink-soft">目标地址</label>
          <input
            v-model="store.target"
            placeholder="IP 地址或域名"
            :disabled="store.running || batchRunning"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink placeholder:text-ink-faint/50 outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20 disabled:opacity-50"
          />
        </div>
        <div class="w-24">
          <label class="mb-1 block text-xs font-medium text-ink-soft">次数</label>
          <input
            v-model.number="store.count"
            type="number"
            min="1"
            max="100"
            :disabled="store.running || batchRunning"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20 disabled:opacity-50"
          />
        </div>
        <div class="w-24">
          <label class="mb-1 block text-xs font-medium text-ink-soft">
            <span class="hidden sm:inline">超时 (ms)</span>
            <span class="sm:hidden">超时</span>
          </label>
          <input
            v-model.number="store.timeout"
            type="number"
            min="100"
            max="30000"
            step="100"
            :disabled="store.running || batchRunning"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20 disabled:opacity-50"
          />
        </div>
        <div class="flex gap-2">
          <Button :disabled="store.running || batchRunning" @click="handleStart">
            <Play class="mr-1.5 h-3.5 w-3.5" />
            <template v-if="batchRunning">
              批量中 ({{ currentBatchIdx + 1 }}/{{ checkedTargets.size }})
            </template>
            <template v-else-if="selectedGroup && checkedTargets.size > 0">
              开始批量 ({{ checkedTargets.size }}个)
            </template>
            <template v-else>
              开始
            </template>
          </Button>
          <Button variant="danger" :disabled="!store.running && !batchRunning" @click="handleStop">
            <Square class="mr-1.5 h-3.5 w-3.5" />
            停止
          </Button>
        </div>
      </div>

      <!-- Batch target checkboxes -->
      <div
        v-if="selectedGroup && selectedGroup.targets.length > 0"
        class="mt-3 border-t border-paper-deep/30 pt-3"
      >
        <div class="flex items-center justify-between mb-2">
          <span class="text-xs font-medium text-ink-soft">
            分组目标 ({{ checkedTargets.size }}/{{ selectedGroup.targets.length }})
          </span>
          <div class="flex gap-2">
            <button
              class="text-xs text-bamboo hover:text-bamboo-light transition-colors"
              @click="checkedTargets = new Set(selectedGroup.targets)"
            >
              全选
            </button>
            <button
              class="text-xs text-ink-faint hover:text-ink-soft transition-colors"
              @click="checkedTargets = new Set()"
            >
              取消
            </button>
          </div>
        </div>
        <div class="flex flex-wrap gap-1.5">
          <label
            v-for="target in selectedGroup.targets"
            :key="target"
            class="inline-flex cursor-pointer items-center gap-1.5 rounded-md border px-2.5 py-1 text-xs font-mono transition-colors"
            :class="
              checkedTargets.has(target)
                ? 'border-bamboo/40 bg-bamboo/5 text-bamboo'
                : 'border-paper-deep/40 text-ink-soft hover:border-paper-deep/70'
            "
          >
            <input
              type="checkbox"
              :checked="checkedTargets.has(target)"
              :disabled="batchRunning"
              class="sr-only"
              @change="toggleTarget(target)"
            />
            <span
              class="flex h-3 w-3 shrink-0 items-center justify-center rounded border transition-colors"
              :class="
                checkedTargets.has(target)
                  ? 'border-bamboo bg-bamboo text-white'
                  : 'border-paper-deep'
              "
            >
              <span v-if="checkedTargets.has(target)" class="text-[8px]">&#10003;</span>
            </span>
            {{ target }}
          </label>
        </div>
      </div>

      <!-- Presets -->
      <div class="mt-3 border-t border-paper-deep/30 pt-3">
        <PresetDropdown
          feature="ping"
          @load="loadPreset"
          @save-request="savePreset"
        />
      </div>
    </div>

    <!-- Batch progress -->
    <div
      v-if="batchRunning"
      class="rounded-xl border border-bamboo/20 bg-bamboo/5 px-4 py-3 text-sm text-bamboo"
    >
      批量测试中：{{ currentBatchIdx + 1 }} / {{ checkedTargets.size }}
      （当前：{{ Array.from(checkedTargets)[currentBatchIdx] || "---" }}）
    </div>

    <!-- Error banner -->
    <div
      v-if="store.error"
      class="rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700 dark:border-red-800/30 dark:bg-red-900/10 dark:text-red-400"
    >
      {{ store.error }}
    </div>

    <!-- Results table -->
    <div
      v-if="store.results.length > 0"
      class="noise-bg rounded-xl border border-paper-deep/60 bg-paper shadow-sm overflow-hidden"
    >
      <div class="px-5 py-3 border-b border-paper-deep/50">
        <h2 class="text-sm font-semibold text-ink">响应结果</h2>
      </div>
      <div class="overflow-x-auto">
        <table class="w-full text-sm">
          <thead>
            <tr class="border-b border-paper-deep/30 text-xs text-ink-faint uppercase tracking-wider">
              <th class="px-5 py-3 text-left font-medium">序号</th>
              <th class="px-5 py-3 text-left font-medium">TTL</th>
              <th class="px-5 py-3 text-right font-medium">延迟</th>
              <th class="px-5 py-3 text-left font-medium">状态</th>
              <th class="px-5 py-3 text-left font-medium w-14">操作</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="r in store.results"
              :key="r.seq"
              class="border-b border-paper-deep/20 last:border-0 animate-slide-up"
            >
              <td class="px-5 py-2.5 text-ink-soft">{{ r.seq }}</td>
              <td class="px-5 py-2.5 font-mono text-ink-soft">{{ r.ttl }}</td>
              <td
                class="px-5 py-2.5 text-right font-mono"
                :class="
                  r.latencyMs !== null
                    ? r.latencyMs < 100
                      ? 'text-bamboo'
                      : r.latencyMs < 300
                        ? 'text-yellow-600 dark:text-yellow-400'
                        : 'text-red-600 dark:text-red-400'
                    : 'text-ink-faint'
                "
              >
                <template v-if="r.latencyMs !== null">
                  {{ r.latencyMs.toFixed(1) }} ms
                </template>
                <template v-else>
                  ---
                </template>
              </td>
              <td class="px-5 py-2.5">
                <span
                  class="inline-block rounded-full px-2 py-0.5 text-xs font-medium"
                  :class="
                    r.status === 'success'
                      ? 'bg-bamboo/10 text-bamboo'
                      : 'bg-red-100 text-red-600 dark:bg-red-900/20 dark:text-red-400'
                  "
                >
                  {{ r.status === "success" ? "成功" : r.status }}
                </span>
              </td>
              <td class="px-5 py-2.5">
                <button
                  class="rounded-lg p-1.5 text-ink-faint transition-colors hover:text-bamboo hover:bg-bamboo/5"
                  title="复制目标 IP"
                  @click="copyIp(store.target)"
                >
                  <Copy class="h-3.5 w-3.5" />
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- Empty state guide -->
    <div
      v-else-if="!store.running && !batchRunning && !store.error"
      class="flex items-center justify-center rounded-xl border border-dashed border-paper-deep/30 bg-paper-warm/20 py-16 text-sm text-ink-faint"
    >
      <div class="text-center max-w-sm">
        <Radio class="mx-auto h-10 w-10 mb-3 opacity-30" />
        <p class="font-medium text-ink-soft">输入目标 IP 或域名开始 Ping 测试</p>
        <p class="mt-2 text-xs opacity-60 leading-relaxed">
          支持 IPv4 地址和域名
          <br />
          默认发送 4 个数据包，可调整发送次数和超时时间
        </p>
      </div>
    </div>

    <!-- Stats panel -->
    <div
      v-if="store.stats"
      class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm animate-scale-in"
    >
      <h2 class="text-sm font-semibold text-ink mb-4">统计</h2>
      <div class="grid grid-cols-2 gap-4 sm:grid-cols-4">
        <div>
          <p class="text-xs text-ink-faint">发送 / 接收</p>
          <p class="mt-0.5 text-lg font-mono font-semibold text-ink">
            {{ store.stats.sent }} / {{ store.stats.received }}
          </p>
        </div>
        <div>
          <p class="text-xs text-ink-faint">丢包率</p>
          <p
            class="mt-0.5 text-lg font-mono font-semibold"
            :class="store.stats.loss_percent > 0 ? 'text-red-600 dark:text-red-400' : 'text-bamboo'"
          >
            {{ store.stats.loss_percent.toFixed(1) }}%
          </p>
        </div>
        <div>
          <p class="text-xs text-ink-faint">最小 / 最大</p>
          <p class="mt-0.5 text-lg font-mono font-semibold text-ink">
            {{ store.stats.min_ms.toFixed(1) }} / {{ store.stats.max_ms.toFixed(1) }} ms
          </p>
        </div>
        <div>
          <p class="text-xs text-ink-faint">平均延迟</p>
          <p class="mt-0.5 text-lg font-mono font-semibold text-ink">
            {{ store.stats.avg_ms.toFixed(1) }} ms
          </p>
        </div>
      </div>
    </div>
  </div>
</template>
