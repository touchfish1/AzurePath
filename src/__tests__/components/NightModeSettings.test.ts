import { describe, it, expect, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { setActivePinia, createPinia } from "pinia";
import { nextTick } from "vue";

import { useThemeStore } from "@/stores/theme";
import NightModeSettings from "@/components/NightModeSettings.vue";

describe("NightModeSettings", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    localStorage.clear();
  });

  it("renders the section title", () => {
    const wrapper = mount(NightModeSettings);
    expect(wrapper.text()).toContain("夜间模式定时切换");
  });

  it("renders the toggle checkbox", () => {
    const wrapper = mount(NightModeSettings);
    const checkbox = wrapper.find('input[type="checkbox"]');
    expect(checkbox.exists()).toBe(true);
  });

  it("checkbox is unchecked when nightModeEnabled is false", () => {
    const wrapper = mount(NightModeSettings);
    const checkbox = wrapper.find('input[type="checkbox"]') as unknown as { element: HTMLInputElement };
    expect(checkbox.element.checked).toBe(false);
  });

  it("checkbox is checked when nightModeEnabled is true", async () => {
    const store = useThemeStore();
    store.nightModeEnabled = true;
    await nextTick();

    const wrapper = mount(NightModeSettings);
    const checkbox = wrapper.find('input[type="checkbox"]') as unknown as { element: HTMLInputElement };
    expect(checkbox.element.checked).toBe(true);
  });

  it("hides time inputs when nightModeEnabled is false", () => {
    const wrapper = mount(NightModeSettings);
    const timeInputs = wrapper.findAll('input[type="time"]');
    expect(timeInputs.length).toBe(0);
  });

  it("shows time inputs when nightModeEnabled is true", async () => {
    const store = useThemeStore();
    store.nightModeEnabled = true;
    await nextTick();

    const wrapper = mount(NightModeSettings);
    const timeInputs = wrapper.findAll('input[type="time"]');
    expect(timeInputs.length).toBe(2);
  });

  it("displays default start time (22:00)", async () => {
    const store = useThemeStore();
    store.nightModeEnabled = true;
    await nextTick();

    const wrapper = mount(NightModeSettings);
    expect(wrapper.text()).toContain("22:00");
  });

  it("displays default end time (07:00)", async () => {
    const store = useThemeStore();
    store.nightModeEnabled = true;
    await nextTick();

    const wrapper = mount(NightModeSettings);
    expect(wrapper.text()).toContain("07:00");
  });

  it("displays the schedule description when enabled", async () => {
    const store = useThemeStore();
    store.nightModeEnabled = true;
    await nextTick();

    const wrapper = mount(NightModeSettings);
    // Should show the description about auto-switching
    expect(wrapper.text()).toContain("自动切换到暗色模式");
  });

  it("toggling checkbox calls setNightModeSchedule with enabled=true", async () => {
    const wrapper = mount(NightModeSettings);
    const store = useThemeStore();
    const spy = vi.spyOn(store, "setNightModeSchedule");

    const checkbox = wrapper.find('input[type="checkbox"]');
    checkbox.trigger("change");
    await nextTick();

    expect(spy).toHaveBeenCalledTimes(1);
    expect(spy).toHaveBeenCalledWith(true);
  });

  it("toggling checkbox when already enabled calls setNightModeSchedule(false)", async () => {
    const store = useThemeStore();
    store.nightModeEnabled = true;
    await nextTick();

    const wrapper = mount(NightModeSettings);
    const spy = vi.spyOn(store, "setNightModeSchedule");

    const checkbox = wrapper.find('input[type="checkbox"]');
    checkbox.trigger("change");
    await nextTick();

    expect(spy).toHaveBeenCalledWith(false);
  });

  it("changing start time input calls setNightModeSchedule with new start", async () => {
    const store = useThemeStore();
    store.nightModeEnabled = true;
    await nextTick();

    const wrapper = mount(NightModeSettings);
    const spy = vi.spyOn(store, "setNightModeSchedule");

    const timeInputs = wrapper.findAll('input[type="time"]');
    const startInput = timeInputs[0];
    (startInput.element as HTMLInputElement).value = "23:00";
    await startInput.trigger("change");
    await nextTick();

    expect(spy).toHaveBeenCalledWith(true, "23:00", undefined);
  });

  it("changing end time input calls setNightModeSchedule with new end", async () => {
    const store = useThemeStore();
    store.nightModeEnabled = true;
    await nextTick();

    const wrapper = mount(NightModeSettings);
    const spy = vi.spyOn(store, "setNightModeSchedule");

    const timeInputs = wrapper.findAll('input[type="time"]');
    const endInput = timeInputs[1];
    (endInput.element as HTMLInputElement).value = "06:00";
    await endInput.trigger("change");
    await nextTick();

    expect(spy).toHaveBeenCalledWith(true, undefined, "06:00");
  });

  it("does not show time inputs when night mode is then disabled", async () => {
    const store = useThemeStore();
    store.nightModeEnabled = true;
    await nextTick();

    const wrapper = mount(NightModeSettings);

    // Disable night mode
    store.nightModeEnabled = false;
    await nextTick();

    const timeInputs = wrapper.findAll('input[type="time"]');
    expect(timeInputs.length).toBe(0);
  });
});
