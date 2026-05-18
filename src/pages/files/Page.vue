<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { FileUp, Download, XCircle, Clock } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import {
  lanInit,
  fileList,
  fileSend,
  fileAccept,
  fileReject,
  onFileRequest,
  onFileProgress,
  onFileComplete,
  onFileError,
  discoveryPeers,
  type FileTransfer,
  type PeerInfo,
} from "@/lib/tauri";
import type { UnlistenFn } from "@tauri-apps/api/event";

const transfers = ref<FileTransfer[]>([]);
const peers = ref<PeerInfo[]>([]);
const initialized = ref(false);
const selectedPeerId = ref("");
const selectedFilePath = ref("");

// Incoming file dialog
const incomingRequest = ref<{ fileId: string; filename: string; size: number; from: string } | null>(null);

let unlistenRequest: UnlistenFn | null = null;
let unlistenProgress: UnlistenFn | null = null;
let unlistenComplete: UnlistenFn | null = null;
let unlistenError: UnlistenFn | null = null;

const progressPercent = (t: FileTransfer) => {
  if (t.size === 0) return 0;
  return Math.round((t.received / t.size) * 100);
};

const formatSize = (bytes: number) => {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
};

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
  }

  try {
    peers.value = await discoveryPeers();
  } catch (e) {
    console.error("Failed to load peers:", e);
  }

  unlistenRequest = await onFileRequest((req) => {
    incomingRequest.value = req;
    // Auto-add to transfer list
    transfers.value.unshift({
      id: req.fileId,
      filename: req.filename,
      path: null,
      size: req.size,
      received: 0,
      status: "pending",
      peer_id: req.from,
      is_incoming: true,
      created_at: new Date().toISOString(),
    });
  });

  unlistenProgress = await onFileProgress((p) => {
    const t = transfers.value.find((x) => x.id === p.fileId);
    if (t) {
      t.received = p.received;
      t.size = p.total;
    }
  });

  unlistenComplete = await onFileComplete((p) => {
    const t = transfers.value.find((x) => x.id === p.fileId);
    if (t) {
      t.status = "completed";
      t.received = t.size;
      t.path = p.path;
    }
  });

  unlistenError = await onFileError((p) => {
    const t = transfers.value.find((x) => x.id === p.fileId);
    if (t) t.status = `error: ${p.error}`;
  });
});

onUnmounted(() => {
  unlistenRequest?.();
  unlistenProgress?.();
  unlistenComplete?.();
  unlistenError?.();
});

async function handleSend() {
  if (!selectedPeerId.value || !selectedFilePath.value) return;
  try {
    const fileId = await fileSend(selectedPeerId.value, selectedFilePath.value);
    transfers.value.unshift({
      id: fileId,
      filename: selectedFilePath.value.split("/").pop() || selectedFilePath.value.split("\\").pop() || "unknown",
      path: selectedFilePath.value,
      size: 0,
      received: 0,
      status: "transferring",
      peer_id: selectedPeerId.value,
      is_incoming: false,
      created_at: new Date().toISOString(),
    });
  } catch (e) {
    console.error("Send file error:", e);
  }
}

async function handleAccept() {
  if (!incomingRequest.value) return;
  // For now, accept with a default port
  // In a full impl, the receiver port comes from the file_response negotiation
  try {
    await fileAccept(incomingRequest.value.fileId, 0);
  } catch (e) {
    console.error("Accept error:", e);
  }
  incomingRequest.value = null;
}

function handleReject() {
  if (!incomingRequest.value) return;
  fileReject(incomingRequest.value.fileId).catch(console.error);
  incomingRequest.value = null;
}
</script>

<template>
  <div class="flex h-full flex-col p-6 space-y-6 animate-view-fade">
    <!-- Header -->
    <div>
      <h1 class="text-2xl font-display font-bold text-ink">文件传输</h1>
      <p class="mt-0.5 text-sm text-ink-faint">在局域网设备间传输文件</p>
    </div>

    <!-- Incoming request dialog -->
    <div
      v-if="incomingRequest"
      class="noise-bg rounded-xl border border-bamboo/40 bg-paper p-5 shadow-sm"
    >
      <div class="flex items-center justify-between">
        <div>
          <p class="text-sm font-medium text-ink">
            接收文件: <strong>{{ incomingRequest.filename }}</strong>
          </p>
          <p class="text-xs text-ink-faint mt-0.5">
            {{ formatSize(incomingRequest.size) }} · 来自 {{ incomingRequest.from }}
          </p>
        </div>
        <div class="flex gap-2">
          <Button variant="danger" @click="handleReject">
            <XCircle class="mr-1.5 h-3.5 w-3.5" />
            拒绝
          </Button>
          <Button @click="handleAccept">
            <Download class="mr-1.5 h-3.5 w-3.5" />
            接受
          </Button>
        </div>
      </div>
    </div>

    <!-- Send file card -->
    <div class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm">
      <h2 class="text-sm font-semibold text-ink mb-4">发送文件</h2>
      <div class="flex flex-wrap items-end gap-3">
        <div class="flex-1 min-w-[160px]">
          <label class="mb-1 block text-xs font-medium text-ink-soft">目标设备</label>
          <select
            v-model="selectedPeerId"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
          >
            <option value="">选择设备...</option>
            <option v-for="p in peers" :key="p.id" :value="p.id">
              {{ p.hostname }} ({{ p.ip }})
            </option>
          </select>
        </div>
        <div class="flex-1 min-w-[200px]">
          <label class="mb-1 block text-xs font-medium text-ink-soft">文件路径</label>
          <input
            v-model="selectedFilePath"
            placeholder="输入文件完整路径..."
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink placeholder:text-ink-faint/50 outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
          />
        </div>
        <Button :disabled="!selectedPeerId || !selectedFilePath" @click="handleSend">
          <FileUp class="mr-1.5 h-3.5 w-3.5" />
          发送
        </Button>
      </div>
    </div>

    <!-- Transfer list -->
    <div class="flex-1 noise-bg rounded-xl border border-paper-deep/60 bg-paper shadow-sm overflow-hidden">
      <div class="px-5 py-3 border-b border-paper-deep/50">
        <h2 class="text-sm font-semibold text-ink">传输记录</h2>
      </div>

      <div v-if="transfers.length === 0" class="flex items-center justify-center py-12 text-sm text-ink-faint">
        <div class="text-center">
          <Clock class="mx-auto h-8 w-8 mb-2 opacity-40" />
          <p>暂无传输记录</p>
        </div>
      </div>

      <div v-else class="divide-y divide-paper-deep/20">
        <div v-for="t in transfers" :key="t.id" class="px-5 py-4">
          <div class="flex items-center justify-between">
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-2">
                <FileUp v-if="!t.is_incoming" class="h-4 w-4 text-ink-faint shrink-0" />
                <Download v-else class="h-4 w-4 text-ink-faint shrink-0" />
                <span class="text-sm font-medium text-ink truncate">{{ t.filename }}</span>
                <span
                  class="inline-block rounded-full px-2 py-0.5 text-xs font-medium"
                  :class="
                    t.status === 'completed'
                      ? 'bg-bamboo/10 text-bamboo'
                      : t.status === 'transferring'
                        ? 'bg-blue-100 text-blue-600 dark:bg-blue-900/20 dark:text-blue-400'
                        : t.status === 'error'
                          ? 'bg-red-100 text-red-600 dark:bg-red-900/20 dark:text-red-400'
                          : 'bg-yellow-100 text-yellow-600 dark:bg-yellow-900/20 dark:text-yellow-400'
                  "
                >
                  {{ t.status }}
                </span>
              </div>
              <div class="mt-1 text-xs text-ink-faint">
                {{ formatSize(t.received) }} / {{ formatSize(t.size) }}
                <span v-if="t.status === 'transferring'" class="ml-2">
                  {{ progressPercent(t) }}%
                </span>
              </div>
            </div>
            <div v-if="t.status === 'transferring'" class="ml-4 w-24">
              <div class="h-2 rounded-full bg-paper-deep/30 overflow-hidden">
                <div
                  class="h-full rounded-full bg-bamboo transition-all duration-300"
                  :style="{ width: progressPercent(t) + '%' }"
                />
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
