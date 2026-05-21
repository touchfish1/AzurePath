import { defineStore } from "pinia";
import { ref } from "vue";
import {
  remoteShellInit,
  remoteShellListSessions,
  remoteShellCreateSession,
  remoteShellUpdateSession,
  remoteShellDeleteSession,
  remoteShellConnect,
  remoteShellDisconnect,
  remoteShellSendInput,
  remoteShellPullOutput,
  remoteShellListSftp,
  remoteShellGetMetrics,
  remoteShellListEnvironments,
  remoteShellCreateEnvironment,
  type RemoteSession,
  type SessionInput,
  type SftpEntry,
  type HostMetrics,
} from "@/lib/tauri";
import { useToastStore } from "@/stores/toast";

interface TerminalState {
  output: string;
  connected: boolean;
}

export const useRemoteShellStore = defineStore("remoteShell", () => {
  const sessions = ref<RemoteSession[]>([]);
  const activeTerminals = ref<Record<string, TerminalState>>({});
  const selectedSessionId = ref<string | null>(null);
  const sftpEntries = ref<SftpEntry[]>([]);
  const hostMetrics = ref<HostMetrics | null>(null);
  const isLoading = ref(false);
  const error = ref("");

  const pollingIntervals: Record<string, ReturnType<typeof setInterval>> = {};

  async function initStore() {
    try {
      await remoteShellInit();
      await loadSessions();
    } catch (e) {
      error.value = String(e);
    }
  }

  async function loadSessions() {
    isLoading.value = true;
    try {
      sessions.value = await remoteShellListSessions();
    } catch (e) {
      error.value = String(e);
    } finally {
      isLoading.value = false;
    }
  }

  async function createSession(input: SessionInput, password: string) {
    try {
      const session = await remoteShellCreateSession(input, password);
      sessions.value.push(session);
      useToastStore().success("会话已创建");
      return session;
    } catch (e) {
      useToastStore().error(`创建会话失败: ${e}`);
      throw e;
    }
  }

  async function updateSession(id: string, input: SessionInput) {
    try {
      const updated = await remoteShellUpdateSession(id, input);
      const idx = sessions.value.findIndex((s) => s.id === id);
      if (idx !== -1) sessions.value[idx] = updated;
      useToastStore().success("会话已更新");
      return updated;
    } catch (e) {
      useToastStore().error(`更新会话失败: ${e}`);
      throw e;
    }
  }

  async function deleteSession(id: string) {
    try {
      await remoteShellDeleteSession(id);
      sessions.value = sessions.value.filter((s) => s.id !== id);
      useToastStore().success("会话已删除");
    } catch (e) {
      useToastStore().error(`删除会话失败: ${e}`);
      throw e;
    }
  }

  async function connect(sessionId: string) {
    try {
      await remoteShellConnect(sessionId);
      if (!activeTerminals.value[sessionId]) {
        activeTerminals.value[sessionId] = { output: "", connected: false };
      }
      activeTerminals.value[sessionId].connected = true;
      startOutputPolling(sessionId);
      useToastStore().success("已连接");
    } catch (e) {
      error.value = String(e);
      useToastStore().error(`连接失败: ${e}`);
      throw e;
    }
  }

  async function disconnect(sessionId: string) {
    try {
      await remoteShellDisconnect(sessionId);
      stopOutputPolling(sessionId);
      if (activeTerminals.value[sessionId]) {
        activeTerminals.value[sessionId].connected = false;
      }
      useToastStore().success("已断开连接");
    } catch (e) {
      useToastStore().error(`断开连接失败: ${e}`);
    }
  }

  async function sendInput(sessionId: string, data: string) {
    try {
      await remoteShellSendInput(sessionId, data);
    } catch (e) {
      useToastStore().error(`发送输入失败: ${e}`);
    }
  }

  function startOutputPolling(sessionId: string) {
    stopOutputPolling(sessionId);
    pollingIntervals[sessionId] = setInterval(async () => {
      try {
        const output = await remoteShellPullOutput(sessionId);
        if (output && activeTerminals.value[sessionId]) {
          activeTerminals.value[sessionId].output += output;
        }
      } catch {
        // Silently ignore polling errors
      }
    }, 200);
  }

  function stopOutputPolling(sessionId: string) {
    if (pollingIntervals[sessionId] !== undefined) {
      clearInterval(pollingIntervals[sessionId]);
      delete pollingIntervals[sessionId];
    }
  }

  function stopAllPolling() {
    Object.keys(pollingIntervals).forEach(stopOutputPolling);
  }

  async function loadSftp(sessionId: string, path: string) {
    try {
      sftpEntries.value = await remoteShellListSftp(sessionId, path);
    } catch (e) {
      useToastStore().error(`加载 SFTP 目录失败: ${e}`);
    }
  }

  async function loadMetrics(sessionId: string) {
    try {
      hostMetrics.value = await remoteShellGetMetrics(sessionId);
    } catch (e) {
      useToastStore().error(`获取主机指标失败: ${e}`);
    }
  }

  async function listEnvironments(): Promise<string[]> {
    try {
      return await remoteShellListEnvironments();
    } catch (e) {
      useToastStore().error(`获取环境列表失败: ${e}`);
      return [];
    }
  }

  async function createEnvironment(name: string) {
    try {
      await remoteShellCreateEnvironment(name);
      useToastStore().success("环境已创建");
    } catch (e) {
      useToastStore().error(`创建环境失败: ${e}`);
      throw e;
    }
  }

  return {
    sessions,
    activeTerminals,
    selectedSessionId,
    sftpEntries,
    hostMetrics,
    isLoading,
    error,
    initStore,
    loadSessions,
    createSession,
    updateSession,
    deleteSession,
    connect,
    disconnect,
    sendInput,
    startOutputPolling,
    stopOutputPolling,
    stopAllPolling,
    loadSftp,
    loadMetrics,
    listEnvironments,
    createEnvironment,
  };
});
