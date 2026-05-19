<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useVirtualList } from "@vueuse/core";
import { useRouter } from "vue-router";
import {
  Clipboard,
  Radio,
  Users,
  Activity,
  History,
  ArrowRight,
  FileText,
  Image,
  File,
  Star,
  Clock,
  Search,
  Trash2,
  ArrowUp,
  ArrowDown,
} from "lucide-vue-next";
import { formatTime, truncate } from "@/lib/format";
import Button from "@/components/ui/button/Button.vue";
import {
  clipboardList,
  snifferList,
  discoveryPeers,
  clipboardDelete,
  clipboardClear,
  type ClipboardEntry,
} from "@/lib/tauri";

const router = useRouter();

type Tab = "all" | "favorites" | "timeline";
const activeTab = ref<Tab>("all");
const searchQuery = ref("");

const clipboardCount = ref(0);
const deviceCount = ref(0);
const peerCount = ref(0);
const allEntries = ref<ClipboardEntry[]>([]);
const loading = ref(true);
const selectedIds = ref<Set<string>>(new Set());

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

interface StatCard {
  label: string;
  value: number;
  icon: object;
  color: string;
  bgClass: string;
  route: string;
}

const stats = ref<StatCard[]>([]);

const filteredEntries = computed(() => {
  let entries = allEntries.value;

  if (activeTab.value === "favorites") {
    entries = entries.filter((e) => e.is_favorite);
  }

  if (searchQuery.value.trim()) {
    const q = searchQuery.value.toLowerCase();
    entries = entries.filter((e) => {
      if (e.content_type === "text" && e.text_content?.toLowerCase().includes(q)) return true;
      if (e.file_paths?.some((p) => p.toLowerCase().includes(q))) return true;
      return false;
    });
  }

  return entries;
});

// Virtual list for large entry lists
const { list: virtualEntries, containerProps, wrapperProps } = useVirtualList(
  sortedEntries,
  { itemHeight: 80, overscan: 10 }
);

interface TimelineGroup {
  label: string;
  entries: ClipboardEntry[];
}

const timelineGroups = computed(() => {
  const entries = filteredEntries.value;
  const groups: TimelineGroup[] = [];
  const now = new Date();
  const todayStart = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const yesterdayStart = new Date(todayStart.getTime() - 86400000);
  const weekStart = new Date(todayStart.getTime() - todayStart.getDay() * 86400000);

  const buckets: Record<string, ClipboardEntry[]> = { today: [], yesterday: [], week: [], older: [] };

  for (const entry of entries) {
    const d = new Date(entry.created_at);
    if (d >= todayStart) buckets.today.push(entry);
    else if (d >= yesterdayStart) buckets.yesterday.push(entry);
    else if (d >= weekStart) buckets.week.push(entry);
    else buckets.older.push(entry);
  }

  if (buckets.today.length) groups.push({ label: "今天", entries: buckets.today });
  if (buckets.yesterday.length) groups.push({ label: "昨天", entries: buckets.yesterday });
  if (buckets.week.length) groups.push({ label: "本周", entries: buckets.week });
  if (buckets.older.length) groups.push({ label: "更早", entries: buckets.older });

  return groups;
});

const tabs = [
  { key: "all" as Tab, label: "全部活动", icon: Activity },
  { key: "favorites" as Tab, label: "收藏", icon: Star },
  { key: "timeline" as Tab, label: "时间线", icon: Clock },
];

onMounted(async () => {
  loading.value = true;
  try {
    const [entries, devices, peers] = await Promise.all([
      clipboardList().catch(() => [] as ClipboardEntry[]),
      snifferList().catch(() => []),
      discoveryPeers().catch(() => []),
    ]);

    allEntries.value = entries;
    clipboardCount.value = entries.length;
    deviceCount.value = devices.length;
    peerCount.value = peers.length;

    stats.value = [
      {
        label: "剪贴板记录",
        value: entries.length,
        icon: Clipboard,
        color: "text-bamboo",
        bgClass: "bg-bamboo/10",
        route: "/clipboard",
      },
      {
        label: "已发现设备",
        value: devices.length,
        icon: Radio,
        color: "text-sky-500",
        bgClass: "bg-sky-500/10",
        route: "/network-sniffer",
      },
      {
        label: "活跃节点",
        value: peers.length,
        icon: Users,
        color: "text-amber-500",
        bgClass: "bg-amber-500/10",
        route: "/chat",
      },
    ];
  } catch (e) {
    console.error("Failed to load activity data:", e);
  } finally {
    loading.value = false;
  }
});

function typeIcon(type: string) {
  if (type === "text") return FileText;
  if (type === "image") return Image;
  return File;
}

function formatFullTime(iso: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleString("zh-CN", { month: "numeric", day: "numeric", hour: "2-digit", minute: "2-digit" });
  } catch {
    return iso;
  }
}

function goTo(path: string) {
  router.push(path);
}

function toggleSelect(id: string) {
  const next = new Set(selectedIds.value);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  selectedIds.value = next;
}

async function deleteSelected() {
  for (const id of selectedIds.value) {
    await clipboardDelete(id).catch(() => {});
  }
  allEntries.value = allEntries.value.filter((e) => !selectedIds.value.has(e.id));
  selectedIds.value = new Set();
}

async function clearAll() {
  await clipboardClear().catch(() => {});
  allEntries.value = [];
  selectedIds.value = new Set();
}
</script>

<template>
  <div class="flex h-full flex-col animate-view-fade">
    <!-- Header -->
    <div class="border-b border-paper-deep/50 px-6 py-4">
      <div class="flex items-center gap-3">
        <div class="flex h-10 w-10 items-center justify-center rounded-xl bg-bamboo/10 text-bamboo">
          <History class="h-5 w-5" />
        </div>
        <div>
          <h1 class="text-xl font-display font-bold text-ink">最近活动</h1>
          <p class="mt-0.5 text-sm text-ink-faint">浏览应用内各功能的最近动态与统计数据</p>
        </div>
      </div>
    </div>

    <!-- Tabs + Search bar -->
    <div class="flex items-center justify-between border-b border-paper-deep/30 px-6 py-2">
      <div class="flex gap-1">
        <button
          v-for="tab in tabs"
          :key="tab.key"
          class="flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-sm font-medium transition-colors"
          :class="activeTab === tab.key ? 'bg-bamboo/10 text-bamboo' : 'text-ink-faint hover:text-ink hover:bg-paper-deep/20'"
          @click="activeTab = tab.key"
        >
          <component :is="tab.icon" class="h-4 w-4" />
          {{ tab.label }}
        </button>
      </div>

      <div v-if="activeTab !== 'timeline'" class="relative flex items-center">
        <Search class="absolute left-2.5 h-3.5 w-3.5 text-ink-faint" />
        <input
          v-model="searchQuery"
          type="text"
          placeholder="搜索剪贴板..."
          class="h-8 rounded-lg border border-paper-deep/30 bg-paper-warm/30 pl-8 pr-3 text-xs text-ink outline-none transition-colors placeholder:text-ink-faint focus:border-bamboo/40 focus:bg-paper-warm/50"
        />
      </div>
    </div>

    <!-- Content -->
    <div class="flex-1 flex flex-col overflow-hidden p-6">
      <div v-if="loading" class="flex items-center justify-center h-full text-sm text-ink-faint">
        <div class="flex flex-col items-center gap-2">
          <Activity class="h-5 w-5 animate-pulse text-bamboo" />
          <span>加载中...</span>
        </div>
      </div>

      <template v-else>
        <!-- Summary cards -->
        <div class="grid grid-cols-1 gap-4 sm:grid-cols-3">
          <div
            v-for="stat in stats"
            :key="stat.label"
            class="group noise-bg cursor-pointer rounded-xl border border-paper-deep/20 bg-paper-warm/30 p-5 shadow-sm transition-all hover:border-bamboo/25 hover:shadow-md hover:-translate-y-0.5"
            @click="goTo(stat.route)"
          >
            <div class="flex items-start justify-between">
              <div class="flex h-10 w-10 items-center justify-center rounded-lg" :class="[stat.bgClass, stat.color]">
                <component :is="stat.icon" class="h-5 w-5" />
              </div>
              <ArrowRight class="h-4 w-4 text-ink-ghost transition-all group-hover:text-bamboo group-hover:translate-x-0.5" />
            </div>
            <p class="mt-4 text-2xl font-display font-bold text-ink">{{ stat.value }}</p>
            <p class="mt-0.5 text-sm text-ink-faint">{{ stat.label }}</p>
          </div>
        </div>

        <!-- Timeline tab -->
        <div v-if="activeTab === 'timeline'" class="mt-8 space-y-8 overflow-y-auto flex-1">
          <div v-if="timelineGroups.length === 0" class="flex items-center justify-center rounded-xl border border-dashed border-paper-deep/30 py-12 text-sm text-ink-faint">
            <div class="text-center">
              <Clock class="mx-auto h-6 w-6 mb-2 opacity-40" />
              <p>暂无时间线记录</p>
            </div>
          </div>
          <div v-for="group in timelineGroups" :key="group.label">
            <div class="mb-3 flex items-center gap-2">
              <div class="h-px flex-1 bg-paper-deep/30" />
              <span class="shrink-0 text-xs font-medium text-ink-soft">{{ group.label }}</span>
              <div class="h-px flex-1 bg-paper-deep/30" />
            </div>
            <div class="space-y-2">
              <div
                v-for="entry in group.entries"
                :key="entry.id"
                class="flex items-start gap-3 rounded-xl border border-paper-deep/20 bg-paper-warm/30 p-3 transition-colors hover:bg-paper-warm/60"
              >
                <div class="mt-0.5 shrink-0 rounded-lg bg-paper-deep/20 p-2">
                  <component :is="typeIcon(entry.content_type)" class="h-4 w-4 text-ink-soft" />
                </div>
                <div class="min-w-0 flex-1">
                  <div v-if="entry.content_type === 'text' && entry.text_content" class="truncate text-sm text-ink">{{ truncate(entry.text_content, 100) }}</div>
                  <div v-else-if="entry.content_type === 'image'" class="truncate text-sm text-ink-soft"><span>图片</span></div>
                  <div v-else-if="entry.content_type === 'file' && entry.file_paths" class="truncate text-sm text-ink-soft"><span>{{ entry.file_paths.join(", ") }}</span></div>
                  <div class="mt-1 text-xs text-ink-faint">{{ formatFullTime(entry.created_at) }}</div>
                </div>
                <div v-if="entry.is_favorite" class="shrink-0 self-center rounded-md bg-yellow-500/10 px-2 py-0.5 text-xs text-yellow-600">
                  <Star class="inline-block h-3 w-3 fill-yellow-500" />
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Favorites / All entries tab -->
        <div v-else class="mt-8 flex flex-col flex-1 overflow-hidden">
          <div class="mb-3 flex items-center justify-between shrink-0">
            <p class="text-xs text-ink-faint">
              {{ sortedEntries.length }} 条记录
              <span v-if="selectedIds.size > 0" class="ml-2 text-bamboo">{{ selectedIds.size }} 已选择</span>
            </p>
            <div class="flex items-center gap-2">
              <button
                class="flex items-center gap-1 rounded px-2 py-0.5 text-xs text-ink-faint transition-colors hover:bg-paper-deep/50 hover:text-ink"
                @click="toggleSort('created_at')"
                :title="sortDir === 'asc' ? '最早在前' : '最新在前'"
              >
                时间
                <ArrowUp v-if="sortKey === 'created_at' && sortDir === 'asc'" class="h-3 w-3" />
                <ArrowDown v-else-if="sortKey === 'created_at' && sortDir === 'desc'" class="h-3 w-3" />
              </button>
            <div class="flex gap-2">
              <Button v-if="selectedIds.size > 0" variant="danger" size="sm" @click="deleteSelected">
                <Trash2 class="mr-1 h-3.5 w-3.5" />删除选中
              </Button>
              <Button v-if="allEntries.length > 0 && selectedIds.size === 0" variant="ghost" size="sm" @click="clearAll">
                <Trash2 class="mr-1 h-3.5 w-3.5" />清空全部
              </Button>
            </div>
            </div>
          </div>

          <div v-if="sortedEntries.length === 0" class="flex items-center justify-center rounded-xl border border-dashed border-paper-deep/30 py-12 text-sm text-ink-faint">
            <div class="text-center">
              <component :is="activeTab === 'favorites' ? Star : Clipboard" class="mx-auto h-6 w-6 mb-2 opacity-40" />
              <p>{{ activeTab === 'favorites' ? '暂无收藏内容' : '暂无剪贴板记录' }}</p>
              <p class="mt-1 text-xs opacity-60">{{ activeTab === 'favorites' ? '在剪贴板页面点击星标即可收藏' : '复制内容后将在剪贴板页面自动记录' }}</p>
            </div>
          </div>

          <div v-else v-bind="containerProps" class="flex-1 overflow-y-auto">
            <div v-bind="wrapperProps" class="space-y-2">
              <div
                v-for="{ data: entry } in virtualEntries"
                :key="entry.id"
                class="flex items-start gap-3 rounded-xl border border-paper-deep/20 bg-paper-warm/30 p-3 transition-colors hover:bg-paper-warm/60"
                :class="{ 'border-bamboo/30 bg-bamboo/5': selectedIds.has(entry.id) }"
              >
                <div class="mt-1 shrink-0">
                  <input type="checkbox" :checked="selectedIds.has(entry.id)" class="h-4 w-4 rounded border-paper-deep/40 text-bamboo focus:ring-bamboo/30" @change="toggleSelect(entry.id)" />
                </div>
                <div class="mt-0.5 shrink-0 rounded-lg bg-paper-deep/20 p-2">
                  <component :is="typeIcon(entry.content_type)" class="h-4 w-4 text-ink-soft" />
                </div>
                <div class="min-w-0 flex-1">
                  <div v-if="entry.content_type === 'text' && entry.text_content" class="truncate text-sm text-ink">{{ truncate(entry.text_content, 120) }}</div>
                  <div v-else-if="entry.content_type === 'image'" class="truncate text-sm text-ink-soft"><span>图片</span></div>
                  <div v-else-if="entry.content_type === 'file' && entry.file_paths" class="truncate text-sm text-ink-soft"><span>{{ entry.file_paths.join(", ") }}</span></div>
                  <div class="mt-1 text-xs text-ink-faint">{{ formatTime(entry.created_at) }}</div>
                </div>
                <div v-if="entry.is_favorite" class="shrink-0 self-center rounded-md bg-yellow-500/10 px-2 py-0.5 text-xs text-yellow-600">
                  <Star class="inline-block h-3 w-3 fill-yellow-500" />
                </div>
              </div>
            </div>
          </div>
        </div>
      </template>
    </div>
  </div>
</template>
