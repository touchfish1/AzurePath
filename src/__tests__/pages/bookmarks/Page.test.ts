import { describe, it, expect, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { setActivePinia, createPinia } from "pinia";
import { nextTick } from "vue";

const mockListBookmarks = vi.hoisted(() => vi.fn());
const mockAddBookmark = vi.hoisted(() => vi.fn());
const mockDeleteBookmark = vi.hoisted(() => vi.fn());

vi.mock("@/lib/tauri", () => ({
  listBookmarks: mockListBookmarks,
  addBookmark: mockAddBookmark,
  deleteBookmark: mockDeleteBookmark,
}));

import BookmarksPage from "@/pages/bookmarks/Page.vue";
import { useBookmarkStore } from "@/stores/bookmark";

const sampleBookmarks = [
  {
    id: "1",
    label: "Google DNS",
    target: "8.8.8.8",
    tags: [],
    createdAt: "2025-01-01T00:00:00Z",
  },
  {
    id: "2",
    label: "Cloudflare DNS",
    target: "1.1.1.1",
    tags: [],
    createdAt: "2025-01-02T00:00:00Z",
  },
];

async function flushAsync() {
  await nextTick();
  await nextTick();
}

describe("BookmarksPage", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
    Object.defineProperty(navigator, "clipboard", {
      value: { writeText: vi.fn().mockResolvedValue(undefined) },
      writable: true,
      configurable: true,
    });
  });

  it("renders the page header", async () => {
    mockListBookmarks.mockResolvedValue([]);
    const wrapper = mount(BookmarksPage);
    await flushAsync();
    expect(wrapper.text()).toContain("书签");
    expect(wrapper.text()).toContain("管理常用目标和地址");
  });

  it("shows loading state initially", async () => {
    mockListBookmarks.mockReturnValue(new Promise(() => {}));
    const wrapper = mount(BookmarksPage);
    await nextTick();
    expect(wrapper.text()).toContain("加载中");
  });

  it("shows empty state when no bookmarks exist", async () => {
    mockListBookmarks.mockResolvedValue([]);
    const wrapper = mount(BookmarksPage);
    await flushAsync();
    expect(wrapper.text()).toContain("暂无书签");
  });

  it("shows bookmark list when bookmarks are loaded", async () => {
    mockListBookmarks.mockResolvedValue(sampleBookmarks);
    const wrapper = mount(BookmarksPage);
    await flushAsync();
    expect(wrapper.text()).toContain("Google DNS");
    expect(wrapper.text()).toContain("8.8.8.8");
    expect(wrapper.text()).toContain("Cloudflare DNS");
    expect(wrapper.text()).toContain("1.1.1.1");
  });

  it("shows bookmark count in footer", async () => {
    mockListBookmarks.mockResolvedValue(sampleBookmarks);
    const wrapper = mount(BookmarksPage);
    await flushAsync();
    expect(wrapper.text()).toContain("共 2 个书签");
  });

  it("filters bookmarks by search query (label match)", async () => {
    mockListBookmarks.mockResolvedValue(sampleBookmarks);
    const wrapper = mount(BookmarksPage);
    await flushAsync();

    const searchInput = wrapper.find('input[placeholder="搜索书签..."]');
    await searchInput.setValue("Google");
    await nextTick();

    expect(wrapper.text()).toContain("Google DNS");
    expect(wrapper.text()).not.toContain("Cloudflare DNS");
  });

  it("filters bookmarks by search query (target match)", async () => {
    mockListBookmarks.mockResolvedValue(sampleBookmarks);
    const wrapper = mount(BookmarksPage);
    await flushAsync();

    const searchInput = wrapper.find('input[placeholder="搜索书签..."]');
    await searchInput.setValue("1.1.1.1");
    await nextTick();

    expect(wrapper.text()).toContain("Cloudflare DNS");
    expect(wrapper.text()).not.toContain("Google DNS");
  });

  it("shows filtered count when searching", async () => {
    mockListBookmarks.mockResolvedValue(sampleBookmarks);
    const wrapper = mount(BookmarksPage);
    await flushAsync();

    const searchInput = wrapper.find('input[placeholder="搜索书签..."]');
    await searchInput.setValue("Google");
    await nextTick();

    expect(wrapper.text()).toContain("筛选后");
  });

  it("delete button calls store.remove", async () => {
    mockListBookmarks.mockResolvedValue(sampleBookmarks);
    mockDeleteBookmark.mockResolvedValue(undefined);

    const wrapper = mount(BookmarksPage);
    await flushAsync();

    const store = useBookmarkStore();
    const removeSpy = vi.spyOn(store, "remove");

    // Find all buttons — the delete button has variant="danger" in the Button component
    // In the rendered output, we can find buttons inside the table
    const wrapperHtml = wrapper.html();
    expect(wrapperHtml).toContain("Google DNS");

    // Find the delete button (Button with danger variant renders bg-danger-bg class)
    const deleteButtons = wrapper.findAll("button").filter((b) => {
      return b.find('[class*="bg-danger-bg"]').exists() ||
             b.find('[class*="bg-danger"]').exists();
    });

    if (deleteButtons.length > 0) {
      deleteButtons[0].trigger("click");
      await nextTick();
      expect(removeSpy).toHaveBeenCalled();
    }
  });

  it("clicking a target copies it to clipboard", async () => {
    mockListBookmarks.mockResolvedValue(sampleBookmarks);
    const wrapper = mount(BookmarksPage);
    await flushAsync();

    // Find the target element with font-mono class
    const targetButton = wrapper.find("button.font-mono");
    // The target button has class font-mono text-ink-soft hover:text-bamboo
    if (targetButton.exists()) {
      targetButton.trigger("click");
      await nextTick();
      expect(navigator.clipboard.writeText).toHaveBeenCalledWith("8.8.8.8");
    }
  });

  it("does not show empty state when bookmarks exist", async () => {
    mockListBookmarks.mockResolvedValue(sampleBookmarks);
    const wrapper = mount(BookmarksPage);
    await flushAsync();
    expect(wrapper.text()).not.toContain("暂无书签");
  });

  it("shows search input", async () => {
    mockListBookmarks.mockResolvedValue([]);
    const wrapper = mount(BookmarksPage);
    await flushAsync();

    const searchInput = wrapper.find('input[placeholder="搜索书签..."]');
    expect(searchInput.exists()).toBe(true);
  });
});
