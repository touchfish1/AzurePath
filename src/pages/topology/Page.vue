<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from "vue";
import { useRouter } from "vue-router";
import {
  discoveryPeers,
  onPeerList,
  onPeerOffline,
  type PeerInfo,
} from "@/lib/tauri";
import type { UnlistenFn } from "@tauri-apps/api/event";

const router = useRouter();

// ============= Topology State =============
const canvasRef = ref<HTMLCanvasElement | null>(null);
const peers = ref<PeerInfo[]>([]);
const selectedPeer = ref<PeerInfo | null>(null);
const zoom = ref(1);
const dragNode = ref<number | null>(null);
const detailPopup = ref<{ x: number; y: number; peer: PeerInfo } | null>(null);

interface Node {
  x: number;
  y: number;
  vx: number;
  vy: number;
  peer: PeerInfo;
}

const nodes = ref<Node[]>([]);
let unlistenPeerList: UnlistenFn | null = null;
let unlistenPeerOffline: UnlistenFn | null = null;
let animFrameId: number | null = null;
let canvasWidth = 800;
let canvasHeight = 600;

// ============= Force-directed Layout =============
function initNodes() {
  const centerX = canvasWidth / 2;
  const centerY = canvasHeight / 2;
  nodes.value = peers.value.map((peer) => ({
    x: centerX + (Math.random() - 0.5) * 400,
    y: centerY + (Math.random() - 0.5) * 400,
    vx: 0,
    vy: 0,
    peer,
  }));
}

function simulateForces() {
  const n = nodes.value.length;
  if (n === 0) return;

  const centerX = canvasWidth / 2;
  const centerY = canvasHeight / 2;
  const repulsion = 5000;
  const gravity = 0.01;
  const damping = 0.85;
  const minDist = 80;

  // Compute forces
  for (let i = 0; i < n; i++) {
    let fx = 0;
    let fy = 0;

    // Center gravity
    fx += (centerX - nodes.value[i].x) * gravity;
    fy += (centerY - nodes.value[i].y) * gravity;

    // Repulsion between nodes
    for (let j = 0; j < n; j++) {
      if (i === j) continue;
      let dx = nodes.value[i].x - nodes.value[j].x;
      let dy = nodes.value[i].y - nodes.value[j].y;
      let dist = Math.sqrt(dx * dx + dy * dy);
      if (dist < 1) dist = 1;
      if (dist < minDist) {
        const force = repulsion / (dist * dist);
        fx += (dx / dist) * force;
        fy += (dy / dist) * force;
      }
    }

    // Apply forces with damping
    nodes.value[i].vx = (nodes.value[i].vx + fx) * damping;
    nodes.value[i].vy = (nodes.value[i].vy + fy) * damping;
    nodes.value[i].x += nodes.value[i].vx;
    nodes.value[i].y += nodes.value[i].vy;

    // Keep within bounds
    nodes.value[i].x = Math.max(30, Math.min(canvasWidth - 30, nodes.value[i].x));
    nodes.value[i].y = Math.max(30, Math.min(canvasHeight - 30, nodes.value[i].y));
  }
}

function getSubnets(): Map<string, PeerInfo[]> {
  const subnets = new Map<string, PeerInfo[]>();
  for (const peer of peers.value) {
    const parts = peer.ip.split(".");
    if (parts.length === 4) {
      const subnet = parts.slice(0, 3).join(".");
      if (!subnets.has(subnet)) subnets.set(subnet, []);
      subnets.get(subnet)!.push(peer);
    }
  }
  return subnets;
}

function draw() {
  const canvas = canvasRef.value;
  if (!canvas) return;

  const ctx = canvas.getContext("2d");
  if (!ctx) return;

  // Update canvas size
  const rect = canvas.getBoundingClientRect();
  canvasWidth = canvas.width = rect.width;
  canvasHeight = canvas.height = rect.height;

  ctx.clearRect(0, 0, canvasWidth, canvasHeight);
  ctx.save();
  ctx.scale(zoom.value, zoom.value);

  // Draw subnet clusters (same subnet = gray lines)
  const subnets = getSubnets();
  const nodeMap = new Map(nodes.value.map((n) => [n.peer.ip, n]));

  // Draw connecting lines between nodes in same subnet
  ctx.strokeStyle = "rgba(100, 116, 139, 0.15)";
  ctx.lineWidth = 1;
  for (const [, subnetPeers] of subnets) {
    for (let i = 0; i < subnetPeers.length; i++) {
      for (let j = i + 1; j < subnetPeers.length; j++) {
        const a = nodeMap.get(subnetPeers[i].ip);
        const b = nodeMap.get(subnetPeers[j].ip);
        if (a && b) {
          ctx.beginPath();
          ctx.moveTo(a.x, a.y);
          ctx.lineTo(b.x, b.y);
          ctx.stroke();
        }
      }
    }
  }

  // Draw nodes
  for (const node of nodes.value) {
    const { x, y } = node;
    const isSelected = selectedPeer.value?.ip === node.peer.ip;

    // Outer glow for selected
    if (isSelected) {
      ctx.beginPath();
      ctx.arc(x, y, 28, 0, Math.PI * 2);
      ctx.fillStyle = "rgba(34, 197, 94, 0.15)";
      ctx.fill();
    }

    // Main circle
    ctx.beginPath();
    ctx.arc(x, y, 22, 0, Math.PI * 2);
    ctx.fillStyle = isSelected ? "#22c55e" : "#1e293b";
    ctx.fill();

    // Inner highlight
    ctx.beginPath();
    ctx.arc(x - 4, y - 4, 8, 0, Math.PI * 2);
    ctx.fillStyle = isSelected
      ? "rgba(255,255,255,0.3)"
      : "rgba(255,255,255,0.1)";
    ctx.fill();

    // Label
    ctx.fillStyle = "#f8fafc";
    ctx.font = "10px monospace";
    ctx.textAlign = "center";
    const label = node.peer.hostname || node.peer.ip;
    ctx.fillText(label.length > 14 ? label.slice(0, 14) + "..." : label, x, y + 38);

    // OS label
    if (node.peer.os) {
      ctx.fillStyle = "rgba(148, 163, 184, 0.6)";
      ctx.font = "8px sans-serif";
      ctx.fillText(node.peer.os, x, y + 50);
    }
  }

  ctx.restore();

  // Continue animation
  simulateForces();
  animFrameId = requestAnimationFrame(draw);
}

// ============= Interaction =============
function getNodeAt(clientX: number, clientY: number): Node | null {
  const canvas = canvasRef.value;
  if (!canvas) return null;

  const rect = canvas.getBoundingClientRect();
  const x = (clientX - rect.left) / zoom.value;
  const y = (clientY - rect.top) / zoom.value;

  for (const node of nodes.value) {
    const dx = x - node.x;
    const dy = y - node.y;
    if (dx * dx + dy * dy < 22 * 22) {
      return node;
    }
  }
  return null;
}

function handleMouseDown(e: MouseEvent) {
  const node = getNodeAt(e.clientX, e.clientY);
  if (node) {
    const idx = nodes.value.indexOf(node);
    dragNode.value = idx;
    selectedPeer.value = node.peer;
    detailPopup.value = {
      x: e.clientX,
      y: e.clientY - 10,
      peer: node.peer,
    };
  } else {
    selectedPeer.value = null;
    detailPopup.value = null;
  }
}

function handleMouseMove(e: MouseEvent) {
  if (dragNode.value !== null) {
    const canvas = canvasRef.value;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    nodes.value[dragNode.value].x = (e.clientX - rect.left) / zoom.value;
    nodes.value[dragNode.value].y = (e.clientY - rect.top) / zoom.value;
  }
}

function handleMouseUp() {
  dragNode.value = null;
}

function handleWheel(e: WheelEvent) {
  e.preventDefault();
  if (e.deltaY < 0) {
    zoom.value = Math.min(3, zoom.value + 0.1);
  } else {
    zoom.value = Math.max(0.3, zoom.value - 0.1);
  }
}

function closeDetail() {
  detailPopup.value = null;
  selectedPeer.value = null;
}

function viewportWidth(): number {
  return window.innerWidth;
}

function goToChat(_ip: string) {
  closeDetail();
  router.push("/chat");
}

// ============= Lifecycle =============
async function loadPeers() {
  try {
    peers.value = await discoveryPeers();
  } catch {
    // LAN services not running
  }
  initNodes();
}

function handlePeerList(updatedPeers: PeerInfo[]) {
  peers.value = updatedPeers;
  initNodes();
}

function handlePeerOffline(payload: { id: string }) {
  peers.value = peers.value.filter((p) => p.id !== payload.id);
  initNodes();
  if (detailPopup.value?.peer.id === payload.id) {
    closeDetail();
  }
}

onMounted(async () => {
  unlistenPeerList = await onPeerList(handlePeerList);
  unlistenPeerOffline = await onPeerOffline(handlePeerOffline);
  await loadPeers();

  // Start animation loop
  await nextTick();
  animFrameId = requestAnimationFrame(draw);
});

onUnmounted(() => {
  unlistenPeerList?.();
  unlistenPeerOffline?.();
  if (animFrameId !== null) cancelAnimationFrame(animFrameId);
});
</script>

<template>
  <div class="flex h-full flex-col animate-view-fade">
    <!-- Header -->
    <div class="shrink-0 px-6 pt-6 pb-4">
      <h1 class="text-2xl font-display font-bold text-ink">网络拓扑</h1>
      <p class="mt-0.5 text-sm text-ink-faint">可视化局域网发现设备</p>
    </div>

    <!-- Canvas -->
    <div class="relative flex-1 overflow-hidden">
      <canvas
        ref="canvasRef"
        class="h-full w-full cursor-grab active:cursor-grabbing"
        @mousedown="handleMouseDown"
        @mousemove="handleMouseMove"
        @mouseup="handleMouseUp"
        @mouseleave="handleMouseUp"
        @wheel.prevent="handleWheel"
      />

      <!-- Zoom controls -->
      <div
        class="absolute bottom-4 right-4 flex items-center gap-2 rounded-xl border border-paper-deep/60 bg-paper/90 px-3 py-2 shadow-sm backdrop-blur"
      >
        <button
          class="rounded-lg px-2 py-1 text-xs text-ink-soft transition-colors hover:bg-paper-deep hover:text-ink"
          @click="zoom = Math.max(0.3, zoom - 0.2)"
        >
          -
        </button>
        <span class="min-w-[3rem] text-center text-xs font-mono text-ink-faint">
          {{ (zoom * 100).toFixed(0) }}%
        </span>
        <button
          class="rounded-lg px-2 py-1 text-xs text-ink-soft transition-colors hover:bg-paper-deep hover:text-ink"
          @click="zoom = Math.min(3, zoom + 0.2)"
        >
          +
        </button>
        <span class="mx-1 h-4 w-px bg-paper-deep" />
        <button
          class="rounded-lg px-2 py-1 text-xs text-ink-soft transition-colors hover:bg-paper-deep hover:text-ink"
          @click="zoom = 1"
        >
          重置
        </button>
      </div>

      <!-- Empty state -->
      <div
        v-if="peers.length === 0"
        class="absolute inset-0 flex items-center justify-center"
      >
        <div class="text-center max-w-sm px-6">
          <div class="mx-auto mb-4 flex h-16 w-16 items-center justify-center rounded-2xl border border-dashed border-paper-deep/40 bg-paper/80">
            <svg class="h-8 w-8 text-ink-faint/50" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M4 7h16M4 12h16m-7 5h7" />
            </svg>
          </div>
          <p class="font-medium text-ink-soft">未发现网络设备</p>
          <p class="mt-2 text-xs text-ink-faint leading-relaxed">
            请先启动局域网服务，设备将在发现后自动显示在拓扑图中
          </p>
        </div>
      </div>

      <!-- Detail popup -->
      <div
        v-if="detailPopup"
        class="absolute z-10 w-72 rounded-xl border border-paper-deep/60 bg-paper shadow-xl backdrop-blur-sm"
        :style="{
          left: Math.min(detailPopup.x, viewportWidth() - 300) + 'px',
          top: Math.max(10, detailPopup.y - 160) + 'px',
        }"
      >
        <div class="flex items-center justify-between border-b border-paper-deep/30 px-4 py-3">
          <h3 class="text-sm font-semibold text-ink">
            {{ detailPopup.peer.hostname || detailPopup.peer.ip }}
          </h3>
          <button
            class="rounded-lg p-1 text-ink-faint transition-colors hover:bg-paper-deep hover:text-ink"
            @click="closeDetail"
          >
            <svg class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
        <div class="space-y-2 px-4 py-3">
          <div class="flex justify-between">
            <span class="text-xs text-ink-faint">IP 地址</span>
            <span class="text-xs font-mono text-ink">{{ detailPopup.peer.ip }}</span>
          </div>
          <div v-if="detailPopup.peer.hostname" class="flex justify-between">
            <span class="text-xs text-ink-faint">主机名</span>
            <span class="text-xs text-ink">{{ detailPopup.peer.hostname }}</span>
          </div>
          <div v-if="detailPopup.peer.os" class="flex justify-between">
            <span class="text-xs text-ink-faint">操作系统</span>
            <span class="text-xs text-ink">{{ detailPopup.peer.os }}</span>
          </div>
          <div class="flex justify-between">
            <span class="text-xs text-ink-faint">状态</span>
            <span
              class="text-xs font-medium"
              :class="detailPopup.peer.status === 'online' ? 'text-bamboo' : 'text-ink-faint'"
            >
              {{ detailPopup.peer.status === 'online' ? '在线' : '离线' }}
            </span>
          </div>
          <div class="flex justify-between">
            <span class="text-xs text-ink-faint">监听端口</span>
            <span class="text-xs font-mono text-ink">{{ detailPopup.peer.listen_port }}</span>
          </div>
        </div>
        <div class="border-t border-paper-deep/30 px-4 py-2">
          <button
            class="w-full rounded-lg px-3 py-1.5 text-xs font-medium text-bamboo transition-colors hover:bg-bamboo/5"
            @click="goToChat(detailPopup.peer.ip)"
          >
            发送消息
          </button>
        </div>
      </div>
    </div>

    <!-- Legend -->
    <div class="shrink-0 border-t border-paper-deep/30 px-6 py-2">
      <div class="flex items-center gap-4 text-xs text-ink-faint">
        <span class="flex items-center gap-1.5">
          <span class="inline-block h-2.5 w-2.5 rounded-full bg-slate-700" />
          设备节点
        </span>
        <span class="flex items-center gap-1.5">
          <span class="inline-block h-px w-6 bg-slate-400/20" />
          同子网连接
        </span>
        <span class="flex items-center gap-1.5">
          <span class="inline-block h-2.5 w-2.5 rounded-full bg-green-500" />
          选中节点
        </span>
        <span class="ml-auto">滚轮缩放 | 拖拽节点移动</span>
      </div>
    </div>
  </div>
</template>
