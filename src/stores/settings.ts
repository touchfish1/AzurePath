import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface AppSettings {
  theme: "light" | "dark" | "system";
  clipboardInterval: number;
  clipboardMaxItems: number;
  pingCount: number;
  pingTimeout: number;
  downloadDir: string;
  retentionDays: number;
  notifyFileTransfer: boolean;
  notifyChatMessage: boolean;
  notifyScanComplete: boolean;
}

const defaultSettings: AppSettings = {
  theme: "system",
  clipboardInterval: 1000,
  clipboardMaxItems: 500,
  pingCount: 4,
  pingTimeout: 3000,
  downloadDir: "",
  retentionDays: 30,
  notifyFileTransfer: true,
  notifyChatMessage: true,
  notifyScanComplete: true,
};

export const useSettingsStore = defineStore("settings", () => {
  const settings = ref<AppSettings>({ ...defaultSettings });
  const loaded = ref(false);
  const saving = ref(false);

  async function load() {
    try {
      const result = await invoke<AppSettings>("get_settings");
      settings.value = { ...defaultSettings, ...result };
      loaded.value = true;
    } catch {
      // If backend is not available, use defaults
      settings.value = { ...defaultSettings };
      loaded.value = true;
    }
  }

  async function save() {
    saving.value = true;
    try {
      await invoke("save_settings", { settings: settings.value });
    } catch (e) {
      console.error("Failed to save settings:", e);
    } finally {
      saving.value = false;
    }
  }

  function update<K extends keyof AppSettings>(key: K, value: AppSettings[K]) {
    settings.value[key] = value;
  }

  return { settings, loaded, saving, load, save, update };
});
