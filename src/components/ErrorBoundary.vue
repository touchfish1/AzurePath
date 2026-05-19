<script setup lang="ts">
import { ref, onErrorCaptured } from "vue";
import { AlertTriangle, RefreshCw } from "lucide-vue-next";

withDefaults(
  defineProps<{
    fallbackTitle?: string;
    fallbackMessage?: string;
  }>(),
  {
    fallbackTitle: "出现错误",
    fallbackMessage: "页面渲染出错，请重试",
  },
);

const emit = defineEmits<{ reset: [] }>();

const error = ref<Error | null>(null);

onErrorCaptured((err) => {
  error.value = err as Error;
  return false; // prevent propagation
});

function handleRetry() {
  error.value = null;
  emit("reset");
}
</script>

<template>
  <slot v-if="!error" />
  <div v-else class="flex h-full items-center justify-center p-8">
    <div class="flex max-w-md flex-col items-center gap-4 text-center">
      <div class="rounded-full bg-red-500/10 p-4">
        <AlertTriangle class="h-8 w-8 text-red-500" />
      </div>
      <h2 class="text-lg font-semibold text-ink">{{ fallbackTitle }}</h2>
      <p class="text-sm text-ink-soft">{{ fallbackMessage }}</p>
      <pre
        class="max-h-32 w-full overflow-auto rounded-lg bg-paper-deep/30 p-3 text-left text-xs text-ink-faint"
      >{{ error.message }}</pre>
      <button
        class="inline-flex items-center gap-2 rounded-lg bg-bamboo px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-bamboo/90"
        @click="handleRetry"
      >
        <RefreshCw class="h-4 w-4" />
        重试
      </button>
    </div>
  </div>
</template>
