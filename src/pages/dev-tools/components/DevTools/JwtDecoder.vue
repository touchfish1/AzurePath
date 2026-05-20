<script setup lang="ts">
import { ref, computed } from "vue";
import { Copy, Check } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";

const jwtInput = ref("");
const jwtHeader = ref("");
const jwtPayload = ref("");
const jwtError = ref("");
const copied = ref(false);

function base64UrlDecode(str: string): string {
  let base64 = str.replace(/-/g, "+").replace(/_/g, "/");
  while (base64.length % 4 !== 0) {
    base64 += "=";
  }
  return base64;
}

function decodeJwt() {
  jwtError.value = "";
  jwtHeader.value = "";
  jwtPayload.value = "";
  const token = jwtInput.value.trim();
  if (!token) {
    jwtError.value = "请输入 JWT Token";
    return;
  }
  const parts = token.split(".");
  if (parts.length !== 3) {
    jwtError.value = "无效 JWT：需要三部分（header.payload.signature）";
    return;
  }
  try {
    const headerJson = atob(base64UrlDecode(parts[0]));
    const headerParsed = JSON.parse(headerJson);
    jwtHeader.value = JSON.stringify(headerParsed, null, 2);

    const payloadJson = atob(base64UrlDecode(parts[1]));
    const payloadParsed = JSON.parse(payloadJson);
    jwtPayload.value = JSON.stringify(payloadParsed, null, 2);
  } catch (e) {
    jwtError.value = `解码失败: ${e}`;
  }
}

const expiryInfo = computed(() => {
  if (!jwtPayload.value) return null;
  try {
    const payload = JSON.parse(jwtPayload.value);
    if (payload.exp) {
      const expDate = new Date(payload.exp * 1000);
      const now = new Date();
      const diff = expDate.getTime() - now.getTime();
      const isExpired = diff < 0;
      const absMin = Math.abs(Math.round(diff / 1000 / 60));
      let relative: string;
      if (isExpired) {
        if (absMin >= 1440) relative = `已过期 ${Math.round(absMin / 1440)} 天前`;
        else if (absMin >= 60) relative = `已过期 ${Math.round(absMin / 60)} 小时前`;
        else relative = `已过期 ${absMin} 分钟前`;
      } else {
        if (absMin >= 1440) relative = `剩余 ${Math.round(absMin / 1440)} 天`;
        else if (absMin >= 60) relative = `剩余 ${Math.round(absMin / 60)} 小时`;
        else relative = `剩余 ${absMin} 分钟`;
      }
      const y = expDate.getFullYear();
      const M = String(expDate.getMonth() + 1).padStart(2, "0");
      const d = String(expDate.getDate()).padStart(2, "0");
      const h = String(expDate.getHours()).padStart(2, "0");
      const m = String(expDate.getMinutes()).padStart(2, "0");
      const s = String(expDate.getSeconds()).padStart(2, "0");
      return `${y}-${M}-${d} ${h}:${m}:${s} (${relative})`;
    }
    return "无过期时间 (exp)";
  } catch {
    return null;
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
    <h3 class="text-lg font-display font-bold text-ink">JWT 解码器</h3>
    <p class="mt-1 text-sm text-ink-faint">解码 JSON Web Token 的 Header 和 Payload。</p>

    <div class="mt-5 space-y-4">
      <div>
        <label class="mb-1.5 block text-xs font-medium text-ink-soft">JWT Token</label>
        <textarea
          v-model="jwtInput"
          rows="3"
          placeholder="eyJhbGciOiJIUzI1NiIs..."
          class="w-full rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/80 resize-y font-mono"
        />
      </div>
      <Button @click="decodeJwt">解码</Button>

      <p v-if="jwtError" class="text-sm text-red-500">{{ jwtError }}</p>

      <div v-if="jwtHeader">
        <div class="mb-1.5 flex items-center justify-between">
          <label class="text-xs font-medium text-ink-soft">Header</label>
          <button
            class="flex items-center gap-1 rounded-md px-2 py-1 text-xs transition-colors"
            :class="copied ? 'text-bamboo bg-bamboo/10' : 'text-ink-faint hover:text-ink hover:bg-paper-deep/30'"
            @click="copy(jwtHeader)"
          >
            <Copy v-if="!copied" class="h-3 w-3" />
            <Check v-else class="h-3 w-3" />
            {{ copied ? "已复制" : "复制" }}
          </button>
        </div>
        <div class="mb-4 rounded-xl border border-paper-deep/20 bg-paper-warm/30 p-4">
          <pre class="whitespace-pre-wrap break-all text-sm text-ink font-mono">{{ jwtHeader }}</pre>
        </div>

        <div class="mb-1.5 flex items-center justify-between">
          <label class="text-xs font-medium text-ink-soft">Payload</label>
          <button
            class="flex items-center gap-1 rounded-md px-2 py-1 text-xs transition-colors"
            :class="copied ? 'text-bamboo bg-bamboo/10' : 'text-ink-faint hover:text-ink hover:bg-paper-deep/30'"
            @click="copy(jwtPayload)"
          >
            <Copy v-if="!copied" class="h-3 w-3" />
            <Check v-else class="h-3 w-3" />
            {{ copied ? "已复制" : "复制" }}
          </button>
        </div>
        <div class="rounded-xl border border-paper-deep/20 bg-paper-warm/30 p-4">
          <pre class="whitespace-pre-wrap break-all text-sm text-ink font-mono">{{ jwtPayload }}</pre>
        </div>

        <div
          v-if="jwtPayload"
          class="mt-3 rounded-lg border border-paper-deep/20 bg-paper-warm/30 px-4 py-3 text-sm"
        >
          <span class="text-xs font-medium text-ink-soft">过期信息：</span>
          <span class="text-ink">{{ expiryInfo || "解析失败" }}</span>
        </div>
      </div>
    </div>
  </div>
</template>
