<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from "vue";
import { useVirtualList } from "@vueuse/core";
import { convertFileSrc } from "@tauri-apps/api/core";
import { Search, Trash2, Star, Copy, FileText, Image, File, X, ArrowUp, ArrowDown, Download } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import {
  clipboardStart,
  clipboardStop,
  clipboardList,
  clipboardDelete,
  clipboardDeleteBatch,
  clipboardExport,
  clipboardSetLimit,
  clipboardToggleFavorite,
  clipboardCopy,
  clipboardClear,
  clipboardGetInterval,
  clipboardSetInterval,
  onClipboardNew,
  type ClipboardEntry,
} from "@/lib/tauri";
import { formatTime, truncate } from "@/lib/format";
import type { UnlistenFn } from "@tauri-apps/api/event";

const entries = ref<ClipboardEntry[]>([]);
const searchQuery = ref("");
const loading = ref(true);
const copiedId = ref<string | null>(null);
const intervalMs = ref(1000);
const storageLimit = ref(500);
let unlistenNew: UnlistenFn | null = null;

// ─── Selection ────────────────────────────────────────────────
const selectedIds = ref(new Set<string>());

function toggleSelect(id: string) {
  const s = selectedIds.value;
  if (s.has(id)) {
    s.delete(id);
  } else {
    s.add(id);
  }
}

const selectAll = computed({
  get: () => sortedEntries.value.length > 0 && sortedEntries.value.every((e) => selectedIds.value.has(e.id)),
  set: (value: boolean) => {
    if (value) {
      sortedEntries.value.forEach((e) => selectedIds.value.add(e.id));
    } else {
      selectedIds.value.clear();
    }
  },
});

const hasSelection = computed(() => selectedIds.value.size > 0);

// ─── Content Type Filter ──────────────────────────────────────
type ContentTypeFilter = "all" | "text" | "image" | "file";
const contentTypeFilter = ref<ContentTypeFilter>("all");

const contentTypeOptions: { value: ContentTypeFilter; label: string }[] = [
  { value: "all", label: "全部" },
  { value: "text", label: "文本" },
  { value: "image", label: "图片" },
  { value: "file", label: "文件" },
];

function setContentTypeFilter(type: ContentTypeFilter) {
  contentTypeFilter.value = type;
  selectedIds.value.clear();
}

// Sorting state
const sortKey = ref<string | null>(null);
const sortDir = ref<"asc" | "desc" | null>(null);

function toggleSort(key: string) {
  if (sortKey.value === key) {
    if (sortDir.value === "asc") {
      sortDir.value = "desc";
    } else if (sortDir.value === "desc") {
      sortKey.value = null;
      sortDir.value = null;
    }
  } else {
    sortKey.value = key;
    sortDir.value = "asc";
  }
}

const filteredEntries = computed(() => {
  let result = entries.value;

  // Content type filter
  if (contentTypeFilter.value !== "all") {
    result = result.filter((e) => e.content_type === contentTypeFilter.value);
  }

  // Search filter
  if (searchQuery.value.trim()) {
    const q = searchQuery.value.toLowerCase();
    result = result.filter(
      (e) => e.text_content?.toLowerCase().includes(q)
    );
  }

  return result;
});

const sortedEntries = computed(() => {
  const items = [...filteredEntries.value];
  if (!sortKey.value || !sortDir.value) return items;
  return items.sort((a: any, b: any) => {
    const valA = a[sortKey.value as keyof typeof a];
    const valB = b[sortKey.value as keyof typeof b];
    if (valA == null) return 1;
    if (valB == null) return -1;
    const cmp = typeof valA === "number" ? valA - valB : String(valA).localeCompare(String(valB));
    return sortDir.value === "asc" ? cmp : -cmp;
  });
});

// Virtual list for clipboard entries
const { list: virtualEntries, containerProps, wrapperProps } = useVirtualList(
  sortedEntries,
  {
    itemHeight: (index: number) => {
      const item = sortedEntries.value[index];
      if (!item) return 84;
      if (item.content_type === "image") return 164;
      if (item.content_type === "file" && item.file_paths && item.file_paths.length > 2) return 104;
      return 84;
    },
    overscan: 10,
  }
);

onMounted(async () => {
  try {
    await clipboardStart();
  } catch (e) {
    console.error("Failed to start clipboard:", e);
  }
  await loadEntries();

  try {
    intervalMs.value = await clipboardGetInterval();
  } catch (e) {
    console.error("Failed to get interval:", e);
  }

  try {
    unlistenNew = await onClipboardNew((entry) => {
      entries.value.unshift(entry);
    });
  } catch (e) {
    console.error("Failed to listen for clipboard events:", e);
  }
});

onUnmounted(() => {
  unlistenNew?.();
  clipboardStop().catch((e) => console.error("Failed to stop clipboard:", e));
});

async function loadEntries() {
  loading.value = true;
  try {
    entries.value = await clipboardList(undefined, 500);
  } catch (e) {
    console.error("Failed to load clipboard entries:", e);
  } finally {
    loading.value = false;
  }
}

async function toggleFavorite(id: string) {
  try {
    const newVal = await clipboardToggleFavorite(id);
    const entry = entries.value.find((e) => e.id === id);
    if (entry) entry.is_favorite = newVal;
  } catch (e) {
    console.error("Failed to toggle favorite:", e);
  }
}

async function copyEntry(id: string) {
  try {
    await clipboardCopy(id);
    copiedId.value = id;
    setTimeout(() => { copiedId.value = null; }, 2000);
  } catch (e) {
    console.error("Failed to copy:", e);
  }
}

async function deleteEntry(id: string) {
  try {
    await clipboardDelete(id);
    entries.value = entries.value.filter((e) => e.id !== id);
    selectedIds.value.delete(id);
  } catch (e) {
    console.error("Failed to delete:", e);
  }
}

async function clearAll() {
  if (!confirm("确定清空所有剪贴板历史？")) return;
  try {
    await clipboardClear();
    entries.value = [];
    selectedIds.value.clear();
  } catch (e) {
    console.error("Failed to clear:", e);
  }
}

async function changeInterval(ms: number) {
  try {
    await clipboardSetInterval(ms);
    intervalMs.value = ms;
  } catch (e) {
    console.error("Failed to set interval:", e);
  }
}

const intervalOptions = [
  { label: "0.5s", value: 500 },
  { label: "1s", value: 1000 },
  { label: "2s", value: 2000 },
  { label: "3s", value: 3000 },
  { label: "5s", value: 5000 },
];

function typeIcon(type: string) {
  if (type === "text") return FileText;
  if (type === "image") return Image;
  return File;
}

function onImageError(event: Event) {
  const img = event.target as HTMLImageElement;
  img.style.display = "none";
  const fallback = img.nextElementSibling;
  if (fallback) fallback.classList.remove("hidden");
}

// ─── Batch Operations ─────────────────────────────────────────
async function batchDelete() {
  const count = selectedIds.value.size;
  if (count === 0) return;
  if (!confirm(`确定删除选中的 ${count} 条记录？`)) return;
  try {
    await clipboardDeleteBatch(Array.from(selectedIds.value));
    entries.value = entries.value.filter((e) => !selectedIds.value.has(e.id));
    selectedIds.value.clear();
  } catch (e) {
    console.error("Failed to batch delete:", e);
  }
}

async function batchExport() {
  const count = selectedIds.value.size;
  if (count === 0) return;
  try {
    await clipboardExport(Array.from(selectedIds.value), "json");
  } catch (e) {
    console.error("Failed to export:", e);
  }
}

async function changeStorageLimit() {
  try {
    await clipboardSetLimit(storageLimit.value);
  } catch (e) {
    console.error("Failed to set storage limit:", e);
  }
}
</script>

<template>
  <div class="flex h-full flex-col animate-view-fade">
    <!-- Header -->
    <div class="border-b border-paper-deep/50 px-6 py-3">
      <div class="flex items-center justify-between">
        <h1 class="text-xl font-display font-bold text-ink">剪贴板管理</h1>
        <div class="flex items-center gap-2">
          <div class="relative">
            <Search class="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-ink-faint" />
            <input
              v-model="searchQuery"
              placeholder="搜索剪贴板内容..."
              class="w-56 rounded-lg border border-paper-deep bg-paper-warm/50 pl-9 pr-3 py-1.5 text-sm text-ink placeholder:text-ink-faint/50 outline-none"
            />
          </div>
          <Button variant="danger" size="sm" @click="clearAll">
            <Trash2 class="mr-1 h-3.5 w-3.5" />
            清空
          </Button>
        </div>
      </div>
    </div>

    <!-- Content type filter bar -->
    <div class="flex items-center gap-1 px-6 py-2 border-b border-paper-deep/20">
      <span class="text-xs font-medium text-ink-faint mr-2">类型筛选:</span>
      <button
        v-for="opt in contentTypeOptions"
        :key="opt.value"
        class="rounded-lg px-3 py-1 text-xs font-medium transition-colors"
        :class="contentTypeFilter === opt.value ? 'bg-bamboo/15 text-bamboo ring-1 ring-bamboo/30' : 'text-ink-soft hover:bg-paper-deep/30 hover:text-ink'"
        @click="setContentTypeFilter(opt.value)"
      >
        {{ opt.label }}
      </button>
    </div>

    <!-- Batch action bar -->
    <div
      v-if="hasSelection"
      class="flex items-center gap-3 px-6 py-2 bg-bamboo/5 border-b border-bamboo/20"
    >
      <span class="text-sm text-ink-soft">
        已选 {{ selectedIds.size }} 项
      </span>
      <div class="flex items-center gap-2 ml-auto">
        <Button variant="danger" size="sm" @click="batchDelete">
          <Trash2 class="mr-1 h-3.5 w-3.5" />
          批量删除
        </Button>
        <Button variant="outline" size="sm" @click="batchExport">
          <Download class="mr-1 h-3.5 w-3.5" />
          批量导出
        </Button>
      </div>
      <button
        class="text-xs text-ink-faint hover:text-ink transition-colors"
        @click="selectedIds.clear()"
      >
        取消选择
      </button>
    </div>

    <!-- Content -->
    <div class="flex-1 flex flex-col overflow-hidden p-4">
      <div v-if="loading" class="flex items-center justify-center h-full text-sm text-ink-faint">
        加载中...
      </div>

      <div v-else-if="sortedEntries.length === 0" class="flex items-center justify-center h-full text-sm text-ink-faint">
        <div class="text-center">
          <FileText class="mx-auto h-8 w-8 mb-2 opacity-40" />
          <p>暂无剪贴板记录</p>
          <p class="mt-1 text-xs opacity-60">复制任何内容后将自动记录在此</p>
        </div>
      </div>

      <div v-else v-bind="containerProps" class="flex-1 overflow-y-auto">
        <div v-bind="wrapperProps" class="space-y-2">
          <div
            v-for="{ data: entry } in virtualEntries"
            :key="entry.id"
            class="flex items-start gap-3 rounded-xl border border-paper-deep/20 bg-paper-warm/30 p-3 transition-colors hover:bg-paper-warm/60"
          >
            <!-- Checkbox -->
            <div class="mt-1 shrink-0">
              <button
                class="rounded p-0.5 transition-colors"
                :class="selectedIds.has(entry.id) ? 'text-bamboo' : 'text-ink-faint hover:text-ink-soft'"
                @click="toggleSelect(entry.id)"
                :aria-label="selectedIds.has(entry.id) ? '取消选择' : '选择'"
              >
                <CheckSquare v-if="selectedIds.has(entry.id)" class="h-4 w-4" />
                <Square v-else class="h-4 w-4" />
              </button>
            </div>

            <!-- Type icon -->
            <div class="mt-0.5 shrink-0 rounded-lg bg-paper-deep/20 p-2">
              <component :is="typeIcon(entry.content_type)" class="h-4 w-4 text-ink-soft" />
            </div>

            <!-- Content -->
            <div class="flex-1 min-w-0">
              <div v-if="entry.content_type === 'text' && entry.text_content" class="text-sm text-ink whitespace-pre-wrap break-words">
                {{ truncate(entry.text_content, 200) }}
              </div>
              <div v-else-if="entry.content_type === 'image' && entry.image_path" class="text-sm text-ink-soft">
                <img
                  :src="convertFileSrc(entry.image_path)"
                  :alt="entry.image_path.split('/').pop() || 'image'"
                  class="h-24 w-auto rounded-lg object-cover"
                  @error="onImageError"
                />
                <p class="mt-1 truncate">
                  {{ entry.image_path.split('/').pop() || entry.image_path }}
                </p>
              </div>
              <div v-else-if="entry.content_type === 'file' && entry.file_paths" class="text-sm text-ink-soft">
                <p v-for="f in entry.file_paths" :key="f" class="truncate">{{ f }}</p>
              </div>
              <div class="mt-1 flex items-center gap-2 text-xs text-ink-faint">
                <span>{{ formatTime(entry.created_at) }}</span>
              </div>
            </div>

            <!-- Actions -->
            <div class="flex shrink-0 items-center gap-1">
              <button
                class="rounded-lg p-1.5 transition-colors"
                :class="entry.is_favorite ? 'text-yellow-500' : 'text-ink-faint hover:text-yellow-500'"
                @click="toggleFavorite(entry.id)"
                :title="entry.is_favorite ? '取消收藏' : '收藏'"
                :aria-label="entry.is_favorite ? '取消收藏' : '收藏'"
              >
                <Star class="h-4 w-4" :fill="entry.is_favorite ? 'currentColor' : 'none'" />
              </button>
              <button
                class="rounded-lg p-1.5 text-ink-faint transition-colors hover:text-bamboo"
                @click="copyEntry(entry.id)"
                :title="copiedId === entry.id ? '已复制!' : '复制'"
                :aria-label="copiedId === entry.id ? '已复制' : '复制到剪贴板'"
              >
                <Copy class="h-4 w-4" />
              </button>
              <button
                class="rounded-lg p-1.5 text-ink-faint transition-colors hover:text-red-500"
                @click="deleteEntry(entry.id)"
                title="删除"
                aria-label="删除"
              >
                <X class="h-4 w-4" />
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Footer -->
    <div class="border-t border-paper-deep/50 px-6 py-2 text-xs text-ink-faint flex items-center justify-between">
      <div class="flex items-center gap-3">
        <span>{{ entries.length }} 条记录 · 自动监听中</span>
        <!-- Select all checkbox -->
        <label class="flex items-center gap-1 cursor-pointer" v-if="sortedEntries.length > 0">
          <button
            class="rounded p-0.5 transition-colors"
            :class="selectAll ? 'text-bamboo' : 'text-ink-faint hover:text-ink-soft'"
            @click="selectAll = !selectAll"
            aria-label="全选"
          >
            <CheckSquare v-if="selectAll" class="h-3.5 w-3.5" />
            <Square v-else class="h-3.5 w-3.5" />
          </button>
          <span class="text-ink-faint/70">全选</span>
        </label>
      </div>
      <div class="flex items-center gap-3">
        <!-- Storage limit -->
        <div class="flex items-center gap-1.5">
          <span class="text-ink-faint/70">保留:</span>
          <input
            v-model.number="storageLimit"
            type="number"
            min="100"
            max="10000"
            class="w-16 rounded border border-paper-deep/30 bg-paper-warm/50 px-1.5 py-0.5 text-xs text-ink outline-none text-center"
            @change="changeStorageLimit"
          />
          <span class="text-ink-faint/50">条</span>
        </div>
        <button
          class="flex items-center gap-1 rounded px-2 py-0.5 transition-colors hover:bg-paper-deep/50"
          @click="toggleSort('created_at')"
          :title="sortDir === 'asc' ? '最早在前' : '最新在前'"
        >
          时间
          <ArrowUp v-if="sortKey === 'created_at' && sortDir === 'asc'" class="h-3 w-3" />
          <ArrowDown v-else-if="sortKey === 'created_at' && sortDir === 'desc'" class="h-3 w-3" />
        </button>
        <span class="text-ink-faint/70">检测间隔:</span>
        <div class="flex gap-1">
          <button
            v-for="opt in intervalOptions"
            :key="opt.value"
            class="rounded px-2 py-0.5 transition-colors"
            :class="intervalMs === opt.value ? 'bg-bamboo/20 text-bamboo' : 'hover:bg-paper-deep/50 text-ink-faint'"
            @click="changeInterval(opt.value)"
          >
            {{ opt.label }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
