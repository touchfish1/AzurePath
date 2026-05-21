<script setup lang="ts">
import { ref, computed } from "vue";
import { Search, Monitor, Plus, Trash2, Pencil, Plug, Wifi } from "lucide-vue-next";
import type { DesktopSession } from "@/lib/tauri";

interface Props {
  sessions: DesktopSession[];
  selectedId: string | null;
  connections: Record<string, { status: string }>;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  select: [id: string];
  connect: [id: string, password: string];
  delete: [id: string];
  edit: [session: DesktopSession];
  create: [];
}>();

const searchQuery = ref("");
const connectPasswords = ref<Record<string, string>>({});

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

function connectionStatus(id: string): string {
  return props.connections[id]?.status ?? "disconnected";
}

function isConnected(id: string): boolean {
  return connectionStatus(id) === "connected";
}

function protocolColor(protocol: string) {
  return protocol === "vnc"
    ? "bg-bamboo/10 text-bamboo border-bamboo/20"
    : "bg-blue-50 text-blue-700 border-blue-200 dark:bg-blue-900/20 dark:text-blue-400 dark:border-blue-800/30";
}

function handleConnect(id: string) {
  const pw = connectPasswords.value[id] || "";
  emit("connect", id, pw);
}
</script>

<template>
  <div class="flex h-full flex-col">
    <!-- Search -->
    <div class="px-3 pb-2">
      <div class="relative">
        <Search class="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-ink-faint" />
        <input
          v-model="searchQuery"
          type="text"
          placeholder="搜索会话..."
          class="w-full rounded-lg border border-paper-deep/60 bg-paper-warm/50 py-1.5 pl-8 pr-3 text-xs text-ink outline-none placeholder:text-ink-faint/50 transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
        />
      </div>
    </div>

    <!-- List -->
    <div class="flex-1 space-y-0.5 overflow-y-auto px-3 pb-3">
      <div
        v-if="filteredSessions.length === 0"
        class="flex flex-col items-center justify-center py-8 text-center"
      >
        <Monitor class="mb-2 h-6 w-6 text-ink-faint/40" />
        <p class="text-xs text-ink-faint/60">无匹配会话</p>
      </div>

      <div
        v-for="session in filteredSessions"
        :key="session.id"
        class="group relative cursor-pointer rounded-lg px-2.5 py-2 transition-colors"
        :class="
          selectedId === session.id
            ? 'bg-bamboo/10'
            : 'hover:bg-paper-deep/40'
        "
        @click="emit('select', session.id)"
      >
        <!-- Header row -->
        <div class="flex items-center gap-2">
          <div class="relative shrink-0">
            <Monitor
              class="h-4 w-4"
              :class="isConnected(session.id) ? 'text-bamboo' : 'text-ink-faint/60'"
            />
            <span
              class="absolute -bottom-0.5 -right-0.5 h-2 w-2 rounded-full border-2 border-paper"
              :class="isConnected(session.id) ? 'bg-bamboo' : 'bg-ink-ghost'"
            />
          </div>

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

          <!-- Hover actions -->
          <div class="hidden shrink-0 items-center gap-0.5 group-hover:flex">
            <button
              v-if="!isConnected(session.id)"
              class="rounded p-1 text-bamboo transition-colors hover:bg-bamboo/10"
              title="连接"
              @click.stop="handleConnect(session.id)"
            >
              <Plug class="h-3.5 w-3.5" />
            </button>
            <button
              class="rounded p-1 text-ink-faint transition-colors hover:text-blue-500 hover:bg-blue-50 dark:hover:bg-blue-900/20"
              title="编辑"
              @click.stop="emit('edit', session)"
            >
              <Pencil class="h-3.5 w-3.5" />
            </button>
            <button
              class="rounded p-1 text-ink-faint transition-colors hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20"
              title="删除"
              @click.stop="emit('delete', session.id)"
            >
              <Trash2 class="h-3.5 w-3.5" />
            </button>
          </div>

          <!-- Connected indicator -->
          <div v-if="isConnected(session.id)" class="hidden shrink-0 items-center group-hover:hidden">
            <Wifi class="h-3.5 w-3.5 text-bamboo" />
          </div>
        </div>

        <!-- Password input for disconnected sessions -->
        <div v-if="!isConnected(session.id)" class="mt-1.5 pl-6">
          <div class="flex items-center gap-1">
            <input
              v-model="connectPasswords[session.id]"
              type="password"
              placeholder="密码"
              class="flex-1 rounded border border-paper-deep/60 bg-paper-warm/50 px-2 py-1 text-[11px] text-ink outline-none placeholder:text-ink-faint/40 transition-colors focus:border-bamboo/50"
              @click.stop
              @keydown.enter.stop="handleConnect(session.id)"
            />
            <button
              class="rounded px-2 py-1 text-[11px] font-medium text-cloud bg-bamboo transition-colors hover:bg-bamboo-light"
              @click.stop="handleConnect(session.id)"
            >
              连接
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Add button -->
    <div class="border-t border-paper-deep/40 px-3 py-2">
      <button
        class="flex w-full items-center justify-center gap-1.5 rounded-lg border border-dashed border-paper-deep/60 py-2 text-xs text-ink-faint transition-colors hover:border-bamboo/40 hover:text-bamboo hover:bg-bamboo/5"
        @click="emit('create')"
      >
        <Plus class="h-3.5 w-3.5" />
        新建会话
      </button>
    </div>
  </div>
</template>
