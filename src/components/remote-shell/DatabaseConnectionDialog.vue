<script setup lang="ts">
import { ref, watch } from "vue";
import { X } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import type { DbConnection, DbConnectionInput } from "@/lib/tauri";

const props = defineProps<{
  show: boolean;
  editConnection: DbConnection | null;
}>();

const emit = defineEmits<{
  save: [input: DbConnectionInput, password: string];
  close: [];
}>();

const name = ref("");
const dbType = ref<"mysql" | "postgresql" | "redis">("mysql");
const host = ref("");
const port = ref(3306);
const username = ref("");
const password = ref("");
const defaultDatabase = ref("");
const saving = ref(false);

const defaultPorts: Record<string, number> = {
  mysql: 3306,
  postgresql: 5432,
  redis: 6379,
};

watch(dbType, (val) => {
  port.value = defaultPorts[val] || 3306;
});

watch(
  () => props.show,
  (val) => {
    if (val) {
      if (props.editConnection) {
        name.value = props.editConnection.name;
        dbType.value = props.editConnection.dbType as "mysql" | "postgresql" | "redis";
        host.value = props.editConnection.host;
        port.value = props.editConnection.port;
        username.value = props.editConnection.username;
        defaultDatabase.value = props.editConnection.defaultDatabase || "";
        password.value = "";
      } else {
        name.value = "";
        dbType.value = "mysql";
        host.value = "";
        port.value = 3306;
        username.value = "";
        password.value = "";
        defaultDatabase.value = "";
      }
    }
  }
);

function doSave() {
  if (!name.value.trim()) return;
  if (!host.value.trim()) return;
  if (!username.value.trim()) return;
  if (!props.editConnection && !password.value.trim()) return;

  saving.value = true;
  try {
    const input: DbConnectionInput = {
      name: name.value.trim(),
      dbType: dbType.value,
      host: host.value.trim(),
      port: port.value,
      username: username.value.trim(),
      defaultDatabase: defaultDatabase.value.trim() || undefined,
    };
    emit("save", input, password.value);
  } finally {
    saving.value = false;
  }
}

function doClose() {
  emit("close");
}
</script>

<template>
  <div
    v-if="show"
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/45"
    @click.self="doClose"
  >
    <div class="flex w-full max-w-lg flex-col rounded-2xl bg-paper shadow-2xl overflow-hidden">
      <!-- Header -->
      <div class="flex items-center justify-between border-b border-paper-deep/50 px-6 py-4">
        <h2 class="text-base font-semibold text-ink">
          {{ editConnection ? "编辑数据库连接" : "新建数据库连接" }}
        </h2>
        <button
          class="flex h-7 w-7 items-center justify-center rounded-md bg-paper-deep/30 text-ink-faint transition-colors hover:bg-paper-deep/60"
          @click="doClose"
        >
          <X class="h-4 w-4" />
        </button>
      </div>

      <!-- Body -->
      <div class="space-y-4 overflow-y-auto px-6 py-4">
        <!-- Name -->
        <div>
          <label class="mb-1 block text-xs font-medium text-ink-soft">连接名称</label>
          <input
            v-model="name"
            type="text"
            placeholder="例如：生产库、测试库"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink placeholder:text-ink-faint/50 outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
          />
        </div>

        <!-- DB Type -->
        <div>
          <label class="mb-1 block text-xs font-medium text-ink-soft">数据库类型</label>
          <div class="flex gap-2">
            <button
              v-for="type in (['mysql', 'postgresql', 'redis'] as const)"
              :key="type"
              class="flex-1 rounded-lg px-3 py-2 text-sm font-medium transition-colors"
              :class="
                dbType === type
                  ? 'bg-bamboo/15 text-bamboo ring-1 ring-bamboo/30'
                  : 'bg-paper-deep/20 text-ink-soft hover:bg-paper-deep/40 hover:text-ink'
              "
              @click="dbType = type"
            >
              {{ type === "mysql" ? "MySQL" : type === "postgresql" ? "PostgreSQL" : "Redis" }}
            </button>
          </div>
        </div>

        <!-- Host & Port -->
        <div class="flex gap-3">
          <div class="flex-1">
            <label class="mb-1 block text-xs font-medium text-ink-soft">主机地址</label>
            <input
              v-model="host"
              type="text"
              placeholder="例如：192.168.1.100"
              class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink placeholder:text-ink-faint/50 outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
            />
          </div>
          <div class="w-24">
            <label class="mb-1 block text-xs font-medium text-ink-soft">端口</label>
            <input
              v-model.number="port"
              type="number"
              min="1"
              max="65535"
              class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
            />
          </div>
        </div>

        <!-- Username & Password -->
        <div class="flex gap-3">
          <div class="flex-1">
            <label class="mb-1 block text-xs font-medium text-ink-soft">用户名</label>
            <input
              v-model="username"
              type="text"
              placeholder="用户名"
              class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink placeholder:text-ink-faint/50 outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
            />
          </div>
          <div class="flex-1">
            <label class="mb-1 block text-xs font-medium text-ink-soft">
              密码
              <span v-if="editConnection" class="text-ink-faint font-normal">（留空则不修改）</span>
            </label>
            <input
              v-model="password"
              type="password"
              placeholder="密码"
              class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink placeholder:text-ink-faint/50 outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
            />
          </div>
        </div>

        <!-- Default Database -->
        <div>
          <label class="mb-1 block text-xs font-medium text-ink-soft">
            默认数据库
            <span class="text-ink-faint font-normal">（可选）</span>
          </label>
          <input
            v-model="defaultDatabase"
            type="text"
            placeholder="例如：my_database"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink placeholder:text-ink-faint/50 outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
          />
        </div>
      </div>

      <!-- Footer -->
      <div class="flex items-center justify-end gap-2 border-t border-paper-deep/50 px-6 py-3">
        <Button variant="ghost" size="sm" @click="doClose">取消</Button>
        <Button size="sm" :disabled="saving" @click="doSave">
          {{ saving ? "保存中..." : editConnection ? "保存修改" : "创建连接" }}
        </Button>
      </div>
    </div>
  </div>
</template>
