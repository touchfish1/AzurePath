<script setup lang="ts">
import { ref } from "vue";
import { Copy, Check } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";

const input = ref("");
const output = ref("");
const error = ref("");
const copied = ref(false);
const spaceSeparated = ref(true);

function encode() {
  error.value = "";
  try {
    const bytes: string[] = [];
    for (let i = 0; i < input.value.length; i++) {
      const hex = input.value.charCodeAt(i).toString(16).toUpperCase();
      bytes.push(hex.padStart(2, "0"));
    }
    output.value = spaceSeparated.value ? bytes.join(" ") : bytes.join("");
  } catch (e) {
    error.value = "编码失败: " + (e as Error).message;
    output.value = "";
  }
}

function decode() {
  error.value = "";
  try {
    // Remove all whitespace first, then regroup
    const stripped = input.value.replace(/\s+/g, "");
    if (stripped.length === 0) {
      throw new Error("输入为空");
    }
    if (stripped.length % 2 !== 0) {
      throw new Error("十六进制字符串长度为奇数，每个字节需要两位十六进制数");
    }
    const chars: string[] = [];
    for (let i = 0; i < stripped.length; i += 2) {
      const pair = stripped.substring(i, i + 2);
      const code = parseInt(pair, 16);
      if (isNaN(code)) {
        throw new Error(`无效的十六进制序列: "${pair}"`);
      }
      chars.push(String.fromCharCode(code));
    }
    output.value = chars.join("");
  } catch (e) {
    error.value = "解码失败: " + (e as Error).message;
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
    <h3 class="text-lg font-display font-bold text-ink">Hex 编解码</h3>
    <p class="mt-1 text-sm text-ink-faint">文本与十六进制字符串互相转换</p>
    <div class="mt-5 space-y-4">
      <div>
        <label class="mb-1.5 block text-xs font-medium text-ink-soft">输入文本</label>
        <textarea
          v-model="input"
          rows="6"
          class="w-full rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/80 resize-y font-mono"
          placeholder="输入要编码或解码的文本 / 十六进制..."
        />
      </div>
      <label class="flex items-center gap-2 text-sm text-ink-soft">
        <input
          v-model="spaceSeparated"
          type="checkbox"
          class="h-4 w-4 rounded border-paper-deep/40 bg-paper-warm/50 text-bamboo focus:ring-bamboo/40"
        />
        十六进制输出以空格分隔
      </label>
      <div class="flex gap-2">
        <Button variant="default" size="sm" @click="encode">编码</Button>
        <Button variant="secondary" size="sm" @click="decode">解码</Button>
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
