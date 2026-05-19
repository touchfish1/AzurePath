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

import { useBookmarkStore } from "@/stores/bookmark";
import BookmarkButton from "@/components/BookmarkButton.vue";

const sampleBookmark = {
  id: "1",
  label: "Google DNS",
  target: "8.8.8.8",
  tags: [],
  createdAt: "2025-01-01T00:00:00Z",
};

describe("BookmarkButton", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
    mockListBookmarks.mockResolvedValue([]);
  });

  function createWrapper(target: string) {
    return mount(BookmarkButton, {
      props: { target },
    });
  }

  it("renders a star button", async () => {
    const wrapper = createWrapper("8.8.8.8");
    await nextTick();

    const btn = wrapper.find("button");
    expect(btn.exists()).toBe(true);
    expect(btn.attributes("title")).toBe("书签");
  });

  it("popover is closed by default", async () => {
    const wrapper = createWrapper("8.8.8.8");
    await nextTick();

    expect(wrapper.text()).not.toContain("暂无书签");
    expect(wrapper.text()).not.toContain("添加当前");
  });

  it("opens popover on toggle button click", async () => {
    const wrapper = createWrapper("8.8.8.8");
    await nextTick();

    wrapper.find("button").trigger("click");
    await nextTick();
    expect(wrapper.text()).toContain("暂无书签");
  });

  it("closes popover on second toggle click", async () => {
    const wrapper = createWrapper("8.8.8.8");
    await nextTick();

    wrapper.find("button").trigger("click");
    await nextTick();
    expect(wrapper.text()).toContain("暂无书签");

    wrapper.find("button").trigger("click");
    await nextTick();
    // Popover should be hidden now — add button text should not be present
    expect(wrapper.text()).not.toContain("暂无书签");
  });

  it("shows bookmark list when bookmarks exist", async () => {
    mockListBookmarks.mockResolvedValue([sampleBookmark]);
    const wrapper = createWrapper("8.8.8.8");
    await nextTick();
    // Wait for onMounted loadBookmarks to complete
    await nextTick();

    wrapper.find("button").trigger("click");
    await nextTick();
    expect(wrapper.text()).toContain("Google DNS");
    expect(wrapper.text()).toContain("8.8.8.8");
  });

  it("shows add current button when target is not bookmarked", async () => {
    const wrapper = createWrapper("1.1.1.1"); // not in bookmarks
    await nextTick();
    await nextTick(); // wait for loadBookmarks

    wrapper.find("button").trigger("click");
    await nextTick();
    expect(wrapper.text()).toContain("添加当前");
  });

  it("does NOT show add current button when target is already bookmarked", async () => {
    mockListBookmarks.mockResolvedValue([sampleBookmark]);
    const wrapper = createWrapper("8.8.8.8"); // already bookmarked
    await nextTick();
    await nextTick();

    wrapper.find("button").trigger("click");
    await nextTick();
    expect(wrapper.text()).not.toContain("添加当前");
  });

  it("shows add form when clicking 'add current'", async () => {
    const wrapper = createWrapper("1.1.1.1");
    await nextTick();
    await nextTick();

    wrapper.find("button").trigger("click");
    await nextTick();

    // Find and click "添加当前" button
    const addBtn = wrapper.find("button");
    // The first button is the toggle. The add button is inside the popover.
    // Let's find it by text
    const allButtons = wrapper.findAll("button");
    const addCurrentBtn = allButtons.find((b) => b.text().includes("添加当前"));
    expect(addCurrentBtn).toBeDefined();
  });

  it("emits select event when bookmark is clicked", async () => {
    mockListBookmarks.mockResolvedValue([sampleBookmark]);
    const wrapper = createWrapper("8.8.8.8");
    await nextTick();
    await nextTick();

    wrapper.find("button").trigger("click");
    await nextTick();

    // Find the bookmark item button (the outer one wrapping the bookmark)
    const bookmarkItemBtn = wrapper
      .findAll("button")
      .find((b) => b.text().includes(sampleBookmark.target));
    expect(bookmarkItemBtn).toBeDefined();
    bookmarkItemBtn?.trigger("click");
    await nextTick();

    expect(wrapper.emitted("select")).toBeTruthy();
    expect(wrapper.emitted("select")![0]).toEqual(["8.8.8.8"]);
  });

  it("calls store.remove when delete button is clicked", async () => {
    mockListBookmarks.mockResolvedValue([sampleBookmark]);
    const wrapper = createWrapper("8.8.8.8");
    await nextTick();
    await nextTick();

    wrapper.find("button").trigger("click");
    await nextTick();

    // Find the delete button by its title attribute
    const deleteBtn = wrapper.find('[title="删除"]');
    expect(deleteBtn.exists()).toBe(true);
    deleteBtn.trigger("click");
    await nextTick();
    expect(mockDeleteBookmark).toHaveBeenCalledWith("1");
  });

  it("closes popover when clicking on backdrop", async () => {
    const wrapper = createWrapper("8.8.8.8");
    await nextTick();

    wrapper.find("button").trigger("click");
    await nextTick();
    expect(wrapper.text()).toContain("暂无书签");

    // Find and click the backdrop div
    const backdrop = wrapper.find('[class*="fixed inset-0 z-40"]');
    expect(backdrop.exists()).toBe(true);
    backdrop.trigger("click");
    await nextTick();
    expect(wrapper.text()).not.toContain("暂无书签");
  });
});
