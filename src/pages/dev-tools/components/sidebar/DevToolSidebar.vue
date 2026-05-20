<script setup lang="ts">
import { useDevToolStore } from "@/pages/dev-tools/stores";

const store = useDevToolStore();
</script>

<template>
  <div class="flex w-44 shrink-0 flex-col border-r border-paper-deep/50 bg-paper-warm/20 p-2">
    <div class="mb-3 px-2 pt-2">
      <h2 class="text-xs font-semibold uppercase tracking-wider text-ink-faint">开发者工具箱</h2>
    </div>
    <nav class="flex flex-col gap-3">
      <div v-for="cat in store.categories" :key="cat.id">
        <p class="mb-1 px-3 text-[10px] font-semibold uppercase tracking-wider text-ink-faint/60">
          {{ cat.label }}
        </p>
        <div class="flex flex-col gap-0.5">
          <button
            v-for="tool in store.getToolsByCategory(cat.id)"
            :key="tool.id"
            class="rounded-lg px-3 py-1.5 text-left text-sm font-medium transition-colors"
            :class="
              store.selectedTool === tool.id
                ? 'bg-bamboo/10 text-bamboo'
                : 'text-ink-soft hover:bg-paper-deep/50 hover:text-ink'
            "
            @click="store.selectTool(tool.id)"
          >
            {{ tool.label }}
          </button>
        </div>
      </div>
    </nav>
  </div>
</template>
