import { defineStore } from "pinia";
import { ref, computed } from "vue";
import type { Component } from "vue";

export interface Command {
  id: string;
  label: string;
  description?: string;
  icon?: Component;
  category: "navigation" | "action" | "tool";
  action: () => void | Promise<void>;
  shortcut?: string;
  keywords?: string[];
}

export const useCommandPaletteStore = defineStore("commandPalette", () => {
  const isOpen = ref(false);
  const query = ref("");
  const commands = ref<Command[]>([]);

  function toggle() {
    isOpen.value = !isOpen.value;
  }

  function open() {
    isOpen.value = true;
    query.value = "";
  }

  function close() {
    isOpen.value = false;
    query.value = "";
  }

  function registerCommands(cmds: Command[]) {
    commands.value = cmds;
  }

  function addCommands(cmds: Command[]) {
    commands.value.push(...cmds);
  }

  const filteredCommands = computed(() => {
    if (!query.value.trim()) return commands.value.slice(0, 20);
    const q = query.value.toLowerCase();
    return commands.value
      .filter(
        (c) =>
          c.label.toLowerCase().includes(q) ||
          c.description?.toLowerCase().includes(q) ||
          c.keywords?.some((k) => k.toLowerCase().includes(q)),
      )
      .slice(0, 20);
  });

  return {
    isOpen,
    query,
    commands,
    filteredCommands,
    toggle,
    open,
    close,
    registerCommands,
    addCommands,
  };
});
