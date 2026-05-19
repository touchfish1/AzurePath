<script setup lang="ts">
import { ref, computed } from "vue";
import { useThemeStore, type ThemeMode } from "@/stores/theme";
import { useSettingsStore } from "@/stores/settings";
import {
  ArrowLeft,
  ArrowRight,
  Check,
  Sun,
  Moon,
  Monitor,
  Download,
} from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";

const emit = defineEmits<{
  completed: [];
}>();

const themeStore = useThemeStore();
const settingsStore = useSettingsStore();

// ============= Steps =============
interface Step {
  title: string;
  description: string;
}

const steps: Step[] = [
  { title: "欢迎", description: "欢迎使用 AzurePath" },
  { title: "主题", description: "选择界面主题" },
  { title: "剪贴板", description: "配置剪贴板监控" },
  { title: "下载目录", description: "选择默认下载目录" },
  { title: "通知", description: "配置通知权限" },
  { title: "完成", description: "设置完成" },
];

const currentStep = ref(0);
const totalSteps = steps.length;

// ============= Step-specific state =============
const selectedTheme = ref<ThemeMode>("system");
const clipboardEnabled = ref(true);
const clipboardInterval = ref(1000);
const downloadDir = ref("");
const notificationsEnabled = ref(true);
const completing = ref(false);

// Computed
const isLastStep = computed(() => currentStep.value === totalSteps - 1);
const isFirstStep = computed(() => currentStep.value === 0);
const progressPercent = computed(
  () => ((currentStep.value + 1) / totalSteps) * 100,
);

// ============= Navigation =============
function next() {
  if (currentStep.value < totalSteps - 1) {
    // Save step data
    if (currentStep.value === 1) {
      // Theme step - apply immediately
      themeStore.setTheme(selectedTheme.value);
    }
    currentStep.value++;
  }
}

function prev() {
  if (currentStep.value > 0) {
    currentStep.value--;
  }
}

async function selectDownloadDir() {
  try {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const dir = await open({ directory: true, multiple: false, title: "选择下载目录" });
    if (dir) {
      downloadDir.value = dir as string;
    }
  } catch {
    // dialog plugin not available
  }
}

async function finish() {
  completing.value = true;

  // Save settings
  try {
    if (clipboardEnabled.value) {
      settingsStore.update("clipboardInterval", clipboardInterval.value);
    }
    if (downloadDir.value) {
      settingsStore.update("downloadDir", downloadDir.value);
    }
    settingsStore.update("notifyFileTransfer", notificationsEnabled.value);
    settingsStore.update("notifyChatMessage", notificationsEnabled.value);
    settingsStore.update("notifyScanComplete", notificationsEnabled.value);
    await settingsStore.save();
  } catch {
    // save might fail, but we continue
  }

  // Persist completion
  try {
    localStorage.setItem("setup_completed", "true");
  } catch {
    // localStorage unavailable
  }

  completing.value = false;
  emit("completed");
}

// ============= Theme previews =============
const themeOptions: { value: ThemeMode; label: string; icon: object; preview: string }[] = [
  {
    value: "light" as ThemeMode,
    label: "浅色",
    icon: Sun,
    preview: "bg-white text-gray-900 border border-gray-200",
  },
  {
    value: "dark" as ThemeMode,
    label: "深色",
    icon: Moon,
    preview: "bg-slate-900 text-gray-100 border border-slate-700",
  },
  {
    value: "system" as ThemeMode,
    label: "跟随系统",
    icon: Monitor,
    preview: "bg-gradient-to-r from-white to-slate-900 text-gray-900 dark:text-gray-100 border border-gray-300 dark:border-slate-600",
  },
];
</script>

<template>
  <div
    class="fixed inset-0 z-[9999] flex items-center justify-center bg-black/60 backdrop-blur-sm"
  >
    <div
      class="mx-4 w-full max-w-lg overflow-hidden rounded-2xl border border-paper-deep/60 bg-paper shadow-2xl"
    >
      <!-- Progress bar -->
      <div class="h-1.5 w-full bg-paper-deep/30">
        <div
          class="h-full bg-bamboo transition-all duration-500 ease-out"
          :style="{ width: progressPercent + '%' }"
        />
      </div>

      <!-- Step indicator -->
      <div class="flex items-center justify-center gap-1.5 px-6 pt-5">
        <div
          v-for="(_, i) in steps"
          :key="i"
          class="flex items-center gap-1.5"
        >
          <div
            class="flex h-6 w-6 items-center justify-center rounded-full text-xs font-medium transition-all duration-300"
            :class="
              i < currentStep
                ? 'bg-bamboo text-cloud'
                : i === currentStep
                  ? 'border-2 border-bamboo bg-bamboo/10 text-bamboo'
                  : 'border border-paper-deep bg-paper-warm/50 text-ink-faint'
            "
          >
            <Check v-if="i < currentStep" class="h-3 w-3" />
            <span v-else>{{ i + 1 }}</span>
          </div>
          <div
            v-if="i < totalSteps - 1"
            class="h-px w-6"
            :class="
              i < currentStep ? 'bg-bamboo' : 'bg-paper-deep'
            "
          />
        </div>
      </div>

      <!-- Content area -->
      <div class="px-6 py-6">
        <!-- Step 0: Welcome -->
        <div v-if="currentStep === 0" class="text-center">
          <div class="mx-auto mb-4 flex h-16 w-16 items-center justify-center rounded-2xl bg-bamboo/10">
            <svg class="h-8 w-8 text-bamboo" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M13 10V3L4 14h7v7l9-11h-7z" />
            </svg>
          </div>
          <h2 class="text-xl font-display font-bold text-ink">欢迎使用 AzurePath</h2>
          <p class="mt-2 text-sm text-ink-faint leading-relaxed">
            您的局域网运维工具箱。快速进行网络诊断、文件传输、聊天通讯和更多操作。
          </p>
          <p class="mt-4 text-xs text-ink-faint/60">
            只需几步即可完成初始配置
          </p>
        </div>

        <!-- Step 1: Theme Selection -->
        <div v-if="currentStep === 1">
          <h2 class="text-lg font-display font-semibold text-ink">选择主题</h2>
          <p class="mt-1 text-sm text-ink-faint">选择您喜欢的界面风格</p>
          <div class="mt-4 grid grid-cols-3 gap-3">
            <button
              v-for="opt in themeOptions"
              :key="opt.value"
              class="flex flex-col items-center gap-2 rounded-xl border-2 p-4 transition-all duration-200"
              :class="
                selectedTheme === opt.value
                  ? 'border-bamboo bg-bamboo/5'
                  : 'border-paper-deep/50 bg-paper-warm/30 hover:border-paper-deep'
              "
              @click="selectedTheme = opt.value"
            >
              <div class="flex h-12 w-full items-center justify-center rounded-lg text-xs font-medium" :class="opt.preview">
                Aa
              </div>
              <component :is="opt.icon" class="h-4 w-4 text-ink" />
              <span class="text-xs font-medium text-ink">{{ opt.label }}</span>
            </button>
          </div>
        </div>

        <!-- Step 2: Clipboard Monitoring -->
        <div v-if="currentStep === 2">
          <h2 class="text-lg font-display font-semibold text-ink">剪贴板监控</h2>
          <p class="mt-1 text-sm text-ink-faint">配置剪贴板历史记录功能</p>
          <div class="mt-4 space-y-4">
            <label class="flex items-center gap-3 rounded-xl border border-paper-deep/50 bg-paper-warm/30 p-4">
              <input
                v-model="clipboardEnabled"
                type="checkbox"
                class="h-4 w-4 rounded border-paper-deep text-bamboo focus:ring-bamboo/20"
              />
              <div>
                <span class="text-sm font-medium text-ink">启用剪贴板监控</span>
                <p class="text-xs text-ink-faint mt-0.5">自动记录复制的内容</p>
              </div>
            </label>
            <div v-if="clipboardEnabled">
              <label class="mb-1 block text-xs font-medium text-ink-soft">监控间隔</label>
              <select
                v-model.number="clipboardInterval"
                class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
              >
                <option :value="500">0.5s</option>
                <option :value="1000">1s</option>
                <option :value="2000">2s</option>
                <option :value="5000">5s</option>
              </select>
            </div>
          </div>
        </div>

        <!-- Step 3: Download Directory -->
        <div v-if="currentStep === 3">
          <h2 class="text-lg font-display font-semibold text-ink">下载目录</h2>
          <p class="mt-1 text-sm text-ink-faint">选择文件传输的默认保存位置</p>
          <div class="mt-4">
            <label class="mb-1 block text-xs font-medium text-ink-soft">目录路径</label>
            <div class="flex gap-2">
              <input
                :value="downloadDir || '未选择（使用系统默认）'"
                readonly
                class="flex-1 rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink placeholder:text-ink-faint/50 outline-none"
              />
              <Button variant="secondary" @click="selectDownloadDir">
                <Download class="mr-1 h-3.5 w-3.5" />
                浏览
              </Button>
            </div>
          </div>
        </div>

        <!-- Step 4: Notifications -->
        <div v-if="currentStep === 4">
          <h2 class="text-lg font-display font-semibold text-ink">通知</h2>
          <p class="mt-1 text-sm text-ink-faint">配置应用通知权限</p>
          <div class="mt-4 space-y-3">
            <label class="flex items-center gap-3 rounded-xl border border-paper-deep/50 bg-paper-warm/30 p-4">
              <input
                v-model="notificationsEnabled"
                type="checkbox"
                class="h-4 w-4 rounded border-paper-deep text-bamboo focus:ring-bamboo/20"
              />
              <div>
                <span class="text-sm font-medium text-ink">启用桌面通知</span>
                <p class="text-xs text-ink-faint mt-0.5">接收文件传输和消息通知</p>
              </div>
            </label>
          </div>
        </div>

        <!-- Step 5: Done -->
        <div v-if="currentStep === 5" class="text-center">
          <div class="mx-auto mb-4 flex h-16 w-16 items-center justify-center rounded-2xl bg-bamboo/10">
            <Check class="h-8 w-8 text-bamboo" />
          </div>
          <h2 class="text-xl font-display font-bold text-ink">设置完成</h2>
          <p class="mt-2 text-sm text-ink-faint leading-relaxed">
            您已完成初始配置，现在可以开始使用 AzurePath 了。
          </p>
          <div class="mt-6 space-y-2 rounded-xl border border-paper-deep/50 bg-paper-warm/30 p-4 text-left text-sm">
            <div class="flex justify-between">
              <span class="text-ink-faint">主题</span>
              <span class="text-ink font-medium">
                {{ selectedTheme === "light" ? "浅色" : selectedTheme === "dark" ? "深色" : "跟随系统" }}
              </span>
            </div>
            <div class="flex justify-between">
              <span class="text-ink-faint">剪贴板</span>
              <span class="text-ink font-medium">{{ clipboardEnabled ? "已启用" : "已禁用" }}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-ink-faint">下载目录</span>
              <span class="text-ink font-medium">{{ downloadDir || "系统默认" }}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-ink-faint">通知</span>
              <span class="text-ink font-medium">{{ notificationsEnabled ? "已启用" : "已禁用" }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Navigation buttons -->
      <div class="flex items-center justify-between border-t border-paper-deep/30 px-6 py-4">
        <Button
          v-if="!isFirstStep"
          variant="ghost"
          @click="prev"
        >
          <ArrowLeft class="mr-1.5 h-3.5 w-3.5" />
          上一步
        </Button>
        <div v-else />

        <Button
          v-if="!isLastStep"
          @click="next"
        >
          下一步
          <ArrowRight class="ml-1.5 h-3.5 w-3.5" />
        </Button>
        <Button
          v-else
          :disabled="completing"
          @click="finish"
        >
          <Check class="mr-1.5 h-3.5 w-3.5" />
          {{ completing ? "保存中..." : "完成" }}
        </Button>
      </div>
    </div>
  </div>
</template>
