<script setup lang="ts">
import { onMounted, ref } from "vue";
import { Save, Trash2, ChevronDown } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import { usePresetStore } from "@/stores/preset";
import type { Preset } from "@/lib/tauri";

const props = defineProps<{
  feature: string;
}>();

const emit = defineEmits<{
  load: [preset: Preset];
  "save-request": [name: string];
}>();

const store = usePresetStore();
const showSaveModal = ref(false);
const saveName = ref("");
const selectedPresetId = ref("");

onMounted(() => {
  store.load(props.feature);
});

function selectPreset(id: string) {
  selectedPresetId.value = id;
  const preset = store.presets.find((p) => p.id === id);
  if (preset) {
    emit("load", preset);
  }
}

function openSaveModal() {
  saveName.value = "";
  showSaveModal.value = true;
}

function confirmSave() {
  if (!saveName.value.trim()) return;

  // The parent page provides its params via a different mechanism.
  // We emit a custom event so the parent can handle serializing its own state.
  emit("save-request", saveName.value.trim());
  showSaveModal.value = false;
}
</script>

<template>
  <div class="flex items-center gap-2">
    <!-- Preset selector -->
    <div class="relative flex-1">
      <select
        v-model="selectedPresetId"
        class="w-full appearance-none rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 pr-8 text-sm text-ink outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
        @change="selectPreset(selectedPresetId)"
      >
        <option value="">预设...</option>
        <option
          v-for="p in store.presets"
          :key="p.id"
          :value="p.id"
        >
          {{ p.name }}
        </option>
      </select>
      <ChevronDown class="pointer-events-none absolute right-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-ink-faint" />
    </div>

    <!-- Delete preset -->
    <button
      v-if="selectedPresetId"
      class="rounded-lg p-2 text-ink-faint transition-colors hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-900/10"
      title="删除预设"
      @click="store.remove(selectedPresetId); selectedPresetId = ''"
    >
      <Trash2 class="h-3.5 w-3.5" />
    </button>

    <!-- Save as preset -->
    <Button variant="secondary" size="sm" @click="openSaveModal">
      <Save class="mr-1 h-3 w-3" />
      保存预设
    </Button>

    <!-- Save modal -->
    <Teleport to="body">
      <div
        v-if="showSaveModal"
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/40"
        @click.self="showSaveModal = false"
      >
        <div class="w-80 rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-xl">
          <h3 class="text-sm font-semibold text-ink mb-4">保存为预设</h3>
          <input
            v-model="saveName"
            placeholder="预设名称"
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm text-ink placeholder:text-ink-faint/50 outline-none transition-colors focus:border-bamboo/50 focus:ring-1 focus:ring-bamboo/20"
            @keyup.enter="confirmSave"
          />
          <div class="mt-4 flex justify-end gap-2">
            <Button variant="ghost" size="sm" @click="showSaveModal = false">
              取消
            </Button>
            <Button size="sm" :disabled="!saveName.trim()" @click="confirmSave">
              <Save class="mr-1 h-3 w-3" />
              保存
            </Button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>
