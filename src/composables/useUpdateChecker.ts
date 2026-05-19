import { ref, onMounted } from "vue";
import { check } from "@tauri-apps/plugin-updater";

interface UpdateInfo {
  version: string;
  date: string;
  body: string;
}

export function useUpdateChecker() {
  const updateAvailable = ref(false);
  const updateInfo = ref<UpdateInfo | null>(null);
  const checking = ref(false);
  const installing = ref(false);

  async function checkForUpdate() {
    checking.value = true;
    try {
      const update = await check();
      if (update?.available) {
        updateAvailable.value = true;
        updateInfo.value = {
          version: update.version ?? "",
          date: update.date ?? "",
          body: update.body ?? "",
        };
      }
    } catch (e) {
      console.info("Update check:", e);
    } finally {
      checking.value = false;
    }
  }

  async function installUpdate() {
    installing.value = true;
    try {
      const update = await check();
      if (!update?.available) return;
      await update.downloadAndInstall();
    } catch (e) {
      console.info("Update install:", e);
    } finally {
      installing.value = false;
    }
  }

  function dismiss() {
    updateAvailable.value = false;
    updateInfo.value = null;
  }

  onMounted(() => {
    checkForUpdate();
  });

  return {
    updateAvailable,
    updateInfo,
    checking,
    installing,
    checkForUpdate,
    installUpdate,
    dismiss,
  };
}
