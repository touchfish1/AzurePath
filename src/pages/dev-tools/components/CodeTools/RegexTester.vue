<script setup lang="ts">
import { ref, computed } from "vue";

const pattern = ref("");
const testText = ref("");
const flags = ref({
  g: true,
  i: false,
  m: false,
  s: false,
  u: false,
});
const tested = ref(false);
const error = ref("");
const matchCount = ref(0);
const matches = ref<RegExpExecArray[]>([]);

const flagsList = [
  { key: "g" as const, label: "g", title: "全局搜索" },
  { key: "i" as const, label: "i", title: "不区分大小写" },
  { key: "m" as const, label: "m", title: "多行模式" },
  { key: "s" as const, label: "s", title: "点号匹配换行" },
  { key: "u" as const, label: "u", title: "Unicode" },
];

function testRegex() {
  tested.value = true;
  error.value = "";
  matches.value = [];
  matchCount.value = 0;

  if (!pattern.value.trim()) {
    error.value = "请输入正则表达式。";
    return;
  }
  if (!testText.value) {
    error.value = "请输入测试文本。";
    return;
  }

  const flagString = Object.entries(flags.value)
    .filter(([, v]) => v)
    .map(([k]) => k)
    .join("");

  try {
    const regex = new RegExp(pattern.value, flagString);
    const allMatches: RegExpExecArray[] = [];
    let execResult: RegExpExecArray | null;

    if (flagString.includes("g")) {
      while ((execResult = regex.exec(testText.value)) !== null) {
        allMatches.push(execResult);
        if (execResult.index === regex.lastIndex) {
          regex.lastIndex++;
        }
      }
    } else {
      execResult = regex.exec(testText.value);
      if (execResult) {
        allMatches.push(execResult);
      }
    }

    matches.value = allMatches;
    matchCount.value = allMatches.length;
  } catch (e: any) {
    error.value = e?.message || "无效的正则表达式。";
  }
}

const hasResults = computed(() => tested.value && !error.value);

const copied = ref(false);
function copy(text: string) {
  navigator.clipboard.writeText(text);
  copied.value = true;
  setTimeout(() => {
    copied.value = false;
  }, 2000);
}

function clearAll() {
  pattern.value = "";
  testText.value = "";
  flags.value = { g: true, i: false, m: false, s: false, u: false };
  tested.value = false;
  error.value = "";
  matches.value = [];
  matchCount.value = 0;
}
</script>

<template>
  <div class="max-w-xl">
    <h3 class="text-lg font-display font-bold text-ink">正则测试器</h3>
    <p class="mt-1 text-sm text-ink-faint">测试正则表达式匹配结果，查看匹配详情与捕获组。</p>
    <div class="mt-5 space-y-4">
      <div>
        <label class="mb-1.5 block text-xs font-medium text-ink-soft">正则表达式</label>
        <input
          v-model="pattern"
          type="text"
          class="w-full rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/80 font-mono"
          placeholder="例如: \d+\.\d+\.\d+\.\d+"
          @keyup.enter="testRegex"
        />
      </div>

      <div>
        <label class="mb-1.5 block text-xs font-medium text-ink-soft">标志</label>
        <div class="flex flex-wrap gap-3">
          <label
            v-for="f in flagsList"
            :key="f.key"
            class="flex cursor-pointer items-center gap-1.5 rounded-lg px-3 py-1.5 text-sm font-medium transition-colors"
            :class="
              flags[f.key]
                ? 'bg-bamboo/15 text-bamboo ring-1 ring-bamboo/30'
                : 'bg-paper-deep/20 text-ink-soft hover:bg-paper-deep/40 hover:text-ink'
            "
          >
            <input
              type="checkbox"
              :checked="flags[f.key]"
              class="sr-only"
              @change="flags[f.key] = !flags[f.key]"
            />
            <span class="font-mono">{{ f.label }}</span>
            <span class="text-[10px] opacity-60">{{ f.title }}</span>
          </label>
        </div>
      </div>

      <div>
        <label class="mb-1.5 block text-xs font-medium text-ink-soft">测试文本</label>
        <textarea
          v-model="testText"
          class="w-full rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/80 resize-y font-mono"
          rows="8"
          placeholder="输入要匹配的文本..."
        ></textarea>
      </div>

      <div class="flex items-center gap-3">
        <button
          class="inline-flex items-center justify-center whitespace-nowrap rounded-lg bg-bamboo px-4 py-2 text-sm font-medium text-cloud shadow-sm transition-colors hover:bg-bamboo-light active:bg-bamboo disabled:pointer-events-none disabled:opacity-50 select-none"
          :disabled="!pattern.trim() || !testText"
          @click="testRegex"
        >
          测试
        </button>
        <button
          class="inline-flex items-center justify-center whitespace-nowrap rounded-lg bg-paper-warm px-4 py-2 text-sm font-medium text-ink-soft shadow-sm transition-colors hover:bg-paper-deep hover:text-ink active:bg-paper-deep disabled:pointer-events-none disabled:opacity-50 select-none"
          :disabled="!pattern.trim() && !testText"
          @click="clearAll"
        >
          清空
        </button>
      </div>

      <p v-if="error" class="text-sm text-red-500">{{ error }}</p>

      <div v-if="hasResults" class="space-y-3">
        <div class="rounded-xl border border-paper-deep/20 bg-paper-warm/30 p-4">
          <div class="mb-2 flex items-center justify-between">
            <span class="text-xs font-medium text-ink-faint">
              匹配结果
              <span class="ml-2 rounded bg-bamboo/15 px-1.5 py-0.5 text-bamboo">{{ matchCount }} 处匹配</span>
            </span>
          </div>

          <div v-if="matchCount === 0" class="text-sm text-ink-faint">无匹配结果。</div>

          <div v-else class="space-y-2.5">
            <div
              v-for="(match, idx) in matches"
              :key="idx"
              class="rounded-lg border border-paper-deep/10 bg-paper-warm/50 p-3"
            >
              <div class="mb-1 flex items-center gap-3 text-xs text-ink-faint">
                <span>#{{ idx + 1 }}</span>
                <span>位置: {{ match.index }}</span>
                <span
                  class="cursor-pointer text-ink-faint hover:text-ink transition-colors"
                  @click="copy(match[0])"
                >
                  {{ copied ? "已复制" : "复制" }}
                </span>
              </div>
              <p class="mb-1.5 whitespace-pre-wrap break-all text-sm text-ink font-mono">
                {{ match[0] }}
              </p>
              <div v-if="match.length > 1" class="space-y-0.5">
                <p
                  v-for="groupIdx in match.length - 1"
                  :key="groupIdx"
                  class="text-xs text-ink-soft"
                >
                  <span class="font-mono text-ink-faint">捕获组 ${{ groupIdx }}:</span>
                  <span class="ml-1.5 font-mono">{{ match[groupIdx] ?? "(无匹配)" }}</span>
                </p>
              </div>
              <p v-else class="text-xs text-ink-faint">无捕获组</p>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
