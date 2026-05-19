<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { Search, Copy } from "lucide-vue-next";
import { useToastStore } from "@/stores/toast";

const toast = useToastStore();

function copyValue(value: string) {
  navigator.clipboard.writeText(value).then(() => {
    toast.add("success", "已复制");
  });
}
import Button from "@/components/ui/button/Button.vue";
import {
  dnsLookup,
  onDnsResult,
  onDnsError,
  type RecordType,
  type DnsRecord,
  type DnsResultPayload,
  type DnsErrorPayload,
} from "@/lib/tauri";
import type { UnlistenFn } from "@tauri-apps/api/event";

const target = ref("");
const dnsServer = ref("8.8.8.8");
const recordType = ref<RecordType>("a");
const loading = ref(false);
const error = ref("");
const records = ref<DnsRecord[]>([]);
const lastTarget = ref("");

const recordTypes: { value: RecordType; label: string }[] = [
  { value: "a", label: "A" },
  { value: "aaaa", label: "AAAA" },
  { value: "cname", label: "CNAME" },
  { value: "mx", label: "MX" },
  { value: "ns", label: "NS" },
  { value: "soa", label: "SOA" },
  { value: "txt", label: "TXT" },
  { value: "all", label: "ALL" },
];

let unlistenResult: UnlistenFn | null = null;
let unlistenError: UnlistenFn | null = null;

async function lookup() {
  if (!target.value.trim()) return;
  loading.value = true;
  error.value = "";
  records.value = [];
  lastTarget.value = target.value.trim();

  try {
    const result = await dnsLookup(target.value.trim(), recordType.value, dnsServer.value);
    records.value = result;
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

function handleResult(payload: DnsResultPayload) {
  records.value = payload.records;
  loading.value = false;
}

function handleError(payload: DnsErrorPayload) {
  error.value = payload.error;
  loading.value = false;
}

onMounted(async () => {
  unlistenResult = await onDnsResult(handleResult);
  unlistenError = await onDnsError(handleError);
});

onUnmounted(() => {
  unlistenResult?.();
  unlistenError?.();
});
</script>

<template>
  <div class="flex h-full flex-col p-6 space-y-6 animate-view-fade">
    <!-- Header -->
    <div>
      <h1 class="text-2xl font-display font-bold text-ink">DNS 查询</h1>
      <p class="mt-0.5 text-sm text-ink-faint">查询域名解析记录</p>
    </div>

    <!-- Input card -->
    <div class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm">
      <div class="flex flex-wrap items-end gap-3">
        <div class="flex-1 min-w-[180px]">
          <label class="mb-1 block text-xs font-medium text-ink-soft">域名</label>
          <input
            v-model="target"
            placeholder="example.com"
            :disabled="loading"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink placeholder:text-ink-faint/50 outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20 disabled:opacity-50"
            @keyup.enter="lookup"
          />
        </div>
        <div class="w-40">
          <label class="mb-1 block text-xs font-medium text-ink-soft">DNS 服务器</label>
          <input
            v-model="dnsServer"
            placeholder="例如 8.8.8.8 或 8.8.8.8:53"
            :disabled="loading"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink placeholder:text-ink-faint/50 outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20 disabled:opacity-50"
          />
        </div>
        <div class="w-28">
          <label class="mb-1 block text-xs font-medium text-ink-soft">记录类型</label>
          <select
            v-model="recordType"
            :disabled="loading"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20 disabled:opacity-50"
          >
            <option
              v-for="rt in recordTypes"
              :key="rt.value"
              :value="rt.value"
            >
              {{ rt.label }}
            </option>
          </select>
        </div>
        <div>
          <Button :disabled="loading || !target.trim()" @click="lookup">
            <Search class="mr-1.5 h-3.5 w-3.5" />
            查询
          </Button>
        </div>
      </div>
    </div>

    <!-- Loading indicator -->
    <div
      v-if="loading"
      class="flex items-center justify-center py-10 text-sm text-ink-faint"
    >
      <span class="animate-pulse">查询中...</span>
    </div>

    <!-- Error banner -->
    <div
      v-if="error"
      class="rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700 dark:border-red-800/30 dark:bg-red-900/10 dark:text-red-400"
    >
      {{ error }}
    </div>

    <!-- Results table -->
    <div
      v-if="records.length > 0"
      class="noise-bg rounded-xl border border-paper-deep/60 bg-paper shadow-sm overflow-hidden"
    >
      <div class="px-5 py-3 border-b border-paper-deep/50">
        <h2 class="text-sm font-semibold text-ink">
          解析结果 - {{ lastTarget }}
        </h2>
      </div>
      <div class="overflow-x-auto">
        <table class="w-full text-sm">
          <thead>
            <tr class="border-b border-paper-deep/30 text-xs text-ink-faint uppercase tracking-wider">
              <th class="px-5 py-3 text-left font-medium">名称</th>
              <th class="px-5 py-3 text-left font-medium">类型</th>
              <th class="px-5 py-3 text-left font-medium">值</th>
              <th class="px-5 py-3 text-right font-medium">TTL</th>
              <th class="px-5 py-3 text-left font-medium w-14">操作</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="(rec, i) in records"
              :key="i"
              class="border-b border-paper-deep/20 last:border-0 animate-slide-up"
            >
              <td class="px-5 py-2.5 font-mono text-ink max-w-[200px] truncate" :title="rec.name">
                {{ rec.name }}
              </td>
              <td class="px-5 py-2.5">
                <span
                  class="inline-block rounded-full bg-bamboo/10 px-2 py-0.5 text-xs font-semibold text-bamboo"
                >
                  {{ rec.type }}
                </span>
              </td>
              <td class="px-5 py-2.5 font-mono text-ink-soft max-w-[300px] truncate" :title="rec.value">
                {{ rec.value }}
              </td>
              <td class="px-5 py-2.5 text-right font-mono text-ink-faint">
                {{ rec.ttl }}
              </td>
              <td class="px-5 py-2.5">
                <button
                  class="rounded-lg p-1.5 text-ink-faint transition-colors hover:text-bamboo hover:bg-bamboo/5"
                  title="复制记录值"
                  @click="copyValue(rec.value)"
                >
                  <Copy class="h-3.5 w-3.5" />
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- No results -->
    <div
      v-if="!loading && !error && records.length === 0 && lastTarget"
      class="rounded-xl border border-paper-deep/60 bg-paper-warm/50 px-5 py-4 text-center text-sm text-ink-faint animate-fade-in"
    >
      未找到 {{ recordType.toUpperCase() }} 记录
    </div>
  </div>
</template>
