<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from "vue";
import { Send, MessageSquare, Wifi, WifiOff } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import {
  lanInit,
  chatSend,
  chatBroadcast,
  chatMessages,
  discoveryPeers,
  onChatMessage,
  onPeerList,
  onPeerOffline,
  type StoredMessage,
  type PeerInfo,
} from "@/lib/tauri";
import type { UnlistenFn } from "@tauri-apps/api/event";

const peers = ref<PeerInfo[]>([]);
const messages = ref<StoredMessage[]>([]);
const selectedPeerId = ref<string>("*"); // "*" = broadcast
const inputText = ref("");
const sending = ref(false);
const initialized = ref(false);

let unlistenMessage: UnlistenFn | null = null;
let unlistenPeerList: UnlistenFn | null = null;
let unlistenPeerOffline: UnlistenFn | null = null;

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

onMounted(async () => {
  if (!initialized.value) {
    try {
      await lanInit();
      initialized.value = true;
    } catch (e) {
      console.error("Failed to init LAN services:", e);
    }
  }

  // Load peers
  try {
    peers.value = await discoveryPeers();
  } catch (e) {
    console.error("Failed to load peers:", e);
  }

  // Load messages
  try {
    messages.value = await chatMessages();
  } catch (e) {
    console.error("Failed to load messages:", e);
  }

  // Listen for new messages
  unlistenMessage = await onChatMessage((msg) => {
    messages.value.push(msg);
  });

  // Listen for peer updates
  unlistenPeerList = await onPeerList((list) => {
    peers.value = list;
  });

  unlistenPeerOffline = await onPeerOffline(({ id }) => {
    const peer = peers.value.find((p) => p.id === id);
    if (peer) peer.status = "offline";
  });
});

onUnmounted(() => {
  unlistenMessage?.();
  unlistenPeerList?.();
  unlistenPeerOffline?.();
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
        <!-- Broadcast option -->
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
          <div v-if="filteredMessages.length === 0" class="flex items-center justify-center h-full text-sm text-ink-faint">
            <div class="text-center">
              <MessageSquare class="mx-auto h-8 w-8 mb-2 opacity-40" />
              <p>{{ selectedPeerId === '*' ? '暂无广播消息' : '暂无与该设备的聊天记录' }}</p>
            </div>
          </div>

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
        </div>

        <!-- Input area -->
        <div class="border-t border-paper-deep/30 p-4">
          <div class="flex items-end gap-3">
            <div class="flex-1">
              <input
                v-model="inputText"
                placeholder="输入消息..."
                class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-4 py-2.5 text-sm text-ink placeholder:text-ink-faint/50 outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
                @keydown.enter="sendMessage"
              />
            </div>
            <Button :disabled="!inputText.trim() || sending" @click="sendMessage">
              <Send class="mr-1.5 h-3.5 w-3.5" />
              发送
            </Button>
          </div>
          <p class="mt-1.5 text-xs text-ink-faint">
            {{ selectedPeerId === '*' ? '消息将广播到所有在线设备' : selectedPeer ? `发送给 ${selectedPeer.hostname} (${selectedPeer.ip})` : '选择收信人' }}
          </p>
        </div>
      </div>
    </div>
  </div>
</template>
