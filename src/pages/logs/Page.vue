<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from "vue";
import { getLogs, clearLogs, type LogEntry } from "@/lib/tauri";
import { ScrollText, Trash2, Search, RotateCw } from "lucide-vue-next";
import { useDebounceFn } from "@vueuse/core";

const logs = ref<LogEntry[]>([]);
const autoRefresh = ref(true);
const levelFilter = ref("All");
const searchQuery = ref("");
const debouncedSearchQuery = ref("");
const searchDebounced = useDebounceFn((v: string) => { debouncedSearchQuery.value = v; }, 250);
watch(searchQuery, (v) => searchDebounced(v));
let intervalId: ReturnType<typeof setInterval> | null = null;

const levels = ["All", "ERROR", "WARN", "INFO", "DEBUG", "TRACE"];

const levelColors: Record<string, string> = {
  ERROR: "text-red-500",
  WARN: "text-yellow-500",
  INFO: "text-blue-500",
  DEBUG: "text-gray-400",
  TRACE: "text-gray-400",
};

const levelBgColors: Record<string, string> = {
  ERROR: "bg-red-500/10",
  WARN: "bg-yellow-500/10",
  INFO: "bg-blue-500/10",
  DEBUG: "bg-gray-500/10",
  TRACE: "bg-gray-500/10",
};

const filteredLogs = computed(() => {
  let result = logs.value;
  if (levelFilter.value !== "All") {
    result = result.filter((log) => log.level === levelFilter.value);
  }
  if (debouncedSearchQuery.value.trim()) {
    const q = debouncedSearchQuery.value.toLowerCase();
    result = result.filter(
      (log) =>
        log.message.toLowerCase().includes(q) ||
        log.target.toLowerCase().includes(q),
    );
  }
  return result.slice(0, 500);
});

async function fetchLogs() {
  try {
    logs.value = await getLogs(500);
  } catch (e) {
    console.error("Failed to fetch logs:", e);
  }
}

async function handleClear() {
  try {
    await clearLogs();
    logs.value = [];
  } catch (e) {
    console.error("Failed to clear logs:", e);
  }
}

function toggleAutoRefresh() {
  autoRefresh.value = !autoRefresh.value;
}

onMounted(() => {
  fetchLogs();
  intervalId = setInterval(() => {
    if (autoRefresh.value) {
      fetchLogs();
    }
  }, 2000);
});

onUnmounted(() => {
  if (intervalId) {
    clearInterval(intervalId);
  }
});
</script>

<template>
  <div class="flex h-full flex-col p-6">
    <!-- Header -->
    <div class="mb-4 flex items-center justify-between">
      <div class="flex items-center gap-3">
        <ScrollText class="h-5 w-5 text-ink-soft" />
        <h1 class="text-lg font-semibold text-ink">应用日志</h1>
      </div>
      <div class="flex items-center gap-2">
        <!-- Search -->
        <div class="relative">
          <Search class="absolute left-2.5 top-1/2 h-4 w-4 -translate-y-1/2 text-ink-faint" />
          <input
            v-model="searchQuery"
            type="text"
            placeholder="搜索日志..."
            class="h-8 w-48 rounded-lg border border-paper-deep bg-paper pl-8 pr-3 text-xs text-ink placeholder:text-ink-faint focus:outline-none focus:ring-1 focus:ring-bamboo/40"
          />
        </div>
        <!-- Level filter -->
        <select
          v-model="levelFilter"
          class="h-8 rounded-lg border border-paper-deep bg-paper px-2 text-xs text-ink focus:outline-none focus:ring-1 focus:ring-bamboo/40"
        >
          <option v-for="lvl in levels" :key="lvl" :value="lvl">{{ lvl }}</option>
        </select>
        <!-- Auto-refresh toggle -->
        <button
          class="inline-flex h-8 items-center gap-1.5 rounded-lg border border-paper-deep px-3 text-xs transition-colors"
          :class="autoRefresh ? 'bg-bamboo/10 text-bamboo' : 'bg-paper text-ink-soft hover:bg-paper-deep'"
          @click="toggleAutoRefresh"
        >
          <RotateCw class="h-3.5 w-3.5" :class="{ 'animate-spin': autoRefresh }" />
          {{ autoRefresh ? "自动刷新" : "已暂停" }}
        </button>
        <!-- Clear button -->
        <button
          class="inline-flex h-8 items-center gap-1.5 rounded-lg border border-paper-deep bg-paper px-3 text-xs text-ink-soft transition-colors hover:bg-danger-bg hover:text-red-600"
          @click="handleClear"
        >
          <Trash2 class="h-3.5 w-3.5" />
          清空
        </button>
      </div>
    </div>

    <!-- Log table -->
    <div class="flex-1 overflow-hidden rounded-xl border border-paper-deep bg-paper">
      <!-- Empty state -->
      <div
        v-if="filteredLogs.length === 0"
        class="flex h-full flex-col items-center justify-center gap-3 text-center"
      >
        <ScrollText class="h-10 w-10 text-ink-faint" />
        <p class="text-sm text-ink-soft">
          {{ logs.length === 0 ? "暂无日志记录" : "没有匹配的日志" }}
        </p>
      </div>

      <!-- Table -->
      <div v-else class="h-full overflow-auto">
        <table class="w-full text-left text-xs">
          <thead class="sticky top-0 bg-paper-warm">
            <tr class="border-b border-paper-deep">
              <th class="px-3 py-2 font-medium text-ink-soft">时间</th>
              <th class="px-3 py-2 font-medium text-ink-soft">级别</th>
              <th class="px-3 py-2 font-medium text-ink-soft">目标</th>
              <th class="px-3 py-2 font-medium text-ink-soft">消息</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-paper-deep/50">
            <tr
              v-for="(log, i) in filteredLogs"
              :key="i"
              class="transition-colors hover:bg-paper-warm/50"
            >
              <td class="whitespace-nowrap px-3 py-1.5 text-ink-faint">
                {{ log.timestamp }}
              </td>
              <td class="px-3 py-1.5">
                <span
                  class="inline-flex rounded-md px-1.5 py-0.5 text-xs font-medium"
                  :class="[levelColors[log.level] || 'text-ink-soft', levelBgColors[log.level] || 'bg-paper-deep/30']"
                >
                  {{ log.level }}
                </span>
              </td>
              <td class="max-w-[200px] truncate px-3 py-1.5 text-ink-soft">
                {{ log.target }}
              </td>
              <td class="px-3 py-1.5 text-ink">{{ log.message }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
</template>
