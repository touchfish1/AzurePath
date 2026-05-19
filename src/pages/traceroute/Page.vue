<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import { Play, Square, Route, Copy } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import { useTracerouteStore } from "@/stores/traceroute";
import { useToastStore } from "@/stores/toast";

const toast = useToastStore();

const store = useTracerouteStore();

function formatLatency(ms: number | null): string {
  if (ms === null) return "*";
  return `${ms.toFixed(1)} ms`;
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
            v-model="store.target"
            placeholder="IP 地址或域名"
            :disabled="store.running"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink placeholder:text-ink-faint/50 outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20 disabled:opacity-50"
          />
        </div>
        <div class="w-24">
          <label class="mb-1 block text-xs font-medium text-ink-soft">最大跳数</label>
          <input
            v-model.number="store.maxHops"
            type="number"
            min="1"
            max="64"
            :disabled="store.running"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20 disabled:opacity-50"
          />
        </div>
        <div class="w-24">
          <label class="mb-1 block text-xs font-medium text-ink-soft">超时 (ms)</label>
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

    <!-- Results card -->
    <div
      v-if="store.hops.length > 0"
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
              <th class="px-5 py-3 text-left font-medium w-14">操作</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="h in store.hops"
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
              <td
                v-for="n in Math.max(0, 3 - h.latencies.length)"
                :key="'empty-' + n"
                class="px-5 py-2.5 text-right font-mono text-ink-faint"
              >
                *
              </td>
              <td class="px-5 py-2.5">
                <button
                  class="rounded-lg p-1.5 text-ink-faint transition-colors hover:text-bamboo hover:bg-bamboo/5"
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
    </div>

    <!-- Empty state guide -->
    <div
      v-else-if="!store.running && !store.error"
      class="flex items-center justify-center rounded-xl border border-dashed border-paper-deep/30 bg-paper-warm/20 py-16 text-sm text-ink-faint"
    >
      <div class="text-center max-w-sm">
        <Route class="mx-auto h-10 w-10 mb-3 opacity-30" />
        <p class="font-medium text-ink-soft">输入目标 IP 或域名开始路由追踪</p>
        <p class="mt-2 text-xs opacity-60 leading-relaxed">
          追踪数据包到达目标所经过的每一个路由节点
          <br />
          最大跳数默认为 30，可调整超时时间
        </p>
      </div>
    </div>

    <!-- Complete info -->
    <div
      v-if="store.completeInfo"
      class="rounded-xl border border-paper-deep/60 bg-paper-warm/50 px-5 py-3 text-sm text-ink-soft animate-fade-in"
    >
      追踪完成：目标 {{ store.completeInfo.target }}，经过 {{ store.completeInfo.hops.length }} 跳
    </div>
  </div>
</template>
