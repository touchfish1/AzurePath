<script setup lang="ts">
import { ref } from "vue";

const sourceText = ref("");
const converted = ref(false);

interface ConversionResult {
  label: string;
  format: string;
  value: string;
}

const results = ref<ConversionResult[]>([]);
const sourceFormat = ref("");

const formatLabels: Record<string, string> = {
  camelCase: "camelCase",
  snake_case: "snake_case",
  "kebab-case": "kebab-case",
  PascalCase: "PascalCase",
  UPPER_CASE: "UPPER_CASE",
};

function detectFormat(str: string): string {
  if (!str) return "unknown";
  const hasUpper = /[A-Z]/.test(str);
  const hasLower = /[a-z]/.test(str);
  const hasUnderscore = str.includes("_");
  const hasHyphen = str.includes("-");

  if (hasUnderscore && !hasLower && hasUpper) return "UPPER_CASE";
  if (hasUnderscore) return "snake_case";
  if (hasHyphen) return "kebab-case";
  if (/^[A-Z]/.test(str) && hasLower && /[A-Z]/.test(str.slice(1))) return "PascalCase";
  if (/^[a-z]/.test(str) && hasUpper) return "camelCase";
  return "unknown";
}

function toCamelCase(str: string): string {
  let result = str.replace(/^([A-Z])/, (_, c: string) => c.toLowerCase());
  result = result.replace(/[-_](.)/g, (_, c: string) => c.toUpperCase());
  return result;
}

function toSnakeCase(str: string): string {
  let result = str.replace(/([A-Z])/g, "_$1");
  result = result.replace(/-/g, "_");
  return result.toLowerCase();
}

function toKebabCase(str: string): string {
  let result = str.replace(/([A-Z])/g, "-$1");
  result = result.replace(/_/g, "-");
  return result.toLowerCase();
}

function toPascalCase(str: string): string {
  const camel = toCamelCase(str);
  return camel.charAt(0).toUpperCase() + camel.slice(1);
}

function toUpperCaseFormat(str: string): string {
  return toSnakeCase(str).toUpperCase();
}

function convert() {
  if (!sourceText.value.trim()) return;
  converted.value = true;

  sourceFormat.value = detectFormat(sourceText.value.trim());
  const input = sourceText.value.trim();

  results.value = [
    { label: "camelCase", format: "camelCase", value: toCamelCase(input) },
    { label: "snake_case", format: "snake_case", value: toSnakeCase(input) },
    { label: "kebab-case", format: "kebab-case", value: toKebabCase(input) },
    { label: "PascalCase", format: "PascalCase", value: toPascalCase(input) },
    { label: "UPPER_CASE", format: "UPPER_CASE", value: toUpperCaseFormat(input) },
  ];
}

const copiedIndex = ref<number | null>(null);
function copy(text: string, index: number) {
  navigator.clipboard.writeText(text);
  copiedIndex.value = index;
  setTimeout(() => {
    copiedIndex.value = null;
  }, 2000);
}
</script>

<template>
  <div class="max-w-xl">
    <h3 class="text-lg font-display font-bold text-ink">命名格式转换</h3>
    <p class="mt-1 text-sm text-ink-faint">
      自动检测变量命名格式并转换到 camelCase、snake_case、kebab-case、PascalCase、UPPER_CASE。
    </p>
    <div class="mt-5 space-y-4">
      <div>
        <label class="mb-1.5 block text-xs font-medium text-ink-soft">输入文本</label>
        <input
          v-model="sourceText"
          type="text"
          class="w-full rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/80"
          placeholder="例如: myVariableName 或 my_variable_name"
          @keyup.enter="convert"
        />
      </div>

      <div class="flex items-center gap-3">
        <button
          class="inline-flex items-center justify-center whitespace-nowrap rounded-lg bg-bamboo px-4 py-2 text-sm font-medium text-cloud shadow-sm transition-colors hover:bg-bamboo-light active:bg-bamboo disabled:pointer-events-none disabled:opacity-50 select-none"
          :disabled="!sourceText.trim()"
          @click="convert"
        >
          转换
        </button>
      </div>

      <div v-if="converted && sourceText.trim()">
        <p class="mb-3 text-sm text-ink-faint">
          检测格式:
          <span class="ml-1 rounded-md bg-paper-deep/20 px-2 py-0.5 font-mono text-sm text-ink">
            {{ formatLabels[sourceFormat] || sourceFormat }}
          </span>
        </p>

        <div class="space-y-2.5">
          <div
            v-for="(item, idx) in results"
            :key="item.format"
            class="rounded-xl border border-paper-deep/20 bg-paper-warm/30 p-4"
          >
            <div class="mb-1.5 flex items-center justify-between">
              <span class="text-xs font-medium text-ink-faint">{{ item.label }}</span>
              <button
                class="text-xs text-ink-faint hover:text-ink transition-colors"
                @click="copy(item.value, idx)"
              >
                {{ copiedIndex === idx ? "已复制" : "复制" }}
              </button>
            </div>
            <p class="whitespace-pre-wrap break-all text-sm text-ink font-mono">{{ item.value }}</p>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
