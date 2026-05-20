<script setup lang="ts">
import { ref } from "vue";
import { format } from "sql-formatter";

const sqlInput = ref("");
const formattedOutput = ref("");
const error = ref("");
const selectedLanguage = ref("sql");
const formatted = ref(false);

interface LanguageOption {
  value: string;
  label: string;
}

const languages: LanguageOption[] = [
  { value: "sql", label: "SQL" },
  { value: "mysql", label: "MySQL" },
  { value: "postgresql", label: "PostgreSQL" },
  { value: "tsql", label: "TSQL" },
  { value: "bigquery", label: "BigQuery" },
  { value: "redshift", label: "Redshift" },
  { value: "spark", label: "Spark" },
];

function formatSql() {
  if (!sqlInput.value.trim()) return;
  formatted.value = true;
  error.value = "";

  try {
    formattedOutput.value = format(sqlInput.value, {
      language: selectedLanguage.value as any,
      tabWidth: 2,
    });
  } catch (e: any) {
    error.value = e?.message || "格式化失败，请检查 SQL 语法。";
    formattedOutput.value = "";
  }
}

const copied = ref(false);
function copy(text: string) {
  navigator.clipboard.writeText(text);
  copied.value = true;
  setTimeout(() => {
    copied.value = false;
  }, 2000);
}

function clearAll() {
  sqlInput.value = "";
  formattedOutput.value = "";
  error.value = "";
  formatted.value = false;
}
</script>

<template>
  <div class="max-w-xl">
    <h3 class="text-lg font-display font-bold text-ink">SQL 格式化</h3>
    <p class="mt-1 text-sm text-ink-faint">
      格式化 SQL 语句，支持多种数据库方言。
    </p>
    <div class="mt-5 space-y-4">
      <div>
        <label class="mb-1.5 block text-xs font-medium text-ink-soft">数据库方言</label>
        <div class="flex flex-wrap gap-2">
          <button
            v-for="lang in languages"
            :key="lang.value"
            class="rounded-lg px-3 py-1.5 text-sm font-medium transition-colors"
            :class="
              selectedLanguage === lang.value
                ? 'bg-bamboo/15 text-bamboo ring-1 ring-bamboo/30'
                : 'bg-paper-deep/20 text-ink-soft hover:bg-paper-deep/40 hover:text-ink'
            "
            @click="selectedLanguage = lang.value"
          >
            {{ lang.label }}
          </button>
        </div>
      </div>

      <div>
        <label class="mb-1.5 block text-xs font-medium text-ink-soft">SQL 语句</label>
        <textarea
          v-model="sqlInput"
          class="w-full rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/80 resize-y font-mono"
          rows="8"
          placeholder="输入 SQL 语句..."
        ></textarea>
      </div>

      <div class="flex items-center gap-3">
        <button
          class="inline-flex items-center justify-center whitespace-nowrap rounded-lg bg-bamboo px-4 py-2 text-sm font-medium text-cloud shadow-sm transition-colors hover:bg-bamboo-light active:bg-bamboo disabled:pointer-events-none disabled:opacity-50 select-none"
          :disabled="!sqlInput.trim()"
          @click="formatSql"
        >
          格式化
        </button>
        <button
          class="inline-flex items-center justify-center whitespace-nowrap rounded-lg bg-paper-warm px-4 py-2 text-sm font-medium text-ink-soft shadow-sm transition-colors hover:bg-paper-deep hover:text-ink active:bg-paper-deep disabled:pointer-events-none disabled:opacity-50 select-none"
          :disabled="!sqlInput.trim()"
          @click="clearAll"
        >
          清空
        </button>
      </div>

      <p v-if="error" class="text-sm text-red-500">{{ error }}</p>

      <div v-if="formatted && formattedOutput" class="rounded-xl border border-paper-deep/20 bg-paper-warm/30 p-4">
        <div class="mb-2 flex items-center justify-between">
          <span class="text-xs font-medium text-ink-faint">格式化结果</span>
          <button
            class="text-xs text-ink-faint hover:text-ink transition-colors"
            @click="copy(formattedOutput)"
          >
            {{ copied ? "已复制" : "复制" }}
          </button>
        </div>
        <div class="whitespace-pre-wrap break-all text-sm text-ink font-mono leading-relaxed">
          {{ formattedOutput }}
        </div>
      </div>
    </div>
  </div>
</template>
