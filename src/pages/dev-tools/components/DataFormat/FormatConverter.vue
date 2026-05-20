<script setup lang="ts">
import { ref } from "vue";
import { Copy, Check, ArrowRightLeft } from "lucide-vue-next";
import yaml from "js-yaml";
import { parse as parseToml, stringify as stringifyToml } from "smol-toml";
import Button from "@/components/ui/button/Button.vue";

type Format = "json" | "yaml" | "toml";

const input = ref("");
const output = ref("");
const error = ref("");
const copied = ref(false);
const fromFormat = ref<Format>("json");
const toFormat = ref<Format>("yaml");

const formatOptions: { value: Format; label: string }[] = [
  { value: "json", label: "JSON" },
  { value: "yaml", label: "YAML" },
  { value: "toml", label: "TOML" },
];

function parseInput(text: string, format: Format): unknown {
  switch (format) {
    case "json":
      return JSON.parse(text);
    case "yaml": {
      const parsed = yaml.load(text);
      if (parsed === null || parsed === undefined) {
        throw new Error("内容为空");
      }
      return parsed;
    }
    case "toml":
      return parseToml(text);
  }
}

function stringifyOutput(data: unknown, format: Format): string {
  switch (format) {
    case "json":
      return JSON.stringify(data, null, 2);
    case "yaml":
      return yaml.dump(data, {
        indent: 2,
        lineWidth: -1,
        noRefs: true,
        sortKeys: true,
      });
    case "toml":
      return stringifyToml(data as Record<string, unknown>);
  }
}

function convert() {
  error.value = "";
  output.value = "";
  try {
    if (!input.value.trim()) {
      throw new Error("请输入要转换的内容");
    }
    const parsed = parseInput(input.value, fromFormat.value);
    output.value = stringifyOutput(parsed, toFormat.value);
  } catch (e) {
    error.value = `转换失败 (${fromFormat.value.toUpperCase()} → ${toFormat.value.toUpperCase()}): ${(e as Error).message}`;
  }
}

function swapFormats() {
  const temp = fromFormat.value;
  fromFormat.value = toFormat.value;
  toFormat.value = temp;
}

function copy(text: string) {
  navigator.clipboard.writeText(text);
  copied.value = true;
  setTimeout(() => {
    copied.value = false;
  }, 2000);
}
</script>

<template>
  <div class="max-w-xl">
    <h3 class="text-lg font-display font-bold text-ink">格式互转</h3>
    <p class="mt-1 text-sm text-ink-faint">在 JSON、YAML、TOML 之间互相转换</p>
    <div class="mt-5 space-y-4">
      <div>
        <label class="mb-1.5 block text-xs font-medium text-ink-soft">输入数据</label>
        <textarea
          v-model="input"
          rows="8"
          class="w-full rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/80 resize-y font-mono"
          placeholder="粘贴要转换的数据..."
        />
      </div>
      <div class="flex items-center gap-3">
        <div class="flex-1">
          <label class="mb-1.5 block text-xs font-medium text-ink-soft">源格式</label>
          <select
            v-model="fromFormat"
            class="w-full rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors focus:border-bamboo/40 focus:bg-paper-warm/80"
          >
            <option
              v-for="opt in formatOptions"
              :key="opt.value"
              :value="opt.value"
            >
              {{ opt.label }}
            </option>
          </select>
        </div>
        <button
          class="mt-5 flex h-9 w-9 items-center justify-center rounded-lg text-ink-faint transition-colors hover:bg-paper-deep/50 hover:text-ink"
          @click="swapFormats"
          title="交换格式"
        >
          <ArrowRightLeft class="h-4 w-4" />
        </button>
        <div class="flex-1">
          <label class="mb-1.5 block text-xs font-medium text-ink-soft">目标格式</label>
          <select
            v-model="toFormat"
            class="w-full rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors focus:border-bamboo/40 focus:bg-paper-warm/80"
          >
            <option
              v-for="opt in formatOptions"
              :key="opt.value"
              :value="opt.value"
            >
              {{ opt.label }}
            </option>
          </select>
        </div>
      </div>
      <div class="flex gap-2">
        <Button variant="default" size="sm" @click="convert">转换</Button>
      </div>
      <p v-if="error" class="text-sm text-red-500">{{ error }}</p>
      <div v-if="output" class="rounded-xl border border-paper-deep/20 bg-paper-warm/30 p-4">
        <div class="mb-2 flex items-center justify-between">
          <span class="text-xs font-medium text-ink-soft">输出</span>
          <button
            class="flex items-center gap-1 rounded-md px-2 py-1 text-xs transition-colors"
            :class="copied ? 'text-bamboo bg-bamboo/10' : 'text-ink-faint hover:text-ink-soft'"
            @click="copy(output)"
          >
            <Copy v-if="!copied" class="h-3.5 w-3.5" />
            <Check v-else class="h-3.5 w-3.5" />
            {{ copied ? "已复制" : "复制" }}
          </button>
        </div>
        <pre class="whitespace-pre-wrap break-all text-sm text-ink font-mono">{{ output }}</pre>
      </div>
    </div>
  </div>
</template>
