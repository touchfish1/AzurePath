import { defineStore } from "pinia";
import { ref } from "vue";
import {
  savePreset,
  loadPresets,
  deletePreset,
  type Preset,
} from "@/lib/tauri";
import { useToastStore } from "@/stores/toast";

export const usePresetStore = defineStore("preset", () => {
  const presets = ref<Preset[]>([]);
  const loading = ref(false);

  async function load(feature?: string) {
    loading.value = true;
    try {
      presets.value = await loadPresets(feature);
    } catch {
      presets.value = [];
    } finally {
      loading.value = false;
    }
  }

  async function save(name: string, feature: string, params: Record<string, unknown>) {
    try {
      const preset = await savePreset(name, feature, JSON.stringify(params));
      presets.value.push(preset);
      const toast = useToastStore();
      toast.add("success", "预设已保存");
      return preset;
    } catch (e) {
      const toast = useToastStore();
      toast.add("error", String(e));
      throw e;
    }
  }

  async function remove(id: string) {
    try {
      await deletePreset(id);
      presets.value = presets.value.filter((p) => p.id !== id);
      const toast = useToastStore();
      toast.add("success", "预设已删除");
    } catch (e) {
      const toast = useToastStore();
      toast.add("error", String(e));
    }
  }

  return { presets, loading, load, save, remove };
});
