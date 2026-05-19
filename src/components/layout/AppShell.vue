<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useThemeStore } from "@/stores/theme";
import { useSettingsStore } from "@/stores/settings";
import { useKeyboardShortcuts } from "@/composables/useKeyboardShortcuts";
import TitleBar from "./TitleBar.vue";
import Sidebar from "./Sidebar.vue";
import UpdateBanner from "@/components/UpdateBanner.vue";
import Toast from "@/components/Toast.vue";
import SetupWizard from "@/components/SetupWizard.vue";
import ErrorBoundary from "@/components/ErrorBoundary.vue";
import CommandPalette from "@/components/CommandPalette.vue";

const themeStore = useThemeStore();
const settingsStore = useSettingsStore();
const sidebarCollapsed = ref(false);
const showSetupWizard = ref(false);

useKeyboardShortcuts();

onMounted(async () => {
  themeStore.init();
  await settingsStore.load();

  // Check if setup wizard should be shown
  try {
    const completed = localStorage.getItem("setup_completed");
    if (!completed) {
      showSetupWizard.value = true;
    }
  } catch {
    // localStorage unavailable, skip wizard
  }
});

function onToggleCollapse() {
  sidebarCollapsed.value = !sidebarCollapsed.value;
}

function onWizardCompleted() {
  showSetupWizard.value = false;
}
</script>

<template>
  <div class="flex h-screen w-screen flex-col overflow-hidden">
    <!-- Setup wizard (full-screen overlay) -->
    <SetupWizard
      v-if="showSetupWizard"
      @completed="onWizardCompleted"
    />

    <!-- Title bar (window chrome) -->
    <TitleBar />

    <!-- Update banner -->
    <UpdateBanner />

    <!-- Body: sidebar + main content -->
    <div class="flex flex-1 overflow-hidden">
      <!-- Sidebar -->
      <Sidebar
        :collapsed="sidebarCollapsed"
        @toggle-collapse="onToggleCollapse"
      />

      <!-- Main content area -->
      <main
        class="flex-1 overflow-y-auto bg-paper noise-bg"
      >
        <ErrorBoundary>
          <router-view />
        </ErrorBoundary>
      </main>
    </div>

    <!-- Toast notifications -->
    <Toast />

    <!-- Command palette (Ctrl+K) -->
    <CommandPalette />
  </div>
</template>
