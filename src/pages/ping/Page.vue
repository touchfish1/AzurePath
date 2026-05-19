<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import { Play, Square, Radio } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import { usePingStore } from "@/stores/ping";

const store = usePingStore();

onMounted(async () => {
  // Re-attach listeners if a task is still running from a previous visit
  if (store.currentTaskId) {
    await store.attachListeners();
  }
});

onUnmounted(() => {
  // Only detach listeners — never stop a running task
  store.detachListeners();
});
</script>

<template>
  <div class="flex h-full flex-col p-6 space-y-6 animate-view-fade">
    <!-- Header -->
    <div>
      <h1 class="text-2xl font-display font-bold text-ink">Ping</h1>
      <p class="mt-0.5 text-sm text-ink-faint">测试网络连通性与延迟</p>
    </div>

    <!-- Input card -->
    <div class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm">
      <div class="flex flex-wrap items-end gap-3">
        <div class="flex-1 min-w-[180px]">
          <label class="mb-1 block text-xs font-medium text-ink-soft">目标地址</label>
          <input
            v-model="store.target"
            placeholder="IP 地址或域名"
            :disabled="store.running"
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
            :disabled="store.running"
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
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- Empty state guide -->
    <div
      v-else-if="!store.running && !store.error"
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
