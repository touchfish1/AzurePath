import { defineStore } from "pinia";
import { ref, computed } from "vue";
import {
  rdListSessions,
  rdCreateSession,
  rdDeleteSession,
  rdConnect,
  rdDisconnect,
  rdSendKey,
  rdSendMouse,
  rdPushClipboard,
  type DesktopSession,
  type DesktopSessionInput,
  type KeyEvent,
  type MouseEvent,
} from "@/lib/tauri";

export const useRemoteDesktopStore = defineStore("remoteDesktop", () => {
  const sessions = ref<DesktopSession[]>([]);
  const activeConnections = ref<
    Record<string, { protocol: string; status: string; width: number; height: number }>
  >({});
  const selectedSessionId = ref<string | null>(null);
  const isLoading = ref(false);
  const error = ref<string | null>(null);

  // ── Clipboard state ──
  const clipboardText = ref<string | null>(null);
  const clipboardSupported = ref(false);

  const selectedSession = computed(() =>
    sessions.value.find((s) => s.id === selectedSessionId.value) ?? null,
  );

  async function init() {
    await loadSessions();
  }

  async function loadSessions() {
    isLoading.value = true;
    error.value = null;
    try {
      sessions.value = await rdListSessions();
    } catch (e: unknown) {
      error.value = String(e);
    } finally {
      isLoading.value = false;
    }
  }

  async function createSession(input: DesktopSessionInput, password: string) {
    const session = await rdCreateSession(input, password);
    sessions.value.push(session);
    return session;
  }

  async function deleteSession(id: string) {
    await rdDeleteSession(id);
    sessions.value = sessions.value.filter((s) => s.id !== id);
    if (selectedSessionId.value === id) selectedSessionId.value = null;
  }

  async function connect(sessionId: string, password: string) {
    const session = sessions.value.find((s) => s.id === sessionId);
    if (!session) throw new Error("Session not found");
    activeConnections.value[sessionId] = {
      protocol: session.protocol,
      status: "connecting",
      width: 0,
      height: 0,
    };
    try {
      await rdConnect(sessionId, password);
      activeConnections.value[sessionId] = {
        ...activeConnections.value[sessionId],
        status: "connected",
      };
      selectedSessionId.value = sessionId;
    } catch (e: unknown) {
      activeConnections.value[sessionId] = {
        ...activeConnections.value[sessionId],
        status: "disconnected",
      };
      throw e;
    }
  }

  async function disconnect(sessionId: string) {
    await rdDisconnect(sessionId);
    delete activeConnections.value[sessionId];
    if (selectedSessionId.value === sessionId) selectedSessionId.value = null;
  }

  async function sendKey(sessionId: string, event: KeyEvent) {
    await rdSendKey(sessionId, event);
  }

  async function sendMouse(sessionId: string, event: MouseEvent) {
    await rdSendMouse(sessionId, event);
  }

  async function pushClipboard(sessionId: string, text: string) {
    await rdPushClipboard(sessionId, text);
  }

  return {
    sessions,
    activeConnections,
    selectedSessionId,
    selectedSession,
    isLoading,
    error,
    clipboardText,
    clipboardSupported,
    init,
    loadSessions,
    createSession,
    deleteSession,
    connect,
    disconnect,
    sendKey,
    sendMouse,
    pushClipboard,
  };
});
