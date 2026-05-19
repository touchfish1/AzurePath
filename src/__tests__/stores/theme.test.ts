import { describe, it, expect, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import { nextTick } from "vue";
import { useThemeStore } from "@/stores/theme";

describe("theme store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    localStorage.clear();
  });

  it("defaults to system theme", () => {
    const store = useThemeStore();
    expect(store.theme).toBe("system");
  });

  it("setTheme changes the theme mode", () => {
    const store = useThemeStore();
    store.setTheme("dark");
    expect(store.theme).toBe("dark");
  });

  it("toggleTheme toggles between light and dark", async () => {
    const store = useThemeStore();
    store.setTheme("light");
    await nextTick();
    store.toggleTheme();
    expect(store.theme).toBe("dark");
    store.toggleTheme();
    expect(store.theme).toBe("light");
  });

  it("toggleTheme from system uses resolved value", async () => {
    const store = useThemeStore();
    store.toggleTheme();
    expect(store.theme).toBe("dark");
  });

  it("persists theme to localStorage", async () => {
    const store = useThemeStore();
    store.setTheme("dark");
    await nextTick();
    expect(localStorage.getItem("azurepath-theme")).toBe("dark");
  });

  it("restores theme from localStorage", () => {
    localStorage.setItem("azurepath-theme", "dark");
    const store = useThemeStore();
    store.init();
    expect(store.theme).toBe("dark");
  });

  it("ignores invalid localStorage values", () => {
    localStorage.setItem("azurepath-theme", "invalid");
    const store = useThemeStore();
    store.init();
    expect(store.theme).toBe("system");
  });

  it("resolved reflects applied theme", async () => {
    const store = useThemeStore();
    store.setTheme("dark");
    await nextTick();
    expect(store.resolved).toBe("dark");
    store.setTheme("light");
    await nextTick();
    expect(store.resolved).toBe("light");
  });
});
