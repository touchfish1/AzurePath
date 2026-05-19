<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from "vue";
import { useVirtualList } from "@vueuse/core";
import { convertFileSrc } from "@tauri-apps/api/core";
import { Search, Trash2, Star, Copy, FileText, Image, File, X } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import {
  clipboardStart,
  clipboardStop,
  clipboardList,
  clipboardDelete,
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
let unlistenNew: UnlistenFn | null = null;

const filteredEntries = computed(() => {
  if (!searchQuery.value.trim()) return entries.value;
  const q = searchQuery.value.toLowerCase();
  return entries.value.filter(
    (e) => e.text_content?.toLowerCase().includes(q)
  );
});

// Virtual list for clipboard entries
const { list: virtualEntries, containerProps, wrapperProps } = useVirtualList(
  filteredEntries,
  {
    itemHeight: (index: number) => {
      const item = filteredEntries.value[index];
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
  } catch (e) {
    console.error("Failed to delete:", e);
  }
}

async function clearAll() {
  if (!confirm("确定清空所有剪贴板历史？")) return;
  try {
    await clipboardClear();
    entries.value = [];
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

    <!-- Content -->
    <div class="flex-1 flex flex-col overflow-hidden p-4">
      <div v-if="loading" class="flex items-center justify-center h-full text-sm text-ink-faint">
        加载中...
      </div>

      <div v-else-if="filteredEntries.length === 0" class="flex items-center justify-center h-full text-sm text-ink-faint">
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
      <span>{{ entries.length }} 条记录 · 自动监听中</span>
      <div class="flex items-center gap-2">
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
