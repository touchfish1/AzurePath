<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from "vue";
import { onRdFrame, type DesktopFrame } from "@/lib/tauri";

interface Props {
  sessionId: string;
  width: number;
  height: number;
}

const props = defineProps<Props>();

const canvasRef = ref<HTMLCanvasElement | null>(null);
let ctx: CanvasRenderingContext2D | null = null;
let unlistenFrame: (() => void) | null = null;

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

onMounted(() => {
  if (canvasRef.value) {
    ctx = canvasRef.value.getContext("2d");
  }
  onRdFrame(handleFrame).then((unlisten) => {
    unlistenFrame = unlisten;
  });
});

onUnmounted(() => {
  if (unlistenFrame) {
    unlistenFrame();
    unlistenFrame = null;
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
  <canvas
    ref="canvasRef"
    class="h-full w-full"
    :style="{ imageRendering: 'pixelated' }"
  />
</template>
