import { describe, it, expect, beforeEach, vi } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import { nextTick } from "vue";
import { useThemeStore } from "@/stores/theme";

describe("theme store — night mode", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    localStorage.clear();
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe("defaults", () => {
    it("nightModeEnabled defaults to false", () => {
      const store = useThemeStore();
      expect(store.nightModeEnabled).toBe(false);
    });

    it("nightModeStart defaults to 22:00", () => {
      const store = useThemeStore();
      expect(store.nightModeStart).toBe("22:00");
    });

    it("nightModeEnd defaults to 07:00", () => {
      const store = useThemeStore();
      expect(store.nightModeEnd).toBe("07:00");
    });
  });

  describe("setNightModeSchedule", () => {
    it("enables night mode when passed true", () => {
      const store = useThemeStore();
      store.setNightModeSchedule(true);
      expect(store.nightModeEnabled).toBe(true);
    });

    it("disables night mode when passed false", () => {
      const store = useThemeStore();
      store.setNightModeSchedule(true);
      store.setNightModeSchedule(false);
      expect(store.nightModeEnabled).toBe(false);
    });

    it("sets custom start time", () => {
      const store = useThemeStore();
      store.setNightModeSchedule(true, "23:00");
      expect(store.nightModeStart).toBe("23:00");
      expect(store.nightModeEnd).toBe("07:00"); // unchanged
    });

    it("sets custom end time", () => {
      const store = useThemeStore();
      store.setNightModeSchedule(true, undefined, "06:00");
      expect(store.nightModeStart).toBe("22:00"); // unchanged
      expect(store.nightModeEnd).toBe("06:00");
    });

    it("sets both start and end times", () => {
      const store = useThemeStore();
      store.setNightModeSchedule(true, "21:00", "06:00");
      expect(store.nightModeStart).toBe("21:00");
      expect(store.nightModeEnd).toBe("06:00");
    });

    it("persists enabled state to localStorage", () => {
      const store = useThemeStore();
      store.setNightModeSchedule(true);
      expect(localStorage.getItem("night-mode-enabled")).toBe("true");
    });

    it("persists false enabled state to localStorage", () => {
      const store = useThemeStore();
      store.setNightModeSchedule(false);
      expect(localStorage.getItem("night-mode-enabled")).toBe("false");
    });

    it("persists start and end times to localStorage", () => {
      const store = useThemeStore();
      store.setNightModeSchedule(true, "21:00", "06:00");
      expect(localStorage.getItem("night-mode-start")).toBe("21:00");
      expect(localStorage.getItem("night-mode-end")).toBe("06:00");
    });
  });

  describe("checkNightMode — same-day range", () => {
    beforeEach(() => {
      vi.useRealTimers();
      setActivePinia(createPinia());
      localStorage.clear();
    });

    it("applies dark theme when current time is within night mode window (start < end)", () => {
      const store = useThemeStore();
      store.setNightModeSchedule(true, "08:00", "17:00");

      // Set "current time" to 12:00
      vi.useFakeTimers();
      vi.setSystemTime(new Date("2025-01-01T12:00:00"));

      store.checkNightMode();
      expect(store.resolved).toBe("dark");
    });

    it("does not apply dark theme outside night mode window", () => {
      const store = useThemeStore();
      store.setNightModeSchedule(true, "08:00", "12:00");
      store.setTheme("light");
      store.applyTheme("light");

      vi.setSystemTime(new Date("2025-01-01T14:00:00"));

      store.checkNightMode();
      expect(store.resolved).toBe("light");
    });

    it("applies dark at start boundary", () => {
      const store = useThemeStore();
      store.setNightModeSchedule(true, "08:00", "17:00");

      vi.setSystemTime(new Date("2025-01-01T08:00:00"));

      store.checkNightMode();
      expect(store.resolved).toBe("dark");
    });

    it("does not apply dark at end boundary (end is exclusive)", () => {
      const store = useThemeStore();
      store.setNightModeSchedule(true, "08:00", "17:00");
      store.setTheme("light");
      store.applyTheme("light");

      vi.setSystemTime(new Date("2025-01-01T17:00:00"));

      store.checkNightMode();
      expect(store.resolved).toBe("light");
    });
  });

  describe("checkNightMode — midnight crossover (start > end)", () => {
    beforeEach(() => {
      vi.useRealTimers();
      setActivePinia(createPinia());
      localStorage.clear();
    });

    it("applies dark when current time is after start (night before midnight)", () => {
      const store = useThemeStore();
      store.setNightModeSchedule(true, "22:00", "07:00");

      // Default start = 22:00, end = 07:00 (crosses midnight)
      vi.setSystemTime(new Date("2025-01-01T23:00:00"));

      store.checkNightMode();
      expect(store.resolved).toBe("dark");
    });

    it("applies dark when current time is before end (morning after midnight)", () => {
      const store = useThemeStore();
      store.setNightModeSchedule(true, "22:00", "07:00");

      vi.setSystemTime(new Date("2025-01-02T06:00:00"));

      store.checkNightMode();
      expect(store.resolved).toBe("dark");
    });

    it("does not apply dark in the middle of the day (between end and start)", () => {
      const store = useThemeStore();
      store.setNightModeSchedule(true, "22:00", "07:00");
      store.setTheme("light");
      store.applyTheme("light");

      vi.setSystemTime(new Date("2025-01-01T12:00:00"));

      store.checkNightMode();
      expect(store.resolved).toBe("light");
    });

    it("applies dark at start boundary (midnight crossover)", () => {
      const store = useThemeStore();
      store.setNightModeSchedule(true, "22:00", "07:00");

      vi.setSystemTime(new Date("2025-01-01T22:00:00"));

      store.checkNightMode();
      expect(store.resolved).toBe("dark");
    });
  });

  describe("checkNightMode — when disabled", () => {
    it("does nothing when nightModeEnabled is false", () => {
      const store = useThemeStore();
      store.setTheme("light");
      store.applyTheme("light");

      // Night mode not enabled
      vi.setSystemTime(new Date("2025-01-01T23:00:00"));

      store.checkNightMode();
      expect(store.resolved).toBe("light");
    });
  });

  describe("startNightMode / stopNightMode", () => {
    it("calls checkNightMode immediately, applying dark when within night window", () => {
      const store = useThemeStore();
      store.nightModeEnabled = true;
      store.nightModeStart = "08:00";
      store.nightModeEnd = "17:00";
      store.setTheme("light");
      store.applyTheme("light");
      expect(store.resolved).toBe("light");

      vi.setSystemTime(new Date("2025-01-01T12:00:00")); // within window
      store.startNightMode(); // calls checkNightMode immediately

      expect(store.resolved).toBe("dark");
    });

    it("does not apply dark when outside night window", () => {
      const store = useThemeStore();
      store.nightModeEnabled = true;
      store.nightModeStart = "08:00";
      store.nightModeEnd = "12:00";
      store.setTheme("light");
      store.applyTheme("light");
      expect(store.resolved).toBe("light");

      vi.setSystemTime(new Date("2025-01-01T14:00:00")); // outside window
      store.startNightMode();

      expect(store.resolved).toBe("light");
    });

    it("clears interval when stopped", () => {
      const store = useThemeStore();
      store.startNightMode();
      expect(() => store.stopNightMode()).not.toThrow();
    });

    it("can start multiple times without error", () => {
      const store = useThemeStore();
      expect(() => {
        store.startNightMode();
        store.startNightMode();
        store.startNightMode();
      }).not.toThrow();
    });

    it("does not crash when stopped without starting", () => {
      const store = useThemeStore();
      expect(() => store.stopNightMode()).not.toThrow();
    });
  });

  describe("init — restores night mode settings", () => {
    it("restores enabled night mode from localStorage", () => {
      localStorage.setItem("night-mode-enabled", "true");
      localStorage.setItem("night-mode-start", "23:00");
      localStorage.setItem("night-mode-end", "06:00");

      const store = useThemeStore();
      store.init();

      expect(store.nightModeEnabled).toBe(true);
      expect(store.nightModeStart).toBe("23:00");
      expect(store.nightModeEnd).toBe("06:00");
    });

    it("starts night mode timer when restored from localStorage", () => {
      // Set night mode to start 1 minute from now (only active after timer fires)
      const now = new Date("2025-01-01T07:59:00");
      vi.setSystemTime(now);

      localStorage.setItem("night-mode-enabled", "true");
      localStorage.setItem("night-mode-start", "08:00");
      localStorage.setItem("night-mode-end", "17:00");

      const store = useThemeStore();
      store.setTheme("light");
      store.applyTheme("light");
      expect(store.resolved).toBe("light");

      store.init();
      // At 07:59, not yet in night mode (starts at 08:00)
      expect(store.resolved).toBe("light");
      expect(store.nightModeEnabled).toBe(true);

      // Advance 60 seconds to trigger the interval
      vi.advanceTimersByTime(60000);
      // Now at 08:00 — checkNightMode should fire and switch to dark
      expect(store.resolved).toBe("dark");
    });

    it("does not enable night mode when localStorage has false", () => {
      localStorage.setItem("night-mode-enabled", "false");

      const store = useThemeStore();
      store.init();
      expect(store.nightModeEnabled).toBe(false);
    });

    it("does not enable night mode when localStorage is absent", () => {
      const store = useThemeStore();
      store.init();
      expect(store.nightModeEnabled).toBe(false);
    });
  });
});
