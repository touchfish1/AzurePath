<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, nextTick } from "vue";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";

interface Props {
  fontSize?: number;
  disabled?: boolean;
  onData?: (data: string) => void;
}

const props = withDefaults(defineProps<Props>(), {
  fontSize: 14,
  disabled: false,
});

const terminalContainer = ref<HTMLDivElement | null>(null);
let terminal: Terminal | null = null;
let fitAddon: FitAddon | null = null;
let resizeObserver: ResizeObserver | null = null;

function getTheme() {
  const theme = document.documentElement.getAttribute("data-theme");
  const isDark = theme === "dark";
  return isDark
    ? {
        background: "#1a1917",
        foreground: "#e5e1da",
        cursor: "#4faa70",
        selectionBackground: "#4faa7033",
        black: "#222120",
        red: "#f87171",
        green: "#4faa70",
        yellow: "#fbbf24",
        blue: "#60a5fa",
        magenta: "#c084fc",
        cyan: "#22d3ee",
        white: "#e5e1da",
        brightBlack: "#3c3935",
        brightRed: "#fca5a5",
        brightGreen: "#5fc085",
        brightYellow: "#fcd34d",
        brightBlue: "#93c5fd",
        brightMagenta: "#d8b4fe",
        brightCyan: "#67e8f9",
        brightWhite: "#f5f2ed",
      }
    : {
        background: "#f6f3ec",
        foreground: "#1a1a18",
        cursor: "#2d5a3d",
        selectionBackground: "#2d5a3d33",
        black: "#1a1a18",
        red: "#dc2626",
        green: "#2d5a3d",
        yellow: "#d97706",
        blue: "#2563eb",
        magenta: "#9333ea",
        cyan: "#0891b2",
        white: "#3d3d38",
        brightBlack: "#8a8a80",
        brightRed: "#ef4444",
        brightGreen: "#3a7a52",
        brightYellow: "#f59e0b",
        brightBlue: "#3b82f6",
        brightMagenta: "#a855f7",
        brightCyan: "#06b6d4",
        brightWhite: "#1a1a18",
      };
}

onMounted(async () => {
  await nextTick();
  if (!terminalContainer.value) return;

  fitAddon = new FitAddon();
  terminal = new Terminal({
    fontSize: props.fontSize,
    fontFamily: '"JetBrains Mono", "Fira Code", "Cascadia Code", monospace',
    theme: getTheme(),
    cursorBlink: true,
    cursorStyle: "bar",
    allowTransparency: true,
    cols: 80,
    rows: 24,
    disableStdin: props.disabled,
    scrollback: 5000,
    tabStopWidth: 4,
  });

  terminal.loadAddon(fitAddon);
  terminal.open(terminalContainer.value);
  fitAddon.fit();

  if (props.onData && !props.disabled) {
    terminal.onData((data: string) => {
      props.onData?.(data);
    });
  }

  resizeObserver = new ResizeObserver(() => {
    try {
      try { fitAddon?.fit(); } catch { /* ignore */ }
    } catch {
      // ignore fit errors during rapid resize
    }
  });
  resizeObserver.observe(terminalContainer.value);
});

onUnmounted(() => {
  resizeObserver?.disconnect();
  terminal?.dispose();
  terminal = null;
  fitAddon = null;
});

watch(
  () => props.fontSize,
  (val) => {
    if (terminal) terminal.options.fontSize = val;
    nextTick(() => {
      try {
        try { fitAddon?.fit(); } catch { /* ignore */ }
      } catch {
        // ignore
      }
    });
  },
);

watch(
  () => props.disabled,
  (val) => {
    if (terminal) {
      terminal.options.disableStdin = val;
    }
  },
);

function write(data: string) {
  if (terminal) terminal.write(data);
}

function clear() {
  terminal?.clear();
}

function resize(cols: number, rows: number) {
  terminal?.resize(cols, rows);
}

function focus() {
  terminal?.focus();
}

defineExpose({ write, clear, resize, focus });
</script>

<template>
  <div
    ref="terminalContainer"
    class="h-full w-full overflow-hidden rounded-lg"
  />
</template>

<style scoped>
:deep(.xterm) {
  height: 100%;
  padding: 8px;
}
:deep(.xterm-viewport) {
  scrollbar-width: thin;
  scrollbar-color: var(--color-ink-ghost) transparent;
}
:deep(.xterm-viewport::-webkit-scrollbar) {
  width: 4px;
}
:deep(.xterm-viewport::-webkit-scrollbar-thumb) {
  background: var(--color-ink-ghost);
  border-radius: 2px;
}
:deep(.xterm-screen) {
  width: auto !important;
}
</style>
