<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from "vue";
import { onRdFrame, type DesktopFrame } from "@/lib/tauri";

interface Props {
  sessionId: string;
  width: number;
  height: number;
  isFullscreen?: boolean;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  resize: [width: number, height: number];
}>();

const canvasRef = ref<HTMLCanvasElement | null>(null);
const containerRef = ref<HTMLDivElement | null>(null);
let ctx: CanvasRenderingContext2D | null = null;
let unlistenFrame: (() => void) | null = null;
let resizeObserver: ResizeObserver | null = null;

function clear() {
  if (!ctx) return;
  ctx.clearRect(0, 0, props.width, props.height);
}

function write(data: Uint8Array) {
  if (!ctx) return;
  const blob = new Blob([data], { type: "image/jpeg" });
  createImageBitmap(blob).then((bitmap) => {
    ctx!.drawImage(bitmap, 0, 0, props.width, props.height);
    bitmap.close();
  });
}

function handleFrame(frame: DesktopFrame) {
  if (!ctx || frame.sessionId !== props.sessionId) return;
  const blob = new Blob([new Uint8Array(frame.data)], { type: "image/jpeg" });
  createImageBitmap(blob).then((bitmap) => {
    ctx!.drawImage(bitmap, frame.x, frame.y, frame.width, frame.height);
    bitmap.close();
  });
}

function setupResizeObserver() {
  if (!containerRef.value) return;
  resizeObserver = new ResizeObserver((entries) => {
    for (const entry of entries) {
      const { width, height } = entry.contentRect;
      emit("resize", Math.round(width), Math.round(height));
    }
  });
  resizeObserver.observe(containerRef.value);
}

onMounted(() => {
  if (canvasRef.value) {
    ctx = canvasRef.value.getContext("2d");
  }
  setupResizeObserver();
  onRdFrame(handleFrame).then((unlisten) => {
    unlistenFrame = unlisten;
  });
});

onUnmounted(() => {
  if (unlistenFrame) {
    unlistenFrame();
    unlistenFrame = null;
  }
  if (resizeObserver) {
    resizeObserver.disconnect();
    resizeObserver = null;
  }
  ctx = null;
});

watch(
  () => [props.width, props.height],
  ([w, h]) => {
    if (canvasRef.value) {
      canvasRef.value.width = w;
      canvasRef.value.height = h;
    }
  },
  { immediate: true },
);

defineExpose({ clear, write });
</script>

<template>
  <div
    ref="containerRef"
    class="h-full w-full"
    :class="{ 'fixed inset-0 z-40 bg-paper': isFullscreen }"
  >
    <canvas
      ref="canvasRef"
      class="h-full w-full"
      :style="{ imageRendering: 'pixelated' }"
    />
  </div>
</template>
