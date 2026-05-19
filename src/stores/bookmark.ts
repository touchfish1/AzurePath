import { defineStore } from "pinia";
import { ref } from "vue";
import {
  listBookmarks,
  addBookmark,
  deleteBookmark,
  type Bookmark,
} from "@/lib/tauri";
import { useToastStore } from "@/stores/toast";

export const useBookmarkStore = defineStore("bookmark", () => {
  const bookmarks = ref<Bookmark[]>([]);
  const loaded = ref(false);
  const loading = ref(false);

  async function loadBookmarks() {
    loading.value = true;
    try {
      bookmarks.value = await listBookmarks();
      loaded.value = true;
    } catch (e) {
      useToastStore().error(`加载书签失败: ${e}`);
    } finally {
      loading.value = false;
    }
  }

  async function add(label: string, target: string, tags?: string[]) {
    const toast = useToastStore();
    try {
      const bm = await addBookmark(label, target, tags);
      bookmarks.value.unshift(bm);
      toast.add("success", "书签已添加");
      return bm;
    } catch (e) {
      toast.add("error", `添加书签失败: ${e}`);
      throw e;
    }
  }

  async function remove(id: string) {
    const toast = useToastStore();
    try {
      await deleteBookmark(id);
      bookmarks.value = bookmarks.value.filter((b) => b.id !== id);
      toast.add("success", "书签已删除");
    } catch (e) {
      toast.add("error", `删除书签失败: ${e}`);
    }
  }

  return { bookmarks, loaded, loading, loadBookmarks, add, remove };
});
