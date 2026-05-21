<script setup lang="ts">
import { ref, watch, onMounted } from "vue";
import { Play, Database, Table } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import {
  remoteShellPgListDatabases,
  remoteShellPgListTables,
  remoteShellPgExecuteQuery,
  type MySqlQueryResult,
} from "@/lib/tauri";

const props = defineProps<{
  connId: string;
}>();

const databases = ref<string[]>([]);
const selectedDatabase = ref("");
const tables = ref<string[]>([]);
const query = ref("");
const queryResult = ref<MySqlQueryResult | null>(null);
const queryError = ref("");
const executing = ref(false);
const loading = ref(false);

async function loadDatabases() {
  loading.value = true;
  try {
    databases.value = await remoteShellPgListDatabases(props.connId);
    if (databases.value.length > 0 && !selectedDatabase.value) {
      selectedDatabase.value = databases.value[0];
    }
  } catch {
    databases.value = [];
  } finally {
    loading.value = false;
  }
}

watch(
  () => props.connId,
  () => {
    selectedDatabase.value = "";
    tables.value = [];
    queryResult.value = null;
    queryError.value = "";
    loadDatabases();
  },
  { immediate: true }
);

watch(selectedDatabase, async (db) => {
  queryResult.value = null;
  queryError.value = "";
  if (!db) {
    tables.value = [];
    return;
  }
  try {
    tables.value = await remoteShellPgListTables(props.connId, db);
  } catch {
    tables.value = [];
  }
});

async function executeQuery() {
  if (!query.value.trim() || !selectedDatabase.value) return;
  executing.value = true;
  queryError.value = "";
  queryResult.value = null;
  try {
    queryResult.value = await remoteShellPgExecuteQuery(
      props.connId,
      selectedDatabase.value,
      query.value
    );
  } catch (e) {
    queryError.value = String(e);
  } finally {
    executing.value = false;
  }
}

onMounted(() => {
  loadDatabases();
});
</script>

<template>
  <div class="flex h-full flex-col gap-4 overflow-y-auto">
    <!-- Database Selector -->
    <div class="flex items-center gap-3">
      <div class="flex items-center gap-1.5">
        <Database class="h-4 w-4 text-ink-faint" />
        <span class="text-xs font-medium text-ink-soft">数据库</span>
      </div>
      <select
        v-model="selectedDatabase"
        class="rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-1.5 text-sm text-ink outline-none transition-colors focus:border-bamboo/40"
      >
        <option v-for="db in databases" :key="db" :value="db">{{ db }}</option>
      </select>
      <span v-if="loading" class="text-xs text-ink-faint">加载中...</span>
    </div>

    <!-- Table List -->
    <div>
      <div class="mb-2 flex items-center gap-1.5">
        <Table class="h-4 w-4 text-ink-faint" />
        <span class="text-xs font-medium text-ink-soft">数据表</span>
      </div>
      <div class="rounded-xl border border-paper-deep/20 bg-paper-warm/30">
        <div
          v-if="tables.length === 0"
          class="px-4 py-8 text-center text-sm text-ink-faint"
        >
          暂无数据表
        </div>
        <div
          v-for="t in tables"
          :key="t"
          class="border-b border-paper-deep/10 px-4 py-2 text-sm text-ink-soft last:border-b-0"
        >
          {{ t }}
        </div>
      </div>
    </div>

    <!-- SQL Query Editor -->
    <div>
      <div class="mb-2 flex items-center justify-between">
        <span class="text-xs font-medium text-ink-soft">SQL 查询</span>
        <Button size="sm" :disabled="executing || !selectedDatabase" @click="executeQuery">
          <Play class="mr-1 h-3.5 w-3.5" />
          {{ executing ? "执行中..." : "执行" }}
        </Button>
      </div>
      <textarea
        v-model="query"
        rows="4"
        placeholder="SELECT * FROM ..."
        class="w-full rounded-xl border border-paper-deep/20 bg-paper-warm/30 px-4 py-3 text-sm font-mono text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/50 resize-y"
        @keydown.ctrl.enter="executeQuery"
      />
    </div>

    <!-- Query Error -->
    <div v-if="queryError" class="rounded-lg bg-danger-bg px-4 py-2 text-sm text-red-600">
      {{ queryError }}
    </div>

    <!-- Query Results -->
    <div v-if="queryResult">
      <div class="mb-2 flex items-center gap-3 text-xs text-ink-faint">
        <span v-if="queryResult.elapsedMs !== undefined">
          耗时：{{ queryResult.elapsedMs }} ms
        </span>
        <span>
          影响行数：{{ queryResult.affectedRows }}
        </span>
      </div>
      <div class="overflow-x-auto rounded-xl border border-paper-deep/20">
        <table class="w-full text-sm">
          <thead v-if="queryResult.columns.length > 0">
            <tr class="bg-paper-deep/10 text-ink-soft text-xs uppercase tracking-wider">
              <th
                v-for="col in queryResult.columns"
                :key="col"
                class="px-3 py-2 text-left whitespace-nowrap"
              >
                {{ col }}
              </th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="(row, ri) in queryResult.rows"
              :key="ri"
              class="border-t border-paper-deep/10"
            >
              <td
                v-for="(cell, ci) in row"
                :key="ci"
                class="px-3 py-2 text-ink whitespace-nowrap"
                :class="typeof cell === 'number' ? 'font-mono text-right' : ''"
              >
                {{ cell === null ? "NULL" : String(cell) }}
              </td>
            </tr>
          </tbody>
        </table>
        <div
          v-if="queryResult.rows.length === 0"
          class="px-4 py-8 text-center text-sm text-ink-faint"
        >
          查询完成，没有返回数据
        </div>
      </div>
    </div>
  </div>
</template>
