<script setup lang="ts">
import { onMounted, onUnmounted, ref, computed } from "vue";
import { Play, Square, Scan, Copy, ArrowUp, ArrowDown } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import PresetDropdown from "@/components/preset/PresetDropdown.vue";
import { usePortScanStore } from "@/stores/portScan";
import { usePresetStore } from "@/stores/preset";
import { useToastStore } from "@/stores/toast";
import type { Preset } from "@/lib/tauri";

const toast = useToastStore();
const presetStore = usePresetStore();

function copyPort(port: number) {
  navigator.clipboard.writeText(String(port)).then(() => {
    toast.add("success", "已复制");
  });
}

const store = usePortScanStore();

function loadPreset(preset: Preset) {
  const params = preset.params as Record<string, unknown>;
  if (params.target) store.target = String(params.target);
  if (params.portStart) store.portStart = Number(params.portStart);
  if (params.portEnd) store.portEnd = Number(params.portEnd);
  if (params.concurrency) store.concurrency = Number(params.concurrency);
  if (params.timeout) store.timeout = Number(params.timeout);
}

function savePreset(name: string) {
  const params = {
    target: store.target,
    portStart: store.portStart,
    portEnd: store.portEnd,
    concurrency: store.concurrency,
    timeout: store.timeout,
  };
  presetStore.save(name, "port_scan", params);
}

// Sorting state
const sortKey = ref<string | null>(null);
const sortDir = ref<"asc" | "desc" | null>(null);

function toggleSort(key: string) {
  if (sortKey.value === key) {
    if (sortDir.value === "asc") {
      sortDir.value = "desc";
    } else if (sortDir.value === "desc") {
      sortKey.value = null;
      sortDir.value = null;
    }
  } else {
    sortKey.value = key;
    sortDir.value = "asc";
  }
}

const sortedPorts = computed(() => {
  const items = [...store.foundPorts];
  if (!sortKey.value || !sortDir.value) return items;
  return items.sort((a: any, b: any) => {
    const valA = a[sortKey.value as keyof typeof a];
    const valB = b[sortKey.value as keyof typeof b];
    if (valA == null) return 1;
    if (valB == null) return -1;
    const cmp = typeof valA === "number" ? valA - valB : String(valA).localeCompare(String(valB));
    return sortDir.value === "asc" ? cmp : -cmp;
  });
});

function sortIndicator(key: string): string {
  if (sortKey.value !== key) return "";
  return sortDir.value === "asc" ? "asc" : "desc";
}

onMounted(async () => {
  if (store.currentTaskId) {
    // Component re-mounted while a scan appears to be running.  The scan may
    // have already completed while we were unmounted (and thus unsubscribed
    // from events), so the "port:complete" payload would have been lost.
    // Clean up: cancel any still-running background task and reset state.
    await store.stop();
    store.reset();
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
      <h1 class="text-2xl font-display font-bold text-ink">端口扫描</h1>
      <p class="mt-0.5 text-sm text-ink-faint">扫描目标主机的开放端口</p>
    </div>

    <!-- Input card -->
    <div class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm">
      <div class="flex flex-wrap items-end gap-3">
        <div class="flex-1 min-w-[160px]">
          <label class="mb-1 block text-xs font-medium text-ink-soft">目标地址</label>
          <input
            v-model="store.target"
            placeholder="IP 地址或域名"
            :disabled="store.running"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink placeholder:text-ink-faint/50 outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20 disabled:opacity-50"
          />
        </div>
        <div class="w-20">
          <label class="mb-1 block text-xs font-medium text-ink-soft">起始端口</label>
          <input
            v-model.number="store.portStart"
            type="number"
            min="1"
            max="65535"
            :disabled="store.running"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20 disabled:opacity-50"
          />
        </div>
        <div class="w-20">
          <label class="mb-1 block text-xs font-medium text-ink-soft">结束端口</label>
          <input
            v-model.number="store.portEnd"
            type="number"
            min="1"
            max="65535"
            :disabled="store.running"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20 disabled:opacity-50"
          />
        </div>
        <div class="flex gap-2">
          <Button :disabled="store.running" @click="store.start">
            <Play class="mr-1.5 h-3.5 w-3.5" />
            开始
          </Button>
          <Button variant="danger" :disabled="!store.running" @click="store.stop">
            <Square class="mr-1.5 h-3.5 w-3.5" />
            停止
          </Button>
        </div>
      </div>
      <!-- Presets -->
      <div class="mt-3 border-t border-paper-deep/30 pt-3">
        <PresetDropdown
          feature="port_scan"
          @load="loadPreset"
          @save-request="savePreset"
        />
      </div>
    </div>

    <!-- Error banner -->
    <div
      v-if="store.error"
      class="rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700 dark:border-red-800/30 dark:bg-red-900/10 dark:text-red-400"
    >
      {{ store.error }}
    </div>

    <!-- Progress bar -->
    <div
      v-if="store.progress"
      class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm"
    >
      <div class="flex items-center justify-between mb-3">
        <span class="text-sm font-medium text-ink">扫描进度</span>
        <span class="text-xs text-ink-faint font-mono">
          {{ store.progress.scanned }} / {{ store.progress.total }}
          ({{ store.progressPercent }}%)
        </span>
      </div>
      <div class="h-2 rounded-full bg-paper-deep overflow-hidden">
        <div
          class="h-full rounded-full bg-bamboo transition-all duration-300 ease-out"
          :style="{ width: store.progressPercent + '%' }"
        />
      </div>
      <p class="mt-2 text-xs text-ink-faint">
        已发现 <span class="font-semibold text-bamboo">{{ store.progress.open }}</span> 个开放端口
      </p>
    </div>

    <!-- Found ports -->
    <div
      v-if="store.foundPorts.length > 0"
      class="noise-bg rounded-xl border border-paper-deep/60 bg-paper shadow-sm overflow-hidden"
    >
      <div class="px-5 py-3 border-b border-paper-deep/50">
        <h2 class="text-sm font-semibold text-ink">
          开放端口 ({{ store.foundPorts.length }})
        </h2>
      </div>
      <div class="overflow-x-auto">
        <table class="w-full text-sm">
          <thead>
            <tr class="border-b border-paper-deep/30 text-xs text-ink-faint uppercase tracking-wider">
              <th
                class="px-5 py-3 text-left font-medium cursor-pointer select-none hover:text-ink"
                @click="toggleSort('port')"
              >
                <span class="inline-flex items-center gap-1">
                  端口
                  <ArrowUp v-if="sortIndicator('port') === 'asc'" class="h-3 w-3" />
                  <ArrowDown v-else-if="sortIndicator('port') === 'desc'" class="h-3 w-3" />
                </span>
              </th>
              <th
                class="px-5 py-3 text-left font-medium cursor-pointer select-none hover:text-ink"
                @click="toggleSort('service')"
              >
                <span class="inline-flex items-center gap-1">
                  服务
                  <ArrowUp v-if="sortIndicator('service') === 'asc'" class="h-3 w-3" />
                  <ArrowDown v-else-if="sortIndicator('service') === 'desc'" class="h-3 w-3" />
                </span>
              </th>
              <th class="px-5 py-3 text-left font-medium w-14">操作</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="fp in sortedPorts"
              :key="fp.port"
              class="border-b border-paper-deep/20 last:border-0 animate-slide-up"
            >
              <td class="px-5 py-2.5 font-mono text-ink">{{ fp.port }}</td>
              <td class="px-5 py-2.5 text-ink-soft">
                {{ fp.service || "未知" }}
              </td>
              <td class="px-5 py-2.5">
                <button
                  class="rounded-lg p-1.5 text-ink-faint transition-colors hover:text-bamboo hover:bg-bamboo/5"
                  title="复制端口"
                  @click="copyPort(fp.port)"
                >
                  <Copy class="h-3.5 w-3.5" />
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- No ports found / Empty state -->
    <div
      v-if="store.foundPorts.length === 0 && !store.running"
      class="flex items-center justify-center rounded-xl border border-dashed border-paper-deep/30 bg-paper-warm/20 py-16 text-sm text-ink-faint"
    >
      <div v-if="store.completeInfo" class="text-center">
        <Scan class="mx-auto h-8 w-8 mb-2 opacity-40" />
        <p>未发现开放端口</p>
        <p class="mt-1 text-xs opacity-60">目标主机可能未运行任何服务或防火墙已过滤端口</p>
      </div>
      <div v-else class="text-center max-w-sm">
        <Scan class="mx-auto h-10 w-10 mb-3 opacity-30" />
        <p class="font-medium text-ink-soft">输入目标 IP 或域名开始端口扫描</p>
        <p class="mt-2 text-xs opacity-60 leading-relaxed">
          支持自定义端口范围，默认扫描常用端口
          <br />
          扫描结果将显示所有开放端口及对应服务
        </p>
      </div>
    </div>
  </div>
</template>
