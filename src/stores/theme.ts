import { defineStore } from "pinia";
import { ref, watch } from "vue";

export type ThemeMode = "light" | "dark" | "system";

export const useThemeStore = defineStore("theme", () => {
  const theme = ref<ThemeMode>("system");
  const resolved = ref<"light" | "dark">("light");

  let mediaQuery: MediaQueryList | null = null;
  let mediaHandler: (() => void) | null = null;

  function resolveSystem(): "light" | "dark" {
    if (typeof window === "undefined") return "light";
    return window.matchMedia("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light";
  }

  function applyTheme(mode: ThemeMode) {
    const r = mode === "system" ? resolveSystem() : mode;
    resolved.value = r;

    const html = document.documentElement;

    // Enable smooth transition
    html.classList.add("theme-transition");
    html.setAttribute("data-theme", r);

    // Remove transition class after animation completes
    setTimeout(() => {
      html.classList.remove("theme-transition");
    }, 400);
  }

  function setTheme(mode: ThemeMode) {
    theme.value = mode;
  }

  function toggleTheme() {
    const next = resolved.value === "dark" ? "light" : "dark";
    theme.value = next;
  }

  // Watch for system preference changes
  function setupMediaListener() {
    if (typeof window === "undefined") return;
    mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");

    mediaHandler = () => {
      if (theme.value === "system") {
        applyTheme("system");
      }
    };

    mediaQuery.addEventListener("change", mediaHandler);
  }

  function teardownMediaListener() {
    if (mediaQuery && mediaHandler) {
      mediaQuery.removeEventListener("change", mediaHandler);
    }
    mediaQuery = null;
    mediaHandler = null;
  }

  // Initialize
  function init() {
    applyTheme(theme.value);
    setupMediaListener();

    watch(theme, (val) => {
      applyTheme(val);
    });
  }

  return {
    theme,
    resolved,
    setTheme,
    toggleTheme,
    init,
    applyTheme,
    setupMediaListener,
    teardownMediaListener,
  };
});
