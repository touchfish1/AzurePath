<script setup lang="ts">
import { CloudDownload, X, Loader2 } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import { useUpdateChecker } from "@/composables/useUpdateChecker";

const { updateAvailable, updateInfo, installing, dismiss, installUpdate } =
  useUpdateChecker();
</script>

<template>
  <Transition
    enter-active-class="transition-all duration-300 ease-out-expo"
    enter-from-class="translate-y-0 opacity-0"
    enter-to-class="translate-y-0 opacity-100"
    leave-active-class="transition-all duration-200 ease-in"
    leave-from-class="translate-y-0 opacity-100"
    leave-to-class="translate-y-0 opacity-0"
  >
    <div
      v-if="updateAvailable"
      class="flex items-center gap-3 border-b border-bamboo/20 bg-bamboo-mist/60 px-6 py-2.5 text-sm dark:bg-bamboo/10"
    >
      <CloudDownload class="h-4 w-4 text-bamboo" />
      <span class="text-ink-soft">
        AzurePath
        <span v-if="updateInfo" class="font-medium text-bamboo">{{ updateInfo.version }}</span>
        可用
      </span>
      <div class="ml-auto flex items-center gap-2">
        <Button
          size="sm"
          :disabled="installing"
          @click="installUpdate"
        >
          <Loader2 v-if="installing" class="mr-1 h-3.5 w-3.5 animate-spin" />
          <CloudDownload v-else class="mr-1 h-3.5 w-3.5" />
          {{ installing ? "安装中..." : "更新" }}
        </Button>
        <button
          class="rounded p-1 text-ink-faint transition-colors hover:text-ink hover:bg-paper-deep/30"
          @click="dismiss"
        >
          <X class="h-4 w-4" />
        </button>
      </div>
    </div>
  </Transition>
</template>
