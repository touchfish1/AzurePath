import { defineStore } from "pinia";
import { ref } from "vue";
import {
  sendApiRequest,
  listApiRequests,
  saveApiRequest,
  deleteApiRequest,
  wsConnect,
  wsSend,
  wsClose,
  wsGetMessages,
  wsClearMessages,
  onWsMessage,
  onWsConnected,
  onWsDisconnected,
  envList,
  envSave,
  envDelete,
  collectionList,
  collectionSave,
  collectionDelete,
  generateHttpCode,
  type ApiRequest,
  type ApiResponse,
  type SavedRequest,
  type WsMessage,
  type Environment,
  type RequestCollection,
} from "@/lib/tauri";
import type { UnlistenFn } from "@tauri-apps/api/event";

export const useApiTestStore = defineStore("apiTest", () => {
  // ── HTTP Request State ──
  const savedRequests = ref<SavedRequest[]>([]);
  const currentRequest = ref<ApiRequest>({
    method: "GET",
    url: "",
    headers: [],
    body: null,
    bodyType: "json",
    auth: { authType: "none", username: null, password: null, token: null, apiKey: null, apiKeyName: null, apiKeyLocation: null },
  });
  const response = ref<ApiResponse | null>(null);
  const sending = ref(false);
  const requestName = ref("");
  const error = ref("");

  // ── WebSocket State ──
  const wsUrl = ref("");
  const wsConnected = ref(false);
  const wsMessages = ref<WsMessage[]>([]);
  const wsSending = ref(false);
  const activeTab = ref<"http" | "websocket">("http");
  let unlistenWsMessage: UnlistenFn | null = null;
  let unlistenWsConnected: UnlistenFn | null = null;
  let unlistenWsDisconnected: UnlistenFn | null = null;

  // ── Environment State ──
  const environments = ref<Environment[]>([]);
  const activeEnvId = ref<string | null>(null);

  // ── Collection State ──
  const collections = ref<RequestCollection[]>([]);
  const showCollectionPanel = ref(false);
  const showCodeGen = ref(false);
  const generatedCode = ref("");

  // ── HTTP Actions ──
  async function send() {
    if (!currentRequest.value.url.trim()) { error.value = "请输入 URL"; return; }
    sending.value = true; error.value = ""; response.value = null;
    try { response.value = await sendApiRequest(currentRequest.value); }
    catch (e) { error.value = String(e); }
    finally { sending.value = false; }
  }

  async function loadSaved() {
    try { savedRequests.value = await listApiRequests(); } catch {}
  }

  async function saveCurrent() {
    if (!requestName.value.trim()) { error.value = "请输入请求名称"; return; }
    if (!currentRequest.value.url.trim()) { error.value = "请输入 URL"; return; }
    try {
      await saveApiRequest(null, requestName.value.trim(), currentRequest.value);
      requestName.value = ""; await loadSaved();
    } catch (e) { error.value = String(e); }
  }

  async function deleteSaved(id: string) {
    try { await deleteApiRequest(id); await loadSaved(); }
    catch (e) { error.value = String(e); }
  }

  function loadRequest(item: SavedRequest) {
    currentRequest.value = { ...item.request };
    requestName.value = item.name; response.value = null; error.value = "";
  }

  function newRequest() {
    currentRequest.value = {
      method: "GET", url: "", headers: [], body: null, bodyType: "json",
      auth: { authType: "none", username: null, password: null, token: null, apiKey: null, apiKeyName: null, apiKeyLocation: null },
    };
    requestName.value = ""; response.value = null; error.value = "";
  }

  // ── WebSocket Actions ──
  async function wsConnectAction() {
    if (!wsUrl.value.trim()) { error.value = "请输入 WebSocket URL"; return; }
    wsSending.value = true; error.value = "";
    try {
      await wsConnect(wsUrl.value);
      wsConnected.value = true;
      // Register listeners
      unlistenWsMessage = await onWsMessage((msg) => { wsMessages.value.push(msg); });
      unlistenWsConnected = await onWsConnected(() => { wsConnected.value = true; });
      unlistenWsDisconnected = await onWsDisconnected(() => { wsConnected.value = false; });
    } catch (e) { error.value = String(e); }
    finally { wsSending.value = false; }
  }

  async function wsSendAction(msg: string) {
    if (!msg.trim()) return;
    try {
      await wsSend(msg);
      wsMessages.value.push({
        id: crypto.randomUUID(), direction: "sent", content: msg, timestamp: new Date().toISOString(),
      });
    } catch (e) { error.value = String(e); }
  }

  async function wsDisconnect() {
    try { await wsClose(); } catch {}
    wsConnected.value = false;
    unlistenWsMessage?.(); unlistenWsConnected?.(); unlistenWsDisconnected?.();
  }

  async function wsLoadMessages() {
    try { wsMessages.value = await wsGetMessages(); } catch {}
  }

  async function wsClear() {
    try { await wsClearMessages(); wsMessages.value = []; } catch {}
  }

  // ── Environment Actions ──
  async function loadEnvironments() {
    try {
      environments.value = await envList();
      if (!activeEnvId.value && environments.value.length > 0) {
        activeEnvId.value = environments.value[0].id;
      }
    } catch {}
  }

  async function saveEnvironment(env: Environment) {
    const saved = await envSave(env);
    await loadEnvironments();
    return saved;
  }

  async function deleteEnvironment(id: string) {
    await envDelete(id);
    await loadEnvironments();
  }

  // ── Collection Actions ──
  async function loadCollections() {
    try { collections.value = await collectionList(); } catch {}
  }

  async function saveCollection(name: string, id: string | null, requests: any[]) {
    const result = await collectionSave(name, id, requests);
    await loadCollections();
    return result;
  }

  async function deleteCollection(id: string) {
    await collectionDelete(id);
    await loadCollections();
  }

  // ── Code Generation ──
  async function generateCode(lang: string) {
    const req = currentRequest.value;
    try {
      generatedCode.value = await generateHttpCode(req.method, req.url, req.headers, req.body, req.bodyType, lang);
      showCodeGen.value = true;
    } catch (e) { error.value = String(e); }
  }

  return {
    // HTTP
    savedRequests, currentRequest, response, sending, requestName, error,
    send, loadSaved, saveCurrent, deleteSaved, loadRequest, newRequest,
    // WebSocket
    wsUrl, wsConnected, wsMessages, wsSending, activeTab,
    wsConnectAction, wsSendAction, wsDisconnect, wsLoadMessages, wsClear,
    // Environment
    environments, activeEnvId, loadEnvironments, saveEnvironment, deleteEnvironment,
    // Collections
    collections, showCollectionPanel, loadCollections, saveCollection, deleteCollection,
    // Code Gen
    showCodeGen, generatedCode, generateCode,
  };
});
