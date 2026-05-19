<script setup lang="ts">
import { useThemeStore } from "@/stores/theme";

const themeStore = useThemeStore();
</script>

<template>
  <div class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm">
    <h2 class="text-sm font-semibold text-ink mb-4">夜间模式定时切换</h2>

    <!-- Toggle -->
    <div class="flex items-center justify-between mb-4">
      <div>
        <p class="text-sm text-ink">启用定时切换</p>
        <p class="text-xs text-ink-faint mt-0.5">
          在指定时间段自动切换到暗色模式
        </p>
      </div>
      <label class="relative inline-flex cursor-pointer items-center">
        <input
          type="checkbox"
          :checked="themeStore.nightModeEnabled"
          class="peer sr-only"
          @change="themeStore.setNightModeSchedule(!themeStore.nightModeEnabled)"
        />
        <div
          class="h-6 w-11 rounded-full border border-paper-deep/40 bg-paper-deep/30 transition-colors peer-checked:bg-bamboo peer-focus:ring-2 peer-focus:ring-bamboo/30"
        >
          <div
            class="h-5 w-5 translate-x-0.5 rounded-full bg-white shadow-sm transition-transform peer-checked:translate-x-[22px]"
          ></div>
        </div>
      </label>
    </div>

    <!-- Time inputs -->
    <div v-if="themeStore.nightModeEnabled" class="space-y-4">
      <div class="grid grid-cols-2 gap-4">
        <div>
          <label class="mb-1.5 block text-xs font-medium text-ink-soft">开始时间</label>
          <input
            type="time"
            :value="themeStore.nightModeStart"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
            @change="
              themeStore.setNightModeSchedule(
                true,
                ($event.target as HTMLInputElement).value,
                undefined,
              )
            "
          />
        </div>
        <div>
          <label class="mb-1.5 block text-xs font-medium text-ink-soft">结束时间</label>
          <input
            type="time"
            :value="themeStore.nightModeEnd"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
            @change="
              themeStore.setNightModeSchedule(
                true,
                undefined,
                ($event.target as HTMLInputElement).value,
              )
            "
          />
        </div>
      </div>

      <p class="text-xs text-ink-faint">
        将在
        <span class="font-mono text-ink-soft">{{ themeStore.nightModeStart }}</span>
        自动切换到暗色模式，并在
        <span class="font-mono text-ink-soft">{{ themeStore.nightModeEnd }}</span>
        恢复
      </p>
    </div>
  </div>
</template>
