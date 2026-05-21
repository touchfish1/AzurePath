<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import {
  Terminal,
  Plus,
  PanelRight,
  PanelRightClose,
  X,
  Monitor,
  Database,
  Layers,
} from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import XtermTerminal from "@/components/remote-shell/XtermTerminal.vue";
import SessionList from "@/components/remote-shell/SessionList.vue";
import SftpPanel from "@/components/remote-shell/SftpPanel.vue";
import MetricsPanel from "@/components/remote-shell/MetricsPanel.vue";
import SessionDialog from "@/components/remote-shell/SessionDialog.vue";
import { useToastStore } from "@/stores/toast";
import {
  remoteShellInit,
  remoteShellListSessions,
  remoteShellListSummaries,
  remoteShellCreateSession,
  remoteShellUpdateSession,
  remoteShellDeleteSession,
  remoteShellConnect,
  remoteShellDisconnect,
  remoteShellSendInput,
  remoteShellPullOutput,
  remoteShellListEnvironments,
  remoteShellCreateEnvironment,
  type RemoteSession,
  type SessionSummary,
  type SessionInput,
} from "@/lib/tauri";

// ─── Toast ────────────────────────────────────────────────────────
const toast = useToastStore();

// ─── Sessions ─────────────────────────────────────────────────────
const sessions = ref<RemoteSession[]>([]);
const summaries = ref<SessionSummary[]>([]);
const activeSessionIds = ref<Set<string>>(new Set());
const selectedTabId = ref<string | null>(null);

// ─── Environments ─────────────────────────────────────────────────
const environments = ref<string[]>(["default"]);
const selectedEnv = ref("default");
const showNewEnvInput = ref(false);
const newEnvName = ref("");

// ─── Dialog ───────────────────────────────────────────────────────
const showDialog = ref(false);
const editSession = ref<RemoteSession | null>(null);

// ─── Right panel ─────────────────────────────────────────────────
const rightPanelOpen = ref(false);
const rightPanelTab = ref<"sftp" | "metrics">("sftp");

// ─── Loading & error ──────────────────────────────────────────────
const loading = ref(true);
const connectingIds = ref<Set<string>>(new Set());
const error = ref("");

// ─── Terminal refs ────────────────────────────────────────────────
const terminalRefs = ref<Record<string, InstanceType<typeof XtermTerminal>>>({});

function setTerminalRef(id: string, el: any) {
  if (el) {
    terminalRefs.value[id] = el;
  } else {
    delete terminalRefs.value[id];
  }
}

// ─── Polling ──────────────────────────────────────────────────────
const pollingIntervals = ref<Record<string, ReturnType<typeof setInterval>>>({});

function startPolling(sessionId: string) {
  if (pollingIntervals.value[sessionId]) return;

  const interval = setInterval(async () => {
    try {
      const b64 = await remoteShellPullOutput(sessionId);
      if (b64) {
        const decoded = atob(b64);
        const term = terminalRefs.value[sessionId];
        if (term) {
          term.write(decoded);
        }
      }
    } catch {
      // If error on polling, stop the interval
      stopPolling(sessionId);
      // Mark as disconnected
      activeSessionIds.value.delete(sessionId);
      activeSessionIds.value = new Set(activeSessionIds.value);
      toast.add("error", `会话连接已断开`);
    }
  }, 200);

  pollingIntervals.value[sessionId] = interval;
}

function stopPolling(sessionId: string) {
  const interval = pollingIntervals.value[sessionId];
  if (interval) {
    clearInterval(interval);
    delete pollingIntervals.value[sessionId];
  }
}

function stopAllPolling() {
  for (const id of Object.keys(pollingIntervals.value)) {
    stopPolling(id);
  }
}

// ─── Computed ─────────────────────────────────────────────────────
const connectedIds = computed(() =>
  summaries.value.filter((s) => s.isConnected).map((s) => s.id),
);

const connectedSummaries = computed(() =>
  summaries.value.filter((s) => s.isConnected),
);

const filteredSessions = computed(() => {
  if (!selectedEnv.value || selectedEnv.value === "default") return summaries.value;
  return summaries.value.filter((s) => s.environment === selectedEnv.value);
});


const welcomeVisible = computed(
  () => connectedIds.value.length === 0 && !loading.value,
);

const rightPanelSessionId = computed(() => {
  if (!rightPanelOpen.value || !selectedTabId.value) return null;
  return selectedTabId.value;
});

// ─── Data loading ─────────────────────────────────────────────────
async function loadData() {
  try {
    const [sess, sum, envs] = await Promise.all([
      remoteShellListSessions(),
      remoteShellListSummaries(),
      remoteShellListEnvironments(),
    ]);
    sessions.value = sess;
    summaries.value = sum;
    if (envs.length > 0) {
      environments.value = envs;
    }
    // Sync active session IDs
    activeSessionIds.value = new Set(
      sum.filter((s) => s.isConnected).map((s) => s.id),
    );

    // Auto-select first connected session
    if (connectedIds.value.length > 0 && !selectedTabId.value) {
      selectedTabId.value = connectedIds.value[0];
    } else if (connectedIds.value.length === 0) {
      selectedTabId.value = null;
    }
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

// ─── Session actions ──────────────────────────────────────────────
async function handleConnect(id: string) {
  connectingIds.value = new Set([...connectingIds.value, id]);
  try {
    await remoteShellConnect(id);
    await loadData();
    selectedTabId.value = id;
    startPolling(id);
    toast.add("success", "已连接");
  } catch (e) {
    toast.add("error", `连接失败: ${e}`);
  } finally {
    const next = new Set(connectingIds.value);
    next.delete(id);
    connectingIds.value = next;
  }
}

async function handleDisconnect(id: string) {
  try {
    stopPolling(id);
    await remoteShellDisconnect(id);
    activeSessionIds.value.delete(id);
    activeSessionIds.value = new Set(activeSessionIds.value);
    if (selectedTabId.value === id) {
      const remaining = connectedIds.value.filter((sid) => sid !== id);
      selectedTabId.value = remaining.length > 0 ? remaining[0] : null;
    }
    await loadData();
    toast.add("info", "已断开连接");
  } catch (e) {
    toast.add("error", `断开失败: ${e}`);
  }
}

async function handleDelete(id: string) {
  try {
    if (activeSessionIds.value.has(id)) {
      stopPolling(id);
      await remoteShellDisconnect(id);
    }
    await remoteShellDeleteSession(id);
    sessions.value = sessions.value.filter((s) => s.id !== id);
    summaries.value = summaries.value.filter((s) => s.id !== id);
    activeSessionIds.value.delete(id);
    activeSessionIds.value = new Set(activeSessionIds.value);
    if (selectedTabId.value === id) {
      selectedTabId.value = connectedIds.value.length > 0 ? connectedIds.value[0] : null;
    }
    toast.add("success", "已删除");
  } catch (e) {
    toast.add("error", `删除失败: ${e}`);
  }
}

// ─── Dialog actions ───────────────────────────────────────────────
function openNewSession() {
  editSession.value = null;
  showDialog.value = true;
}

function openEditSession(id: string) {
  const session = sessions.value.find((s) => s.id === id);
  if (session) {
    editSession.value = session;
    showDialog.value = true;
  }
}

async function handleSave(input: SessionInput, password: string) {
  try {
    if (editSession.value) {
      await remoteShellUpdateSession(editSession.value.id, input);
      toast.add("success", "已更新");
    } else {
      await remoteShellCreateSession(input, password);
      toast.add("success", "已创建");
    }
    showDialog.value = false;
    await loadData();
  } catch (e) {
    toast.add("error", `保存失败: ${e}`);
  }
}

// ─── Terminal input ───────────────────────────────────────────────
function handleTerminalData(sessionId: string, data: string) {
  remoteShellSendInput(sessionId, data).catch(() => {
    // silently fail — will be caught by polling error
  });
}

// ─── Environment ──────────────────────────────────────────────────
async function handleCreateEnv() {
  const name = newEnvName.value.trim();
  if (!name) return;
  try {
    await remoteShellCreateEnvironment(name);
    environments.value.push(name);
    selectedEnv.value = name;
    newEnvName.value = "";
    showNewEnvInput.value = false;
    toast.add("success", `环境 "${name}" 已创建`);
  } catch (e) {
    toast.add("error", String(e));
  }
}

// ─── Tab management ───────────────────────────────────────────────
function selectTab(id: string) {
  selectedTabId.value = id;
  // Ensure polling is running for this session
  if (activeSessionIds.value.has(id)) {
    startPolling(id);
  }
}

function closeTab(id: string) {
  handleDisconnect(id);
}

function handleSelectSession(id: string) {
  if (activeSessionIds.value.has(id)) {
    selectTab(id);
  }
}

// ─── Lifecycle ────────────────────────────────────────────────────
onMounted(async () => {
  try {
    await remoteShellInit();
  } catch {
    // init may fail if already initialized
  }
  await loadData();

  // Start polling for any already-connected sessions
  for (const id of connectedIds.value) {
    startPolling(id);
  }
});

onUnmounted(() => {
  stopAllPolling();
});
</script>

<template>
  <div class="flex h-full flex-col animate-view-fade">
    <!-- ═══ Toolbar ═══ -->
    <div class="flex shrink-0 items-center gap-3 border-b border-paper-deep/60 bg-paper px-4 py-2.5">
      <div class="flex items-center gap-2">
        <Terminal class="h-4 w-4 text-bamboo" />
        <h1 class="text-sm font-semibold text-ink">远程终端</h1>
      </div>

      <div class="h-4 w-px bg-paper-deep/60" />

      <!-- Environment selector -->
      <div class="flex items-center gap-1.5">
        <Layers class="h-3.5 w-3.5 text-ink-faint" />
        <select
          v-model="selectedEnv"
          class="rounded-lg border border-paper-deep/60 bg-paper-warm/50 px-2 py-1 text-xs text-ink outline-none transition-colors focus:border-bamboo/50"
        >
          <option v-for="env in environments" :key="env" :value="env">
            {{ env }}
          </option>
        </select>
        <div v-if="showNewEnvInput" class="flex items-center gap-1">
          <input
            v-model="newEnvName"
            type="text"
            placeholder="环境名"
            class="w-24 rounded-lg border border-paper-deep/60 bg-paper-warm/50 px-2 py-1 text-xs text-ink outline-none focus:border-bamboo/50"
            @keydown.enter="handleCreateEnv"
          />
          <button
            class="rounded p-1 text-xs text-bamboo hover:bg-bamboo/10 transition-colors"
            @click="handleCreateEnv"
          >
            添加
          </button>
        </div>
        <button
          v-else
          class="rounded p-1 text-ink-faint hover:text-ink hover:bg-paper-deep/50 transition-colors"
          title="新建环境"
          @click="showNewEnvInput = true"
        >
          <Plus class="h-3 w-3" />
        </button>
      </div>

      <div class="flex-1" />

      <!-- Actions -->
      <Button size="sm" @click="openNewSession">
        <Plus class="mr-1 h-3.5 w-3.5" />
        新建会话
      </Button>

      <!-- Right panel toggle -->
      <button
        class="rounded-lg p-1.5 text-ink-faint hover:text-ink hover:bg-paper-deep/50 transition-colors"
        :class="{ 'text-bamboo bg-bamboo/10': rightPanelOpen }"
        title="右侧面板"
        @click="rightPanelOpen = !rightPanelOpen"
      >
        <PanelRight v-if="!rightPanelOpen" class="h-4 w-4" />
        <PanelRightClose v-else class="h-4 w-4" />
      </button>
    </div>

    <!-- ═══ Main content ═══ -->
    <div class="flex flex-1 overflow-hidden">
      <!-- Left: Session list -->
      <div
        class="flex w-60 shrink-0 flex-col border-r border-paper-deep/60 bg-paper-warm/30"
      >
        <div class="px-3 py-2 border-b border-paper-deep/40">
          <span class="text-xs font-medium text-ink-faint">会话列表</span>
        </div>
        <div class="flex-1 overflow-hidden">
          <SessionList
            :sessions="filteredSessions"
            :active-sessions="activeSessionIds"
            :selected-id="selectedTabId"
            @select="handleSelectSession"
            @connect="handleConnect"
            @disconnect="handleDisconnect"
            @edit="openEditSession"
            @delete="handleDelete"
          />
        </div>
      </div>

      <!-- Center: Terminal area -->
      <div class="flex flex-1 flex-col overflow-hidden bg-paper">
        <!-- Welcome message -->
        <div
          v-if="welcomeVisible"
          class="flex flex-1 items-center justify-center"
        >
          <div class="text-center max-w-sm">
            <Terminal class="mx-auto h-12 w-12 text-ink-faint/20 mb-4" />
            <h2 class="text-lg font-display font-semibold text-ink-soft mb-2">
              远程终端
            </h2>
            <p class="text-sm text-ink-faint leading-relaxed">
              在左侧会话列表中选择一个会话，或点击
              <br />
              <strong class="text-bamboo">新建会话</strong> 添加远程连接。
            </p>
            <p class="mt-3 text-xs text-ink-faint/60">
              支持 SSH 和 Telnet 协议，集成 SFTP 文件浏览和主机指标监控
            </p>
          </div>
        </div>

        <!-- Loading -->
        <div
          v-else-if="loading"
          class="flex flex-1 items-center justify-center"
        >
          <div class="h-5 w-5 animate-spin rounded-full border-2 border-bamboo border-t-transparent" />
        </div>

        <!-- Connected sessions with tabs -->
        <template v-else>
          <!-- Tab bar -->
          <div
            v-if="connectedIds.length > 0"
            class="flex shrink-0 items-center overflow-x-auto border-b border-paper-deep/60 bg-paper-warm/30 scrollbar-hidden"
          >
            <button
              v-for="session in connectedSummaries"
              :key="session.id"
              class="group relative flex shrink-0 items-center gap-1.5 border-r border-paper-deep/40 px-3 py-2 text-xs transition-colors"
              :class="
                selectedTabId === session.id
                  ? 'bg-paper text-ink border-b-2 border-b-bamboo'
                  : 'text-ink-faint hover:text-ink hover:bg-paper-deep/20'
              "
              @click="selectTab(session.id)"
            >
              <span
                class="h-1.5 w-1.5 rounded-full"
                :class="
                  activeSessionIds.has(session.id)
                    ? 'bg-bamboo'
                    : 'bg-ink-ghost'
                "
              />
              <span class="truncate max-w-[100px]">{{ session.name }}</span>
              <span class="text-[10px] text-ink-faint/60">({{ session.protocol }})</span>
              <button
                class="ml-1 rounded p-0.5 text-ink-faint/50 opacity-0 group-hover:opacity-100 hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 transition-all"
                @click.stop="closeTab(session.id)"
              >
                <X class="h-3 w-3" />
              </button>
            </button>
          </div>

          <!-- Terminal view -->
          <div class="flex-1 overflow-hidden p-2">
            <div
              v-for="id in connectedIds"
              v-show="selectedTabId === id"
              :key="id"
              class="h-full w-full"
            >
              <XtermTerminal
                :ref="(el: any) => setTerminalRef(id, el)"
                :font-size="14"
                :disabled="false"
                :on-data="(data: string) => handleTerminalData(id, data)"
              />
            </div>
          </div>
        </template>
      </div>

      <!-- Right panel -->
      <div
        v-if="rightPanelOpen"
        class="flex w-72 shrink-0 flex-col border-l border-paper-deep/60 bg-paper-warm/30 transition-all"
      >
        <!-- Panel tabs -->
        <div class="flex border-b border-paper-deep/40">
          <button
            class="flex-1 px-3 py-2 text-xs font-medium transition-colors"
            :class="
              rightPanelTab === 'sftp'
                ? 'text-bamboo border-b-2 border-b-bamboo'
                : 'text-ink-faint hover:text-ink'
            "
            @click="rightPanelTab = 'sftp'"
          >
            <Database class="mr-1.5 inline-block h-3 w-3" />
            SFTP
          </button>
          <button
            class="flex-1 px-3 py-2 text-xs font-medium transition-colors"
            :class="
              rightPanelTab === 'metrics'
                ? 'text-bamboo border-b-2 border-b-bamboo'
                : 'text-ink-faint hover:text-ink'
            "
            @click="rightPanelTab = 'metrics'"
          >
            <Monitor class="mr-1.5 inline-block h-3 w-3" />
            指标
          </button>
        </div>

        <!-- Panel content -->
        <div class="flex-1 overflow-hidden pt-2">
          <SftpPanel v-show="rightPanelTab === 'sftp'" :session-id="rightPanelSessionId" />
          <MetricsPanel v-show="rightPanelTab === 'metrics'" :session-id="rightPanelSessionId" />
        </div>
      </div>
    </div>

    <!-- ═══ Session dialog ═══ -->
    <SessionDialog
      :show="showDialog"
      :edit-session="editSession"
      @save="handleSave"
      @close="showDialog = false"
    />
  </div>
</template>
