import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { defineComponent, nextTick } from "vue";
import { setActivePinia, createPinia } from "pinia";

const mockPush = vi.hoisted(() => vi.fn());

vi.mock("vue-router", () => ({
  useRouter: () => ({ push: mockPush }),
  useRoute: () => ({}),
}));

import { useKeyboardShortcuts } from "@/composables/useKeyboardShortcuts";

function createTestComponent() {
  return defineComponent({
    setup() {
      useKeyboardShortcuts();
    },
    template: '<div><input data-testid="input" /><div data-testid="output">text</div></div>',
  });
}

describe("useKeyboardShortcuts", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockPush.mockClear();
  });

  it("is a function", () => {
    expect(typeof useKeyboardShortcuts).toBe("function");
  });

  it("navigates to dashboard on Ctrl+1", async () => {
    mount(createTestComponent(), { attachTo: document.body });
    await nextTick();

    window.dispatchEvent(
      new KeyboardEvent("keydown", {
        key: "1",
        ctrlKey: true,
        bubbles: true,
      }),
    );

    expect(mockPush).toHaveBeenCalledWith({ name: "dashboard" });
  });

  it("skips shortcut when input is focused", async () => {
    const wrapper = mount(createTestComponent(), { attachTo: document.body });
    await nextTick();

    const input = wrapper.find('[data-testid="input"]');
    (input.element as HTMLElement).dispatchEvent(
      new KeyboardEvent("keydown", {
        key: "1",
        ctrlKey: true,
        bubbles: true,
      }),
    );

    expect(mockPush).not.toHaveBeenCalled();
  });
});
