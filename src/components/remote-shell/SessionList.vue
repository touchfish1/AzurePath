<script setup lang="ts">
import { computed, ref } from "vue";
import { Search, Terminal, Wifi, Trash2, Plug, PlugZap, Server, Pencil } from "lucide-vue-next";
import type { SessionSummary } from "@/lib/tauri";

interface Props {
  sessions: SessionSummary[];
  activeSessions: Set<string>;
  selectedId: string | null;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  select: [id: string];
  connect: [id: string];
  disconnect: [id: string];
  delete: [id: string];
  edit: [id: string];
}>();

const searchQuery = ref("");

const filteredSessions = computed(() => {
  if (!searchQuery.value.trim()) return props.sessions;
  const q = searchQuery.value.toLowerCase();
  return props.sessions.filter(
    (s) =>
      s.name.toLowerCase().includes(q) ||
      s.host.toLowerCase().includes(q) ||
      s.username.toLowerCase().includes(q),
  );
});

function protocolColor(protocol: string) {
  return protocol === "ssh"
    ? "bg-bamboo/10 text-bamboo border-bamboo/20"
    : "bg-yellow-50 text-yellow-700 border-yellow-200 dark:bg-yellow-900/20 dark:text-yellow-400 dark:border-yellow-800/30";
}

function handleDoubleClick(session: SessionSummary) {
  if (props.activeSessions.has(session.id)) {
    emit("disconnect", session.id);
  } else {
    emit("connect", session.id);
  }
}
</script>

<template>
  <div class="flex h-full flex-col">
    <!-- Search -->
    <div class="px-3 pb-2">
      <div class="relative">
        <Search class="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-ink-faint pointer-events-none" />
        <input
          v-model="searchQuery"
          type="text"
          placeholder="搜索会话..."
          class="w-full rounded-lg border border-paper-deep/60 bg-paper-warm/50 pl-8 pr-3 py-1.5 text-xs text-ink placeholder:text-ink-faint/50 outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
        />
      </div>
    </div>

    <!-- List -->
    <div class="flex-1 overflow-y-auto space-y-0.5 px-3 pb-3">
      <div v-if="filteredSessions.length === 0" class="flex flex-col items-center justify-center py-8 text-center">
        <Terminal class="h-6 w-6 text-ink-faint/40 mb-2" />
        <p class="text-xs text-ink-faint/60">无匹配会话</p>
      </div>

      <div
        v-for="session in filteredSessions"
        :key="session.id"
        class="group relative flex cursor-pointer items-center gap-2 rounded-lg px-2.5 py-2 transition-colors"
        :class="
          selectedId === session.id
            ? 'bg-bamboo/10'
            : 'hover:bg-paper-deep/40'
        "
        @click="emit('select', session.id)"
        @dblclick="handleDoubleClick(session)"
      >
        <!-- Online indicator -->
        <div class="relative shrink-0">
          <Server class="h-4 w-4" :class="session.isConnected ? 'text-bamboo' : 'text-ink-faint/60'" />
          <span
            class="absolute -bottom-0.5 -right-0.5 h-2 w-2 rounded-full border-2 border-paper"
            :class="session.isConnected ? 'bg-bamboo' : 'bg-ink-ghost'"
          />
        </div>

        <!-- Info -->
        <div class="min-w-0 flex-1">
          <div class="flex items-center gap-1.5">
            <span class="truncate text-xs font-medium text-ink">{{ session.name }}</span>
            <span
              class="shrink-0 rounded border px-1 py-0.5 text-[10px] font-medium uppercase leading-none"
              :class="protocolColor(session.protocol)"
            >
              {{ session.protocol }}
            </span>
          </div>
          <div class="mt-0.5 flex items-center gap-1.5 text-[11px] text-ink-faint">
            <span class="truncate">{{ session.username }}@{{ session.host }}:{{ session.port }}</span>
          </div>
        </div>

        <!-- Actions -->
        <div class="hidden group-hover:flex items-center gap-0.5 shrink-0">
          <button
            v-if="!session.isConnected"
            class="rounded p-1 text-bamboo hover:bg-bamboo/10 transition-colors"
            title="连接"
            @click.stop="emit('connect', session.id)"
          >
            <Plug class="h-3.5 w-3.5" />
          </button>
          <button
            v-else
            class="rounded p-1 text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors"
            title="断开"
            @click.stop="emit('disconnect', session.id)"
          >
            <PlugZap class="h-3.5 w-3.5" />
          </button>
          <button
            class="rounded p-1 text-ink-faint hover:text-blue-500 hover:bg-blue-50 dark:hover:bg-blue-900/20 transition-colors"
            title="编辑"
            @click.stop="emit('edit', session.id)"
          >
            <Pencil class="h-3.5 w-3.5" />
          </button>
          <button
            class="rounded p-1 text-ink-faint hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors"
            title="删除"
            @click.stop="emit('delete', session.id)"
          >
            <Trash2 class="h-3.5 w-3.5" />
          </button>
        </div>

        <!-- Always visible status for connected sessions -->
        <div v-if="session.isConnected" class="flex shrink-0 items-center group-hover:hidden">
          <Wifi class="h-3.5 w-3.5 text-bamboo" />
        </div>
      </div>
    </div>
  </div>
</template>
