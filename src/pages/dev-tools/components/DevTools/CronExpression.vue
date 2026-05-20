<script setup lang="ts">
import { ref } from "vue";
import cronstrue from "cronstrue";
import { CronExpressionParser } from "cron-parser";
import Button from "@/components/ui/button/Button.vue";

const cronInput = ref("");
const description = ref("");
const nextTimes = ref<Date[]>([]);
const cronError = ref("");

function parseCron() {
  cronError.value = "";
  description.value = "";
  nextTimes.value = [];

  const expr = cronInput.value.trim();
  if (!expr) {
    cronError.value = "请输入 Cron 表达式";
    return;
  }

  // Basic validation: should have 5 fields
  const parts = expr.split(/\s+/);
  if (parts.length !== 5) {
    cronError.value = "无效 Cron：表达式需要 5 个字段（分 时 日 月 周）";
    return;
  }

  try {
    description.value = cronstrue.toString(expr, { use24HourTimeFormat: true });
  } catch (e) {
    cronError.value = `描述解析失败: ${e}`;
    return;
  }

  try {
    const interval = CronExpressionParser.parse(expr);
    const times: Date[] = [];
    for (let i = 0; i < 5; i++) {
      times.push(interval.next().toDate());
    }
    nextTimes.value = times;
  } catch (e) {
    cronError.value = `时间解析失败: ${e}`;
  }
}

function formatDate(date: Date): string {
  const y = date.getFullYear();
  const M = String(date.getMonth() + 1).padStart(2, "0");
  const d = String(date.getDate()).padStart(2, "0");
  const h = String(date.getHours()).padStart(2, "0");
  const m = String(date.getMinutes()).padStart(2, "0");
  const s = String(date.getSeconds()).padStart(2, "0");
  return `${y}-${M}-${d} ${h}:${m}:${s}`;
}
</script>

<template>
  <div class="max-w-xl">
    <h3 class="text-lg font-display font-bold text-ink">Cron 表达式</h3>
    <p class="mt-1 text-sm text-ink-faint">解析 Cron 表达式，查看人类可读描述和下次执行时间。</p>

    <div class="mt-5 space-y-4">
      <div>
        <label class="mb-1.5 block text-xs font-medium text-ink-soft">Cron 表达式</label>
        <input
          v-model="cronInput"
          type="text"
          placeholder='如 */5 * * * *'
          class="w-full rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/80 font-mono"
          @keyup.enter="parseCron"
        />
      </div>
      <Button @click="parseCron">解析</Button>

      <p v-if="cronError" class="text-sm text-red-500">{{ cronError }}</p>

      <div v-if="description" class="space-y-4">
        <div class="rounded-xl border border-paper-deep/20 bg-paper-warm/30 p-4">
          <div class="mb-1 text-xs font-medium text-ink-soft">描述</div>
          <div class="text-sm text-ink">{{ description }}</div>
        </div>

        <div v-if="nextTimes.length" class="rounded-xl border border-paper-deep/20 bg-paper-warm/30 p-4">
          <div class="mb-2 text-xs font-medium text-ink-soft">接下来 5 次执行时间</div>
          <ul class="space-y-1.5">
            <li
              v-for="(time, idx) in nextTimes"
              :key="idx"
              class="flex items-center gap-3 text-sm"
            >
              <span class="flex h-5 w-5 items-center justify-center rounded-full bg-bamboo/10 text-xs font-medium text-bamboo">{{ idx + 1 }}</span>
              <span class="font-mono text-ink">{{ formatDate(time) }}</span>
            </li>
          </ul>
        </div>
      </div>
    </div>
  </div>
</template>
