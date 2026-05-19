<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import {
  Send,
  Plus,
  Trash2,
  Save,
  FolderOpen,
  X,
  Copy,
  Check,
  ChevronDown,
  ChevronRight,
} from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import { useApiTestStore } from "@/stores/apiTest";

const store = useApiTestStore();

const showSavedList = ref(false);
const bodyCopied = ref(false);
const showHeaders = ref(true);

const methods = ["GET", "POST", "PUT", "DELETE", "PATCH"];

const bodyTypes = [
  { value: "json", label: "JSON" },
  { value: "form", label: "Form" },
  { value: "text", label: "Text" },
];

const isBodyVisible = computed(() => {
  return ["POST", "PUT", "PATCH"].includes(store.currentRequest.method);
});

const statusColor = computed(() => {
  if (!store.response) return "";
  const s = store.response.status;
  if (s >= 200 && s < 300) return "text-green-600 bg-green-50 dark:bg-green-900/20";
  if (s >= 300 && s < 400) return "text-blue-600 bg-blue-50 dark:bg-blue-900/20";
  if (s >= 400 && s < 500) return "text-yellow-600 bg-yellow-50 dark:bg-yellow-900/20";
  if (s >= 500) return "text-red-600 bg-red-50 dark:bg-red-900/20";
  return "text-ink-soft bg-paper-deep/30";
});

function addHeader() {
  store.currentRequest.headers.push(["", ""]);
}

function removeHeader(index: number) {
  store.currentRequest.headers.splice(index, 1);
}

function copyBody() {
  if (!store.response?.body) return;
  navigator.clipboard.writeText(store.response.body);
  bodyCopied.value = true;
  setTimeout(() => {
    bodyCopied.value = false;
  }, 2000);
}

function formatBodySize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function loadAndSelect(id: string) {
  const item = store.savedRequests.find((r) => r.id === id);
  if (item) {
    store.loadRequest(item);
    showSavedList.value = false;
  }
}

onMounted(() => {
  store.loadSaved();
});
</script>

<template>
  <div class="flex h-full flex-col animate-view-fade">
    <!-- Top bar -->
    <div class="flex items-center gap-3 border-b border-paper-deep/40 bg-paper-warm/20 px-5 py-3">
      <div class="flex items-center gap-1.5">
        <input
          v-model="store.requestName"
          type="text"
          placeholder="请求名称（保存时使用）"
          class="h-8 w-48 rounded-lg border border-paper-deep/30 bg-paper-warm/50 px-3 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/80"
        />
        <Button size="sm" variant="secondary" @click="store.saveCurrent()">
          <Save class="h-3.5 w-3.5" />
          <span class="ml-1">保存</span>
        </Button>
      </div>

      <div class="relative">
        <Button size="sm" variant="ghost" @click="showSavedList = !showSavedList">
          <FolderOpen class="h-3.5 w-3.5" />
          <span class="ml-1">已保存 ({{ store.savedRequests.length }})</span>
        </Button>
        <div
          v-if="showSavedList"
          class="absolute left-0 top-full z-50 mt-1 w-72 rounded-xl border border-paper-deep/30 bg-paper-warm shadow-lg"
        >
          <div class="flex items-center justify-between border-b border-paper-deep/20 px-3 py-2">
            <span class="text-xs font-medium text-ink-soft">已保存的请求</span>
            <button
              class="rounded-md p-1 text-ink-faint hover:text-ink hover:bg-paper-deep/30"
              @click="showSavedList = false"
            >
              <X class="h-3.5 w-3.5" />
            </button>
          </div>
          <div class="max-h-60 overflow-y-auto p-1">
            <div
              v-if="store.savedRequests.length === 0"
              class="px-3 py-4 text-center text-sm text-ink-faint"
            >
              暂无保存的请求
            </div>
            <button
              v-for="item in store.savedRequests"
              :key="item.id"
              class="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-sm text-ink transition-colors hover:bg-paper-deep/30"
              @click="loadAndSelect(item.id)"
            >
              <span
                class="shrink-0 rounded-md px-1.5 py-0.5 text-xs font-mono font-bold"
                :class="{
                  'text-green-600 bg-green-50 dark:bg-green-900/20': item.request.method === 'GET',
                  'text-blue-600 bg-blue-50 dark:bg-blue-900/20': item.request.method === 'POST',
                  'text-yellow-600 bg-yellow-50 dark:bg-yellow-900/20': item.request.method === 'PUT',
                  'text-red-600 bg-red-50 dark:bg-red-900/20': item.request.method === 'DELETE',
                  'text-purple-600 bg-purple-50 dark:bg-purple-900/20': item.request.method === 'PATCH',
                }"
              >
                {{ item.request.method }}
              </span>
              <div class="min-w-0 flex-1">
                <div class="truncate font-medium">{{ item.name }}</div>
                <div class="truncate text-xs text-ink-faint">{{ item.request.url }}</div>
              </div>
              <button
                class="shrink-0 rounded-md p-1 text-ink-faint opacity-0 transition-opacity hover:text-red-500 hover:bg-red-50 group-hover:opacity-100"
                @click.stop="store.deleteSaved(item.id)"
                title="删除"
              >
                <Trash2 class="h-3.5 w-3.5" />
              </button>
            </button>
          </div>
        </div>
      </div>

      <div class="ml-auto flex items-center gap-2">
        <Button size="sm" variant="ghost" @click="store.newRequest()">新建</Button>
      </div>
    </div>

    <!-- Main content: two panels -->
    <div class="flex flex-1 overflow-hidden">
      <!-- Left: Request Builder -->
      <div class="flex w-2/5 shrink-0 flex-col overflow-y-auto border-r border-paper-deep/30 p-5">
        <!-- Method + URL row -->
        <div class="flex items-start gap-2">
          <!-- Method selector as tabs -->
          <div class="flex shrink-0 flex-col gap-0.5">
            <button
              v-for="m in methods"
              :key="m"
              class="rounded-lg px-3 py-1.5 text-xs font-bold font-mono transition-colors"
              :class="
                store.currentRequest.method === m
                  ? m === 'GET'
                    ? 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400'
                    : m === 'POST'
                      ? 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400'
                      : m === 'PUT'
                        ? 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400'
                        : m === 'DELETE'
                          ? 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400'
                          : 'bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400'
                  : 'bg-paper-deep/20 text-ink-faint hover:bg-paper-deep/40 hover:text-ink'
              "
              @click="store.currentRequest.method = m"
            >
              {{ m }}
            </button>
          </div>

          <!-- URL input and send button -->
          <div class="flex flex-1 items-start gap-2">
            <input
              v-model="store.currentRequest.url"
              type="text"
              placeholder="https://api.example.com/endpoint"
              class="flex-1 rounded-lg border border-paper-deep/40 bg-paper-deep/20 px-3 py-2 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/80 font-mono"
              @keyup.enter="store.send()"
            />
            <Button
              :disabled="store.sending"
              class="shrink-0"
              @click="store.send()"
            >
              <Send class="h-4 w-4" />
              <span class="ml-1.5">{{ store.sending ? '发送中...' : '发送' }}</span>
            </Button>
          </div>
        </div>

        <!-- Headers section -->
        <div class="mt-6">
          <div class="mb-2 flex items-center justify-between">
            <h3 class="text-xs font-semibold uppercase tracking-wider text-ink-soft">
              请求头
              <span class="ml-1 text-ink-faint">({{ store.currentRequest.headers.length }})</span>
            </h3>
            <Button size="sm" variant="ghost" @click="addHeader">
              <Plus class="h-3.5 w-3.5" />
              <span class="ml-1">添加</span>
            </Button>
          </div>
          <div v-if="store.currentRequest.headers.length === 0" class="rounded-lg border border-dashed border-paper-deep/30 px-3 py-4 text-center text-sm text-ink-faint">
            暂无请求头，点击"添加"按钮添加
          </div>
          <div v-else class="space-y-1.5">
            <div
              v-for="(header, index) in store.currentRequest.headers"
              :key="index"
              class="flex items-center gap-1.5"
            >
              <input
                v-model="header[0]"
                type="text"
                placeholder="Key"
                class="flex-1 rounded-lg border border-paper-deep/30 bg-paper-deep/15 px-2.5 py-1.5 text-xs text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/30 font-mono"
              />
              <input
                v-model="header[1]"
                type="text"
                placeholder="Value"
                class="flex-[2] rounded-lg border border-paper-deep/30 bg-paper-deep/15 px-2.5 py-1.5 text-xs text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/30 font-mono"
              />
              <button
                class="shrink-0 rounded-md p-1 text-ink-faint transition-colors hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20"
                @click="removeHeader(index)"
              >
                <Trash2 class="h-3.5 w-3.5" />
              </button>
            </div>
          </div>
        </div>

        <!-- Body section (only for POST/PUT/PATCH) -->
        <div v-if="isBodyVisible" class="mt-6">
          <h3 class="mb-2 text-xs font-semibold uppercase tracking-wider text-ink-soft">请求体</h3>
          <!-- Body type selector -->
          <div class="mb-3 flex gap-2">
            <button
              v-for="bt in bodyTypes"
              :key="bt.value"
              class="rounded-lg px-3 py-1 text-xs font-medium transition-colors"
              :class="
                store.currentRequest.bodyType === bt.value
                  ? 'bg-bamboo/15 text-bamboo ring-1 ring-bamboo/30'
                  : 'bg-paper-deep/20 text-ink-soft hover:bg-paper-deep/40 hover:text-ink'
              "
              @click="store.currentRequest.bodyType = bt.value"
            >
              {{ bt.label }}
            </button>
          </div>
          <textarea
            v-model="store.currentRequest.body"
            rows="8"
            placeholder='{"key": "value"}'
            class="w-full rounded-lg border border-paper-deep/40 bg-paper-deep/20 px-3 py-2 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/80 resize-y font-mono"
          />
        </div>
      </div>

      <!-- Right: Response Viewer -->
      <div class="flex flex-1 flex-col overflow-hidden">
        <!-- Response meta bar -->
        <div class="flex items-center gap-3 border-b border-paper-deep/30 bg-paper-warm/10 px-5 py-2.5">
          <template v-if="store.response">
            <span
              class="rounded-md px-2.5 py-1 text-xs font-bold font-mono"
              :class="statusColor"
            >
              {{ store.response.status }} {{ store.response.statusText }}
            </span>
            <span class="text-xs text-ink-soft">
              {{ store.response.durationMs }} ms
            </span>
            <span class="text-xs text-ink-soft">
              {{ formatBodySize(store.response.bodySize) }}
            </span>
          </template>
          <span v-else-if="store.sending" class="text-sm text-ink-soft">
            发送请求中...
          </span>
          <span v-else class="text-sm text-ink-faint">
            发送请求以查看响应
          </span>

          <div v-if="store.response" class="ml-auto">
            <button
              class="flex items-center gap-1 rounded-md px-2 py-1 text-xs transition-colors"
              :class="bodyCopied ? 'text-bamboo bg-bamboo/10' : 'text-ink-faint hover:text-ink hover:bg-paper-deep/30'"
              @click="copyBody"
            >
              <Copy v-if="!bodyCopied" class="h-3 w-3" />
              <Check v-else class="h-3 w-3" />
              {{ bodyCopied ? '已复制' : '复制' }}
            </button>
          </div>
        </div>

        <!-- Error display -->
        <div v-if="store.error" class="border-b border-paper-deep/30 px-5 py-3">
          <p class="text-sm text-red-500">{{ store.error }}</p>
        </div>

        <!-- Response headers (collapsible) -->
        <div v-if="store.response && store.response.headers.length > 0" class="border-b border-paper-deep/30">
          <button
            class="flex w-full items-center gap-2 px-5 py-2 text-left text-xs font-medium text-ink-soft transition-colors hover:bg-paper-deep/20"
            @click="showHeaders = !showHeaders"
          >
            <component :is="showHeaders ? ChevronDown : ChevronRight" class="h-3.5 w-3.5" />
            响应头 ({{ store.response.headers.length }})
          </button>
          <div v-if="showHeaders" class="space-y-0.5 px-5 pb-2">
            <div
              v-for="(header, index) in store.response.headers"
              :key="index"
              class="flex gap-3 text-xs"
            >
              <span class="shrink-0 font-medium text-ink-soft">{{ header[0] }}:</span>
              <span class="break-all text-ink font-mono">{{ header[1] }}</span>
            </div>
          </div>
        </div>

        <!-- Response body -->
        <div class="flex-1 overflow-y-auto p-5">
          <div v-if="store.response" class="h-full">
            <textarea
              :value="store.response.body"
              readonly
              rows="20"
              class="h-full w-full rounded-lg border border-paper-deep/20 bg-paper-warm/10 px-4 py-3 text-sm text-ink outline-none font-mono resize-none"
              style="min-height: 200px;"
            />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
