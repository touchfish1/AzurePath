<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch } from "vue";
import {
  FileUp,
  Download,
  X,
  Check,
  Loader2,
  Copy,
  ArrowUpFromLine,
  ArrowDownToLine,
} from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import {
  lanInit,
  fileList,
  fileAccept,
  fileReject,
  getFileDownloadUrl,
  discoveryPeers,
  type FileTransfer,
  type PeerInfo,
} from "@/lib/tauri";
import { formatSize, progressPercent, formatTime } from "@/lib/format";
import { useFileTransferListeners } from "@/composables/useFileTransfer";
import { sendSystemNotification } from "@/composables/useNotification";

const transfers = ref<FileTransfer[]>([]);
const peers = ref<PeerInfo[]>([]);
const initialized = ref(false);
const loading = ref(true);

// Incoming file dialog
const incomingRequest = ref<{ fileId: string; filename: string; size: number; from: string } | null>(null);
const downloadingId = ref<string | null>(null);

const { setup: setupFileListeners, teardown: teardownFileListeners } = useFileTransferListeners(transfers, incomingRequest);

// Watch for newly completed transfers and send system notification
watch(
  transfers,
  (newTransfers, oldTransfers) => {
    for (const t of newTransfers) {
      const old = oldTransfers?.find((o) => o.id === t.id);
      if (t.status === "completed" && old && old.status !== "completed") {
        sendSystemNotification("文件传输完成", `${t.filename} 已成功传输`);
      }
    }
  },
  { deep: true },
);

function peerLabel(peerId: string): string {
  const peer = peers.value.find((p) => p.id === peerId);
  return peer ? `${peer.hostname} (${peer.ip})` : peerId;
}

const statusConfig = computed(() => (status: string) => {
  switch (status) {
    case "completed":
      return { label: "已完成", class: "text-green-600 bg-green-100" };
    case "transferring":
      return { label: "传输中", class: "text-yellow-500 bg-yellow-500/10" };
    case "pending":
      return { label: "等待接收", class: "text-ink-soft bg-paper-deep/30" };
    case "rejected":
      return { label: "已拒绝", class: "text-red-500 bg-red-500/10" };
    case "failed":
    case "error":
      return { label: "失败", class: "text-red-500 bg-red-500/10" };
    default:
      return { label: status, class: "text-ink-soft bg-paper-deep/30" };
  }
});

onMounted(async () => {
  if (!initialized.value) {
    try {
      await lanInit();
      initialized.value = true;
    } catch (e) {
      console.error("Failed to init LAN:", e);
    }
  }

  try {
    transfers.value = await fileList();
  } catch (e) {
    console.error("Failed to list transfers:", e);
  } finally {
    loading.value = false;
  }

  try {
    peers.value = await discoveryPeers();
  } catch (e) {
    console.error("Failed to load peers:", e);
  }

  await setupFileListeners();
});

onUnmounted(() => {
  teardownFileListeners();
});

async function handleAccept(fileId: string) {
  try {
    await fileAccept(fileId);
    const t = transfers.value.find((x) => x.id === fileId);
    if (t) t.status = "transferring";
  } catch (e) {
    console.error("Accept error:", e);
  }
  incomingRequest.value = null;
}

async function handleReject(fileId: string) {
  try {
    await fileReject(fileId);
    const t = transfers.value.find((x) => x.id === fileId);
    if (t) t.status = "rejected";
  } catch (e) {
    console.error("Reject error:", e);
  }
  incomingRequest.value = null;
}

async function handleDownload(fileId: string) {
  downloadingId.value = fileId;
  try {
    const url = await getFileDownloadUrl(fileId);
    if (url) {
      window.open(url, "_blank");
    }
  } catch (e) {
    console.error("Download error:", e);
  } finally {
    downloadingId.value = null;
  }
}

const copyBtnTexts = ref<Record<string, string>>({});

async function copyFileDownloadUrl(fileId: string) {
  try {
    const url = await getFileDownloadUrl(fileId);
    if (url) {
      await navigator.clipboard.writeText(url);
      copyBtnTexts.value[fileId] = "已复制!";
      setTimeout(() => { copyBtnTexts.value[fileId] = "复制下载链接"; }, 2000);
    }
  } catch (e) {
    console.error("Failed to copy download URL:", e);
  }
}
</script>

<template>
  <div class="flex h-full flex-col p-6 space-y-6 animate-view-fade">
    <!-- Header -->
    <div>
      <h1 class="text-xl font-display font-bold text-ink">文件传输</h1>
      <p class="mt-0.5 text-sm text-ink-faint">管理和查看局域网文件传输记录</p>
    </div>

    <!-- Incoming request banner -->
    <div
      v-if="incomingRequest"
      class="animate-fade-up rounded-xl border border-bamboo/30 bg-bamboo/5 p-4"
    >
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-3">
          <div class="rounded-lg bg-bamboo/10 p-2">
            <Download class="h-5 w-5 text-bamboo" />
          </div>
          <div>
            <p class="text-sm font-medium text-ink">
              收到文件请求: <strong>{{ incomingRequest.filename }}</strong>
            </p>
            <p class="text-xs text-ink-faint mt-0.5">
              {{ formatSize(incomingRequest.size) }} · 来自 {{ incomingRequest.from }}
            </p>
          </div>
        </div>
        <div class="flex gap-2 shrink-0">
          <Button variant="danger" size="sm" @click="handleReject(incomingRequest.fileId)" aria-label="拒绝文件">
            <X class="mr-1 h-3.5 w-3.5" />
            拒绝
          </Button>
          <Button size="sm" @click="handleAccept(incomingRequest.fileId)" aria-label="接受文件">
            <Check class="mr-1 h-3.5 w-3.5" />
            接受
          </Button>
        </div>
      </div>
    </div>

    <!-- Transfer history -->
    <div class="flex-1 rounded-xl border border-paper-deep/20 bg-paper-warm/30 overflow-hidden">
      <!-- List header -->
      <div class="flex items-center justify-between px-5 py-3 border-b border-paper-deep/20">
        <h2 class="text-sm font-semibold text-ink">传输记录</h2>
        <span v-if="transfers.length > 0" class="text-xs text-ink-faint">
          共 {{ transfers.length }} 项
        </span>
      </div>

      <!-- Empty state -->
      <div
        v-if="!loading && transfers.length === 0"
        class="flex items-center justify-center py-16 text-sm text-ink-faint"
      >
        <div class="text-center">
          <div class="mx-auto mb-3 flex h-12 w-12 items-center justify-center rounded-full bg-paper-deep/20">
            <FileUp class="h-6 w-6 opacity-40" />
          </div>
          <p>暂无传输记录</p>
          <p class="mt-1 text-xs opacity-60">发送或接收文件后将显示在此处</p>
        </div>
      </div>

      <!-- Loading state -->
      <div
        v-else-if="loading"
        class="flex items-center justify-center py-16 text-sm text-ink-faint"
      >
        <div class="flex items-center gap-2">
          <Loader2 class="h-4 w-4 animate-spin" />
          <span>加载中...</span>
        </div>
      </div>

      <!-- Transfer list -->
      <div v-else class="divide-y divide-paper-deep/10">
        <div
          v-for="t in transfers"
          :key="t.id"
          class="animate-fade-up px-5 py-4 transition-colors hover:bg-paper-warm/50"
        >
          <div class="flex items-start gap-4">
            <!-- Direction icon -->
            <div
              class="mt-0.5 shrink-0 rounded-lg p-2"
              :class="t.is_incoming ? 'bg-blue-500/10' : 'bg-bamboo/10'"
            >
              <ArrowDownToLine
                v-if="t.is_incoming"
                class="h-4 w-4 text-blue-500"
              />
              <ArrowUpFromLine
                v-else
                class="h-4 w-4 text-bamboo"
              />
            </div>

            <!-- Main info -->
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-2 flex-wrap">
                <span class="text-sm font-medium text-ink truncate max-w-[240px]">{{ t.filename }}</span>
                <span
                  class="inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium"
                  :class="statusConfig(t.status).class"
                >
                  {{ statusConfig(t.status).label }}
                </span>
              </div>
              <div class="mt-1 flex items-center gap-3 text-xs text-ink-faint flex-wrap">
                <span>{{ formatSize(t.size) }}</span>
                <span class="text-paper-deep/40">|</span>
                <span>{{ t.is_incoming ? '来自' : '发送至' }} {{ peerLabel(t.peer_id) }}</span>
                <span class="text-paper-deep/40">|</span>
                <span>{{ formatTime(t.created_at) }}</span>
              </div>

              <!-- Progress bar for active transfers -->
              <div v-if="t.status === 'transferring' && t.size > 0" class="mt-3 max-w-sm">
                <div class="flex items-center justify-between text-xs text-ink-faint mb-1">
                  <span>{{ formatSize(t.received) }} / {{ formatSize(t.size) }}</span>
                  <span class="font-medium text-ink-soft">{{ progressPercent(t.received, t.size) }}%</span>
                </div>
                <div class="h-2 rounded-full bg-stone-100 dark:bg-stone-700 overflow-hidden">
                  <div
                    class="h-full rounded-full bg-bamboo transition-all duration-300"
                    :style="{ width: progressPercent(t.received, t.size) + '%' }"
                  />
                </div>
              </div>

              <!-- Size info for completed/other -->
              <div v-else-if="t.status === 'completed'" class="mt-1 flex items-center gap-2">
                <span class="text-xs text-green-600">{{ formatSize(t.size) }} · 完成</span>
              </div>
            </div>

            <!-- Action buttons -->
            <div class="flex shrink-0 items-center gap-1.5">
              <!-- Pending: Accept / Reject -->
              <template v-if="t.status === 'pending' && t.is_incoming">
                <button
                  class="rounded-lg p-1.5 text-red-500 transition-colors hover:bg-red-500/10"
                  title="拒绝"
                  aria-label="拒绝文件"
                  @click="handleReject(t.id)"
                >
                  <X class="h-4 w-4" />
                </button>
                <button
                  class="rounded-lg p-1.5 text-bamboo transition-colors hover:bg-bamboo/10"
                  title="接受"
                  aria-label="接受文件"
                  @click="handleAccept(t.id)"
                >
                  <Check class="h-4 w-4" />
                </button>
              </template>

              <!-- Completed: Download -->
              <template v-if="t.status === 'completed'">
                <button
                  class="inline-flex items-center gap-1 rounded-lg px-2.5 py-1.5 text-xs font-medium text-bamboo transition-colors hover:bg-bamboo/10"
                  :disabled="downloadingId === t.id"
                  @click="handleDownload(t.id)"
                >
                  <Loader2 v-if="downloadingId === t.id" class="h-3.5 w-3.5 animate-spin" />
                  <Download v-else class="h-3.5 w-3.5" />
                  {{ downloadingId === t.id ? '获取中...' : '下载' }}
                </button>
                <button
                  class="inline-flex items-center gap-1 rounded-lg px-2.5 py-1.5 text-xs font-medium text-ink-soft transition-colors hover:bg-paper-deep/30 hover:text-ink"
                  @click="copyFileDownloadUrl(t.id)"
                  :title="copyBtnTexts[t.id] || '复制下载链接'"
                >
                  <Copy class="h-3.5 w-3.5" />
                  {{ copyBtnTexts[t.id] || '复制链接' }}
                </button>
              </template>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
