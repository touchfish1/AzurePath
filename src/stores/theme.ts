import { defineStore } from "pinia";
import { ref, watch } from "vue";

export type ThemeMode = "light" | "dark" | "system";

export const useThemeStore = defineStore("theme", () => {
  const theme = ref<ThemeMode>("system");
  const resolved = ref<"light" | "dark">("light");

  let mediaQuery: MediaQueryList | null = null;
  let mediaHandler: (() => void) | null = null;

  // Night mode schedule
  const nightModeEnabled = ref(false);
  const nightModeStart = ref("22:00");
  const nightModeEnd = ref("07:00");
  let nightModeTimer: ReturnType<typeof setInterval> | null = null;

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
    const base = theme.value === "system" ? resolved.value : theme.value;
    theme.value = base === "dark" ? "light" : "dark";
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

  function persistTheme() {
    try {
      localStorage.setItem("azurepath-theme", theme.value);
    } catch {
      // localStorage unavailable (SSR, privacy mode, etc.)
    }
  }

  // Night mode functions

  function checkNightMode() {
    if (!nightModeEnabled.value) return;
    const now = new Date();
    const hours = now.getHours();
    const mins = now.getMinutes();
    const current = hours * 60 + mins;

    const [startH, startM] = nightModeStart.value.split(":").map(Number);
    const [endH, endM] = nightModeEnd.value.split(":").map(Number);
    const start = startH * 60 + startM;
    const end = endH * 60 + endM;

    const isNight =
      start <= end
        ? current >= start && current < end
        : current >= start || current < end; // crosses midnight

    if (isNight && resolved.value !== "dark") {
      applyTheme("dark");
    } else if (!isNight && resolved.value === "dark" && nightModeEnabled.value) {
      // Revert to preference
      const stored = getStoredPreference();
      if (stored && stored !== "dark") {
        applyTheme(stored);
      } else {
        applyTheme("system");
      }
    }
  }

  function getStoredPreference(): ThemeMode | null {
    try {
      const saved = localStorage.getItem("azurepath-theme");
      if (saved === "light" || saved === "dark" || saved === "system") {
        return saved;
      }
    } catch {
      // localStorage unavailable
    }
    return null;
  }

  function startNightMode() {
    stopNightMode();
    checkNightMode(); // immediate check
    nightModeTimer = setInterval(checkNightMode, 60000); // check every minute
  }

  function stopNightMode() {
    if (nightModeTimer) {
      clearInterval(nightModeTimer);
      nightModeTimer = null;
    }
  }

  function setNightModeSchedule(enabled: boolean, start?: string, end?: string) {
    nightModeEnabled.value = enabled;
    if (start !== undefined) nightModeStart.value = start;
    if (end !== undefined) nightModeEnd.value = end;
    try {
      localStorage.setItem("night-mode-enabled", String(enabled));
      localStorage.setItem("night-mode-start", nightModeStart.value);
      localStorage.setItem("night-mode-end", nightModeEnd.value);
    } catch {
      // localStorage unavailable
    }
    if (enabled) startNightMode();
    else {
      stopNightMode();
      // Restore preference when disabled
      const stored = getStoredPreference();
      if (stored && stored !== theme.value) {
        setTheme(stored);
      }
    }
  }

  // Initialize
  function init() {
    // Restore persisted theme preference
    const stored = getStoredPreference();
    if (stored) {
      theme.value = stored;
    }

    applyTheme(theme.value);
    setupMediaListener();

    // Restore night mode settings
    try {
      const enabled = localStorage.getItem("night-mode-enabled");
      if (enabled === "true") {
        const savedStart = localStorage.getItem("night-mode-start");
        const savedEnd = localStorage.getItem("night-mode-end");
        nightModeEnabled.value = true;
        if (savedStart) nightModeStart.value = savedStart;
        if (savedEnd) nightModeEnd.value = savedEnd;
        startNightMode();
      }
    } catch {
      // localStorage unavailable
    }
  }

  // Watch for theme changes (always active, not just after init)
  watch(theme, (val) => {
    applyTheme(val);
    persistTheme();
  });

  return {
    theme,
    resolved,
    nightModeEnabled,
    nightModeStart,
    nightModeEnd,
    setTheme,
    toggleTheme,
    init,
    applyTheme,
    setupMediaListener,
    teardownMediaListener,
    setNightModeSchedule,
    startNightMode,
    stopNightMode,
    checkNightMode,
  };
});
