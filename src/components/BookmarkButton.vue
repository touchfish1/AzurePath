<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { Star, Bookmark } from "lucide-vue-next";
import { useBookmarkStore } from "@/stores/bookmark";

const props = defineProps<{
  target: string;
}>();

const emit = defineEmits<{
  select: [target: string];
}>();

const store = useBookmarkStore();
const open = ref(false);
const showAddForm = ref(false);
const newLabel = ref("");
const popoverRef = ref<HTMLElement | null>(null);

onMounted(async () => {
  await store.loadBookmarks();
});

const isAlreadyBookmarked = computed(() => {
  return store.bookmarks.some((b) => b.target === props.target);
});

function toggle() {
  open.value = !open.value;
  if (!open.value) {
    showAddForm.value = false;
    newLabel.value = "";
  }
}

function handleSelect(target: string) {
  emit("select", target);
  open.value = false;
}

async function handleAdd() {
  const label = newLabel.value.trim() || props.target;
  await store.add(label, props.target);
  showAddForm.value = false;
  newLabel.value = "";
}

async function handleDelete(id: string, event: Event) {
  event.stopPropagation();
  await store.remove(id);
}

function onBackdropClick(event: MouseEvent) {
  if (popoverRef.value && !popoverRef.value.contains(event.target as Node)) {
    open.value = false;
    showAddForm.value = false;
    newLabel.value = "";
  }
}
</script>

<template>
  <div class="relative">
    <button
      class="rounded-lg p-2 text-ink-faint transition-colors hover:text-yellow-500 hover:bg-yellow-500/5"
      title="书签"
      @click="toggle"
    >
      <Star
        class="h-4 w-4"
        :class="isAlreadyBookmarked ? 'fill-yellow-500 text-yellow-500' : ''"
      />
    </button>

    <!-- Backdrop -->
    <div
      v-if="open"
      class="fixed inset-0 z-40"
      @click="onBackdropClick"
    />

    <!-- Popover -->
    <div
      v-if="open"
      ref="popoverRef"
      class="absolute right-0 top-full z-50 mt-1 w-72 rounded-xl border border-paper-deep/60 bg-paper p-4 shadow-lg noise-bg"
    >
      <!-- Add current as bookmark -->
      <div v-if="!showAddForm" class="space-y-3">
        <div class="flex items-center justify-between">
          <span class="text-xs font-semibold text-ink">书签</span>
          <button
            v-if="props.target && !isAlreadyBookmarked"
            class="text-xs text-bamboo hover:text-bamboo-light transition-colors"
            @click="showAddForm = true"
          >
            + 添加当前
          </button>
        </div>

        <!-- Bookmark list -->
        <div v-if="store.bookmarks.length === 0" class="py-4 text-center text-xs text-ink-faint">
          暂无书签
        </div>
        <div v-else class="max-h-60 overflow-y-auto space-y-1">
          <button
            v-for="bm in store.bookmarks"
            :key="bm.id"
            class="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-sm transition-colors hover:bg-paper-deep/50"
            @click="handleSelect(bm.target)"
          >
            <Bookmark class="h-3.5 w-3.5 shrink-0 text-yellow-500" />
            <div class="flex-1 min-w-0">
              <div class="truncate text-ink font-medium">{{ bm.label }}</div>
              <div class="truncate text-xs text-ink-faint font-mono">{{ bm.target }}</div>
            </div>
            <span
              class="shrink-0 rounded p-1 text-ink-faint hover:text-red-500 hover:bg-red-500/5 transition-colors cursor-pointer"
              role="button"
              tabindex="0"
              title="删除"
              @click="handleDelete(bm.id, $event)"
              @keydown.enter="handleDelete(bm.id, $event)"
            >
              <span class="text-xs">&times;</span>
            </span>
          </button>
        </div>
      </div>

      <!-- Add form -->
      <div v-else class="space-y-3">
        <div class="flex items-center justify-between">
          <span class="text-xs font-semibold text-ink">添加书签</span>
          <button
            class="text-xs text-ink-faint hover:text-ink transition-colors"
            @click="showAddForm = false"
          >
            取消
          </button>
        </div>
        <div>
          <label class="mb-1 block text-xs text-ink-faint">标签</label>
          <input
            v-model="newLabel"
            :placeholder="props.target"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink placeholder:text-ink-faint/50 outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
            @keyup.enter="handleAdd"
          />
        </div>
        <div>
          <label class="mb-1 block text-xs text-ink-faint">目标</label>
          <input
            :value="props.target"
            disabled
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/30 px-3 py-2 text-sm text-ink/70 font-mono outline-none"
          />
        </div>
        <button
          class="w-full rounded-lg bg-bamboo px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-bamboo-light"
          @click="handleAdd"
        >
          保存
        </button>
      </div>
    </div>
  </div>
</template>
