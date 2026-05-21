<script setup lang="ts">
import { ref, computed, watch, reactive } from "vue";
import { X, Save, KeyRound } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import type { RemoteSession, SessionInput } from "@/lib/tauri";

interface Props {
  show: boolean;
  editSession: RemoteSession | null;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  save: [input: SessionInput, password: string];
  close: [];
}>();

const form = reactive({
  name: "",
  protocol: "ssh" as "ssh" | "telnet",
  host: "",
  port: 22,
  username: "",
  encoding: "utf-8",
  keepaliveSecs: 30,
  environment: "",
});

const password = ref("");
const passwordConfirm = ref("");
const errors = ref<Record<string, string>>({});
const showPassword = ref(false);

const isEditing = computed(() => props.editSession !== null);

const title = computed(() => (isEditing.value ? "编辑会话" : "新建会话"));

const encodingOptions = [
  { value: "utf-8", label: "UTF-8" },
  { value: "gbk", label: "GBK" },
  { value: "latin-1", label: "Latin-1" },
  { value: "shift-jis", label: "Shift-JIS" },
  { value: "euc-kr", label: "EUC-KR" },
];

const keepaliveOptions = [
  { value: 0, label: "禁用" },
  { value: 10, label: "10s" },
  { value: 30, label: "30s" },
  { value: 60, label: "60s" },
  { value: 120, label: "120s" },
  { value: 300, label: "300s" },
];

function resetForm() {
  form.name = "";
  form.protocol = "ssh";
  form.host = "";
  form.port = 22;
  form.username = "";
  form.encoding = "utf-8";
  form.keepaliveSecs = 30;
  form.environment = "";
  password.value = "";
  passwordConfirm.value = "";
  errors.value = {};
  showPassword.value = false;
}

function fillFromSession(session: RemoteSession) {
  form.name = session.name;
  form.protocol = session.protocol;
  form.host = session.host;
  form.port = session.port;
  form.username = session.username;
  form.encoding = session.encoding;
  form.keepaliveSecs = session.keepaliveSecs;
  form.environment = session.environment;
  password.value = "";
  passwordConfirm.value = "";
  errors.value = {};
}

watch(
  () => props.show,
  (val) => {
    if (val) {
      if (props.editSession) {
        fillFromSession(props.editSession);
      } else {
        resetForm();
      }
    }
  },
);

watch(
  () => props.editSession,
  (session) => {
    if (session && props.show) {
      fillFromSession(session);
    }
  },
);

function validate(): boolean {
  const errs: Record<string, string> = {};

  if (!form.name.trim()) {
    errs.name = "名称不能为空";
  }
  if (!form.host.trim()) {
    errs.host = "主机地址不能为空";
  }
  if (!form.port || form.port < 1 || form.port > 65535) {
    errs.port = "端口范围 1-65535";
  }
  if (!form.username.trim()) {
    errs.username = "用户名不能为空";
  }
  if (!isEditing.value && !password.value.trim()) {
    errs.password = "密码不能为空";
  }
  if (!isEditing.value && password.value !== passwordConfirm.value) {
    errs.passwordConfirm = "两次密码不一致";
  }

  errors.value = errs;
  return Object.keys(errs).length === 0;
}

function handleSave() {
  if (!validate()) return;

  const input: SessionInput = {
    name: form.name.trim(),
    protocol: form.protocol,
    host: form.host.trim(),
    port: form.port,
    username: form.username.trim(),
    encoding: form.encoding,
    keepaliveSecs: form.keepaliveSecs,
    environment: form.environment || undefined,
  };

  // Send password only when creating, or when user entered a new one during edit
  const pw = password.value.trim();
  emit("save", input, pw);
}

function handleClose() {
  emit("close");
}

function handleOverlayClick(e: MouseEvent) {
  if ((e.target as HTMLElement).classList.contains("dialog-overlay")) {
    handleClose();
  }
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") {
    handleClose();
  }
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="show"
      class="dialog-overlay fixed inset-0 z-50 flex items-center justify-center bg-black/30 backdrop-blur-sm animate-fade-in"
      @click="handleOverlayClick"
      @keydown="handleKeydown"
    >
      <div
        class="noise-bg w-full max-w-lg rounded-xl border border-paper-deep/60 bg-paper p-6 shadow-lg animate-scale-in"
        @click.stop
      >
        <!-- Header -->
        <div class="flex items-center justify-between mb-5">
          <h2 class="text-base font-semibold text-ink">{{ title }}</h2>
          <button
            class="rounded-lg p-1.5 text-ink-faint hover:text-ink hover:bg-paper-deep/50 transition-colors"
            @click="handleClose"
          >
            <X class="h-4 w-4" />
          </button>
        </div>

        <!-- Form -->
        <div class="space-y-3.5">
          <!-- Name -->
          <div>
            <label class="mb-1 block text-xs font-medium text-ink-soft">名称</label>
            <input
              v-model="form.name"
              type="text"
              placeholder="例如: 生产服务器-1"
              class="w-full rounded-lg border px-3 py-2 text-sm text-ink placeholder:text-ink-faint/50 outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
              :class="errors.name ? 'border-red-300 dark:border-red-700' : 'border-paper-deep bg-paper-warm/50'"
            />
            <p v-if="errors.name" class="mt-0.5 text-xs text-red-500">{{ errors.name }}</p>
          </div>

          <!-- Protocol -->
          <div>
            <label class="mb-1 block text-xs font-medium text-ink-soft">协议</label>
            <div class="flex gap-2">
              <label
                class="flex-1 cursor-pointer rounded-lg border px-3 py-2 text-center text-sm transition-colors"
                :class="
                  form.protocol === 'ssh'
                    ? 'border-bamboo/40 bg-bamboo/5 text-bamboo'
                    : 'border-paper-deep bg-paper-warm/50 text-ink-soft hover:border-paper-deep'
                "
              >
                <input v-model="form.protocol" type="radio" value="ssh" class="sr-only" />
                SSH
              </label>
              <label
                class="flex-1 cursor-pointer rounded-lg border px-3 py-2 text-center text-sm transition-colors"
                :class="
                  form.protocol === 'telnet'
                    ? 'border-bamboo/40 bg-bamboo/5 text-bamboo'
                    : 'border-paper-deep bg-paper-warm/50 text-ink-soft hover:border-paper-deep'
                "
              >
                <input v-model="form.protocol" type="radio" value="telnet" class="sr-only" />
                Telnet
              </label>
            </div>
          </div>

          <!-- Host + Port -->
          <div class="flex gap-3">
            <div class="flex-1">
              <label class="mb-1 block text-xs font-medium text-ink-soft">主机地址</label>
              <input
                v-model="form.host"
                type="text"
                placeholder="IP 或域名"
                class="w-full rounded-lg border px-3 py-2 text-sm text-ink placeholder:text-ink-faint/50 outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
                :class="errors.host ? 'border-red-300 dark:border-red-700' : 'border-paper-deep bg-paper-warm/50'"
              />
              <p v-if="errors.host" class="mt-0.5 text-xs text-red-500">{{ errors.host }}</p>
            </div>
            <div class="w-24">
              <label class="mb-1 block text-xs font-medium text-ink-soft">端口</label>
              <input
                v-model.number="form.port"
                type="number"
                min="1"
                max="65535"
                class="w-full rounded-lg border px-3 py-2 text-sm text-ink outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
                :class="errors.port ? 'border-red-300 dark:border-red-700' : 'border-paper-deep bg-paper-warm/50'"
              />
              <p v-if="errors.port" class="mt-0.5 text-xs text-red-500">{{ errors.port }}</p>
            </div>
          </div>

          <!-- Username -->
          <div>
            <label class="mb-1 block text-xs font-medium text-ink-soft">用户名</label>
            <input
              v-model="form.username"
              type="text"
              placeholder="root"
              class="w-full rounded-lg border px-3 py-2 text-sm text-ink placeholder:text-ink-faint/50 outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
              :class="errors.username ? 'border-red-300 dark:border-red-700' : 'border-paper-deep bg-paper-warm/50'"
            />
            <p v-if="errors.username" class="mt-0.5 text-xs text-red-500">{{ errors.username }}</p>
          </div>

          <!-- Password -->
          <div>
            <label class="mb-1 block text-xs font-medium text-ink-soft">
              {{ isEditing ? '密码 (留空不修改)' : '密码' }}
            </label>
            <div class="relative">
              <input
                v-model="password"
                :type="showPassword ? 'text' : 'password'"
                placeholder="********"
                class="w-full rounded-lg border px-3 py-2 pr-9 text-sm text-ink placeholder:text-ink-faint/50 outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
                :class="errors.password ? 'border-red-300 dark:border-red-700' : 'border-paper-deep bg-paper-warm/50'"
              />
              <button
                class="absolute right-2 top-1/2 -translate-y-1/2 rounded p-0.5 text-ink-faint hover:text-ink transition-colors"
                type="button"
                @click="showPassword = !showPassword"
              >
                <KeyRound class="h-3.5 w-3.5" />
              </button>
            </div>
            <p v-if="errors.password" class="mt-0.5 text-xs text-red-500">{{ errors.password }}</p>
          </div>

          <!-- Password Confirm (only for new) -->
          <div v-if="!isEditing">
            <label class="mb-1 block text-xs font-medium text-ink-soft">确认密码</label>
            <input
              v-model="passwordConfirm"
              type="password"
              placeholder="再次输入密码"
              class="w-full rounded-lg border px-3 py-2 text-sm text-ink placeholder:text-ink-faint/50 outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
              :class="errors.passwordConfirm ? 'border-red-300 dark:border-red-700' : 'border-paper-deep bg-paper-warm/50'"
            />
            <p v-if="errors.passwordConfirm" class="mt-0.5 text-xs text-red-500">{{ errors.passwordConfirm }}</p>
          </div>

          <!-- Encoding -->
          <div>
            <label class="mb-1 block text-xs font-medium text-ink-soft">编码</label>
            <select
              v-model="form.encoding"
              class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
            >
              <option v-for="opt in encodingOptions" :key="opt.value" :value="opt.value">
                {{ opt.label }}
              </option>
            </select>
          </div>

          <!-- Keepalive -->
          <div>
            <label class="mb-1 block text-xs font-medium text-ink-soft">心跳保活</label>
            <select
              v-model="form.keepaliveSecs"
              class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
            >
              <option v-for="opt in keepaliveOptions" :key="opt.value" :value="opt.value">
                {{ opt.label }}
              </option>
            </select>
          </div>
        </div>

        <!-- Actions -->
        <div class="mt-6 flex justify-end gap-2">
          <Button variant="ghost" @click="handleClose">取消</Button>
          <Button @click="handleSave">
            <Save class="mr-1.5 h-3.5 w-3.5" />
            {{ isEditing ? '保存' : '创建' }}
          </Button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
