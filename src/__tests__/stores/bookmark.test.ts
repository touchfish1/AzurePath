import { describe, it, expect, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";

const mockListBookmarks = vi.hoisted(() => vi.fn());
const mockAddBookmark = vi.hoisted(() => vi.fn());
const mockDeleteBookmark = vi.hoisted(() => vi.fn());

vi.mock("@/lib/tauri", () => ({
  listBookmarks: mockListBookmarks,
  addBookmark: mockAddBookmark,
  deleteBookmark: mockDeleteBookmark,
}));

import { useBookmarkStore } from "@/stores/bookmark";

const sampleBookmark = {
  id: "1",
  label: "Google DNS",
  target: "8.8.8.8",
  tags: ["dns"],
  createdAt: "2025-01-01T00:00:00Z",
};

const sampleBookmark2 = {
  id: "2",
  label: "Cloudflare DNS",
  target: "1.1.1.1",
  tags: [],
  createdAt: "2025-01-02T00:00:00Z",
};

describe("bookmark store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
  });

  it("has default values", () => {
    const store = useBookmarkStore();
    expect(store.bookmarks).toEqual([]);
    expect(store.loaded).toBe(false);
    expect(store.loading).toBe(false);
  });

  describe("loadBookmarks", () => {
    it("loads bookmarks from the backend", async () => {
      mockListBookmarks.mockResolvedValueOnce([sampleBookmark]);
      const store = useBookmarkStore();
      await store.loadBookmarks();

      expect(mockListBookmarks).toHaveBeenCalledTimes(1);
      expect(store.bookmarks).toEqual([sampleBookmark]);
      expect(store.loaded).toBe(true);
      expect(store.loading).toBe(false);
    });

    it("handles empty list", async () => {
      mockListBookmarks.mockResolvedValueOnce([]);
      const store = useBookmarkStore();
      await store.loadBookmarks();

      expect(store.bookmarks).toEqual([]);
      expect(store.loaded).toBe(true);
    });

    it("sets loading state during fetch", async () => {
      let resolvePromise!: (val: typeof sampleBookmark[]) => void;
      mockListBookmarks.mockReturnValueOnce(
        new Promise<typeof sampleBookmark[]>((resolve) => {
          resolvePromise = resolve;
        }),
      );

      const store = useBookmarkStore();
      const promise = store.loadBookmarks();
      expect(store.loading).toBe(true);

      resolvePromise([sampleBookmark]);
      await promise;
      expect(store.loading).toBe(false);
    });

    it("handles error gracefully", async () => {
      mockListBookmarks.mockRejectedValueOnce(new Error("Network error"));
      const store = useBookmarkStore();
      await store.loadBookmarks();

      // loaded stays false because it's only set inside the try block
      expect(store.loaded).toBe(false);
      expect(store.loading).toBe(false);
      expect(store.bookmarks).toEqual([]);
    });

    it("can be called multiple times", async () => {
      mockListBookmarks.mockImplementation(() => Promise.resolve([sampleBookmark]));

      const store = useBookmarkStore();
      await store.loadBookmarks();
      expect(store.bookmarks).toHaveLength(1);
      expect(store.loaded).toBe(true);

      await store.loadBookmarks();
      expect(store.bookmarks).toHaveLength(1);
      expect(store.loaded).toBe(true);
    });
  });

  describe("add", () => {
    it("adds a bookmark to the beginning of the list", async () => {
      mockAddBookmark.mockResolvedValueOnce(sampleBookmark);
      const store = useBookmarkStore();
      const result = await store.add("Google DNS", "8.8.8.8", ["dns"]);

      expect(result).toEqual(sampleBookmark);
      expect(store.bookmarks).toHaveLength(1);
      expect(store.bookmarks[0]).toEqual(sampleBookmark);
      expect(mockAddBookmark).toHaveBeenCalledWith("Google DNS", "8.8.8.8", ["dns"]);
    });

    it("prepends new bookmarks", async () => {
      mockAddBookmark.mockResolvedValueOnce(sampleBookmark);
      mockAddBookmark.mockResolvedValueOnce(sampleBookmark2);

      const store = useBookmarkStore();
      await store.add("Google DNS", "8.8.8.8");
      await store.add("Cloudflare DNS", "1.1.1.1");

      expect(store.bookmarks).toHaveLength(2);
      expect(store.bookmarks[0].id).toBe("2");
    });

    it("works without tags", async () => {
      mockAddBookmark.mockResolvedValueOnce(sampleBookmark);
      const store = useBookmarkStore();
      await store.add("Google DNS", "8.8.8.8");

      expect(mockAddBookmark).toHaveBeenCalledWith("Google DNS", "8.8.8.8", undefined);
    });

    it("handles error and rethrows", async () => {
      mockAddBookmark.mockRejectedValueOnce(new Error("Failed to add"));
      const store = useBookmarkStore();

      await expect(store.add("Test", "test.com")).rejects.toThrow("Failed to add");
      expect(store.bookmarks).toEqual([]);
    });

    it("does not add to list on error", async () => {
      mockAddBookmark.mockRejectedValueOnce(new Error("Failed"));
      const store = useBookmarkStore();
      store.bookmarks = [sampleBookmark];

      await expect(store.add("New", "new.com")).rejects.toThrow("Failed");
      expect(store.bookmarks).toHaveLength(1); // unchanged
    });
  });

  describe("remove", () => {
    it("removes a bookmark by id", async () => {
      mockDeleteBookmark.mockResolvedValueOnce(undefined);
      const store = useBookmarkStore();
      store.bookmarks = [sampleBookmark, sampleBookmark2];

      await store.remove("1");
      expect(store.bookmarks).toHaveLength(1);
      expect(store.bookmarks[0].id).toBe("2");
      expect(mockDeleteBookmark).toHaveBeenCalledWith("1");
    });

    it("handles deleting non-existent id gracefully", async () => {
      mockDeleteBookmark.mockResolvedValueOnce(undefined);
      const store = useBookmarkStore();
      store.bookmarks = [sampleBookmark];

      await store.remove("nonexistent");
      // The filter keeps all bookmarks because no id matched "nonexistent"
      expect(store.bookmarks).toHaveLength(1);
    });

    it("handles error gracefully", async () => {
      mockDeleteBookmark.mockRejectedValueOnce(new Error("Delete failed"));
      const store = useBookmarkStore();
      store.bookmarks = [sampleBookmark];

      await store.remove("1");
      // Bookmark should remain in list since deletion failed
      expect(store.bookmarks).toHaveLength(1);
    });

    it("works on empty list", async () => {
      mockDeleteBookmark.mockResolvedValueOnce(undefined);
      const store = useBookmarkStore();

      await store.remove("1");
      expect(store.bookmarks).toEqual([]);
    });
  });
});
