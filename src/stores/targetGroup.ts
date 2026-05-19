import { defineStore } from "pinia";
import { ref } from "vue";
import {
  listTargetGroups,
  saveTargetGroup,
  deleteTargetGroup,
  type TargetGroup,
} from "@/lib/tauri";

export const useTargetGroupStore = defineStore("targetGroup", () => {
  const groups = ref<TargetGroup[]>([]);
  const loading = ref(false);

  async function loadGroups() {
    loading.value = true;
    try {
      groups.value = await listTargetGroups();
    } catch (e) {
      console.error("Failed to load target groups:", e);
    } finally {
      loading.value = false;
    }
  }

  async function saveGroup(id: string | null, name: string, targets: string[]): Promise<TargetGroup> {
    const group = await saveTargetGroup(id, name, targets);
    await loadGroups();
    return group;
  }

  async function removeGroup(id: string) {
    await deleteTargetGroup(id);
    await loadGroups();
  }

  return {
    groups,
    loading,
    loadGroups,
    saveGroup,
    removeGroup,
  };
});
