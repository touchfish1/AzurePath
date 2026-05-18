import { defineStore } from "pinia";
import { ref } from "vue";
import {
  tracerouteStart,
  tracerouteStop,
  onTraceHop,
  onTraceComplete,
  onTraceError,
  type TraceHopPayload,
  type TraceCompletePayload,
  type TraceErrorPayload,
} from "@/lib/tauri";
import type { UnlistenFn } from "@tauri-apps/api/event";

export interface HopResult {
  hop: number;
  addr: string | null;
  hostname: string | null;
  latencies: (number | null)[];
}

export const useTracerouteStore = defineStore("traceroute", () => {
  const target = ref("8.8.8.8");
  const maxHops = ref(30);
  const timeout = ref(3000);
  const running = ref(false);
  const error = ref("");
  const currentTaskId = ref("");
  const hops = ref<HopResult[]>([]);
  const completeInfo = ref<TraceCompletePayload | null>(null);

  let unlistenHop: UnlistenFn | null = null;
  let unlistenComplete: UnlistenFn | null = null;
  let unlistenError: UnlistenFn | null = null;

  async function attachListeners() {
    detachListeners();
    unlistenHop = await onTraceHop(handleHop);
    unlistenComplete = await onTraceComplete(handleComplete);
    unlistenError = await onTraceError(handleError);
  }

  function detachListeners() {
    unlistenHop?.();
    unlistenComplete?.();
    unlistenError?.();
    unlistenHop = null;
    unlistenComplete = null;
    unlistenError = null;
  }

  function handleError(payload: TraceErrorPayload) {
    error.value = payload.error;
    running.value = false;
    currentTaskId.value = "";
  }

  function handleHop(payload: TraceHopPayload) {
    hops.value.push({
      hop: payload.hop,
      addr: payload.addr,
      hostname: payload.hostname,
      latencies: payload.latencies,
    });
  }

  function handleComplete(payload: TraceCompletePayload) {
    completeInfo.value = payload;
    running.value = false;
    currentTaskId.value = "";
  }

  async function start() {
    if (!target.value.trim()) return;
    running.value = true;
    error.value = "";
    hops.value = [];
    completeInfo.value = null;

    try {
      const taskId = await tracerouteStart(target.value, {
        maxHops: maxHops.value,
        timeoutMs: timeout.value,
        probesPerHop: 3,
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
      await tracerouteStop(currentTaskId.value);
    } catch {
      // ignore
    }
    running.value = false;
    currentTaskId.value = "";
    detachListeners();
  }

  function reset() {
    hops.value = [];
    completeInfo.value = null;
    error.value = "";
  }

  return {
    target,
    maxHops,
    timeout,
    running,
    error,
    currentTaskId,
    hops,
    completeInfo,
    start,
    stop,
    reset,
    attachListeners,
    detachListeners,
  };
});
