<script setup lang="ts">
import { ref } from "vue";
import Button from "@/components/ui/button/Button.vue";

type ConvertMode = "ts-to-date" | "date-to-ts";

const mode = ref<ConvertMode>("ts-to-date");

// Timestamp to Date
const tsInput = ref<number | null>(null);
const localTime = ref("");
const utcTime = ref("");
const relativeTime = ref("");
const tsError = ref("");

// Date to Timestamp
const dateInput = ref("");
const tsOutput = ref<number | null>(null);
const dateError = ref("");

function now() {
  tsInput.value = Math.floor(Date.now() / 1000);
}

function convertTs() {
  tsError.value = "";
  localTime.value = "";
  utcTime.value = "";
  relativeTime.value = "";

  const val = tsInput.value;
  if (val === null || val === undefined || (typeof val === "number" && isNaN(val))) {
    tsError.value = "请输入有效的时间戳";
    return;
  }

  let ms: number;
  if (val > 1e11) {
    ms = val;
  } else {
    ms = val * 1000;
  }

  const date = new Date(ms);
  if (isNaN(date.getTime())) {
    tsError.value = "无效的时间戳";
    return;
  }

  // Local time
  const ly = date.getFullYear();
  const lM = String(date.getMonth() + 1).padStart(2, "0");
  const ld = String(date.getDate()).padStart(2, "0");
  const lh = String(date.getHours()).padStart(2, "0");
  const lm = String(date.getMinutes()).padStart(2, "0");
  const ls = String(date.getSeconds()).padStart(2, "0");
  localTime.value = `${ly}-${lM}-${ld} ${lh}:${lm}:${ls}`;

  // UTC time
  const uy = date.getUTCFullYear();
  const uM = String(date.getUTCMonth() + 1).padStart(2, "0");
  const ud = String(date.getUTCDate()).padStart(2, "0");
  const uh = String(date.getUTCHours()).padStart(2, "0");
  const um = String(date.getUTCMinutes()).padStart(2, "0");
  const us = String(date.getUTCSeconds()).padStart(2, "0");
  utcTime.value = `${uy}-${uM}-${ud} ${uh}:${um}:${us}`;

  // Relative time
  const nowMs = Date.now();
  const diffMs = ms - nowMs;
  const absDiffMs = Math.abs(diffMs);
  const secondsDiff = Math.round(absDiffMs / 1000);
  const minutesDiff = Math.round(secondsDiff / 60);
  const hoursDiff = Math.round(minutesDiff / 60);
  const daysDiff = Math.round(hoursDiff / 24);

  if (diffMs > 0) {
    if (daysDiff > 0) relativeTime.value = `${daysDiff} 天后`;
    else if (hoursDiff > 0) relativeTime.value = `${hoursDiff} 小时后`;
    else if (minutesDiff > 0) relativeTime.value = `${minutesDiff} 分钟后`;
    else relativeTime.value = `${secondsDiff} 秒后`;
  } else {
    if (daysDiff > 0) relativeTime.value = `${daysDiff} 天前`;
    else if (hoursDiff > 0) relativeTime.value = `${hoursDiff} 小时前`;
    else if (minutesDiff > 0) relativeTime.value = `${minutesDiff} 分钟前`;
    else relativeTime.value = `${secondsDiff} 秒前`;
  }
}

function convertDate() {
  dateError.value = "";
  tsOutput.value = null;

  const input = dateInput.value.trim();
  if (!input) {
    dateError.value = "请输入日期时间字符串";
    return;
  }

  const date = new Date(input);
  if (isNaN(date.getTime())) {
    dateError.value = "无效的日期格式，请使用 ISO 8601 格式（如 2024-01-15T10:30:00）";
    return;
  }

  tsOutput.value = Math.floor(date.getTime() / 1000);
}
</script>

<template>
  <div class="max-w-xl">
    <h3 class="text-lg font-display font-bold text-ink">时间戳转换</h3>
    <p class="mt-1 text-sm text-ink-faint">Unix 时间戳与日期时间互转。支持秒和毫秒自动识别。</p>

    <div class="mt-5 space-y-4">
      <!-- Mode Toggle -->
      <div class="flex w-fit rounded-lg border border-paper-deep/30 bg-paper-warm/30 p-1">
        <button
          class="rounded-md px-4 py-1.5 text-sm font-medium transition-colors"
          :class="mode === 'ts-to-date' ? 'bg-bamboo/15 text-bamboo ring-1 ring-bamboo/30' : 'text-ink-soft hover:text-ink'"
          @click="mode = 'ts-to-date'"
        >
          时间戳 → 日期
        </button>
        <button
          class="rounded-md px-4 py-1.5 text-sm font-medium transition-colors"
          :class="mode === 'date-to-ts' ? 'bg-bamboo/15 text-bamboo ring-1 ring-bamboo/30' : 'text-ink-soft hover:text-ink'"
          @click="mode = 'date-to-ts'"
        >
          日期 → 时间戳
        </button>
      </div>

      <!-- Timestamp to Date -->
      <div v-if="mode === 'ts-to-date'" class="space-y-4">
        <div>
          <label class="mb-1.5 block text-xs font-medium text-ink-soft">Unix 时间戳</label>
          <div class="flex gap-2">
            <input
              v-model.number="tsInput"
              type="number"
              placeholder="如 1700000000 或 1700000000000"
              class="flex-1 rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/80 font-mono"
              @keyup.enter="convertTs"
            />
            <Button variant="ghost" @click="now">现在</Button>
          </div>
        </div>
        <Button @click="convertTs">转换</Button>

        <p v-if="tsError" class="text-sm text-red-500">{{ tsError }}</p>

        <div
          v-if="localTime"
          class="space-y-3 rounded-xl border border-paper-deep/20 bg-paper-warm/30 p-5"
        >
          <div class="flex items-center justify-between">
            <span class="text-xs font-medium text-ink-soft">本地时间</span>
            <span class="text-sm font-mono text-ink">{{ localTime }}</span>
          </div>
          <div class="flex items-center justify-between">
            <span class="text-xs font-medium text-ink-soft">UTC 时间</span>
            <span class="text-sm font-mono text-ink">{{ utcTime }}</span>
          </div>
          <div class="flex items-center justify-between">
            <span class="text-xs font-medium text-ink-soft">相对时间</span>
            <span class="text-sm text-ink">{{ relativeTime }}</span>
          </div>
        </div>
      </div>

      <!-- Date to Timestamp -->
      <div v-if="mode === 'date-to-ts'" class="space-y-4">
        <div>
          <label class="mb-1.5 block text-xs font-medium text-ink-soft">日期时间</label>
          <div class="relative">
            <input
              v-model="dateInput"
              type="datetime-local"
              class="w-full appearance-none rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors focus:border-bamboo/40 focus:bg-paper-warm/80 [&::-webkit-calendar-picker-indicator]:cursor-pointer [&::-webkit-calendar-picker-indicator]:opacity-40 [&::-webkit-calendar-picker-indicator]:hover:opacity-70 [&::-webkit-calendar-picker-indicator]:transition-opacity"
            />
            <div class="pointer-events-none absolute inset-y-0 right-2 flex items-center text-ink-faint/50">
              <svg class="h-4 w-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="4" width="18" height="18" rx="2" ry="2"/><line x1="16" y1="2" x2="16" y2="6"/><line x1="8" y1="2" x2="8" y2="6"/><line x1="3" y1="10" x2="21" y2="10"/></svg>
            </div>
          </div>
          <p class="mt-1 text-xs text-ink-faint">也支持手动输入 ISO 8601 格式（如 2024-01-15T10:30:00）</p>
        </div>
        <Button @click="convertDate">转换为时间戳</Button>

        <p v-if="dateError" class="text-sm text-red-500">{{ dateError }}</p>

        <div
          v-if="tsOutput !== null"
          class="rounded-xl border border-paper-deep/20 bg-paper-warm/30 p-5"
        >
          <div class="flex items-center justify-between">
            <span class="text-xs font-medium text-ink-soft">Unix 时间戳（秒）</span>
            <span class="text-sm font-mono text-ink select-all">{{ tsOutput }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
