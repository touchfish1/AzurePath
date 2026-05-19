import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { mount } from "@vue/test-utils";
import { setActivePinia, createPinia } from "pinia";
import { nextTick } from "vue";

const mockPush = vi.hoisted(() => vi.fn());
const mockGetRoutes = vi.hoisted(() => vi.fn(() => []));

vi.mock("vue-router", () => ({
  useRouter: () => ({
    push: mockPush,
    getRoutes: mockGetRoutes,
  }),
}));

import { useCommandPaletteStore } from "@/stores/commandPalette";
import CommandPalette from "@/components/CommandPalette.vue";

function createCmd(id: string, label: string, overrides: Record<string, unknown> = {}) {
  return {
    id,
    label,
    category: "action" as const,
    action: vi.fn(),
    ...overrides,
  };
}

/**
 * Mount the CommandPalette with Teleport stubbed so content renders
 * inside the wrapper instead of being teleported to document.body.
 */
function mountPalette() {
  return mount(CommandPalette, {
    global: {
      stubs: { Teleport: true },
    },
  });
}

describe("CommandPalette", () => {
  let wrapper: ReturnType<typeof mount> | null = null;

  beforeEach(() => {
    setActivePinia(createPinia());
    mockGetRoutes.mockReturnValue([]);
  });

  afterEach(() => {
    if (wrapper) {
      wrapper.unmount();
      wrapper = null;
    }
  });

  it("does not render when isOpen is false", () => {
    wrapper = mountPalette();
    expect(wrapper.find('[class*="fixed inset-0"]').exists()).toBe(false);
  });

  it("renders when isOpen is true", async () => {
    wrapper = mountPalette();
    const store = useCommandPaletteStore();
    store.isOpen = true;
    await nextTick();
    expect(wrapper.find('[class*="fixed inset-0"]').exists()).toBe(true);
  });

  it("shows initial hint when no commands are available and no query", async () => {
    wrapper = mountPalette();
    const store = useCommandPaletteStore();
    store.commands = [];
    store.isOpen = true;
    await nextTick();
    expect(wrapper.text()).toContain("输入关键词搜索命令");
  });

  it("shows no matches when query has no results", async () => {
    wrapper = mountPalette();
    const store = useCommandPaletteStore();
    store.commands = [];
    store.isOpen = true;
    store.query = "zzzznotfound";
    await nextTick();
    expect(wrapper.text()).toContain("没有找到匹配的命令");
  });

  it("shows command list when filteredCommands has items", async () => {
    wrapper = mountPalette();
    const store = useCommandPaletteStore();
    store.commands = [createCmd("1", "Test Command", { description: "A test command" })];
    store.isOpen = true;
    await nextTick();
    expect(wrapper.text()).toContain("Test Command");
    expect(wrapper.text()).toContain("A test command");
  });

  it("shows category label for each command", async () => {
    wrapper = mountPalette();
    const store = useCommandPaletteStore();
    store.commands = [createCmd("1", "Navigate", { category: "navigation" })];
    store.isOpen = true;
    await nextTick();
    expect(wrapper.text()).toContain("导航");
  });

  it("clicking backdrop closes palette", async () => {
    wrapper = mountPalette();
    const store = useCommandPaletteStore();
    store.isOpen = true;
    await nextTick();

    const backdrop = wrapper.find('[class*="bg-black/30"]');
    expect(backdrop.exists()).toBe(true);
    backdrop.trigger("click");
    await nextTick();
    expect(store.isOpen).toBe(false);
  });

  it("clicking a command executes its action and closes", async () => {
    const action = vi.fn();
    wrapper = mountPalette();
    const store = useCommandPaletteStore();
    store.commands = [createCmd("1", "Execute Me", { action })];
    store.isOpen = true;
    await nextTick();

    const cmdItem = wrapper.find('[data-command-index="0"]');
    cmdItem.trigger("click");
    await nextTick();

    expect(action).toHaveBeenCalledTimes(1);
    expect(store.isOpen).toBe(false);
  });

  it("keyboard Escape closes the palette", async () => {
    wrapper = mountPalette();
    const store = useCommandPaletteStore();
    store.isOpen = true;
    await nextTick();

    const input = wrapper.find("input");
    input.trigger("keydown", { key: "Escape" });
    await nextTick();
    expect(store.isOpen).toBe(false);
  });

  it("keyboard Enter executes the selected command", async () => {
    const action = vi.fn();
    wrapper = mountPalette();
    const store = useCommandPaletteStore();
    store.commands = [createCmd("1", "First", { action })];
    store.isOpen = true;
    await nextTick();

    const input = wrapper.find("input");
    input.trigger("keydown", { key: "Enter" });
    await nextTick();
    expect(action).toHaveBeenCalledTimes(1);
    expect(store.isOpen).toBe(false);
  });

  it("keyboard ArrowDown moves selection to next item", async () => {
    wrapper = mountPalette();
    const store = useCommandPaletteStore();
    store.commands = [createCmd("1", "First"), createCmd("2", "Second")];
    store.isOpen = true;
    await nextTick();

    const input = wrapper.find("input");
    input.trigger("keydown", { key: "ArrowDown" });
    await nextTick();

    const secondItem = wrapper.find('[data-command-index="1"]');
    expect(secondItem.classes()).toContain("bg-bamboo/10");
  });

  it("keyboard ArrowUp moves selection to previous item", async () => {
    wrapper = mountPalette();
    const store = useCommandPaletteStore();
    store.commands = [createCmd("1", "First"), createCmd("2", "Second")];
    store.isOpen = true;
    await nextTick();

    const input = wrapper.find("input");
    input.trigger("keydown", { key: "ArrowDown" });
    await nextTick();
    input.trigger("keydown", { key: "ArrowUp" });
    await nextTick();

    const firstItem = wrapper.find('[data-command-index="0"]');
    expect(firstItem.classes()).toContain("bg-bamboo/10");
  });

  it("keyboard ArrowDown does not go beyond last item", async () => {
    wrapper = mountPalette();
    const store = useCommandPaletteStore();
    store.commands = [createCmd("1", "Only One")];
    store.isOpen = true;
    await nextTick();

    const input = wrapper.find("input");
    input.trigger("keydown", { key: "ArrowDown" });
    await nextTick();
    input.trigger("keydown", { key: "ArrowDown" });
    await nextTick();

    const firstItem = wrapper.find('[data-command-index="0"]');
    expect(firstItem.classes()).toContain("bg-bamboo/10");
  });

  it("mouseenter on a command updates selectedIndex", async () => {
    wrapper = mountPalette();
    const store = useCommandPaletteStore();
    store.commands = [createCmd("1", "First"), createCmd("2", "Second")];
    store.isOpen = true;
    await nextTick();

    wrapper.find('[data-command-index="1"]').trigger("mouseenter");
    await nextTick();

    const secondItem = wrapper.find('[data-command-index="1"]');
    expect(secondItem.classes()).toContain("bg-bamboo/10");

    const firstItem = wrapper.find('[data-command-index="0"]');
    expect(firstItem.classes()).not.toContain("bg-bamboo/10");
  });

  it("Ctrl+K toggles the palette via window event listener", () => {
    wrapper = mountPalette();
    const store = useCommandPaletteStore();

    window.dispatchEvent(new KeyboardEvent("keydown", { key: "k", ctrlKey: true }));
    expect(store.isOpen).toBe(true);

    window.dispatchEvent(new KeyboardEvent("keydown", { key: "k", ctrlKey: true }));
    expect(store.isOpen).toBe(false);
  });

  it("Meta+K also toggles the palette", () => {
    wrapper = mountPalette();
    const store = useCommandPaletteStore();

    window.dispatchEvent(new KeyboardEvent("keydown", { key: "k", metaKey: true }));
    expect(store.isOpen).toBe(true);
  });

  it("removes global keydown listener on unmount", () => {
    wrapper = mountPalette();
    const store = useCommandPaletteStore();
    wrapper.unmount();
    wrapper = null;

    window.dispatchEvent(new KeyboardEvent("keydown", { key: "k", ctrlKey: true }));
    expect(store.isOpen).toBe(false);
  });
});
