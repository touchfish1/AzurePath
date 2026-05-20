<script setup lang="ts">
import { ref, computed } from "vue";
import { diffLines } from "diff";

const original = ref("");
const modified = ref("");
const compared = ref(false);

interface DiffLine {
  type: "added" | "removed" | "unchanged";
  value: string;
}

const diffResult = ref<DiffLine[]>([]);
const additions = ref(0);
const deletions = ref(0);

function compare() {
  if (!original.value && !modified.value) return;
  compared.value = true;

  const changes = diffLines(original.value, modified.value);
  const lines: DiffLine[] = [];
  let added = 0;
  let removed = 0;

  for (const change of changes) {
    const type = change.added ? "added" : change.removed ? "removed" : "unchanged";
    const value = change.value;
    // Split the block into individual lines, preserving the final empty segment
    const parts = value.split(/\n/);

    for (let i = 0; i < parts.length; i++) {
      const part = parts[i];
      const isLast = i === parts.length - 1;
      if (part === "" && isLast) continue;
      lines.push({ type, value: part + (isLast ? "" : "\n") });
    }

    if (change.added) added += change.count ?? 0;
    if (change.removed) removed += change.count ?? 0;
  }

  diffResult.value = lines;
  additions.value = added;
  deletions.value = removed;
}

const hasDiff = computed(() => compared.value && diffResult.value.length > 0);

const outputText = computed(() => {
  return diffResult.value
    .map((line) => {
      if (line.type === "added") return "+" + line.value;
      if (line.type === "removed") return "-" + line.value;
      return " " + line.value;
    })
    .join("");
});

const copied = ref(false);
function copy(text: string) {
  navigator.clipboard.writeText(text);
  copied.value = true;
  setTimeout(() => {
    copied.value = false;
  }, 2000);
}

function clearAll() {
  original.value = "";
  modified.value = "";
  compared.value = false;
  diffResult.value = [];
  additions.value = 0;
  deletions.value = 0;
}
</script>

<template>
  <div class="max-w-xl">
    <h3 class="text-lg font-display font-bold text-ink">文本 Diff</h3>
    <p class="mt-1 text-sm text-ink-faint">比较两段文本的差异，支持按行对比并显示增删统计。</p>
    <div class="mt-5 space-y-4">
      <div class="grid grid-cols-2 gap-4">
        <div>
          <label class="mb-1.5 block text-xs font-medium text-ink-soft">原始文本</label>
          <textarea
            v-model="original"
            class="w-full rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/80 resize-y font-mono"
            rows="8"
            placeholder="输入原始文本..."
          ></textarea>
        </div>
        <div>
          <label class="mb-1.5 block text-xs font-medium text-ink-soft">修改后文本</label>
          <textarea
            v-model="modified"
            class="w-full rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/80 resize-y font-mono"
            rows="8"
            placeholder="输入修改后文本..."
          ></textarea>
        </div>
      </div>

      <div class="flex items-center gap-3">
        <button
          class="inline-flex items-center justify-center whitespace-nowrap rounded-lg bg-bamboo px-4 py-2 text-sm font-medium text-cloud shadow-sm transition-colors hover:bg-bamboo-light active:bg-bamboo disabled:pointer-events-none disabled:opacity-50 select-none"
          :disabled="!original && !modified"
          @click="compare"
        >
          比较
        </button>
        <button
          class="inline-flex items-center justify-center whitespace-nowrap rounded-lg bg-paper-warm px-4 py-2 text-sm font-medium text-ink-soft shadow-sm transition-colors hover:bg-paper-deep hover:text-ink active:bg-paper-deep disabled:pointer-events-none disabled:opacity-50 select-none"
          :disabled="!original && !modified"
          @click="clearAll"
        >
          清空
        </button>
      </div>

      <div v-if="hasDiff" class="space-y-3">
        <div class="flex items-center gap-4 text-sm">
          <span class="text-green-600 font-medium">+{{ additions }} 处增加</span>
          <span class="text-red-500 font-medium">-{{ deletions }} 处删除</span>
          <span class="text-ink-faint">{{ diffResult.filter((l) => l.type === 'unchanged').length }} 处未变动</span>
        </div>

        <div class="rounded-xl border border-paper-deep/20 bg-paper-warm/30 p-4">
          <div class="mb-2 flex items-center justify-between">
            <span class="text-xs font-medium text-ink-faint">对比结果</span>
            <button
              class="text-xs text-ink-faint hover:text-ink transition-colors"
              @click="copy(outputText)"
            >
              {{ copied ? "已复制" : "复制" }}
            </button>
          </div>
          <div class="whitespace-pre-wrap break-all text-sm text-ink font-mono leading-relaxed">
            <template v-for="(line, idx) in diffResult" :key="idx">
              <span
                :data-index="idx"
                :class="{
                  'bg-green-100 dark:bg-green-900/20 text-green-800 dark:text-green-300': line.type === 'added',
                  'bg-red-100 dark:bg-red-900/20 text-red-700 dark:text-red-300': line.type === 'removed',
                }"
                class="block"
              ><span class="select-none mr-1">{{ line.type === "added" ? "+" : line.type === "removed" ? "-" : " " }}</span>{{ line.value }}</span>
            </template>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
