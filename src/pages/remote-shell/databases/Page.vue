<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import {
  Plus,
  Trash2,
  TestTube,
  Database,
  Search,
  Plug,
} from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import DatabaseConnectionDialog from "@/components/remote-shell/DatabaseConnectionDialog.vue";
import MySqlPanel from "@/components/remote-shell/MySqlPanel.vue";
import PostgreSqlPanel from "@/components/remote-shell/PostgreSqlPanel.vue";
import RedisPanel from "@/components/remote-shell/RedisPanel.vue";
import {
  remoteShellListDbConnections,
  remoteShellCreateDbConnection,
  remoteShellDeleteDbConnection,
  remoteShellTestDbConnection,
  type DbConnection,
  type DbConnectionInput,
} from "@/lib/tauri";

// ─── State ──────────────────────────────────────────────────────
const connections = ref<DbConnection[]>([]);
const searchQuery = ref("");
const selectedConnId = ref<string | null>(null);
const loading = ref(false);
const showDialog = ref(false);
const editingConn = ref<DbConnection | null>(null);
const testResult = ref<{ id: string; success: boolean; message: string } | null>(null);

// ─── Computed ───────────────────────────────────────────────────
const filteredConnections = computed(() => {
  const q = searchQuery.value.toLowerCase().trim();
  if (!q) return connections.value;
  return connections.value.filter(
    (c) =>
      c.name.toLowerCase().includes(q) ||
      c.host.toLowerCase().includes(q) ||
      c.dbType.toLowerCase().includes(q)
  );
});

interface ConnectionGroup {
  label: string;
  type: string;
  icon: string;
  items: DbConnection[];
}

const groupedConnections = computed(() => {
  const groups: Record<string, ConnectionGroup> = {
    mysql: { label: "MySQL", type: "mysql", icon: "M", items: [] },
    postgresql: { label: "PostgreSQL", type: "postgresql", icon: "P", items: [] },
    redis: { label: "Redis", type: "redis", icon: "R", items: [] },
  };

  for (const conn of filteredConnections.value) {
    if (groups[conn.dbType]) {
      groups[conn.dbType].items.push(conn);
    }
  }

  // Remove empty groups and order
  return Object.values(groups).filter((g) => g.items.length > 0);
});

const selectedConn = computed(() =>
  connections.value.find((c) => c.id === selectedConnId.value)
);

// ─── Methods ────────────────────────────────────────────────────
async function loadConnections() {
  loading.value = true;
  try {
    connections.value = await remoteShellListDbConnections();
  } catch (e) {
    connections.value = [];
  } finally {
    loading.value = false;
  }
}

function openNewDialog() {
  editingConn.value = null;
  showDialog.value = true;
}

async function handleSave(input: DbConnectionInput, password: string) {
  try {
    await remoteShellCreateDbConnection(input, password);
    showDialog.value = false;
    editingConn.value = null;
    await loadConnections();
  } catch (e) {
    // Re-throw so the dialog can handle it if needed
    throw e;
  }
}

async function handleDelete(id: string) {
  try {
    await remoteShellDeleteDbConnection(id);
    if (selectedConnId.value === id) {
      selectedConnId.value = null;
    }
    await loadConnections();
  } catch (e) {
    // Error handled by the UI
  }
}

async function handleTest(id: string) {
  testResult.value = null;
  try {
    const message = await remoteShellTestDbConnection(id);
    testResult.value = { id, success: true, message };
  } catch (e) {
    testResult.value = { id, success: false, message: String(e) };
  }
  // Clear the result after 5 seconds
  setTimeout(() => {
    if (testResult.value?.id === id) {
      testResult.value = null;
    }
  }, 5000);
}

function selectConnection(id: string) {
  selectedConnId.value = id;
  testResult.value = null;
}

onMounted(() => {
  loadConnections();
});
</script>

<template>
  <div class="flex h-full animate-view-fade">
    <!-- Left: Connection Sidebar -->
    <div class="flex w-64 shrink-0 flex-col border-r border-paper-deep/50 bg-paper-warm/20">
      <!-- Header -->
      <div class="border-b border-paper-deep/30 px-4 py-3">
        <div class="mb-3 flex items-center justify-between">
          <h2 class="text-xs font-semibold uppercase tracking-wider text-ink-faint">数据库连接</h2>
          <Button size="sm" variant="ghost" @click="openNewDialog">
            <Plus class="h-3.5 w-3.5" />
          </Button>
        </div>
        <div class="relative">
          <Search class="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-ink-faint" />
          <input
            v-model="searchQuery"
            type="text"
            placeholder="搜索连接..."
            class="w-full rounded-lg border border-paper-deep/30 bg-paper-warm/50 py-1.5 pl-8 pr-3 text-xs text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40"
          />
        </div>
      </div>

      <!-- Connection List -->
      <div class="flex-1 overflow-y-auto p-2">
        <div v-if="loading" class="px-3 py-8 text-center text-xs text-ink-faint">
          加载中...
        </div>
        <template v-else>
          <div v-for="group in groupedConnections" :key="group.type" class="mb-3">
            <div class="mb-1 px-2 text-[10px] font-semibold uppercase tracking-wider text-ink-faint">
              {{ group.label }}
            </div>
            <div
              v-for="conn in group.items"
              :key="conn.id"
              class="group relative mb-0.5 cursor-pointer rounded-lg px-3 py-2 transition-colors"
              :class="
                selectedConnId === conn.id
                  ? 'bg-bamboo/10 text-bamboo'
                  : 'text-ink-soft hover:bg-paper-deep/30 hover:text-ink'
              "
              @click="selectConnection(conn.id)"
            >
              <div class="flex items-center justify-between">
                <div class="min-w-0 flex-1">
                  <div class="flex items-center gap-1.5">
                    <Database class="h-3.5 w-3.5 shrink-0" />
                    <span class="truncate text-sm font-medium">{{ conn.name }}</span>
                  </div>
                  <div class="mt-0.5 truncate pl-5 text-[11px] text-ink-faint">
                    {{ conn.host }}:{{ conn.port }}
                  </div>
                  <div v-if="conn.defaultDatabase" class="truncate pl-5 text-[11px] text-ink-faint">
                    {{ conn.username }}@{{ conn.defaultDatabase }}
                  </div>
                </div>
              </div>

              <!-- Hover Actions -->
              <div
                class="absolute right-1 top-1/2 hidden -translate-y-1/2 items-center gap-0.5 rounded-lg bg-paper-warm/90 px-1 py-0.5 shadow-sm group-hover:flex"
              >
                <button
                  class="rounded p-1 text-ink-faint transition-colors hover:text-blue-500"
                  title="测试连接"
                  @click.stop="handleTest(conn.id)"
                >
                  <TestTube class="h-3 w-3" />
                </button>
                <button
                  class="rounded p-1 text-ink-faint transition-colors hover:text-red-500"
                  title="删除连接"
                  @click.stop="handleDelete(conn.id)"
                >
                  <Trash2 class="h-3 w-3" />
                </button>
              </div>
            </div>
          </div>

          <div
            v-if="connections.length === 0 && !loading"
            class="px-3 py-8 text-center text-xs text-ink-faint"
          >
            暂无数据库连接，点击右上角 + 新建
          </div>
          <div
            v-else-if="filteredConnections.length === 0"
            class="px-3 py-8 text-center text-xs text-ink-faint"
          >
            无匹配的连接
          </div>
        </template>
      </div>

      <!-- Bottom: New Connection Button -->
      <div class="border-t border-paper-deep/30 p-3">
        <Button size="sm" class="w-full" variant="outline" @click="openNewDialog">
          <Plus class="mr-1 h-3.5 w-3.5" />
          新建连接
        </Button>
      </div>
    </div>

    <!-- Right: Connection Detail / Panel -->
    <div class="flex-1 overflow-y-auto p-6">
      <!-- No selection -->
      <div
        v-if="!selectedConn"
        class="flex h-full items-center justify-center"
      >
        <div class="text-center">
          <Plug class="mx-auto h-12 w-12 text-ink-faint/30" />
          <h3 class="mt-4 text-base font-medium text-ink-soft">选择一个数据库连接</h3>
          <p class="mt-1 text-sm text-ink-faint">从左侧列表中选择一个连接，或创建一个新连接</p>
        </div>
      </div>

      <template v-else>
        <!-- Connection Info Header -->
        <div class="mb-6">
          <div class="flex items-center justify-between">
            <div>
              <div class="flex items-center gap-2">
                <h3 class="text-lg font-display font-bold text-ink">{{ selectedConn.name }}</h3>
                <span
                  class="inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium"
                  :class="
                    selectedConn.dbType === 'mysql'
                      ? 'bg-blue/10 text-blue ring-1 ring-blue/30'
                      : selectedConn.dbType === 'postgresql'
                        ? 'bg-cyan/10 text-cyan ring-1 ring-cyan/30'
                        : 'bg-red/10 text-red ring-1 ring-red/30'
                  "
                >
                  {{ selectedConn.dbType === "mysql" ? "MySQL" : selectedConn.dbType === "postgresql" ? "PostgreSQL" : "Redis" }}
                </span>
              </div>
              <p class="mt-1 text-sm text-ink-soft">
                {{ selectedConn.host }}:{{ selectedConn.port }}
                <span v-if="selectedConn.username" class="ml-2 text-ink-faint">
                  {{ selectedConn.username }}
                </span>
                <span v-if="selectedConn.defaultDatabase" class="ml-2 text-ink-faint">
                  / {{ selectedConn.defaultDatabase }}
                </span>
              </p>
            </div>
            <div class="flex items-center gap-2">
              <Button
                size="sm"
                variant="outline"
                @click="handleTest(selectedConn.id)"
              >
                <TestTube class="mr-1 h-3.5 w-3.5" />
                测试连接
              </Button>
            </div>
          </div>

          <!-- Test Result -->
          <div
            v-if="testResult && testResult.id === selectedConn.id"
            class="mt-3 rounded-lg px-4 py-2 text-sm"
            :class="
              testResult.success
                ? 'bg-emerald/10 text-emerald'
                : 'bg-danger-bg text-red-600'
            "
          >
            {{ testResult.success ? "连接成功：" : "连接失败：" }}
            {{ testResult.message }}
          </div>
        </div>

        <!-- DB specific panel -->
        <MySqlPanel
          v-if="selectedConn.dbType === 'mysql'"
          :conn-id="selectedConn.id"
        />
        <PostgreSqlPanel
          v-else-if="selectedConn.dbType === 'postgresql'"
          :conn-id="selectedConn.id"
        />
        <RedisPanel
          v-else-if="selectedConn.dbType === 'redis'"
          :conn-id="selectedConn.id"
        />
      </template>
    </div>

    <!-- Connection Dialog -->
    <DatabaseConnectionDialog
      :show="showDialog"
      :edit-connection="editingConn"
      @save="handleSave"
      @close="
        showDialog = false;
        editingConn = null;
      "
    />
  </div>
</template>
