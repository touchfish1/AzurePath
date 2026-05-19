<script setup lang="ts">
import { useToastStore } from "@/stores/toast";

const store = useToastStore();
</script>

<template>
  <Teleport to="body">
    <div
      class="fixed bottom-4 right-4 z-50 flex flex-col gap-2 pointer-events-none"
    >
      <TransitionGroup name="toast-fade">
        <div
          v-for="toast in store.toasts"
          :key="toast.id"
          class="pointer-events-auto flex items-center gap-3 rounded-xl px-4 py-3 text-sm font-medium shadow-lg transition-all duration-300"
          :class="{
            'bg-green-600 text-white': toast.type === 'success',
            'bg-red-600 text-white': toast.type === 'error',
            'bg-blue-600 text-white': toast.type === 'info',
          }"
          @click="store.remove(toast.id)"
        >
          <span
            class="shrink-0 h-2 w-2 rounded-full"
            :class="{
              'bg-green-200': toast.type === 'success',
              'bg-red-200': toast.type === 'error',
              'bg-blue-200': toast.type === 'info',
            }"
          />
          <span>{{ toast.message }}</span>
        </div>
      </TransitionGroup>
    </div>
  </Teleport>
</template>

<style scoped>
.toast-fade-enter-active {
  transition: all 0.3s ease-out;
}
.toast-fade-leave-active {
  transition: all 0.3s ease-in;
}
.toast-fade-enter-from {
  opacity: 0;
  transform: translateX(30px) scale(0.95);
}
.toast-fade-leave-to {
  opacity: 0;
  transform: translateX(30px) scale(0.95);
}
</style>
