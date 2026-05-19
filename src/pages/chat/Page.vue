<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from "vue";
import { Send, MessageSquare, Wifi, WifiOff, Paperclip, FileUp, Download, XCircle, X, History, Search, Trash2, Calendar, Menu, X as XIcon } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import { useToastStore } from "@/stores/toast";

const toast = useToastStore();
import {
  lanInit,
  chatSend,
  chatBroadcast,
  chatMessages,
  chatSearch,
  chatDelete,
  chatClear,
  fileSend,
  fileBroadcast,
  fileAccept,
  fileReject,
  fileList,
  discoveryPeers,
  onChatMessage,
  onPeerList,
  onPeerOffline,
  type StoredMessage,
  type PeerInfo,
  type FileTransfer,
  type FileSendResult,
} from "@/lib/tauri";
import { formatTime, formatSize, progressPercent } from "@/lib/format";
import { exportAsJson, exportAsCsv, exportAsTxt } from "@/lib/export";
import { useFileTransferListeners } from "@/composables/useFileTransfer";
import { sendSystemNotification } from "@/composables/useNotification";
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

const { setup: setupFileListeners, teardown: teardownFileListeners } = useFileTransferListeners(transfers, incomingRequest);

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

// ─── History Management ──────────────────────────────────────
const showHistory = ref(false);
const historyKeyword = ref("");
const historyDateFrom = ref("");
const historyDateTo = ref("");
const historyMessages = ref<StoredMessage[]>([]);
const historyLoading = ref(false);

// Sidebar collapse for narrow screens
const showSidebar = ref(window.innerWidth >= 1024);

async function openHistory() {
  showHistory.value = true;
  historyKeyword.value = "";
  historyDateFrom.value = "";
  historyDateTo.value = "";
  await loadHistoryMessages();
}

async function loadHistoryMessages() {
  historyLoading.value = true;
  try {
    historyMessages.value = await chatSearch(
      historyKeyword.value,
      historyDateFrom.value || undefined,
      historyDateTo.value || undefined,
    );
  } catch (e) {
    toast.error(`搜索历史记录失败: ${e}`);
  } finally {
    historyLoading.value = false;
  }
}

async function deleteHistoryMessage(id: number) {
  try {
    await chatDelete([id]);
    historyMessages.value = historyMessages.value.filter((m) => Number(m.id) !== id);
  } catch (e) {
    toast.error(`删除历史消息失败: ${e}`);
  }
}

async function clearAllHistory() {
  if (!confirm("确定清空所有聊天历史？此操作不可撤销。")) return;
  try {
    await chatClear();
    historyMessages.value = [];
    messages.value = await chatMessages();
  } catch (e) {
    toast.error(`清空历史记录失败: ${e}`);
  }
}

// ─── Export ──────────────────────────────────────────────────────
const exportFormat = ref<"json" | "csv" | "txt">("json");

function exportChatHistory() {
  const msgs = historyMessages.value;
  if (msgs.length === 0) {
    toast.error("没有可导出的记录");
    return;
  }
  if (exportFormat.value === "json") {
    exportAsJson(msgs, "chat_history");
  } else if (exportFormat.value === "csv") {
    const rows = msgs.map((m) => ({
      id: m.id,
      peer_name: m.peer_name,
      peer_ip: m.peer_ip,
      content: m.content,
      is_incoming: m.is_incoming,
      is_broadcast: m.is_broadcast,
      created_at: m.created_at,
    }));
    exportAsCsv(rows, "chat_history");
  } else if (exportFormat.value === "txt") {
    let text = "=== AzurePath Chat Export ===\n\n";
    for (const msg of msgs) {
      const direction = msg.is_incoming ? "接收" : "发送";
      text += `[${msg.created_at}] ${direction} ${msg.peer_name} (${msg.peer_ip}): ${msg.content}\n`;
    }
    exportAsTxt(text, "chat_history");
  }
  toast.add("success", "导出成功");
}

onMounted(async () => {
  if (!initialized.value) {
    try {
      await lanInit();
      initialized.value = true;
    } catch (e) {
      toast.error(`初始化局域网服务失败: ${e}`);
    }
  }

  try {
    peers.value = await discoveryPeers();
  } catch (e) {
    toast.error(`加载设备列表失败: ${e}`);
  }

  try {
    messages.value = await chatMessages();
  } catch (e) {
    toast.error(`加载消息记录失败: ${e}`);
  }

  try {
    transfers.value = await fileList();
  } catch (e) {
    toast.error(`加载传输记录失败: ${e}`);
  }

  unlistenMessage = await onChatMessage((msg) => {
    messages.value.push(msg);
    // Send system notification when window is not focused
    if (!document.hasFocus()) {
      sendSystemNotification(
        "新消息",
        `${msg.peer_name}: ${msg.content}`,
      );
    }
  });

  unlistenPeerList = await onPeerList((list) => {
    peers.value = list;
  });

  unlistenPeerOffline = await onPeerOffline(({ id }) => {
    const peer = peers.value.find((p) => p.id === id);
    if (peer) peer.status = "offline";
  });

  await setupFileListeners();
});

onUnmounted(() => {
  unlistenMessage?.();
  unlistenPeerList?.();
  unlistenPeerOffline?.();
  teardownFileListeners();
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
    toast.error(`发送消息失败: ${e}`);
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
    toast.error(`选择文件失败: ${e}`);
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
    toast.error(`选择文件夹失败: ${e}`);
  }
}

async function sendFileMessage() {
  const path = filePath.value.trim();
  if (!path || sendingFile.value) return;

  sendingFile.value = true;
  const filename = path.split("/").pop() || path.split("\\").pop() || "unknown";
  const targetPeer = selectedPeerId.value === "*" ? "*" : selectedPeerId.value;
  const tempId = "pending-" + Date.now();
  let downloadUrl: string | undefined;

  // Add entry to array immediately so it shows up in the UI
  transfers.value.unshift({
    id: tempId,
    filename,
    path,
    size: 0,
    received: 0,
    status: "sending",
    peer_id: targetPeer,
    is_incoming: false,
    created_at: new Date().toISOString(),
  });

  try {
    let result: FileSendResult;
    let status: string;
    let id: string;

    if (selectedPeerId.value === "*") {
      result = await fileBroadcast(path);
      id = tempId; // keep tempId for broadcast (real ID created per-peer on accept)
      status = "broadcasting";
      downloadUrl = result.download_url;
    } else {
      result = await fileSend(selectedPeerId.value, path);
      id = result.file_id;
      status = "transferring";
      downloadUrl = result.download_url;
    }

    // Replace entire entry in reactive array (triggers Vue reactivity)
    const replaceIdx = transfers.value.findIndex(t => t.id === tempId);
    if (replaceIdx >= 0) {
      transfers.value[replaceIdx] = {
        id, filename, path,
        size: result.file_size, received: 0,
        status, peer_id: targetPeer,
        is_incoming: false,
        created_at: new Date().toISOString(),
        download_url: downloadUrl,
      };
    }
    filePath.value = "";
    showFileInput.value = false;
  } catch (e) {
    toast.error(`发送文件失败: ${e}`);
    const errIdx = transfers.value.findIndex(t => t.id === tempId);
    if (errIdx >= 0) {
      transfers.value[errIdx] = {
        ...transfers.value[errIdx],
        status: "error",
      };
    }
  } finally {
    sendingFile.value = false;
  }
}

async function handleAccept() {
  if (!incomingRequest.value) return;
  try {
    await fileAccept(incomingRequest.value.fileId);
  } catch (e) {
    toast.error(`接受文件失败: ${e}`);
  }
  incomingRequest.value = null;
}

function handleReject() {
  if (!incomingRequest.value) return;
  fileReject(incomingRequest.value.fileId).catch((e) => toast.error(`拒绝文件失败: ${e}`));
  incomingRequest.value = null;
}

function statusClass(status: string): string {
  if (status === "completed") return "bg-green-100 text-green-600";
  if (status === "transferring") return "bg-blue-100 text-blue-600";
  if (status === "broadcasting") return "bg-purple-100 text-purple-600";
  if (status === "sending") return "bg-gray-100 text-gray-500";
  if (status.includes("error")) return "bg-red-100 text-red-600";
  return "bg-yellow-100 text-yellow-600";
}

async function copyDownloadUrl(t: FileTransfer) {
  if (!t.download_url) return;
  try {
    await navigator.clipboard.writeText(t.download_url);
    // Brief visual feedback
    const btn = document.getElementById("copy-btn-" + t.id);
    if (btn) {
      btn.textContent = "已复制!";
      setTimeout(() => { btn.textContent = "复制下载链接"; }, 2000);
    }
  } catch (e) {
    toast.error(`复制下载链接失败: ${e}`);
  }
}
</script>

<template>
  <div class="flex h-full flex-col animate-view-fade">
    <!-- Header -->
    <div class="flex items-center justify-between border-b border-paper-deep/50 px-4 md:px-6 py-3">
      <div class="flex items-center gap-2">
        <button
          class="lg:hidden rounded-lg p-1.5 text-ink-faint hover:text-ink hover:bg-paper-deep/30 transition-colors"
          @click="showSidebar = !showSidebar"
          :title="showSidebar ? '隐藏设备列表' : '显示设备列表'"
        >
          <Menu v-if="!showSidebar" class="h-4 w-4" />
          <XIcon v-else class="h-4 w-4" />
        </button>
        <div>
          <h1 class="text-xl font-display font-bold text-ink">聊天</h1>
          <p class="text-xs text-ink-faint">
            在线: {{ onlinePeers.length }} / 共 {{ peers.length }}
          </p>
        </div>
      </div>
      <Button variant="ghost" size="sm" @click="openHistory">
        <History class="mr-1 h-3.5 w-3.5" />
        历史记录
      </Button>
    </div>

    <div class="flex flex-1 overflow-hidden">
      <!-- Peer list sidebar -->
      <aside
        class="shrink-0 border-r border-paper-deep/30 overflow-y-auto bg-paper-warm/30 transition-all duration-200"
        :class="showSidebar ? 'w-52' : 'w-0 lg:w-52 overflow-hidden lg:overflow-y-auto'"
      >
        <button
          class="flex w-full items-center gap-3 px-4 py-3 text-sm transition-colors hover:bg-paper-deep/30"
          :class="selectedPeerId === '*' ? 'bg-bamboo/10 text-bamboo border-l-2 border-bamboo' : 'text-ink-soft'"
          @click="selectedPeerId = '*'"
          :aria-selected="selectedPeerId === '*'"
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
          :aria-selected="selectedPeerId === peer.id"
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
          :aria-selected="selectedPeerId === peer.id"
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
                  {{ t.status === 'completed' ? '已完成' : t.status === 'transferring' ? '传输中' : t.status === 'broadcasting' ? '广播中' : t.status === 'sending' ? '发送中' : t.status.includes('error') ? '失败' : '等待接收' }}
                </span>
                <span class="text-xs text-ink-faint">
                  {{ formatSize(t.received) }} / {{ formatSize(t.size) }}
                </span>
              </div>
              <div v-if="t.status === 'transferring' && t.size > 0" class="mt-2">
                <div class="flex items-center gap-2 text-xs text-ink-faint mb-1">
                  <span>{{ formatSize(t.received) }} / {{ formatSize(t.size) }}</span>
                  <span>({{ progressPercent(t.received, t.size) }}%)</span>
                </div>
                <div class="h-2 rounded-full bg-stone-100 dark:bg-stone-700 overflow-hidden">
                  <div
                    class="h-full rounded-full bg-bamboo transition-all duration-300"
                    :style="{ width: progressPercent(t.received, t.size) + '%' }"
                  />
                </div>
              </div>
              <div class="mt-1 text-xs text-ink-faint">
                {{ formatTime(t.created_at) }}
              </div>
              <div v-if="t.download_url" class="mt-2">
                <button
                  :id="'copy-btn-' + t.id"
                  class="inline-flex items-center gap-1 rounded-md bg-bamboo/10 px-2.5 py-1 text-xs font-medium text-bamboo transition-colors hover:bg-bamboo/20"
                  @click="copyDownloadUrl(t)"
                >
                  复制下载链接
                </button>
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
            <Button variant="danger" @click="handleReject" aria-label="拒绝文件">
              <XCircle class="mr-1.5 h-3.5 w-3.5" />
              拒绝
            </Button>
            <Button @click="handleAccept" aria-label="接受文件">
              <Download class="mr-1.5 h-3.5 w-3.5" />
              接受
            </Button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- History Modal -->
    <Teleport to="body">
      <div
        v-if="showHistory"
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/30 backdrop-blur-sm"
        @click.self="showHistory = false"
      >
        <div class="noise-bg flex max-h-[80vh] w-full max-w-2xl flex-col rounded-xl border border-paper-deep/60 bg-paper p-6 shadow-lg">
          <!-- Modal header -->
          <div class="flex items-center justify-between mb-4">
            <h3 class="text-base font-semibold text-ink">聊天历史记录</h3>
            <button class="text-ink-faint hover:text-ink" @click="showHistory = false">
              <X class="h-4 w-4" />
            </button>
          </div>

          <!-- Search and filters -->
          <div class="flex items-end gap-3 mb-4">
            <div class="flex-1">
              <label class="mb-1 block text-xs font-medium text-ink-soft">关键词搜索</label>
              <div class="relative">
                <Search class="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-ink-faint" />
                <input
                  v-model="historyKeyword"
                  placeholder="搜索消息内容..."
                  class="w-full rounded-lg border border-paper-deep/40 bg-paper-warm/50 pl-9 pr-3 py-2 text-sm text-ink outline-none placeholder:text-ink-faint/40 focus:border-bamboo/40"
                  @keyup.enter="loadHistoryMessages"
                />
              </div>
            </div>
            <div>
              <label class="mb-1 block text-xs font-medium text-ink-soft">从</label>
              <div class="relative">
                <Calendar class="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-ink-faint pointer-events-none" />
                <input
                  v-model="historyDateFrom"
                  type="date"
                  class="w-40 rounded-lg border border-paper-deep/40 bg-paper-warm/50 pl-8 pr-3 py-2 text-sm text-ink outline-none focus:border-bamboo/40"
                />
              </div>
            </div>
            <div>
              <label class="mb-1 block text-xs font-medium text-ink-soft">到</label>
              <div class="relative">
                <Calendar class="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-ink-faint pointer-events-none" />
                <input
                  v-model="historyDateTo"
                  type="date"
                  class="w-40 rounded-lg border border-paper-deep/40 bg-paper-warm/50 pl-8 pr-3 py-2 text-sm text-ink outline-none focus:border-bamboo/40"
                />
              </div>
            </div>
            <Button size="sm" @click="loadHistoryMessages">
              <Search class="mr-1 h-3.5 w-3.5" />
              搜索
            </Button>
          </div>

          <!-- Message list -->
          <div class="flex-1 overflow-y-auto min-h-0 space-y-2">
            <!-- Loading -->
            <div v-if="historyLoading" class="flex items-center justify-center py-12 text-sm text-ink-faint">
              加载中...
            </div>

            <!-- Empty -->
            <div v-else-if="historyMessages.length === 0" class="flex items-center justify-center py-12 text-sm text-ink-faint">
              <div class="text-center">
                <History class="mx-auto h-8 w-8 mb-2 opacity-40" />
                <p>暂无历史消息</p>
              </div>
            </div>

            <!-- Messages -->
            <div
              v-for="msg in historyMessages"
              :key="msg.id"
              class="flex items-start gap-3 rounded-xl border border-paper-deep/20 bg-paper-warm/30 p-3 transition-colors hover:bg-paper-warm/60"
            >
              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-2 mb-1">
                  <span class="text-xs font-medium text-ink-soft">
                    {{ msg.peer_name }}
                  </span>
                  <span class="text-xs text-ink-faint">{{ msg.peer_ip }}</span>
                  <span class="text-xs text-ink-faint">{{ formatTime(msg.created_at) }}</span>
                  <span v-if="msg.is_broadcast" class="inline-block rounded-full px-1.5 py-0.5 text-xs bg-bamboo/20 text-bamboo">广播</span>
                  <span v-if="msg.is_incoming" class="text-xs text-ink-faint">接收</span>
                  <span v-else class="text-xs text-ink-faint">发送</span>
                </div>
                <p class="text-sm text-ink whitespace-pre-wrap break-words line-clamp-2">{{ msg.content }}</p>
              </div>
              <button
                class="shrink-0 rounded-lg p-1.5 text-ink-faint transition-colors hover:text-red-500 hover:bg-red-500/10"
                title="删除"
                aria-label="删除此消息"
                @click="deleteHistoryMessage(Number(msg.id))"
              >
                <Trash2 class="h-3.5 w-3.5" />
              </button>
            </div>
          </div>

          <!-- Footer -->
          <div class="flex items-center justify-between mt-4 pt-3 border-t border-paper-deep/20">
            <span class="text-xs text-ink-faint">共 {{ historyMessages.length }} 条记录</span>
            <div class="flex items-center gap-2">
              <div class="flex rounded-lg border border-paper-deep/30 overflow-hidden">
                <button
                  v-for="fmt in (['json', 'csv', 'txt'] as const)"
                  :key="fmt"
                  class="px-2.5 py-1 text-xs font-medium transition-colors uppercase"
                  :class="exportFormat === fmt ? 'bg-bamboo/15 text-bamboo' : 'text-ink-faint hover:text-ink hover:bg-paper-deep/20'"
                  @click="exportFormat = fmt"
                >{{ fmt }}</button>
              </div>
              <Button variant="outline" size="sm" @click="exportChatHistory">
                <Download class="mr-1 h-3.5 w-3.5" />
                导出
              </Button>
              <Button variant="danger" size="sm" @click="clearAllHistory">
                <Trash2 class="mr-1 h-3.5 w-3.5" />
                清空全部
              </Button>
            </div>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>
