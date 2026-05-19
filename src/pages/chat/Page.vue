<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from "vue";
import { Send, MessageSquare, Wifi, WifiOff, Paperclip, FileUp, Download, XCircle, X } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import {
  lanInit,
  chatSend,
  chatBroadcast,
  chatMessages,
  fileSend,
  fileBroadcast,
  fileAccept,
  fileReject,
  fileList,
  discoveryPeers,
  onChatMessage,
  onPeerList,
  onPeerOffline,
  onFileRequest,
  onFileProgress,
  onFileComplete,
  onFileError,
  type StoredMessage,
  type PeerInfo,
  type FileTransfer,
} from "@/lib/tauri";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";

const peers = ref<PeerInfo[]>([]);
const messages = ref<StoredMessage[]>([]);
const transfers = ref<FileTransfer[]>([]);
const selectedPeerId = ref<string>("*");
const inputText = ref("");
const sending = ref(false);
const initialized = ref(false);
const showFileInput = ref(false);
const filePath = ref("");
const sendingFile = ref(false);

const incomingRequest = ref<{ fileId: string; filename: string; size: number; from: string } | null>(null);

let unlistenMessage: UnlistenFn | null = null;
let unlistenPeerList: UnlistenFn | null = null;
let unlistenPeerOffline: UnlistenFn | null = null;
let unlistenFileRequest: UnlistenFn | null = null;
let unlistenFileProgress: UnlistenFn | null = null;
let unlistenFileComplete: UnlistenFn | null = null;
let unlistenFileError: UnlistenFn | null = null;

const selectedPeer = computed(() =>
  peers.value.find((p) => p.id === selectedPeerId.value),
);

const filteredMessages = computed(() => {
  if (selectedPeerId.value === "*") {
    return messages.value;
  }
  return messages.value.filter(
    (m) => m.peer_id === selectedPeerId.value,
  );
});

const onlinePeers = computed(() =>
  peers.value.filter((p) => p.status === "online"),
);
const offlinePeers = computed(() =>
  peers.value.filter((p) => p.status === "offline"),
);

const peerTransfers = computed(() => {
  if (selectedPeerId.value === "*") {
    return transfers.value;
  }
  return transfers.value.filter(t => t.peer_id === selectedPeerId.value);
});

const canSendFile = computed(() =>
  !sendingFile.value &&
  filePath.value.trim().length > 0
);

const canSendText = computed(() =>
  !sending.value && inputText.value.trim().length > 0
);

onMounted(async () => {
  if (!initialized.value) {
    try {
      await lanInit();
      initialized.value = true;
    } catch (e) {
      console.error("Failed to init LAN services:", e);
    }
  }

  try {
    peers.value = await discoveryPeers();
  } catch (e) {
    console.error("Failed to load peers:", e);
  }

  try {
    messages.value = await chatMessages();
  } catch (e) {
    console.error("Failed to load messages:", e);
  }

  try {
    transfers.value = await fileList();
  } catch (e) {
    console.error("Failed to load transfers:", e);
  }

  unlistenMessage = await onChatMessage((msg) => {
    messages.value.push(msg);
  });

  unlistenPeerList = await onPeerList((list) => {
    peers.value = list;
  });

  unlistenPeerOffline = await onPeerOffline(({ id }) => {
    const peer = peers.value.find((p) => p.id === id);
    if (peer) peer.status = "offline";
  });

  unlistenFileRequest = await onFileRequest((req) => {
    incomingRequest.value = req;
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

  unlistenFileProgress = await onFileProgress((p) => {
    const t = transfers.value.find((x) => x.id === p.fileId);
    if (t) {
      t.received = p.received;
      t.size = p.total;
      t.status = "transferring";
    }
  });

  unlistenFileComplete = await onFileComplete((p) => {
    const t = transfers.value.find((x) => x.id === p.fileId);
    if (t) {
      t.status = "completed";
      t.received = t.size;
      t.path = p.path;
    }
  });

  unlistenFileError = await onFileError((p) => {
    const t = transfers.value.find((x) => x.id === p.fileId);
    if (t) t.status = `error: ${p.error}`;
  });
});

onUnmounted(() => {
  unlistenMessage?.();
  unlistenPeerList?.();
  unlistenPeerOffline?.();
  unlistenFileRequest?.();
  unlistenFileProgress?.();
  unlistenFileComplete?.();
  unlistenFileError?.();
});

async function sendMessage() {
  const text = inputText.value.trim();
  if (!text || sending.value) return;

  sending.value = true;
  try {
    let msg: StoredMessage;
    if (selectedPeerId.value === "*") {
      msg = await chatBroadcast(text);
    } else {
      msg = await chatSend(selectedPeerId.value, text);
    }
    messages.value.push(msg);
    inputText.value = "";
  } catch (e) {
    console.error("Failed to send message:", e);
  } finally {
    sending.value = false;
  }
}

async function pickFile() {
  try {
    const selected = await open({
      multiple: false,
      title: "选择文件",
    });
    if (selected) {
      filePath.value = selected as string;
    }
  } catch (e) {
    console.error("Failed to pick file:", e);
  }
}

async function pickFolder() {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "选择文件夹",
    });
    if (selected) {
      filePath.value = selected as string;
    }
  } catch (e) {
    console.error("Failed to pick folder:", e);
  }
}

async function sendFileMessage() {
  const path = filePath.value.trim();
  if (!path || sendingFile.value) return;

  sendingFile.value = true;
  try {
    if (selectedPeerId.value === "*") {
      // Broadcast to all connected peers
      await fileBroadcast(path);
    } else {
      const fileId = await fileSend(selectedPeerId.value, path);
      const filename = path.split("/").pop() || path.split("\\").pop() || "unknown";
      transfers.value.unshift({
        id: fileId,
        filename,
        path: path,
        size: 0,
        received: 0,
        status: "transferring",
        peer_id: selectedPeerId.value,
        is_incoming: false,
        created_at: new Date().toISOString(),
      });
    }
    filePath.value = "";
    showFileInput.value = false;
  } catch (e) {
    console.error("Failed to send file:", e);
  } finally {
    sendingFile.value = false;
  }
}

async function handleAccept() {
  if (!incomingRequest.value) return;
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

function formatTime(iso: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleTimeString("zh-CN", {
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return iso;
  }
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function progressPercent(t: FileTransfer): number {
  if (t.size === 0) return 0;
  return Math.round((t.received / t.size) * 100);
}

function statusClass(status: string): string {
  if (status === "completed") return "bg-bamboo/10 text-bamboo";
  if (status === "transferring") return "bg-blue-100 text-blue-600";
  if (status.includes("error")) return "bg-red-100 text-red-600";
  return "bg-yellow-100 text-yellow-600";
}
</script>

<template>
  <div class="flex h-full flex-col animate-view-fade">
    <!-- Header -->
    <div class="border-b border-paper-deep/50 px-6 py-3">
      <h1 class="text-xl font-display font-bold text-ink">聊天</h1>
      <p class="text-xs text-ink-faint">
        在线: {{ onlinePeers.length }} / 共 {{ peers.length }}
      </p>
    </div>

    <div class="flex flex-1 overflow-hidden">
      <!-- Peer list sidebar -->
      <aside class="w-52 shrink-0 border-r border-paper-deep/30 overflow-y-auto bg-paper-warm/30">
        <button
          class="flex w-full items-center gap-3 px-4 py-3 text-sm transition-colors hover:bg-paper-deep/30"
          :class="selectedPeerId === '*' ? 'bg-bamboo/10 text-bamboo border-l-2 border-bamboo' : 'text-ink-soft'"
          @click="selectedPeerId = '*'"
        >
          <MessageSquare class="h-4 w-4" />
          <span class="font-medium">广播</span>
        </button>

        <div class="px-3 py-2 text-xs font-medium text-ink-faint uppercase tracking-wider">
          在线
        </div>
        <button
          v-for="peer in onlinePeers"
          :key="peer.id"
          class="flex w-full items-center gap-3 px-4 py-2.5 text-sm transition-colors hover:bg-paper-deep/30"
          :class="selectedPeerId === peer.id ? 'bg-bamboo/10 text-bamboo border-l-2 border-bamboo' : 'text-ink-soft'"
          @click="selectedPeerId = peer.id"
        >
          <Wifi class="h-3.5 w-3.5 text-bamboo shrink-0" />
          <div class="min-w-0 text-left">
            <div class="truncate font-medium">{{ peer.hostname }}</div>
            <div class="text-xs text-ink-faint truncate">{{ peer.ip }}</div>
          </div>
        </button>

        <div v-if="offlinePeers.length > 0" class="px-3 py-2 text-xs font-medium text-ink-faint uppercase tracking-wider">
          离线
        </div>
        <button
          v-for="peer in offlinePeers"
          :key="peer.id"
          class="flex w-full items-center gap-3 px-4 py-2.5 text-sm text-ink-faint transition-colors hover:bg-paper-deep/30"
          :class="selectedPeerId === peer.id ? 'bg-paper-deep/30 border-l-2 border-paper-deep' : ''"
          @click="selectedPeerId = peer.id"
        >
          <WifiOff class="h-3.5 w-3.5 shrink-0" />
          <div class="min-w-0 text-left">
            <div class="truncate">{{ peer.hostname }}</div>
            <div class="text-xs truncate">{{ peer.ip }}</div>
          </div>
        </button>

        <div v-if="peers.length === 0" class="px-4 py-6 text-center text-xs text-ink-faint">
          未发现局域网设备
        </div>
      </aside>

      <!-- Main chat area -->
      <div class="flex flex-1 flex-col">
        <!-- Messages -->
        <div class="flex-1 overflow-y-auto p-4 space-y-3">
          <!-- Empty state -->
          <div v-if="filteredMessages.length === 0 && peerTransfers.length === 0" class="flex items-center justify-center h-full text-sm text-ink-faint">
            <div class="text-center">
              <MessageSquare class="mx-auto h-8 w-8 mb-2 opacity-40" />
              <p>{{ selectedPeerId === '*' ? '暂无广播消息' : '暂无与该设备的聊天记录' }}</p>
              <p v-if="selectedPeerId !== '*'" class="mt-1 text-xs opacity-60">可发送文字消息或文件</p>
            </div>
          </div>

          <!-- Text messages -->
          <div
            v-for="msg in filteredMessages"
            :key="msg.id"
            class="flex"
            :class="msg.is_incoming ? 'justify-start' : 'justify-end'"
          >
            <div
              class="max-w-[70%] rounded-xl px-4 py-2.5 text-sm"
              :class="
                msg.is_incoming
                  ? 'bg-paper-deep/30 text-ink rounded-bl-none'
                  : 'bg-bamboo/10 text-ink rounded-br-none'
              "
            >
              <div class="flex items-center gap-2 mb-1">
                <span class="text-xs font-medium text-ink-soft">
                  {{ msg.is_incoming ? msg.peer_name : "我" }}
                </span>
                <span class="text-xs text-ink-faint">{{ msg.peer_ip }}</span>
                <span class="text-xs text-ink-faint">{{ formatTime(msg.created_at) }}</span>
              </div>
              <p class="whitespace-pre-wrap break-words">{{ msg.content }}</p>
              <div v-if="msg.is_broadcast" class="mt-1">
                <span class="inline-block rounded-full px-2 py-0.5 text-xs bg-bamboo/20 text-bamboo">广播</span>
              </div>
            </div>
          </div>

          <!-- File transfer entries inline -->
          <div
            v-for="t in peerTransfers"
            :key="'file-' + t.id"
            class="flex"
            :class="t.is_incoming ? 'justify-start' : 'justify-end'"
          >
            <div
              class="max-w-[70%] rounded-xl px-4 py-3 text-sm border"
              :class="
                t.is_incoming
                  ? 'bg-paper-deep/20 text-ink rounded-bl-none border-paper-deep/20'
                  : 'bg-bamboo/5 text-ink rounded-br-none border-bamboo/10'
              "
            >
              <div class="flex items-center gap-2 mb-1.5">
                <FileUp class="h-4 w-4 text-ink-faint" />
                <span class="text-xs font-medium text-ink-soft">
                  {{ t.is_incoming ? "接收文件" : "发送文件" }}
                </span>
              </div>
              <p class="font-medium text-sm">{{ t.filename }}</p>
              <div class="flex items-center gap-2 mt-1.5">
                <span
                  class="inline-block rounded-full px-2 py-0.5 text-xs font-medium"
                  :class="statusClass(t.status)"
                >
                  {{ t.status === 'completed' ? '已完成' : t.status === 'transferring' ? '传输中' : t.status.includes('error') ? '失败' : '等待中' }}
                </span>
                <span class="text-xs text-ink-faint">
                  {{ formatSize(t.received) }} / {{ formatSize(t.size) }}
                </span>
              </div>
              <div v-if="t.status === 'transferring' && t.size > 0" class="mt-2 h-1.5 rounded-full bg-paper-deep/30 overflow-hidden">
                <div
                  class="h-full rounded-full bg-bamboo transition-all duration-300"
                  :style="{ width: progressPercent(t) + '%' }"
                />
              </div>
              <div class="mt-1 text-xs text-ink-faint">
                {{ formatTime(t.created_at) }}
              </div>
            </div>
          </div>
        </div>

        <!-- Input area -->
        <div class="border-t border-paper-deep/30 p-4">
          <!-- File path input (togglable) -->
          <div v-if="showFileInput" class="mb-3">
            <div class="flex items-end gap-2">
              <div class="flex-1">
                <label class="mb-1 block text-xs font-medium text-ink-soft">文件路径</label>
                <div class="flex gap-2">
                  <input
                    v-model="filePath"
                    placeholder="选择文件或输入路径..."
                    readonly
                    class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink placeholder:text-ink-faint/50 outline-none cursor-pointer"
                    @click="pickFile"
                  />
                  <Button variant="secondary" size="sm" @click="pickFile">
                    选择文件
                  </Button>
                  <Button variant="ghost" size="sm" @click="pickFolder">
                    选择文件夹
                  </Button>
                </div>
              </div>
              <Button
                variant="outline"
                size="sm"
                :disabled="!canSendFile"
                @click="sendFileMessage"
              >
                <FileUp class="mr-1 h-3.5 w-3.5" />
                发送文件
              </Button>
            </div>
          </div>

          <!-- Text input -->
          <div class="flex items-end gap-3">
            <button
              class="shrink-0 rounded-lg p-2.5 transition-colors"
              :class="showFileInput ? 'bg-bamboo/15 text-bamboo' : 'text-ink-faint hover:text-ink hover:bg-paper-deep/30'"
              @click="showFileInput = !showFileInput"
              title="附加文件"
            >
              <Paperclip class="h-4 w-4" />
            </button>
            <div class="flex-1">
              <input
                v-model="inputText"
                placeholder="输入消息..."
                class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-4 py-2.5 text-sm text-ink placeholder:text-ink-faint/50 outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
                @keydown.enter="sendMessage"
              />
            </div>
            <Button :disabled="!canSendText" @click="sendMessage">
              <Send class="mr-1.5 h-3.5 w-3.5" />
              发送
            </Button>
          </div>
          <p class="mt-1.5 text-xs text-ink-faint">
            {{ selectedPeerId === '*' ? '消息和文件将广播到所有在线设备' : selectedPeer ? `发送给 ${selectedPeer.hostname} (${selectedPeer.ip})` : '选择收信人' }}
          </p>
        </div>
      </div>
    </div>

    <!-- Incoming file request dialog -->
    <Teleport to="body">
      <div
        v-if="incomingRequest"
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/30 backdrop-blur-sm"
        @click.self="incomingRequest = null"
      >
        <div class="noise-bg w-96 rounded-xl border border-paper-deep/60 bg-paper p-6 shadow-lg">
          <div class="flex items-center justify-between mb-4">
            <h3 class="text-base font-semibold text-ink">接收文件</h3>
            <button class="text-ink-faint hover:text-ink" @click="handleReject">
              <X class="h-4 w-4" />
            </button>
          </div>
          <div class="space-y-2">
            <div class="flex items-center gap-3">
              <Download class="h-8 w-8 text-bamboo shrink-0" />
              <div>
                <p class="text-sm font-medium text-ink">{{ incomingRequest.filename }}</p>
                <p class="text-xs text-ink-faint">
                  {{ formatSize(incomingRequest.size) }} · 来自 {{ incomingRequest.from }}
                </p>
              </div>
            </div>
          </div>
          <div class="flex justify-end gap-2 mt-6">
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
    </Teleport>
  </div>
</template>
