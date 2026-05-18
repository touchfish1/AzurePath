<script setup lang="ts">
import { History } from "lucide-vue-next";
</script>

<template>
  <div class="flex h-full flex-col items-center justify-center p-8 animate-fade-up">
    <div class="flex h-16 w-16 items-center justify-center rounded-2xl bg-bamboo/10 text-bamboo">
      <History class="h-7 w-7" />
    </div>
    <h1 class="mt-5 text-xl font-display font-bold text-ink">历史记录</h1>
    <p class="mt-2 text-sm text-ink-faint text-center leading-relaxed max-w-xs">
      功能开发中，敬请期待。
    </p>
  </div>
</template>
