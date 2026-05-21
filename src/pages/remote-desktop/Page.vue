<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { Monitor, Terminal } from "lucide-vue-next";
import DesktopCanvas from "@/components/remote-desktop/DesktopCanvas.vue";
import SessionList from "@/components/remote-desktop/SessionList.vue";
import SessionDialog from "@/components/remote-desktop/SessionDialog.vue";
import Toolbar from "@/components/remote-desktop/Toolbar.vue";
import { useToastStore } from "@/stores/toast";
import { useRemoteDesktopStore } from "@/stores/remoteDesktop";
import {
  rdUpdateSession,
  onRdClipboard,
  type DesktopSessionInput,
} from "@/lib/tauri";

const toast = useToastStore();
const store = useRemoteDesktopStore();

// ─── Dialog ────────────────────────────────────────────────────────
const showDialog = ref(false);
const editSessionValue = ref<import("@/lib/tauri").DesktopSession | null>(null);

// ─── Zoom ──────────────────────────────────────────────────────────
const zoom = ref(100);
const isFullscreen = ref(false);

// ─── Canvas ref ────────────────────────────────────────────────────
const canvasRef = ref<InstanceType<typeof DesktopCanvas> | null>(null);

// ─── Computed ──────────────────────────────────────────────────────
const isConnected = computed(() => {
  if (!store.selectedSessionId) return false;
  return store.activeConnections[store.selectedSessionId]?.status === "connected";
});

const currentConnection = computed(() => {
  if (!store.selectedSessionId) return null;
  return store.activeConnections[store.selectedSessionId] ?? null;
});

// ─── Dialog actions ────────────────────────────────────────────────
function openNewSession() {
  editSessionValue.value = null;
  showDialog.value = true;
}

function openEditSession(session: import("@/lib/tauri").DesktopSession) {
  editSessionValue.value = session;
  showDialog.value = true;
}

async function handleSave(input: DesktopSessionInput, password: string) {
  try {
    if (editSessionValue.value) {
      await rdUpdateSession(editSessionValue.value.id, input);
      toast.add("success", "会话已更新");
    } else {
      await store.createSession(input, password);
      toast.add("success", "会话已创建");
    }
    showDialog.value = false;
    await store.loadSessions();
  } catch (e) {
    toast.add("error", `保存失败: ${e}`);
  }
}

// ─── Session actions ───────────────────────────────────────────────
async function handleSelect(id: string) {
  store.selectedSessionId = id;
}

async function handleConnect(id: string, password: string) {
  try {
    await store.connect(id, password);
    toast.add("success", "已连接");
  } catch (e) {
    toast.add("error", `连接失败: ${e}`);
  }
}

async function handleDisconnect() {
  if (!store.selectedSessionId) return;
  try {
    await store.disconnect(store.selectedSessionId);
    toast.add("info", "已断开连接");
  } catch (e) {
    toast.add("error", `断开失败: ${e}`);
  }
}

async function handleDelete(id: string) {
  try {
    if (store.activeConnections[id]) {
      await store.disconnect(id);
    }
    await store.deleteSession(id);
    toast.add("success", "会话已删除");
  } catch (e) {
    toast.add("error", `删除失败: ${e}`);
  }
}

// ─── Zoom ──────────────────────────────────────────────────────────
function handleZoomChange(level: number) {
  zoom.value = level;
}

// ─── Fullscreen ────────────────────────────────────────────────────
async function toggleFullscreen() {
  if (isFullscreen.value) {
    await document.exitFullscreen();
  } else {
    await document.documentElement.requestFullscreen();
  }
  // isFullscreen will be toggled by the fullscreenchange handler
}

function onFullscreenChange() {
  isFullscreen.value = !!document.fullscreenElement;
}

function handleCanvasResize(_width: number, _height: number) {
  if (store.selectedSessionId && isConnected.value) {
    // Emit resize to backend when canvas dimensions change (e.g. fullscreen)
    store.sendKey(store.selectedSessionId, {
      keyCode: 0,
      pressed: false,
    });
  }
}

// ─── Clipboard ─────────────────────────────────────────────────────
let unlistenClipboard: (() => void) | null = null;

function handleCopyClipboard() {
  // Placeholder: will request clipboard from backend
  toast.add("info", "剪贴板同步功能开发中");
}

function handlePasteClipboard(text: string) {
  if (!store.selectedSessionId) return;
  store.pushClipboard(store.selectedSessionId, text);
  toast.add("info", "剪贴板已发送");
}

// ─── Keyboard / Mouse forwarding ───────────────────────────────────
function handleCanvasKeydown(e: KeyboardEvent) {
  if (!store.selectedSessionId || !isConnected.value) return;
  store.sendKey(store.selectedSessionId, {
    keyCode: e.keyCode,
    pressed: true,
  });
}

function handleCanvasKeyup(e: KeyboardEvent) {
  if (!store.selectedSessionId || !isConnected.value) return;
  store.sendKey(store.selectedSessionId, {
    keyCode: e.keyCode,
    pressed: false,
  });
}

function handleCanvasMouse(e: MouseEvent) {
  if (!store.selectedSessionId || !isConnected.value) return;
  const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
  store.sendMouse(store.selectedSessionId, {
    x: e.clientX - rect.left,
    y: e.clientY - rect.top,
    button: e.button,
    pressed: e.type === "mousedown",
  });
}

// ─── Lifecycle ─────────────────────────────────────────────────────
onMounted(async () => {
  await store.init();
  document.addEventListener("fullscreenchange", onFullscreenChange);

  // Listen for clipboard events from backend
  onRdClipboard((text: string) => {
    store.clipboardText = text;
    store.clipboardSupported = true;
  }).then((unlisten) => {
    unlistenClipboard = unlisten;
  });
});

onUnmounted(async () => {
  document.removeEventListener("fullscreenchange", onFullscreenChange);

  if (unlistenClipboard) {
    unlistenClipboard();
    unlistenClipboard = null;
  }

  // Disconnect active connections on leave
  for (const id of Object.keys(store.activeConnections)) {
    if (store.activeConnections[id]?.status === "connected") {
      try {
        await store.disconnect(id);
      } catch {
        // Silently ignore cleanup errors
      }
    }
  }
});
</script>

<template>
  <div class="flex h-full animate-view-fade">
    <!-- Session Sidebar (hidden in fullscreen) -->
    <div
      v-show="!isFullscreen"
      class="flex w-60 shrink-0 flex-col border-r border-paper-deep/60 bg-paper-warm/30"
    >
      <div class="border-b border-paper-deep/40 px-3 py-2">
        <span class="text-xs font-medium text-ink-faint">远程桌面</span>
      </div>
      <div class="flex-1 overflow-hidden">
        <SessionList
          :sessions="store.sessions"
          :selected-id="store.selectedSessionId"
          :connections="store.activeConnections"
          @select="handleSelect"
          @connect="handleConnect"
          @delete="handleDelete"
          @edit="openEditSession"
          @create="openNewSession"
        />
      </div>
    </div>

    <!-- Main Area -->
    <div :class="['flex flex-1 flex-col overflow-hidden', isFullscreen ? 'bg-paper' : 'bg-paper']">
      <!-- Toolbar -->
      <Toolbar
        v-if="isConnected"
        :zoom="zoom"
        :is-fullscreen="isFullscreen"
        :is-connected="isConnected"
        :clipboard-text="store.clipboardText"
        :clipboard-supported="store.clipboardSupported"
        @update:zoom="handleZoomChange"
        @toggle-fullscreen="toggleFullscreen"
        @copy-clipboard="handleCopyClipboard"
        @paste-clipboard="handlePasteClipboard"
        @disconnect="handleDisconnect"
      />

      <!-- Canvas or Empty State -->
      <div v-if="isConnected" class="flex-1 overflow-hidden">
        <div
          class="h-full w-full"
          :style="{
            transform: `scale(${zoom > 0 ? zoom / 100 : 1})`,
            transformOrigin: 'center center',
          }"
          tabindex="0"
          @keydown="handleCanvasKeydown"
          @keyup="handleCanvasKeyup"
          @mousedown="handleCanvasMouse"
          @mouseup="handleCanvasMouse"
        >
          <DesktopCanvas
            v-if="store.selectedSessionId"
            ref="canvasRef"
            :session-id="store.selectedSessionId"
            :width="currentConnection?.width || 1024"
            :height="currentConnection?.height || 768"
            :is-fullscreen="isFullscreen"
            @resize="handleCanvasResize"
          />
        </div>
      </div>

      <!-- Empty State -->
      <div v-else class="flex flex-1 items-center justify-center">
        <div class="text-center">
          <Monitor class="mx-auto mb-4 h-16 w-16 text-ink-faint/30" />
          <p class="text-sm text-ink-faint">选择或创建一个远程桌面连接</p>
          <p class="mt-1 text-xs text-ink-faint/60">支持 VNC 和 RDP 协议</p>
          <button
            class="mt-4 inline-flex items-center gap-1.5 rounded-lg bg-bamboo px-4 py-2 text-xs font-medium text-cloud transition-colors hover:bg-bamboo-light"
            @click="openNewSession"
          >
            <Terminal class="h-3.5 w-3.5" />
            新建会话
          </button>
        </div>
      </div>

      <!-- Loading overlay -->
      <div
        v-if="store.isLoading"
        class="absolute inset-0 flex items-center justify-center bg-paper/50"
      >
        <div class="h-5 w-5 animate-spin rounded-full border-2 border-bamboo border-t-transparent" />
      </div>
    </div>

    <!-- Session Dialog -->
    <SessionDialog
      :session="editSessionValue"
      :visible="showDialog"
      @close="showDialog = false"
      @save="handleSave"
    />
  </div>
</template>
