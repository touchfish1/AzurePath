<script setup lang="ts">
import { ref, watch } from "vue";
import { Copy, Check } from "lucide-vue-next";
import CryptoJS from "crypto-js";
import Button from "@/components/ui/button/Button.vue";

type HashAlgo = "MD5" | "SHA1" | "SHA224" | "SHA256" | "SHA384" | "SHA512";

const hashInput = ref("");
const selectedAlgo = ref<HashAlgo>("SHA256");
const hashResult = ref("");
const hashError = ref("");
const copied = ref(false);
const computing = ref(false);

const algorithms: { key: HashAlgo; label: string }[] = [
  { key: "MD5", label: "MD5" },
  { key: "SHA1", label: "SHA1" },
  { key: "SHA224", label: "SHA224" },
  { key: "SHA256", label: "SHA256" },
  { key: "SHA384", label: "SHA384" },
  { key: "SHA512", label: "SHA512" },
];

function computeHash() {
  hashError.value = "";
  hashResult.value = "";
  const input = hashInput.value;
  if (!input) {
    hashError.value = "请输入要计算哈希的文本";
    return;
  }

  computing.value = true;
  try {
    const algo = selectedAlgo.value;
    let result: string;
    switch (algo) {
      case "MD5":
        result = CryptoJS.MD5(input).toString();
        break;
      case "SHA1":
        result = CryptoJS.SHA1(input).toString();
        break;
      case "SHA224":
        result = CryptoJS.SHA224(input).toString();
        break;
      case "SHA256":
        result = CryptoJS.SHA256(input).toString();
        break;
      case "SHA384":
        result = CryptoJS.SHA384(input).toString();
        break;
      case "SHA512":
        result = CryptoJS.SHA512(input).toString();
        break;
    }
    hashResult.value = result;
  } catch (e) {
    hashError.value = `哈希计算失败: ${e}`;
  } finally {
    computing.value = false;
  }
}

let debounceTimer: ReturnType<typeof setTimeout> | null = null;

watch(hashInput, () => {
  if (debounceTimer) {
    clearTimeout(debounceTimer);
  }
  debounceTimer = setTimeout(() => {
    if (hashInput.value.trim()) {
      computeHash();
    } else {
      hashResult.value = "";
      hashError.value = "";
    }
  }, 400);
});

watch(selectedAlgo, () => {
  if (hashInput.value.trim()) {
    computeHash();
  }
});

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
    <h3 class="text-lg font-display font-bold text-ink">Hash 生成器</h3>
    <p class="mt-1 text-sm text-ink-faint">使用 CryptoJS 计算文本的哈希值。支持多种算法。</p>

    <div class="mt-5 space-y-4">
      <div>
        <label class="mb-1.5 block text-xs font-medium text-ink-soft">输入文本</label>
        <textarea
          v-model="hashInput"
          rows="4"
          placeholder="输入要计算哈希的文本..."
          class="w-full rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/80 resize-y font-mono"
        />
      </div>

      <div>
        <label class="mb-1.5 block text-xs font-medium text-ink-soft">算法</label>
        <div class="flex flex-wrap gap-2">
          <button
            v-for="algo in algorithms"
            :key="algo.key"
            class="rounded-lg px-3 py-1.5 text-sm font-medium transition-colors"
            :class="selectedAlgo === algo.key ? 'bg-bamboo/15 text-bamboo ring-1 ring-bamboo/30' : 'bg-paper-deep/20 text-ink-soft hover:bg-paper-deep/40 hover:text-ink'"
            @click="selectedAlgo = algo.key"
          >
            {{ algo.label }}
          </button>
        </div>
      </div>

      <Button :disabled="computing || !hashInput.trim()" @click="computeHash">
        {{ computing ? "计算中..." : "计算哈希" }}
      </Button>

      <p v-if="hashError" class="text-sm text-red-500">{{ hashError }}</p>

      <div v-if="hashResult">
        <div class="mb-1.5 flex items-center justify-between">
          <label class="text-xs font-medium text-ink-soft">{{ selectedAlgo }} 结果</label>
          <button
            class="flex items-center gap-1 rounded-md px-2 py-1 text-xs transition-colors"
            :class="copied ? 'text-bamboo bg-bamboo/10' : 'text-ink-faint hover:text-ink hover:bg-paper-deep/30'"
            @click="copy(hashResult)"
          >
            <Copy v-if="!copied" class="h-3 w-3" />
            <Check v-else class="h-3 w-3" />
            {{ copied ? "已复制" : "复制" }}
          </button>
        </div>
        <div class="rounded-xl border border-paper-deep/20 bg-paper-warm/30 p-4">
          <pre class="whitespace-pre-wrap break-all text-sm text-ink font-mono select-all">{{ hashResult }}</pre>
        </div>
      </div>
    </div>
  </div>
</template>
