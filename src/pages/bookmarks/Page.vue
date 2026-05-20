<script setup lang="ts">
import { onMounted, watch } from "vue";
import { Star, Trash2, Search } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import { useBookmarkStore } from "@/stores/bookmark";
import { useToastStore } from "@/stores/toast";
import { ref, computed } from "vue";
import { useDebounceFn } from "@vueuse/core";

const store = useBookmarkStore();
const toast = useToastStore();
const searchQuery = ref("");
const debouncedSearchQuery = ref("");
const searchDebounced = useDebounceFn((v: string) => { debouncedSearchQuery.value = v; }, 250);
watch(searchQuery, (v) => searchDebounced(v));

const filteredBookmarks = computed(() => {
  if (!debouncedSearchQuery.value.trim()) return store.bookmarks;
  const q = debouncedSearchQuery.value.toLowerCase();
  return store.bookmarks.filter(
    (b) => b.label.toLowerCase().includes(q) || b.target.toLowerCase().includes(q),
  );
});

async function handleDelete(id: string) {
  await store.remove(id);
}

function copyTarget(target: string) {
  navigator.clipboard.writeText(target).then(() => {
    toast.add("success", "已复制");
  });
}

onMounted(async () => {
  await store.loadBookmarks();
});
</script>

<template>
  <div class="flex h-full flex-col p-4 md:p-6 space-y-4 md:space-y-6 animate-view-fade">
    <!-- Header -->
    <div>
      <h1 class="text-2xl font-display font-bold text-ink">书签</h1>
      <p class="mt-0.5 text-sm text-ink-faint">管理常用目标和地址</p>
    </div>

    <!-- Search -->
    <div class="relative">
      <Search class="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-ink-faint" />
      <input
        v-model="searchQuery"
        placeholder="搜索书签..."
        class="w-full rounded-xl border border-paper-deep/60 bg-paper pl-10 pr-4 py-3 text-sm text-ink placeholder:text-ink-faint/50 outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20 noise-bg"
      />
    </div>

    <!-- Loading state -->
    <div
      v-if="store.loading"
      class="flex items-center justify-center py-16 text-sm text-ink-faint"
    >
      <span class="animate-pulse">加载中...</span>
    </div>

    <!-- Empty state -->
    <div
      v-else-if="store.bookmarks.length === 0"
      class="flex items-center justify-center rounded-xl border border-dashed border-paper-deep/30 bg-paper-warm/20 py-16 text-sm text-ink-faint"
    >
      <div class="text-center max-w-sm">
        <Star class="mx-auto h-10 w-10 mb-3 opacity-30" />
        <p class="font-medium text-ink-soft">暂无书签</p>
        <p class="mt-2 text-xs opacity-60 leading-relaxed">
          在 Ping、Traceroute、DNS 等页面使用书签按钮
          <br />
          快速保存常用目标地址
        </p>
      </div>
    </div>

    <!-- Bookmarks list -->
    <div
      v-else
      class="noise-bg rounded-xl border border-paper-deep/60 bg-paper shadow-sm overflow-hidden"
    >
      <div class="overflow-x-auto">
        <table class="w-full text-sm">
          <thead>
            <tr class="border-b border-paper-deep/30 text-xs text-ink-faint uppercase tracking-wider">
              <th class="px-5 py-3 text-left font-medium">标签</th>
              <th class="px-5 py-3 text-left font-medium">目标</th>
              <th class="px-5 py-3 text-left font-medium">创建时间</th>
              <th class="px-5 py-3 text-left font-medium w-24">操作</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="bm in filteredBookmarks"
              :key="bm.id"
              class="border-b border-paper-deep/20 last:border-0 animate-slide-up hover:bg-paper-warm/30 transition-colors"
            >
              <td class="px-5 py-3">
                <div class="flex items-center gap-2">
                  <Star class="h-3.5 w-3.5 text-yellow-500 shrink-0" />
                  <span class="text-ink font-medium">{{ bm.label }}</span>
                </div>
              </td>
              <td class="px-5 py-3 font-mono text-ink-soft">
                <button
                  class="hover:text-bamboo transition-colors"
                  @click="copyTarget(bm.target)"
                  :title="'复制 ' + bm.target"
                >
                  {{ bm.target }}
                </button>
              </td>
              <td class="px-5 py-3 text-ink-faint text-xs">
                {{ new Date(bm.createdAt).toLocaleString("zh-CN") }}
              </td>
              <td class="px-5 py-3">
                <div class="flex gap-1">
                  <Button
                    variant="danger"
                    size="sm"
                    @click="handleDelete(bm.id)"
                  >
                    <Trash2 class="h-3 w-3" />
                  </Button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- Stats -->
    <div
      v-if="store.bookmarks.length > 0"
      class="text-xs text-ink-faint text-center"
    >
      共 {{ store.bookmarks.length }} 个书签
      <template v-if="searchQuery">（筛选后 {{ filteredBookmarks.length }} 个）</template>
    </div>
  </div>
</template>
