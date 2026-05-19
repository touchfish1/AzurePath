<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useThemeStore } from "@/stores/theme";
import { useKeyboardShortcuts } from "@/composables/useKeyboardShortcuts";
import TitleBar from "./TitleBar.vue";
import Sidebar from "./Sidebar.vue";
import UpdateBanner from "@/components/UpdateBanner.vue";
import Toast from "@/components/Toast.vue";

const themeStore = useThemeStore();
const sidebarCollapsed = ref(false);

useKeyboardShortcuts();

onMounted(() => {
  themeStore.init();
});

function onToggleCollapse() {
  sidebarCollapsed.value = !sidebarCollapsed.value;
}
</script>

<template>
  <div class="flex h-screen w-screen flex-col overflow-hidden">
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
        <router-view />
      </main>
    </div>

    <!-- Toast notifications -->
    <Toast />
  </div>
</template>
