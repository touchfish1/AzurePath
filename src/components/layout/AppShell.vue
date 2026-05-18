<script setup lang="ts">
import { onMounted } from "vue";
import { useThemeStore } from "@/stores/theme";
import TitleBar from "./TitleBar.vue";
import Sidebar from "./Sidebar.vue";

const themeStore = useThemeStore();

onMounted(() => {
  themeStore.init();
});
</script>

<template>
  <div class="flex h-screen w-screen flex-col overflow-hidden">
    <!-- Title bar (window chrome) -->
    <TitleBar />

    <!-- Body: sidebar + main content -->
    <div class="flex flex-1 overflow-hidden">
      <!-- Sidebar -->
      <Sidebar />

      <!-- Main content area -->
      <main
        class="flex-1 overflow-y-auto bg-paper noise-bg"
      >
        <router-view />
      </main>
    </div>
  </div>
</template>
