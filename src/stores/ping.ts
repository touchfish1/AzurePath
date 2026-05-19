import { defineStore } from "pinia";
import { ref } from "vue";
import {
  pingStart,
  pingStop,
  onPingProgress,
  onPingComplete,
  onPingError,
  type PingProgressPayload,
  type PingCompletePayload,
  type PingErrorPayload,
} from "@/lib/tauri";
import type { UnlistenFn } from "@tauri-apps/api/event";

export interface PingResultItem {
  seq: number;
  ttl: number;
  latencyMs: number | null;
  status: string;
}

const MAX_RESULTS = 5000;

export const usePingStore = defineStore("ping", () => {
  const target = ref("8.8.8.8");
  const count = ref(4);
  const timeout = ref(3000);
  const running = ref(false);
  const error = ref("");
  const currentTaskId = ref("");
  const results = ref<PingResultItem[]>([]);
  const stats = ref<PingCompletePayload | null>(null);

  let unlistenProgress: UnlistenFn | null = null;
  let unlistenComplete: UnlistenFn | null = null;
  let unlistenError: UnlistenFn | null = null;

  async function attachListeners() {
    detachListeners();
    unlistenProgress = await onPingProgress(handleProgress);
    unlistenComplete = await onPingComplete(handleComplete);
    unlistenError = await onPingError(handleError);
  }

  function detachListeners() {
    unlistenProgress?.();
    unlistenComplete?.();
    unlistenError?.();
    unlistenProgress = null;
    unlistenComplete = null;
    unlistenError = null;
  }

  function handleProgress(payload: PingProgressPayload) {
    results.value.push({
      seq: payload.seq,
      ttl: payload.ttl,
      latencyMs: payload.latency_ms,
      status: payload.status,
    });
    if (results.value.length > MAX_RESULTS * 2) {
      results.value = results.value.slice(-MAX_RESULTS);
    }
  }

  function handleComplete(payload: PingCompletePayload) {
    stats.value = payload;
    running.value = false;
    currentTaskId.value = "";
  }

  function handleError(payload: PingErrorPayload) {
    error.value = payload.error;
    running.value = false;
    currentTaskId.value = "";
  }

  async function start() {
    if (!target.value.trim()) return;
    running.value = true;
    error.value = "";
    results.value = [];
    stats.value = null;

    try {
      const taskId = await pingStart(target.value, {
        count: count.value,
        intervalMs: 1000,
        timeoutMs: timeout.value,
        payloadSize: 56,
      });
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
      await pingStop(currentTaskId.value);
    } catch {
      // ignore
    }
    running.value = false;
    currentTaskId.value = "";
    detachListeners();
  }

  function reset() {
    results.value = [];
    stats.value = null;
    error.value = "";
  }

  return {
    target,
    count,
    timeout,
    running,
    error,
    currentTaskId,
    results,
    stats,
    start,
    stop,
    reset,
    attachListeners,
    detachListeners,
  };
});
