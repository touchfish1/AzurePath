<script setup lang="ts">
import { ref, computed, watch, reactive } from "vue";
import { X, Save, KeyRound } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import type { DesktopSession, DesktopSessionInput } from "@/lib/tauri";

interface Props {
  session: DesktopSession | null;
  visible: boolean;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  close: [];
  save: [input: DesktopSessionInput, password: string];
}>();

const form = reactive({
  name: "",
  protocol: "vnc" as "rdp" | "vnc",
  host: "",
  port: 5900,
  username: "",
  quality: 70,
  desktopWidth: 1280,
  desktopHeight: 720,
  domain: "",
});

const password = ref("");
const errors = ref<Record<string, string>>({});

const isEditing = computed(() => props.session !== null);

const title = computed(() => (isEditing.value ? "编辑会话" : "新建会话"));

function resetForm() {
  form.name = "";
  form.protocol = "vnc";
  form.host = "";
  form.port = 5900;
  form.username = "";
  form.quality = 70;
  form.desktopWidth = 1280;
  form.desktopHeight = 720;
  form.domain = "";
  password.value = "";
  errors.value = {};
}

function fillFromSession(session: DesktopSession) {
  form.name = session.name;
  form.protocol = session.protocol;
  form.host = session.host;
  form.port = session.port;
  form.username = session.username;
  form.quality = session.quality;
  form.desktopWidth = session.desktopWidth || 1280;
  form.desktopHeight = session.desktopHeight || 720;
  form.domain = session.domain || "";
  password.value = "";
  errors.value = {};
}

// Auto-switch port when protocol changes
watch(
  () => form.protocol,
  (protocol) => {
    if (protocol === "rdp") {
      form.port = 3389;
    } else {
      form.port = 5900;
    }
  },
);

watch(
  () => props.visible,
  (val) => {
    if (val) {
      if (props.session) {
        fillFromSession(props.session);
      } else {
        resetForm();
      }
    }
  },
);

watch(
  () => props.session,
  (session) => {
    if (session && props.visible) {
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

  errors.value = errs;
  return Object.keys(errs).length === 0;
}

function handleSave() {
  if (!validate()) return;

  const input: DesktopSessionInput = {
    name: form.name.trim(),
    protocol: form.protocol,
    host: form.host.trim(),
    port: form.port,
    username: form.username.trim(),
    quality: form.quality,
    ...(form.protocol === "rdp"
      ? {
          desktopWidth: form.desktopWidth,
          desktopHeight: form.desktopHeight,
          domain: form.domain.trim() || undefined,
        }
      : {}),
  };

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
      v-if="visible"
      class="dialog-overlay fixed inset-0 z-50 flex items-center justify-center bg-black/30 backdrop-blur-sm animate-fade-in"
      @click="handleOverlayClick"
      @keydown="handleKeydown"
    >
      <div
        class="noise-bg w-full max-w-lg rounded-xl border border-paper-deep/60 bg-paper p-6 shadow-lg animate-scale-in"
        @click.stop
      >
        <!-- Header -->
        <div class="mb-5 flex items-center justify-between">
          <h2 class="text-base font-semibold text-ink">{{ title }}</h2>
          <button
            class="rounded-lg p-1.5 text-ink-faint transition-colors hover:bg-paper-deep/50 hover:text-ink"
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
              placeholder="例如: 开发服务器"
              class="w-full rounded-lg border px-3 py-2 text-sm text-ink outline-none placeholder:text-ink-faint/50 transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
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
                  form.protocol === 'vnc'
                    ? 'border-bamboo/40 bg-bamboo/5 text-bamboo'
                    : 'border-paper-deep bg-paper-warm/50 text-ink-soft hover:border-paper-deep'
                "
              >
                <input v-model="form.protocol" type="radio" value="vnc" class="sr-only" />
                VNC
              </label>
              <label
                class="flex-1 cursor-pointer rounded-lg border px-3 py-2 text-center text-sm transition-colors"
                :class="
                  form.protocol === 'rdp'
                    ? 'border-bamboo/40 bg-bamboo/5 text-bamboo'
                    : 'border-paper-deep bg-paper-warm/50 text-ink-soft hover:border-paper-deep'
                "
              >
                <input v-model="form.protocol" type="radio" value="rdp" class="sr-only" />
                RDP
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
                class="w-full rounded-lg border px-3 py-2 text-sm text-ink outline-none placeholder:text-ink-faint/50 transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
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
              class="w-full rounded-lg border px-3 py-2 text-sm text-ink outline-none placeholder:text-ink-faint/50 transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
              :class="errors.username ? 'border-red-300 dark:border-red-700' : 'border-paper-deep bg-paper-warm/50'"
            />
          </div>

          <!-- Password -->
          <div>
            <label class="mb-1 block text-xs font-medium text-ink-soft">
              {{ isEditing ? "密码 (留空不修改)" : "密码" }}
            </label>
            <div class="relative">
              <input
                v-model="password"
                type="password"
                placeholder="********"
                class="w-full rounded-lg border px-3 py-2 pr-9 text-sm text-ink outline-none placeholder:text-ink-faint/50 transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
                :class="errors.password ? 'border-red-300 dark:border-red-700' : 'border-paper-deep bg-paper-warm/50'"
              />
              <button
                class="absolute right-2 top-1/2 -translate-y-1/2 rounded p-0.5 text-ink-faint transition-colors hover:text-ink"
                type="button"
              >
                <KeyRound class="h-3.5 w-3.5" />
              </button>
            </div>
            <p v-if="errors.password" class="mt-0.5 text-xs text-red-500">{{ errors.password }}</p>
          </div>

          <!-- Quality -->
          <div>
            <label class="mb-1 block text-xs font-medium text-ink-soft">
              画质: {{ form.quality }}%
            </label>
            <input
              v-model.number="form.quality"
              type="range"
              min="1"
              max="100"
              class="w-full accent-bamboo"
            />
            <div class="flex justify-between text-[10px] text-ink-faint/60">
              <span>流畅</span>
              <span>清晰</span>
            </div>
          </div>
        </div>

        <!-- RDP extra options -->
        <template v-if="form.protocol === 'rdp'">
          <div class="flex gap-3">
            <div class="flex-1">
              <label class="mb-1 block text-xs font-medium text-ink-soft">桌面宽度</label>
              <input
                v-model.number="form.desktopWidth"
                type="number"
                min="640"
                max="7680"
                class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
              />
            </div>
            <div class="flex-1">
              <label class="mb-1 block text-xs font-medium text-ink-soft">桌面高度</label>
              <input
                v-model.number="form.desktopHeight"
                type="number"
                min="480"
                max="4320"
                class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
              />
            </div>
          </div>
          <div>
            <label class="mb-1 block text-xs font-medium text-ink-soft">域 (可选)</label>
            <input
              v-model="form.domain"
              type="text"
              placeholder="WORKGROUP"
              class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none placeholder:text-ink-faint/50 transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
            />
          </div>
        </template>

        <!-- Actions -->
        <div class="mt-6 flex justify-end gap-2">
          <Button variant="ghost" @click="handleClose">取消</Button>
          <Button @click="handleSave">
            <Save class="mr-1.5 h-3.5 w-3.5" />
            {{ isEditing ? "保存" : "创建" }}
          </Button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
