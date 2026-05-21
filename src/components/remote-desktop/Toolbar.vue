<script setup lang="ts">
import {
  ZoomIn,
  Maximize2,
  Minimize2,
  ClipboardCopy,
  Unplug,
} from "lucide-vue-next";

interface Props {
  zoom: number;
  isFullscreen: boolean;
  isConnected: boolean;
}

defineProps<Props>();

const emit = defineEmits<{
  "update:zoom": [level: number];
  "toggle-fullscreen": [];
  "copy-clipboard": [];
  disconnect: [];
}>();

const zoomOptions = [
  { value: 25, label: "25%" },
  { value: 50, label: "50%" },
  { value: 75, label: "75%" },
  { value: 100, label: "100%" },
  { value: -1, label: "适应" },
];
</script>

<template>
  <div class="flex shrink-0 items-center gap-2 border-b border-paper-deep/60 bg-paper px-4 py-2">
    <!-- Zoom controls -->
    <div class="flex items-center gap-1">
      <ZoomIn class="h-3.5 w-3.5 text-ink-faint" />
      <select
        :value="zoom"
        class="rounded-lg border border-paper-deep/60 bg-paper-warm/50 px-2 py-1 text-xs text-ink outline-none transition-colors focus:border-bamboo/50"
        @change="emit('update:zoom', Number(($event.target as HTMLSelectElement).value))"
      >
        <option v-for="opt in zoomOptions" :key="opt.value" :value="opt.value">
          {{ opt.label }}
        </option>
      </select>
    </div>

    <div class="h-4 w-px bg-paper-deep/60" />

    <!-- Fullscreen toggle -->
    <button
      class="rounded-lg p-1.5 text-ink-faint transition-colors hover:bg-paper-deep/50 hover:text-ink"
      :class="{ 'text-bamboo bg-bamboo/10': isFullscreen }"
      :title="isFullscreen ? '退出全屏' : '全屏'"
      @click="emit('toggle-fullscreen')"
    >
      <Maximize2 v-if="!isFullscreen" class="h-4 w-4" />
      <Minimize2 v-else class="h-4 w-4" />
    </button>

    <!-- Clipboard sync -->
    <button
      class="rounded-lg p-1.5 text-ink-faint transition-colors hover:bg-paper-deep/50 hover:text-ink"
      title="同步剪贴板"
      @click="emit('copy-clipboard')"
    >
      <ClipboardCopy class="h-4 w-4" />
    </button>

    <div class="flex-1" />

    <!-- Disconnect -->
    <button
      v-if="isConnected"
      class="flex items-center gap-1.5 rounded-lg bg-red-50 px-3 py-1.5 text-xs font-medium text-red-600 transition-colors hover:bg-red-100 dark:bg-red-900/20 dark:text-red-400 dark:hover:bg-red-900/30"
      @click="emit('disconnect')"
    >
      <Unplug class="h-3.5 w-3.5" />
      断开
    </button>
  </div>
</template>
