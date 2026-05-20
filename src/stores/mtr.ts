import { defineStore } from "pinia";
import { ref } from "vue";
import {
  mtrStart,
  mtrStop,
  onMtrProgress,
  onMtrComplete,
  onMtrError,
  type MtrOptions,
  type MtrHopStats,
  type MtrProgressPayload,
  type MtrCompletePayload,
  type MtrErrorPayload,
} from "@/lib/tauri";
import type { UnlistenFn } from "@tauri-apps/api/event";

export const useMtrStore = defineStore("mtr", () => {
  const target = ref("8.8.8.8");
  const maxHops = ref(30);
  const intervalMs = ref(1000);
  const timeoutMs = ref(3000);

  const isRunning = ref(false);
  const error = ref("");
  const currentTaskId = ref("");
  const hops = ref<MtrHopStats[]>([]);
  const totalHops = ref(0);
  const totalRounds = ref(0);

  let unlistenProgress: UnlistenFn | null = null;
  let unlistenComplete: UnlistenFn | null = null;
  let unlistenError: UnlistenFn | null = null;

  async function attachListeners() {
    detachListeners();
    unlistenProgress = await onMtrProgress(handleProgress);
    unlistenComplete = await onMtrComplete(handleComplete);
    unlistenError = await onMtrError(handleError);
  }

  function detachListeners() {
    unlistenProgress?.();
    unlistenComplete?.();
    unlistenError?.();
    unlistenProgress = null;
    unlistenComplete = null;
    unlistenError = null;
  }

  function handleProgress(payload: MtrProgressPayload) {
    totalHops.value = payload.totalHops;
    totalRounds.value = payload.round;
    hops.value = payload.hops;
  }

  function handleComplete(payload: MtrCompletePayload) {
    totalRounds.value = payload.totalRounds;
    hops.value = payload.hops;
    isRunning.value = false;
    currentTaskId.value = "";
  }

  function handleError(payload: MtrErrorPayload) {
    error.value = payload.error;
    isRunning.value = false;
    currentTaskId.value = "";
  }

  async function start() {
    if (!target.value.trim()) return;
    isRunning.value = true;
    error.value = "";
    hops.value = [];
    totalHops.value = 0;
    totalRounds.value = 0;

    const options: MtrOptions = {
      target: target.value,
      maxHops: maxHops.value,
      intervalMs: intervalMs.value,
      timeoutMs: timeoutMs.value,
    };

    try {
      const taskId = await mtrStart(options);
      currentTaskId.value = taskId;
      await attachListeners();
    } catch (e) {
      error.value = String(e);
      isRunning.value = false;
    }
  }

  async function stop() {
    if (!currentTaskId.value) return;
    try {
      await mtrStop(currentTaskId.value);
    } catch {
      // ignore
    }
    isRunning.value = false;
    currentTaskId.value = "";
    detachListeners();
  }

  function reset() {
    hops.value = [];
    totalHops.value = 0;
    totalRounds.value = 0;
    error.value = "";
  }

  return {
    target,
    maxHops,
    intervalMs,
    timeoutMs,
    isRunning,
    error,
    currentTaskId,
    hops,
    totalHops,
    totalRounds,
    start,
    stop,
    reset,
    attachListeners,
    detachListeners,
  };
});
