import { defineStore } from "pinia";
import { ref, computed } from "vue";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { Terminal } from "xterm";
import { FitAddon } from "@xterm/addon-fit";
import {
  sshConnect,
  sshDisconnect,
  sshSendInput,
  sshResize,
  sshListSessions,
  onSshConnected,
  onSshOutput,
  onSshDisconnected,
  onSshError,
  onSshSessionCreated,
  type SshSession,
} from "@/lib/tauri";

export interface SessionTab {
  id: string;
  host: string;
  port: number;
  username: string;
  terminal: Terminal;
  fitAddon: FitAddon;
  containerEl: HTMLDivElement | null;
  connected: boolean;
}

export const useTerminalStore = defineStore("terminal", () => {
  // State
  const sessions = ref<SessionTab[]>([]);
  const activeSessionId = ref<string>("");
  const connecting = ref(false);

  // Connection form state
  const formHost = ref("");
  const formPort = ref(22);
  const formUsername = ref("");
  const formPassword = ref("");
  const savedSessions = ref<Array<{ label: string; host: string; port: number; username: string }>>([]);

  // Unlisten functions
  let unlistenConnected: UnlistenFn | null = null;
  let unlistenOutput: UnlistenFn | null = null;
  let unlistenDisconnected: UnlistenFn | null = null;
  let unlistenError: UnlistenFn | null = null;
  let unlistenCreated: UnlistenFn | null = null;

  // Computed
  const activeSession = computed(() => {
    return sessions.value.find((s) => s.id === activeSessionId.value) ?? null;
  });

  const isConnected = computed(() => {
    return activeSession.value?.connected ?? false;
  });

  // Private: create a new xterm Terminal instance
  function createTerminal(): { terminal: Terminal; fitAddon: FitAddon } {
    const fitAddon = new FitAddon();
    const terminal = new Terminal({
      cursorBlink: true,
      cursorStyle: "block",
      fontSize: 13,
      fontFamily: "'Cascadia Code', 'Fira Code', 'JetBrains Mono', 'Consolas', monospace",
      theme: {
        background: "#1a1b26",
        foreground: "#a9b1d6",
        cursor: "#c0caf5",
        selectionBackground: "#33467c",
        black: "#32344a",
        red: "#f7768e",
        green: "#9ece6a",
        yellow: "#e0af68",
        blue: "#7aa2f7",
        magenta: "#bb9af7",
        cyan: "#7dcfff",
        white: "#a9b1d6",
        brightBlack: "#444b6a",
        brightRed: "#f7768e",
        brightGreen: "#9ece6a",
        brightYellow: "#e0af68",
        brightBlue: "#7aa2f7",
        brightMagenta: "#bb9af7",
        brightCyan: "#7dcfff",
        brightWhite: "#c0caf5",
      },
      allowTransparency: true,
    });
    terminal.loadAddon(fitAddon);
    return { terminal, fitAddon };
  }

  // Actions
  async function attachListeners() {
    detachListeners();
    unlistenConnected = await onSshConnected(handleConnected);
    unlistenOutput = await onSshOutput(handleOutput);
    unlistenDisconnected = await onSshDisconnected(handleDisconnected);
    unlistenError = await onSshError(handleError);
    unlistenCreated = await onSshSessionCreated(handleSessionCreated);
  }

  function detachListeners() {
    unlistenConnected?.();
    unlistenConnected = null;
    unlistenOutput?.();
    unlistenOutput = null;
    unlistenDisconnected?.();
    unlistenDisconnected = null;
    unlistenError?.();
    unlistenError = null;
    unlistenCreated?.();
    unlistenCreated = null;
  }

  function handleSessionCreated(payload: { sessionId: string }) {
    // The session ID is already set in the tab; nothing extra needed
  }

  function handleConnected(payload: {
    sessionId: string;
    host: string;
    port: number;
    username: string;
  }) {
    const tab = sessions.value.find((s) => s.id === payload.sessionId);
    if (tab) {
      tab.connected = true;
    }
    connecting.value = false;
  }

  function handleOutput(payload: { sessionId: string; data: string }) {
    // Decode base64 and write to the corresponding terminal
    const tab = sessions.value.find((s) => s.id === payload.sessionId);
    if (!tab) return;

    try {
      const binaryStr = atob(payload.data);
      const bytes = new Uint8Array(binaryStr.length);
      for (let i = 0; i < binaryStr.length; i++) {
        bytes[i] = binaryStr.charCodeAt(i);
      }
      tab.terminal.write(bytes);
    } catch {
      // If base64 decoding fails, write raw string as fallback
      tab.terminal.write(payload.data);
    }
  }

  function handleDisconnected(payload: { sessionId: string }) {
    const tab = sessions.value.find((s) => s.id === payload.sessionId);
    if (tab) {
      tab.connected = false;
      tab.terminal.write("\r\n\x1b[31m[Connection closed]\x1b[0m\r\n");
    }
  }

  function handleError(payload: { sessionId: string; error: string }) {
    const tab = sessions.value.find((s) => s.id === payload.sessionId);
    if (tab) {
      tab.terminal.write(`\r\n\x1b[31m[Error: ${payload.error}]\x1b[0m\r\n`);
      tab.connected = false;
    }
    connecting.value = false;
  }

  async function connect() {
    if (!formHost.value.trim() || !formUsername.value.trim() || !formPassword.value.trim()) {
      return;
    }

    connecting.value = true;

    // Generate a temporary session ID
    const sessionId = crypto.randomUUID();

    try {
      await attachListeners();

      // Create terminal BEFORE connecting so it's ready to receive output
      const { terminal, fitAddon } = createTerminal();

      const tab: SessionTab = {
        id: sessionId,
        host: formHost.value,
        port: formPort.value,
        username: formUsername.value,
        terminal,
        fitAddon,
        containerEl: null,
        connected: false,
      };

      sessions.value.push(tab);
      activeSessionId.value = sessionId;

      // Connect to SSH
      await sshConnect(formHost.value, formPort.value, formUsername.value, formPassword.value, sessionId);
    } catch (e) {
      // Remove the tab if connection failed
      sessions.value = sessions.value.filter((s) => s.id !== sessionId);
      connecting.value = false;
      throw e;
    }
  }

  async function disconnect(id?: string) {
    const targetId = id ?? activeSessionId.value;
    if (!targetId) return;

    try {
      await sshDisconnect(targetId);
    } catch {
      // ignore
    }

    const tab = sessions.value.find((s) => s.id === targetId);
    if (tab) {
      tab.terminal.dispose();
    }

    sessions.value = sessions.value.filter((s) => s.id !== targetId);

    if (activeSessionId.value === targetId) {
      activeSessionId.value = sessions.value.length > 0 ? sessions.value[sessions.value.length - 1].id : "";
    }

    if (sessions.value.length === 0) {
      detachListeners();
    }
  }

  function sendInput(data: string) {
    if (!activeSessionId.value) return;
    const encoded = btoa(data);
    sshSendInput(activeSessionId.value, encoded).catch(() => {});
  }

  function resize(cols: number, rows: number) {
    if (!activeSessionId.value) return;
    sshResize(activeSessionId.value, cols, rows).catch(() => {});
  }

  function switchTab(id: string) {
    activeSessionId.value = id;
    // Fit the terminal when switching to it (will be triggered after next tick)
    setTimeout(() => {
      const tab = sessions.value.find((s) => s.id === id);
      if (tab) {
        try {
          tab.fitAddon.fit();
        } catch {
          // fit may fail if terminal is not visible
        }
      }
    }, 50);
  }

  function closeTab(id: string) {
    disconnect(id);
  }

  function setActiveTerminalContainer(id: string, el: HTMLDivElement | null) {
    const tab = sessions.value.find((s) => s.id === id);
    if (tab) {
      tab.containerEl = el;
    }
  }

  // Saved sessions management
  function saveSession() {
    const existing = savedSessions.value.find(
      (s) => s.host === formHost.value && s.port === formPort.value && s.username === formUsername.value,
    );
    if (!existing) {
      savedSessions.value.push({
        label: `${formUsername.value}@${formHost.value}`,
        host: formHost.value,
        port: formPort.value,
        username: formUsername.value,
      });
    }
  }

  function loadSavedSession(idx: number) {
    const saved = savedSessions.value[idx];
    if (saved) {
      formHost.value = saved.host;
      formPort.value = saved.port;
      formUsername.value = saved.username;
    }
  }

  function removeSavedSession(idx: number) {
    savedSessions.value.splice(idx, 1);
  }

  function focusTerminal() {
    const tab = activeSession.value;
    if (tab) {
      tab.terminal.focus();
    }
  }

  return {
    // State
    sessions,
    activeSessionId,
    connecting,
    formHost,
    formPort,
    formUsername,
    formPassword,
    savedSessions,

    // Computed
    activeSession,
    isConnected,

    // Actions
    connect,
    disconnect,
    sendInput,
    resize,
    switchTab,
    closeTab,
    setActiveTerminalContainer,
    saveSession,
    loadSavedSession,
    removeSavedSession,
    focusTerminal,
    attachListeners,
    detachListeners,
  };
});
