<script setup lang="ts">
import { ref, watch, computed } from "vue";
import {
  Folder,
  File,
  FileText,
  FileImage,
  FileCode,
  FileArchive,
  ChevronRight,
  ArrowLeft,
  RotateCw,
  Download,
  HardDrive,
} from "lucide-vue-next";
import {
  remoteShellListSftp,
  type SftpEntry,
} from "@/lib/tauri";
import { formatSize, formatTime } from "@/lib/format";

interface Props {
  sessionId: string | null;
}

const props = defineProps<Props>();

const entries = ref<SftpEntry[]>([]);
const currentPath = ref("/");
const loading = ref(false);
const error = ref("");

const breadcrumbs = computed(() => {
  const parts = currentPath.value.split("/").filter(Boolean);
  const crumbs: { label: string; path: string }[] = [];
  let accumulated = "";
  for (const part of parts) {
    accumulated += "/" + part;
    crumbs.push({ label: part, path: accumulated });
  }
  return crumbs;
});

async function loadDir(path: string) {
  if (!props.sessionId) return;
  loading.value = true;
  error.value = "";
  try {
    entries.value = await remoteShellListSftp(props.sessionId, path);
    currentPath.value = path;
  } catch (e) {
    error.value = String(e);
    entries.value = [];
  } finally {
    loading.value = false;
  }
}

function navigateToDir(entry: SftpEntry) {
  if (entry.isDir) {
    loadDir(entry.path);
  }
}

function goBack() {
  const parts = currentPath.value.split("/").filter(Boolean);
  if (parts.length <= 1) {
    loadDir("/");
  } else {
    parts.pop();
    loadDir("/" + parts.join("/"));
  }
}

function navigateCrumb(path: string) {
  loadDir(path);
}

function fileIcon(entry: SftpEntry) {
  if (entry.isDir) return Folder;
  const ext = entry.name.split(".").pop()?.toLowerCase() || "";
  if (["jpg", "jpeg", "png", "gif", "bmp", "svg", "webp"].includes(ext))
    return FileImage;
  if (["js", "ts", "py", "rs", "go", "java", "c", "cpp", "h", "hpp", "css", "html", "vue", "svelte"].includes(ext))
    return FileCode;
  if (["zip", "tar", "gz", "bz2", "7z", "rar"].includes(ext))
    return FileArchive;
  if (["txt", "md", "json", "xml", "yml", "yaml", "toml", "ini", "cfg", "log", "csv"].includes(ext))
    return FileText;
  return File;
}

watch(
  () => props.sessionId,
  (id) => {
    if (id) {
      loadDir("/");
    } else {
      entries.value = [];
      currentPath.value = "/";
      error.value = "";
    }
  },
  { immediate: true },
);
</script>

<template>
  <div class="flex h-full flex-col">
    <!-- Header -->
    <div class="flex items-center justify-between px-3 pb-2">
      <div class="flex items-center gap-1.5">
        <HardDrive class="h-3.5 w-3.5 text-ink-faint" />
        <span class="text-xs font-medium text-ink">SFTP 浏览</span>
      </div>
      <button
        class="rounded p-1 text-ink-faint hover:text-ink hover:bg-paper-deep/50 transition-colors"
        title="刷新"
        :disabled="loading || !sessionId"
        @click="loadDir(currentPath)"
      >
        <RotateCw class="h-3.5 w-3.5" :class="{ 'animate-spin': loading }" />
      </button>
    </div>

    <!-- Breadcrumb -->
    <div class="flex items-center gap-1 px-3 pb-2 overflow-x-auto scrollbar-hidden">
      <button
        class="shrink-0 rounded p-1 text-ink-faint hover:text-ink hover:bg-paper-deep/50 transition-colors disabled:opacity-30"
        :disabled="currentPath === '/'"
        @click="goBack"
      >
        <ArrowLeft class="h-3 w-3" />
      </button>
      <button
        class="shrink-0 rounded px-1.5 py-0.5 text-[11px] text-ink-faint hover:text-ink hover:bg-paper-deep/50 transition-colors"
        @click="navigateCrumb('/')"
      >
        /
      </button>
      <template v-for="(crumb, idx) in breadcrumbs" :key="crumb.path">
        <ChevronRight class="h-2.5 w-2.5 shrink-0 text-ink-faint/50" />
        <button
          class="shrink-0 truncate rounded px-1.5 py-0.5 text-[11px] max-w-[80px] transition-colors"
          :class="
            idx === breadcrumbs.length - 1
              ? 'text-bamboo font-medium'
              : 'text-ink-faint hover:text-ink hover:bg-paper-deep/50'
          "
          @click="navigateCrumb(crumb.path)"
        >
          {{ crumb.label }}
        </button>
      </template>
    </div>

    <!-- Error -->
    <div
      v-if="error"
      class="mx-3 mb-2 rounded-lg border border-red-200 bg-red-50 px-2.5 py-1.5 text-[11px] text-red-700 dark:border-red-800/30 dark:bg-red-900/10 dark:text-red-400"
    >
      {{ error }}
    </div>

    <!-- Loading -->
    <div v-if="loading" class="flex items-center justify-center py-8">
      <div class="h-4 w-4 animate-spin rounded-full border-2 border-bamboo border-t-transparent" />
    </div>

    <!-- Empty -->
    <div
      v-else-if="!sessionId"
      class="flex flex-col items-center justify-center py-8 text-center"
    >
      <HardDrive class="h-6 w-6 text-ink-faint/40 mb-2" />
      <p class="text-xs text-ink-faint/60">选择一个已连接的会话</p>
    </div>

    <!-- File list -->
    <div v-else class="flex-1 overflow-y-auto px-3 pb-3 space-y-0.5">
      <div
        v-for="entry in entries"
        :key="entry.path"
        class="flex cursor-pointer items-center gap-2 rounded-lg px-2 py-1.5 transition-colors hover:bg-paper-deep/40"
        :class="{ 'opacity-60': !entry.isDir }"
        @click="navigateToDir(entry)"
      >
        <component :is="fileIcon(entry)" class="h-4 w-4 shrink-0" :class="entry.isDir ? 'text-yellow-600 dark:text-yellow-400' : 'text-ink-faint'" />
        <div class="min-w-0 flex-1">
          <p class="truncate text-xs text-ink">{{ entry.name }}</p>
          <p v-if="!entry.isDir" class="text-[10px] text-ink-faint">
            {{ formatSize(entry.size) }}
          </p>
        </div>
        <span class="shrink-0 text-[10px] text-ink-faint">{{ formatTime(new Date(entry.mtime * 1000).toISOString()) }}</span>
        <button
          v-if="!entry.isDir"
          class="shrink-0 rounded p-1 text-ink-faint opacity-0 group-hover:opacity-100 hover:text-bamboo hover:bg-bamboo/10 transition-all"
          title="下载"
          @click.stop
        >
          <Download class="h-3 w-3" />
        </button>
      </div>

      <div
        v-if="entries.length === 0 && !loading"
        class="flex items-center justify-center py-8 text-xs text-ink-faint/60"
      >
        空目录
      </div>
    </div>
  </div>
</template>
