<script setup lang="ts">
import { onMounted, onUnmounted, nextTick, watch, ref } from "vue";
import { Terminal as TerminalIcon, Plus, X, Plug, PlugZap, Save, Clock, Monitor } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import { useTerminalStore } from "@/stores/terminal";
import { useToastStore } from "@/stores/toast";

const store = useTerminalStore();
const toast = useToastStore();

// Refs for terminal container elements, keyed by session id
const terminalContainers = ref<Record<string, HTMLDivElement>>({});

function setContainerRef(sessionId: string) {
  return (el: Element | null) => {
    if (el) {
      terminalContainers.value[sessionId] = el as HTMLDivElement;
      store.setActiveTerminalContainer(sessionId, el as HTMLDivElement);
    }
  };
}

// Show/hide connect form
const showForm = ref(false);

// Initialize terminal UI for a session
function initTerminal(sessionId: string) {
  nextTick(() => {
    const container = terminalContainers.value[sessionId];
    const tab = store.sessions.find((s) => s.id === sessionId);
    if (container && tab && !tab.terminal.element) {
      tab.terminal.open(container);
      tab.fitAddon.fit();

      // Set up input handler
      tab.terminal.onData((data: string) => {
        store.sendInput(data);
      });

      // Set up resize handler
      tab.terminal.onResize((size: { cols: number; rows: number }) => {
        store.resize(size.cols, size.rows);
      });

      // Focus the terminal
      tab.terminal.focus();
    }
  });
}

// Watch for new sessions and re-initialize
watch(
  () => store.sessions.length,
  () => {
    if (store.activeSessionId) {
      nextTick(() => initTerminal(store.activeSessionId));
    }
  },
);

// Watch for active session changes
watch(
  () => store.activeSessionId,
  (newId) => {
    if (newId) {
      nextTick(() => initTerminal(newId));
    }
  },
);

onMounted(async () => {
  await store.attachListeners();

  // Initialize any existing sessions
  if (store.activeSessionId) {
    nextTick(() => initTerminal(store.activeSessionId));
  }
});

onUnmounted(() => {
  store.detachListeners();
});

// Handle connect form submission
async function handleConnect() {
  try {
    await store.connect();
    showForm.value = false;
    // Clear password field after connecting
    store.formPassword = "";
  } catch (e) {
    toast.add("error", `连接失败: ${e}`);
  }
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === "Escape" && showForm.value) {
    showForm.value = false;
  }
}
</script>

<template>
  <div class="flex h-full flex-col">
    <!-- Header -->
    <div class="flex shrink-0 items-center justify-between border-b border-paper-deep/50 px-4 py-2">
      <div class="flex items-center gap-2">
        <TerminalIcon class="h-4 w-4 text-bamboo" />
        <h1 class="text-sm font-semibold text-ink">SSH 终端</h1>
      </div>
      <div class="flex items-center gap-1.5">
        <Button size="sm" variant="ghost" @click="showForm = !showForm">
          <Plus class="mr-1 h-3.5 w-3.5" />
          新建连接
        </Button>
      </div>
    </div>

    <!-- Connect form (collapsible) -->
    <div
      v-if="showForm"
      class="shrink-0 border-b border-paper-deep/50 bg-paper-warm/30 px-4 py-3 animate-fade-in"
    >
      <div class="flex flex-wrap items-end gap-3">
        <div class="flex-1 min-w-[140px]">
          <label class="mb-1 block text-xs font-medium text-ink-soft">主机</label>
          <input
            v-model="store.formHost"
            placeholder="IP 地址或域名"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-1.5 text-sm text-ink placeholder:text-ink-faint/50 outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
            @keydown.enter="handleConnect"
          />
        </div>
        <div class="w-20">
          <label class="mb-1 block text-xs font-medium text-ink-soft">端口</label>
          <input
            v-model.number="store.formPort"
            type="number"
            min="1"
            max="65535"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-1.5 text-sm text-ink outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
          />
        </div>
        <div class="flex-1 min-w-[120px]">
          <label class="mb-1 block text-xs font-medium text-ink-soft">用户名</label>
          <input
            v-model="store.formUsername"
            placeholder="用户名"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-1.5 text-sm text-ink placeholder:text-ink-faint/50 outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
            @keydown.enter="handleConnect"
          />
        </div>
        <div class="flex-1 min-w-[120px]">
          <label class="mb-1 block text-xs font-medium text-ink-soft">密码</label>
          <input
            v-model="store.formPassword"
            type="password"
            placeholder="密码"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-1.5 text-sm text-ink placeholder:text-ink-faint/50 outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
            @keydown.enter="handleConnect"
          />
        </div>
        <div class="flex items-center gap-1.5 pb-px">
          <Button size="sm" :disabled="store.connecting" @click="handleConnect">
            <Plug class="mr-1 h-3.5 w-3.5" />
            {{ store.connecting ? "连接中..." : "连接" }}
          </Button>
          <Button size="sm" variant="ghost" title="保存会话" @click="store.saveSession()">
            <Save class="h-3.5 w-3.5" />
          </Button>
        </div>
      </div>

      <!-- Saved sessions -->
      <div v-if="store.savedSessions.length > 0" class="mt-2 flex flex-wrap items-center gap-2 border-t border-paper-deep/20 pt-2">
        <span class="text-xs text-ink-faint flex items-center gap-1">
          <Clock class="h-3 w-3" />
          已保存:
        </span>
        <button
          v-for="(s, idx) in store.savedSessions"
          :key="idx"
          class="rounded-md border border-paper-deep/40 bg-paper-warm/30 px-2 py-0.5 text-xs text-ink-soft transition-colors hover:bg-paper-deep/30 hover:text-ink"
          @click="store.loadSavedSession(idx)"
        >
          {{ s.label }}
        </button>
      </div>
    </div>

    <!-- Main content area -->
    <div class="flex flex-1 overflow-hidden">
      <!-- Session tabs sidebar -->
      <div
        v-if="store.sessions.length > 0"
        class="flex shrink-0 flex-col border-r border-paper-deep/50 bg-paper-warm/20 w-48"
      >
        <div class="flex items-center justify-between px-3 py-2 border-b border-paper-deep/30">
          <span class="text-xs font-medium text-ink-faint uppercase tracking-wider">会话</span>
          <span class="text-xs text-ink-faint">{{ store.sessions.length }}</span>
        </div>
        <div class="flex-1 overflow-y-auto p-1.5 space-y-0.5">
          <button
            v-for="tab in store.sessions"
            :key="tab.id"
            class="group flex w-full items-center gap-2 rounded-lg px-2.5 py-2 text-left text-xs transition-colors"
            :class="
              store.activeSessionId === tab.id
                ? 'bg-bamboo/10 text-bamboo'
                : 'text-ink-soft hover:bg-paper-deep/30 hover:text-ink'
            "
            @click="store.switchTab(tab.id)"
          >
            <Monitor class="h-3.5 w-3.5 shrink-0" />
            <div class="flex-1 min-w-0">
              <div class="truncate font-medium">{{ tab.username }}@{{ tab.host }}</div>
              <div class="text-[10px] opacity-60">:{{ tab.port }}</div>
            </div>
            <div class="flex shrink-0 gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity">
              <span
                class="inline-block h-2 w-2 rounded-full"
                :class="tab.connected ? 'bg-bamboo' : 'bg-red-500'"
                :title="tab.connected ? '已连接' : '已断开'"
              />
              <button
                class="rounded p-0.5 text-ink-faint hover:text-red-500 hover:bg-red-500/10"
                title="关闭会话"
                @click.stop="store.closeTab(tab.id)"
              >
                <X class="h-3 w-3" />
              </button>
            </div>
          </button>
        </div>
      </div>

      <!-- Terminal area -->
      <div class="flex-1 flex flex-col bg-[#1a1b26] relative">
        <!-- No sessions state -->
        <div
          v-if="store.sessions.length === 0"
          class="flex flex-1 items-center justify-center"
        >
          <div class="text-center max-w-sm">
            <TerminalIcon class="mx-auto h-12 w-12 mb-3 opacity-20 text-ink-faint" />
            <p class="text-sm font-medium text-ink-faint">没有活动的 SSH 会话</p>
            <p class="mt-2 text-xs text-ink-faint/60 leading-relaxed">
              点击右上角「新建连接」按钮
              <br />
              输入主机、端口、用户名和密码进行连接
            </p>
          </div>
        </div>

        <!-- Terminal instances -->
        <template v-for="tab in store.sessions" :key="tab.id">
          <div
            v-show="store.activeSessionId === tab.id"
            :ref="setContainerRef(tab.id)"
            class="flex-1 w-full h-full"
          />
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* Ensure the terminal fills its container */
.terminal {
  width: 100%;
  height: 100%;
}
</style>
