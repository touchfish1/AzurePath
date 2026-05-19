<script setup lang="ts">
import { onMounted, watch, ref } from "vue";
import { Sun, Moon, Monitor, Save, Power } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import NightModeSettings from "@/components/NightModeSettings.vue";
import { useSettingsStore } from "@/stores/settings";
import { useToastStore } from "@/stores/toast";
import { useThemeStore } from "@/stores/theme";

const store = useSettingsStore();
const themeStore = useThemeStore();
const toast = useToastStore();

// Autostart state
const autostartEnabled = ref(false);
const autostartChecking = ref(true);

onMounted(async () => {
  store.load();
  // Check autostart status
  try {
    const { isEnabled } = await import("@tauri-apps/plugin-autostart");
    autostartEnabled.value = await isEnabled();
  } catch {
    autostartEnabled.value = false;
  } finally {
    autostartChecking.value = false;
  }
});

async function toggleAutostart() {
  try {
    const { enable, disable } = await import("@tauri-apps/plugin-autostart");
    if (autostartEnabled.value) {
      await disable();
      autostartEnabled.value = false;
      toast.add("success", "已关闭开机自启动");
    } else {
      await enable();
      autostartEnabled.value = true;
      toast.add("success", "已开启开机自启动");
    }
  } catch (e) {
    toast.add("error", `自启动设置失败: ${e}`);
  }
}

// Sync theme changes
watch(
  () => store.settings.theme,
  (val) => {
    if (val === "light" || val === "dark" || val === "system") {
      themeStore.setTheme(val);
    }
  },
);

async function handleSave() {
  await store.save();
  toast.add("success", "设置已保存");
}

async function selectDownloadDir() {
  try {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const dir = await open({ directory: true, multiple: false, title: "选择下载目录" });
    if (dir) {
      store.update("downloadDir", dir as string);
    }
  } catch {
    // dialog plugin might not be available
  }
}

const intervalOptions = [
  { value: 1000, label: "1s" },
  { value: 3000, label: "3s" },
  { value: 5000, label: "5s" },
  { value: 10000, label: "10s" },
];

const maxItemsOptions = [
  { value: 100, label: "100" },
  { value: 500, label: "500" },
  { value: 1000, label: "1000" },
  { value: 0, label: "不限制" },
];
</script>

<template>
  <div class="flex h-full flex-col p-6 space-y-6 animate-view-fade overflow-y-auto">
    <!-- Header -->
    <div>
      <h1 class="text-2xl font-display font-bold text-ink">设置</h1>
      <p class="mt-0.5 text-sm text-ink-faint">自定义应用行为和偏好</p>
    </div>

    <!-- Theme -->
    <div class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm">
      <h2 class="text-sm font-semibold text-ink mb-4">外观</h2>
      <div class="flex flex-wrap gap-3">
        <button
          class="flex items-center gap-2 rounded-lg border-2 px-4 py-3 text-sm font-medium transition-all"
          :class="store.settings.theme === 'light'
            ? 'border-bamboo/50 bg-bamboo/5 text-bamboo'
            : 'border-paper-deep/40 text-ink-soft hover:border-paper-deep hover:text-ink'"
          @click="store.update('theme', 'light')"
        >
          <Sun class="h-4 w-4" />
          浅色
        </button>
        <button
          class="flex items-center gap-2 rounded-lg border-2 px-4 py-3 text-sm font-medium transition-all"
          :class="store.settings.theme === 'dark'
            ? 'border-bamboo/50 bg-bamboo/5 text-bamboo'
            : 'border-paper-deep/40 text-ink-soft hover:border-paper-deep hover:text-ink'"
          @click="store.update('theme', 'dark')"
        >
          <Moon class="h-4 w-4" />
          深色
        </button>
        <button
          class="flex items-center gap-2 rounded-lg border-2 px-4 py-3 text-sm font-medium transition-all"
          :class="store.settings.theme === 'system'
            ? 'border-bamboo/50 bg-bamboo/5 text-bamboo'
            : 'border-paper-deep/40 text-ink-soft hover:border-paper-deep hover:text-ink'"
          @click="store.update('theme', 'system')"
        >
          <Monitor class="h-4 w-4" />
          跟随系统
        </button>
      </div>
    </div>

    <!-- Night Mode Schedule -->
    <NightModeSettings />

    <!-- Autostart -->
    <div class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm">
      <h2 class="text-sm font-semibold text-ink mb-4 flex items-center gap-2">
        <Power class="h-4 w-4 text-bamboo" />
        启动
      </h2>
      <div class="flex items-center justify-between">
        <div>
          <p class="text-sm text-ink">开机自启动</p>
          <p class="text-xs text-ink-faint mt-0.5">登录时自动启动 AzurePath</p>
        </div>
        <label class="relative inline-flex cursor-pointer items-center">
          <input
            type="checkbox"
            :checked="autostartEnabled"
            :disabled="autostartChecking"
            class="peer sr-only"
            @change="toggleAutostart"
          />
          <div
            class="h-6 w-11 rounded-full border border-paper-deep/40 bg-paper-deep/30 transition-colors peer-checked:bg-bamboo peer-focus:ring-2 peer-focus:ring-bamboo/30"
          >
            <div
              class="h-5 w-5 translate-x-0.5 rounded-full bg-white shadow-sm transition-transform peer-checked:translate-x-[22px]"
            ></div>
          </div>
        </label>
      </div>
    </div>

    <!-- Clipboard -->
    <div class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm">
      <h2 class="text-sm font-semibold text-ink mb-4">剪贴板</h2>
      <div class="space-y-4">
        <div>
          <label class="mb-1.5 block text-xs font-medium text-ink-soft">监控间隔</label>
          <div class="flex flex-wrap gap-2">
            <button
              v-for="opt in intervalOptions"
              :key="opt.value"
              class="rounded-lg border px-3 py-1.5 text-xs font-medium transition-all"
              :class="store.settings.clipboardInterval === opt.value
                ? 'border-bamboo/40 bg-bamboo/5 text-bamboo'
                : 'border-paper-deep text-ink-faint hover:border-paper-deep hover:text-ink'"
              @click="store.update('clipboardInterval', opt.value)"
            >
              {{ opt.label }}
            </button>
          </div>
        </div>
        <div>
          <label class="mb-1.5 block text-xs font-medium text-ink-soft">存储上限</label>
          <div class="flex flex-wrap gap-2">
            <button
              v-for="opt in maxItemsOptions"
              :key="opt.value"
              class="rounded-lg border px-3 py-1.5 text-xs font-medium transition-all"
              :class="store.settings.clipboardMaxItems === opt.value
                ? 'border-bamboo/40 bg-bamboo/5 text-bamboo'
                : 'border-paper-deep text-ink-faint hover:border-paper-deep hover:text-ink'"
              @click="store.update('clipboardMaxItems', opt.value)"
            >
              {{ opt.label }}
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Scan defaults -->
    <div class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm">
      <h2 class="text-sm font-semibold text-ink mb-4">扫描</h2>
      <div class="flex flex-wrap gap-6">
        <div>
          <label class="mb-1 block text-xs font-medium text-ink-soft">Ping 默认次数</label>
          <input
            v-model.number="store.settings.pingCount"
            type="number"
            min="1"
            max="100"
            class="w-24 rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
          />
        </div>
        <div>
          <label class="mb-1 block text-xs font-medium text-ink-soft">默认超时 (ms)</label>
          <input
            v-model.number="store.settings.pingTimeout"
            type="number"
            min="100"
            max="30000"
            step="100"
            class="w-24 rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
          />
        </div>
      </div>
    </div>

    <!-- File download -->
    <div class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm">
      <h2 class="text-sm font-semibold text-ink mb-4">文件</h2>
      <div>
        <label class="mb-1 block text-xs font-medium text-ink-soft">下载目录</label>
        <div class="flex items-center gap-2">
          <input
            :value="store.settings.downloadDir"
            readonly
            placeholder="未设置，使用系统默认下载目录"
            class="flex-1 rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors"
          />
          <Button variant="secondary" size="sm" @click="selectDownloadDir">
            选择目录
          </Button>
        </div>
      </div>
    </div>

    <!-- Data retention -->
    <div class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm">
      <h2 class="text-sm font-semibold text-ink mb-4">数据</h2>
      <div>
        <label class="mb-1 block text-xs font-medium text-ink-soft">保留策略</label>
        <div class="flex items-center gap-2">
          <span class="text-sm text-ink-soft">保留最近</span>
          <input
            v-model.number="store.settings.retentionDays"
            type="number"
            min="1"
            max="365"
            class="w-20 rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
          />
          <span class="text-sm text-ink-soft">天的数据</span>
        </div>
      </div>
    </div>

    <!-- Notifications -->
    <div class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm">
      <h2 class="text-sm font-semibold text-ink mb-4">通知</h2>
      <div class="space-y-3">
        <label class="flex items-center gap-3 cursor-pointer">
          <input
            type="checkbox"
            :checked="store.settings.notifyFileTransfer"
            class="h-4 w-4 rounded border-paper-deep/40 text-bamboo focus:ring-bamboo/30"
            @change="store.update('notifyFileTransfer', ($event.target as HTMLInputElement).checked)"
          />
          <span class="text-sm text-ink">文件传输</span>
        </label>
        <label class="flex items-center gap-3 cursor-pointer">
          <input
            type="checkbox"
            :checked="store.settings.notifyChatMessage"
            class="h-4 w-4 rounded border-paper-deep/40 text-bamboo focus:ring-bamboo/30"
            @change="store.update('notifyChatMessage', ($event.target as HTMLInputElement).checked)"
          />
          <span class="text-sm text-ink">聊天消息</span>
        </label>
        <label class="flex items-center gap-3 cursor-pointer">
          <input
            type="checkbox"
            :checked="store.settings.notifyScanComplete"
            class="h-4 w-4 rounded border-paper-deep/40 text-bamboo focus:ring-bamboo/30"
            @change="store.update('notifyScanComplete', ($event.target as HTMLInputElement).checked)"
          />
          <span class="text-sm text-ink">扫描完成</span>
        </label>
      </div>
    </div>

    <!-- Save button -->
    <div class="flex justify-end">
      <Button :disabled="store.saving" @click="handleSave">
        <Save class="mr-1.5 h-3.5 w-3.5" />
        {{ store.saving ? "保存中..." : "保存设置" }}
      </Button>
    </div>
  </div>
</template>
