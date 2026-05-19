import { defineStore } from "pinia";
import { ref } from "vue";

export interface ToastItem {
  id: number;
  type: "success" | "error" | "info";
  message: string;
}

export const useToastStore = defineStore("toast", () => {
  const toasts = ref<ToastItem[]>([]);
  let nextId = 1;

  function add(type: "success" | "error" | "info", message: string) {
    const id = nextId++;
    toasts.value.push({ id, type, message });

    // Enforce max 3 items — remove oldest
    if (toasts.value.length > 3) {
      toasts.value.shift();
    }

    // Auto-remove after 2.5 seconds
    setTimeout(() => {
      remove(id);
    }, 2500);
  }

  function remove(id: number) {
    toasts.value = toasts.value.filter((t) => t.id !== id);
  }

  return { toasts, add, remove };
});
