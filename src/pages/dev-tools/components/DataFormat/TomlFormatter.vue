<script setup lang="ts">
import { ref } from "vue";
import { Copy, Check } from "lucide-vue-next";
import { parse, stringify } from "smol-toml";
import Button from "@/components/ui/button/Button.vue";

const input = ref("");
const output = ref("");
const error = ref("");
const copied = ref(false);

function format() {
  error.value = "";
  try {
    const parsed = parse(input.value);
    output.value = stringify(parsed);
  } catch (e) {
    error.value = "无效的 TOML: " + (e as Error).message;
    output.value = "";
  }
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
    <h3 class="text-lg font-display font-bold text-ink">TOML 格式化</h3>
    <p class="mt-1 text-sm text-ink-faint">格式化和验证 TOML 数据</p>
    <div class="mt-5 space-y-4">
      <div>
        <label class="mb-1.5 block text-xs font-medium text-ink-soft">输入 TOML</label>
        <textarea
          v-model="input"
          rows="8"
          class="w-full rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/80 resize-y font-mono"
          placeholder='[package]&#10;name = "AzurePath"&#10;version = "1.0"'
        />
      </div>
      <div class="flex gap-2">
        <Button variant="default" size="sm" @click="format">格式化</Button>
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
