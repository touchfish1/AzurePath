<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import { Play, Square } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import { usePortScanStore } from "@/stores/portScan";

const store = usePortScanStore();

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
              <th class="px-5 py-3 text-left font-medium">端口</th>
              <th class="px-5 py-3 text-left font-medium">服务</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="fp in store.foundPorts"
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
      v-if="!store.running && store.completeInfo && store.foundPorts.length === 0"
      class="rounded-xl border border-paper-deep/60 bg-paper-warm/50 px-5 py-4 text-center text-sm text-ink-faint animate-fade-in"
    >
      未发现开放端口
    </div>
  </div>
</template>
