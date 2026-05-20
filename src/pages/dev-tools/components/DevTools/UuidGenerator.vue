<script setup lang="ts">
import { ref } from "vue";
import { Copy, Check } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";

const uuidResult = ref<string[]>([]);
const bulkCount = ref(5);
const copiedIndex = ref(-1);

function generateSingle() {
  uuidResult.value = [crypto.randomUUID()];
}

function generateBulk() {
  const count = Math.max(1, Math.min(100, bulkCount.value || 1));
  const uuids: string[] = [];
  for (let i = 0; i < count; i++) {
    uuids.push(crypto.randomUUID());
  }
  uuidResult.value = uuids;
}

function copy(text: string, index: number) {
  navigator.clipboard.writeText(text);
  copiedIndex.value = index;
  setTimeout(() => {
    copiedIndex.value = -1;
  }, 2000);
}
</script>

<template>
  <div class="max-w-xl">
    <h3 class="text-lg font-display font-bold text-ink">UUID 生成器</h3>
    <p class="mt-1 text-sm text-ink-faint">生成 UUID v4 标识符，支持单次和批量生成。</p>

    <div class="mt-5 space-y-4">
      <div class="flex items-end gap-3">
        <Button @click="generateSingle">生成 UUID</Button>
        <div class="flex items-end gap-2">
          <div>
            <label class="mb-1.5 block text-xs font-medium text-ink-soft">批量数量</label>
            <input
              v-model.number="bulkCount"
              type="number"
              min="1"
              max="100"
              class="w-20 rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/80"
            />
          </div>
          <Button variant="outline" @click="generateBulk">批量生成</Button>
        </div>
      </div>

      <div
        v-if="uuidResult.length"
        class="rounded-xl border border-paper-deep/20 bg-paper-warm/30 p-4"
      >
        <div class="mb-2 flex items-center justify-between">
          <span class="text-xs font-medium text-ink-soft">
            结果（共 {{ uuidResult.length }} 个）
          </span>
        </div>
        <ul class="space-y-2">
          <li
            v-for="(uuid, idx) in uuidResult"
            :key="idx"
            class="flex items-center justify-between gap-2 rounded-lg border border-paper-deep/10 bg-paper-warm/50 px-3 py-2"
          >
            <span class="font-mono text-sm text-ink select-all">{{ uuid }}</span>
            <button
              class="flex shrink-0 items-center gap-1 rounded-md px-2 py-1 text-xs transition-colors"
              :class="copiedIndex === idx ? 'text-bamboo bg-bamboo/10' : 'text-ink-faint hover:text-ink hover:bg-paper-deep/30'"
              @click="copy(uuid, idx)"
            >
              <Copy v-if="copiedIndex !== idx" class="h-3 w-3" />
              <Check v-else class="h-3 w-3" />
              {{ copiedIndex === idx ? "已复制" : "复制" }}
            </button>
          </li>
        </ul>
      </div>
    </div>
  </div>
</template>
