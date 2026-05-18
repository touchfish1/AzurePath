<script setup lang="ts">
import { ref, onMounted } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Square, X, Sun, Moon } from "lucide-vue-next";
import { useThemeStore } from "@/stores/theme";

const appWindow = getCurrentWindow();
const store = useThemeStore();
const isMaximized = ref(false);

onMounted(async () => {
  isMaximized.value = await appWindow.isMaximized();
});

async function minimize() {
  await appWindow.minimize();
}

async function toggleMaximize() {
  await appWindow.toggleMaximize();
  isMaximized.value = await appWindow.isMaximized();
}

async function closeWindow() {
  await appWindow.close();
}

function handleThemeToggle() {
  store.toggleTheme();
}
</script>

<template>
  <header
    class="relative z-50 flex h-10 shrink-0 items-center justify-between border-b border-paper-deep bg-paper select-none"
    data-tauri-drag-region
  >
    <!-- App title / drag region -->
    <div class="flex items-center gap-2 pl-4" data-tauri-drag-region>
      <span
        class="text-xs font-display font-bold tracking-wide text-ink-faint uppercase"
        data-tauri-drag-region
      >
        AzurePath
      </span>
    </div>

    <!-- Right side controls -->
    <div class="flex h-full items-center">
      <!-- Theme toggle -->
      <button
        class="flex h-full items-center justify-center px-3 text-ink-faint hover:text-ink hover:bg-paper-warm transition-colors"
        @click="handleThemeToggle"
        :title="store.resolved === 'dark' ? '切换到亮色模式' : '切换到暗色模式'"
      >
        <Sun v-if="store.resolved === 'dark'" class="h-3.5 w-3.5" />
        <Moon v-else class="h-3.5 w-3.5" />
      </button>

      <!-- Minimize -->
      <button
        class="flex h-full items-center justify-center px-3 text-ink-faint hover:text-ink hover:bg-paper-warm transition-colors"
        @click="minimize"
        title="最小化"
      >
        <Minus class="h-3.5 w-3.5" />
      </button>

      <!-- Maximize / Restore -->
      <button
        class="flex h-full items-center justify-center px-3 text-ink-faint hover:text-ink hover:bg-paper-warm transition-colors"
        @click="toggleMaximize"
        :title="isMaximized ? '还原' : '最大化'"
      >
        <Square class="h-3 w-3" />
      </button>

      <!-- Close -->
      <button
        class="flex h-full items-center justify-center px-3 text-ink-faint hover:text-cloud hover:bg-red-500 transition-colors"
        @click="closeWindow"
        title="关闭"
      >
        <X class="h-3.5 w-3.5" />
      </button>
    </div>
  </header>
</template>
