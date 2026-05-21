<script setup lang="ts">
import { ref, watch, onMounted } from "vue";
import { Search, Key, Clock, Save, RefreshCw } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import {
  remoteShellRedisListKeys,
  remoteShellRedisGetValue,
  remoteShellRedisSetValue,
  remoteShellRedisSetTtl,
  type RedisKeyEntry,
} from "@/lib/tauri";

const props = defineProps<{
  connId: string;
}>();

const pattern = ref("*");
const keys = ref<RedisKeyEntry[]>([]);
const selectedKey = ref("");
const keyValue = ref("");
const originalValue = ref("");
const ttl = ref(-1);
const originalTtl = ref(-1);
const ttlInput = ref("");
const loadingKeys = ref(false);
const loadingValue = ref(false);
const saving = ref(false);
const error = ref("");

const typeColors: Record<string, string> = {
  string: "bg-emerald/10 text-emerald ring-1 ring-emerald/30",
  list: "bg-blue/10 text-blue ring-1 ring-blue/30",
  set: "bg-amber/10 text-amber ring-1 ring-amber/30",
  zset: "bg-purple/10 text-purple ring-1 ring-purple/30",
  hash: "bg-cyan/10 text-cyan ring-1 ring-cyan/30",
  stream: "bg-rose/10 text-rose ring-1 ring-rose/30",
};

function prettyType(t: string): string {
  const map: Record<string, string> = {
    string: "String",
    list: "List",
    set: "Set",
    zset: "ZSet",
    hash: "Hash",
    stream: "Stream",
  };
  return map[t] || t;
}

function formatTtl(seconds: number): string {
  if (seconds < 0) return "永不过期";
  if (seconds < 60) return `${seconds} 秒`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)} 分`;
  return `${Math.floor(seconds / 3600)} 时 ${Math.floor((seconds % 3600) / 60)} 分`;
}

async function listKeys() {
  loadingKeys.value = true;
  error.value = "";
  try {
    keys.value = await remoteShellRedisListKeys(props.connId, pattern.value || "*");
  } catch (e) {
    error.value = String(e);
    keys.value = [];
  } finally {
    loadingKeys.value = false;
  }
}

async function selectKey(key: string) {
  selectedKey.value = key;
  loadingValue.value = true;
  error.value = "";
  try {
    const entry = keys.value.find((k) => k.key === key);
    if (entry) {
      ttl.value = entry.ttl;
      originalTtl.value = entry.ttl;
      ttlInput.value = entry.ttl < 0 ? "" : String(entry.ttl);
    }
    const value = await remoteShellRedisGetValue(props.connId, key);
    keyValue.value = value;
    originalValue.value = value;
  } catch (e) {
    error.value = String(e);
    keyValue.value = "";
    originalValue.value = "";
  } finally {
    loadingValue.value = false;
  }
}

async function saveValue() {
  if (!selectedKey.value) return;
  saving.value = true;
  error.value = "";
  try {
    await remoteShellRedisSetValue(props.connId, selectedKey.value, keyValue.value);
    originalValue.value = keyValue.value;
  } catch (e) {
    error.value = String(e);
  } finally {
    saving.value = false;
  }
}

async function setTtl() {
  if (!selectedKey.value) return;
  saving.value = true;
  error.value = "";
  try {
    const seconds = ttlInput.value.trim() === "" ? -1 : parseInt(ttlInput.value);
    if (isNaN(seconds) || seconds < -1) {
      error.value = "TTL 必须为 -1（永不过期）或大于等于 0 的值（秒）";
      saving.value = false;
      return;
    }
    await remoteShellRedisSetTtl(props.connId, selectedKey.value, seconds);
    ttl.value = seconds;
    originalTtl.value = seconds;
    // Refresh key list to update TTL display
    await listKeys();
  } catch (e) {
    error.value = String(e);
  } finally {
    saving.value = false;
  }
}

watch(
  () => props.connId,
  () => {
    selectedKey.value = "";
    keyValue.value = "";
    originalValue.value = "";
    ttl.value = -1;
    originalTtl.value = -1;
    ttlInput.value = "";
    error.value = "";
    listKeys();
  },
  { immediate: true }
);

onMounted(() => {
  listKeys();
});
</script>

<template>
  <div class="flex h-full flex-col gap-4 overflow-y-auto">
    <!-- Key Search -->
    <div class="flex items-center gap-2">
      <div class="relative flex-1">
        <Search class="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-ink-faint" />
        <input
          v-model="pattern"
          type="text"
          placeholder="键名匹配模式（支持 * 通配符）"
          class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 py-2 pl-9 pr-3 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40"
          @keydown.enter="listKeys"
        />
      </div>
      <Button size="sm" :disabled="loadingKeys" @click="listKeys">
        <RefreshCw class="mr-1 h-3.5 w-3.5" :class="{ 'animate-spin': loadingKeys }" />
        搜索
      </Button>
    </div>

    <!-- Error Display -->
    <div v-if="error" class="rounded-lg bg-danger-bg px-4 py-2 text-sm text-red-600">
      {{ error }}
    </div>

    <!-- Main: Key List + Value -->
    <div class="grid min-h-0 flex-1 grid-cols-[1fr_1.5fr] gap-4">
      <!-- Key List -->
      <div class="flex flex-col overflow-hidden">
        <div class="mb-2 text-xs font-medium text-ink-soft">
          共 {{ keys.length }} 个键
        </div>
        <div class="flex-1 overflow-y-auto rounded-xl border border-paper-deep/20 bg-paper-warm/30">
          <div
            v-if="keys.length === 0 && !loadingKeys"
            class="px-4 py-8 text-center text-sm text-ink-faint"
          >
            暂无匹配的键
          </div>
          <div
            v-for="entry in keys"
            :key="entry.key"
            class="cursor-pointer border-b border-paper-deep/10 px-3 py-2.5 transition-colors last:border-b-0 hover:bg-paper-deep/20"
            :class="selectedKey === entry.key ? 'bg-bamboo/10' : ''"
            @click="selectKey(entry.key)"
          >
            <div class="flex items-center gap-2">
              <Key class="h-3.5 w-3.5 shrink-0 text-ink-faint" />
              <span class="min-w-0 flex-1 truncate text-sm text-ink font-mono">
                {{ entry.key }}
              </span>
              <span
                class="shrink-0 rounded-full px-2 py-0.5 text-[10px] font-medium uppercase"
                :class="typeColors[entry.keyType] || 'bg-gray/10 text-gray ring-1 ring-gray/30'"
              >
                {{ prettyType(entry.keyType) }}
              </span>
            </div>
            <div class="mt-1 flex items-center gap-3 pl-5">
              <span class="flex items-center gap-1 text-[11px] text-ink-faint">
                <Clock class="h-3 w-3" />
                {{ formatTtl(entry.ttl) }}
              </span>
              <span v-if="entry.size > 0" class="text-[11px] text-ink-faint">
                {{ entry.size }} 字节
              </span>
            </div>
          </div>
        </div>
      </div>

      <!-- Value Panel -->
      <div class="flex flex-col overflow-hidden">
        <div v-if="loadingValue" class="flex items-center justify-center py-12 text-sm text-ink-faint">
          加载中...
        </div>
        <template v-else-if="selectedKey">
          <!-- Value Editor -->
          <div class="flex-1 overflow-y-auto">
            <div class="mb-2 flex items-center justify-between">
              <span class="text-xs font-medium text-ink-soft">
                值
                <span v-if="keyValue !== originalValue" class="ml-2 text-amber">（已修改）</span>
              </span>
              <Button
                size="sm"
                variant="outline"
                :disabled="saving || keyValue === originalValue"
                @click="saveValue"
              >
                <Save class="mr-1 h-3.5 w-3.5" />
                {{ saving ? "保存中..." : "保存" }}
              </Button>
            </div>
            <textarea
              v-model="keyValue"
              rows="8"
              class="w-full rounded-xl border border-paper-deep/20 bg-paper-warm/30 px-4 py-3 text-sm font-mono text-ink outline-none transition-colors focus:border-bamboo/40 focus:bg-paper-warm/50 resize-y"
            />
          </div>

          <!-- TTL Setter -->
          <div class="mt-4 flex items-center gap-3 rounded-xl border border-paper-deep/20 bg-paper-warm/30 px-4 py-3">
            <Clock class="h-4 w-4 text-ink-faint shrink-0" />
            <div class="flex-1">
              <div class="text-xs text-ink-faint">
                当前 TTL：{{ formatTtl(ttl) }}
              </div>
              <div class="mt-1 flex items-center gap-2">
                <input
                  v-model="ttlInput"
                  type="number"
                  placeholder="-1 (永不过期)"
                  class="w-28 rounded-lg border border-paper-deep bg-paper-warm/50 px-2.5 py-1 text-sm font-mono text-ink outline-none transition-colors focus:border-bamboo/40"
                  min="-1"
                />
                <span class="text-xs text-ink-faint">秒</span>
                <Button size="sm" variant="ghost" :disabled="saving" @click="setTtl">
                  设置 TTL
                </Button>
              </div>
            </div>
          </div>
        </template>
        <div
          v-else
          class="flex items-center justify-center py-12 text-sm text-ink-faint"
        >
          从左侧选择一个键查看值
        </div>
      </div>
    </div>
  </div>
</template>
