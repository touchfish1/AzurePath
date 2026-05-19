<script setup lang="ts">
import { ref, onMounted } from "vue";
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
} from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import {
  clipboardList,
  snifferList,
  discoveryPeers,
  type ClipboardEntry,
} from "@/lib/tauri";

const router = useRouter();

const clipboardCount = ref(0);
const deviceCount = ref(0);
const peerCount = ref(0);
const recentEntries = ref<ClipboardEntry[]>([]);
const loading = ref(true);

interface StatCard {
  label: string;
  value: number;
  icon: object;
  color: string;
  bgClass: string;
  route: string;
}

const stats = ref<StatCard[]>([]);

onMounted(async () => {
  loading.value = true;
  try {
    const [entries, devices, peers] = await Promise.all([
      clipboardList().catch(() => [] as ClipboardEntry[]),
      snifferList().catch(() => []),
      discoveryPeers().catch(() => []),
    ]);

    clipboardCount.value = entries.length;
    deviceCount.value = devices.length;
    peerCount.value = peers.length;

    recentEntries.value = entries.slice(0, 8);

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

function formatTime(iso: string): string {
  try {
    const d = new Date(iso);
    const now = new Date();
    const diffMs = now.getTime() - d.getTime();
    const diffMin = Math.floor(diffMs / 60000);

    if (diffMin < 1) return "刚刚";
    if (diffMin < 60) return `${diffMin} 分钟前`;

    const diffHour = Math.floor(diffMin / 60);
    if (diffHour < 24) return `${diffHour} 小时前`;

    const diffDay = Math.floor(diffHour / 24);
    if (diffDay < 7) return `${diffDay} 天前`;

    return d.toLocaleDateString("zh-CN");
  } catch {
    return iso;
  }
}

function truncate(text: string, len: number): string {
  return text.length > len ? text.slice(0, len) + "..." : text;
}

function goTo(path: string) {
  router.push(path);
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

    <!-- Content -->
    <div class="flex-1 overflow-y-auto p-6">
      <!-- Loading state -->
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
              <div
                class="flex h-10 w-10 items-center justify-center rounded-lg"
                :class="[stat.bgClass, stat.color]"
              >
                <component :is="stat.icon" class="h-5 w-5" />
              </div>
              <ArrowRight
                class="h-4 w-4 text-ink-ghost transition-all group-hover:text-bamboo group-hover:translate-x-0.5"
              />
            </div>
            <p class="mt-4 text-2xl font-display font-bold text-ink">{{ stat.value }}</p>
            <p class="mt-0.5 text-sm text-ink-faint">{{ stat.label }}</p>
          </div>
        </div>

        <!-- Recent clipboard entries feed -->
        <div class="mt-8">
          <div class="mb-4 flex items-center justify-between">
            <div class="flex items-center gap-2">
              <Clipboard class="h-4 w-4 text-ink-soft" />
              <h2 class="text-base font-display font-semibold text-ink">最近剪贴板</h2>
            </div>
            <Button variant="ghost" size="sm" @click="goTo('/clipboard')">
              查看全部
              <ArrowRight class="ml-1 h-3.5 w-3.5" />
            </Button>
          </div>

          <!-- Empty -->
          <div
            v-if="recentEntries.length === 0"
            class="flex items-center justify-center rounded-xl border border-dashed border-paper-deep/30 py-12 text-sm text-ink-faint"
          >
            <div class="text-center">
              <Clipboard class="mx-auto h-6 w-6 mb-2 opacity-40" />
              <p>暂无剪贴板记录</p>
              <p class="mt-1 text-xs opacity-60">复制内容后将在剪贴板页面自动记录</p>
            </div>
          </div>

          <!-- Feed items -->
          <div v-else class="space-y-2">
            <div
              v-for="entry in recentEntries"
              :key="entry.id"
              class="flex items-start gap-3 rounded-xl border border-paper-deep/20 bg-paper-warm/30 p-3 transition-colors hover:bg-paper-warm/60"
            >
              <!-- Type icon -->
              <div class="mt-0.5 shrink-0 rounded-lg bg-paper-deep/20 p-2">
                <component :is="typeIcon(entry.content_type)" class="h-4 w-4 text-ink-soft" />
              </div>

              <!-- Content -->
              <div class="min-w-0 flex-1">
                <div
                  v-if="entry.content_type === 'text' && entry.text_content"
                  class="truncate text-sm text-ink"
                >
                  {{ truncate(entry.text_content, 100) }}
                </div>
                <div
                  v-else-if="entry.content_type === 'image'"
                  class="truncate text-sm text-ink-soft"
                >
                  <span>图片</span>
                </div>
                <div
                  v-else-if="entry.content_type === 'file' && entry.file_paths"
                  class="truncate text-sm text-ink-soft"
                >
                  <span>{{ entry.file_paths.join(", ") }}</span>
                </div>
                <div class="mt-1 text-xs text-ink-faint">
                  {{ formatTime(entry.created_at) }}
                </div>
              </div>

              <!-- Favorite badge -->
              <div
                v-if="entry.is_favorite"
                class="shrink-0 self-center rounded-md bg-yellow-500/10 px-2 py-0.5 text-xs text-yellow-600"
              >
                收藏
              </div>
            </div>
          </div>
        </div>
      </template>
    </div>
  </div>
</template>
