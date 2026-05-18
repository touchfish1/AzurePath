import { defineStore } from "pinia";
import { ref, computed } from "vue";
import {
  portScanStart,
  portScanStop,
  onPortProgress,
  onPortFound,
  onPortComplete,
  onPortError,
  type PortProgressPayload,
  type PortFoundPayload,
  type PortCompletePayload,
  type PortErrorPayload,
} from "@/lib/tauri";
import type { UnlistenFn } from "@tauri-apps/api/event";

export const usePortScanStore = defineStore("portScan", () => {
  const target = ref("127.0.0.1");
  const portStart = ref(1);
  const portEnd = ref(1024);
  const concurrency = ref(100);
  const timeout = ref(2000);
  const running = ref(false);
  const error = ref("");
  const currentTaskId = ref("");
  const progress = ref<PortProgressPayload | null>(null);
  const foundPorts = ref<PortFoundPayload[]>([]);
  const completeInfo = ref<PortCompletePayload | null>(null);

  const progressPercent = computed(() => {
    if (!progress.value || progress.value.total === 0) return 0;
    return Math.round((progress.value.scanned / progress.value.total) * 100);
  });

  let unlistenProgress: UnlistenFn | null = null;
  let unlistenFound: UnlistenFn | null = null;
  let unlistenComplete: UnlistenFn | null = null;
  let unlistenError: UnlistenFn | null = null;

  async function attachListeners() {
    detachListeners();
    unlistenProgress = await onPortProgress(handleProgress);
    unlistenFound = await onPortFound(handleFound);
    unlistenComplete = await onPortComplete(handleComplete);
    unlistenError = await onPortError(handleError);
  }

  function detachListeners() {
    unlistenProgress?.();
    unlistenFound?.();
    unlistenComplete?.();
    unlistenError?.();
    unlistenProgress = null;
    unlistenFound = null;
    unlistenComplete = null;
    unlistenError = null;
  }

  function handleError(payload: PortErrorPayload) {
    error.value = payload.error;
    running.value = false;
    currentTaskId.value = "";
  }

  function handleProgress(payload: PortProgressPayload) {
    progress.value = payload;
  }

  function handleFound(payload: PortFoundPayload) {
    foundPorts.value.push(payload);
  }

  function handleComplete(payload: PortCompletePayload) {
    completeInfo.value = payload;
    progress.value = null;
    running.value = false;
    currentTaskId.value = "";
  }

  async function start() {
    if (!target.value.trim()) return;
    if (portStart.value < 1 || portEnd.value > 65535 || portStart.value > portEnd.value) {
      error.value = "端口范围无效";
      return;
    }

    running.value = true;
    error.value = "";
    progress.value = null;
    foundPorts.value = [];
    completeInfo.value = null;

    try {
      const taskId = await portScanStart(
        target.value,
        { start: portStart.value, end: portEnd.value },
        { concurrency: concurrency.value, timeoutMs: timeout.value },
      );
      currentTaskId.value = taskId;
      await attachListeners();
    } catch (e) {
      error.value = String(e);
      running.value = false;
    }
  }

  async function stop() {
    if (!currentTaskId.value) return;
    try {
      await portScanStop(currentTaskId.value);
    } catch {
      // ignore
    }
    running.value = false;
    currentTaskId.value = "";
    detachListeners();
  }

  function reset() {
    progress.value = null;
    foundPorts.value = [];
    completeInfo.value = null;
    error.value = "";
  }

  return {
    target,
    portStart,
    portEnd,
    concurrency,
    timeout,
    running,
    error,
    currentTaskId,
    progress,
    foundPorts,
    completeInfo,
    progressPercent,
    start,
    stop,
    reset,
    attachListeners,
    detachListeners,
  };
});
