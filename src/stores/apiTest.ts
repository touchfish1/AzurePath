import { defineStore } from "pinia";
import { ref } from "vue";
import {
  sendApiRequest,
  listApiRequests,
  saveApiRequest,
  deleteApiRequest,
  type ApiRequest,
  type ApiResponse,
  type SavedRequest,
} from "@/lib/tauri";

export const useApiTestStore = defineStore("apiTest", () => {
  const savedRequests = ref<SavedRequest[]>([]);
  const currentRequest = ref<ApiRequest>({
    method: "GET",
    url: "",
    headers: [],
    body: null,
    bodyType: "json",
  });
  const response = ref<ApiResponse | null>(null);
  const sending = ref(false);
  const requestName = ref("");
  const error = ref("");

  async function send() {
    if (!currentRequest.value.url.trim()) {
      error.value = "请输入 URL";
      return;
    }
    sending.value = true;
    error.value = "";
    response.value = null;
    try {
      response.value = await sendApiRequest(currentRequest.value);
    } catch (e) {
      error.value = String(e);
    } finally {
      sending.value = false;
    }
  }

  async function loadSaved() {
    try {
      savedRequests.value = await listApiRequests();
    } catch {
      // silently fail
    }
  }

  async function saveCurrent() {
    if (!requestName.value.trim()) {
      error.value = "请输入请求名称";
      return;
    }
    if (!currentRequest.value.url.trim()) {
      error.value = "请输入 URL";
      return;
    }
    try {
      await saveApiRequest(null, requestName.value.trim(), currentRequest.value);
      requestName.value = "";
      await loadSaved();
    } catch (e) {
      error.value = String(e);
    }
  }

  async function deleteSaved(id: string) {
    try {
      await deleteApiRequest(id);
      await loadSaved();
    } catch (e) {
      error.value = String(e);
    }
  }

  function loadRequest(item: SavedRequest) {
    currentRequest.value = { ...item.request };
    requestName.value = item.name;
    response.value = null;
    error.value = "";
  }

  function newRequest() {
    currentRequest.value = {
      method: "GET",
      url: "",
      headers: [],
      body: null,
      bodyType: "json",
    };
    requestName.value = "";
    response.value = null;
    error.value = "";
  }

  return {
    savedRequests,
    currentRequest,
    response,
    sending,
    requestName,
    error,
    send,
    loadSaved,
    saveCurrent,
    deleteSaved,
    loadRequest,
    newRequest,
  };
});
