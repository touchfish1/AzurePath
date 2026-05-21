<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick, watch } from "vue";
import { useRouter } from "vue-router";
import * as d3 from "d3";
import {
  discoveryPeers,
  onPeerList,
  onPeerOffline,
  discoverTopology,
  cancelTopologyDiscovery,
  onTopologyProgress,
  onTopologyResult,
  onTopologyError,
  type PeerInfo,
  type DiscoverProgress,
  type TopologyResult,
} from "@/lib/tauri";
import { useTopologyStore } from "@/stores/topology";
import type { UnlistenFn } from "@tauri-apps/api/event";

const router = useRouter();
const topoStore = useTopologyStore();

// ============= State =============
const svgRef = ref<SVGSVGElement | null>(null);
const peers = ref<PeerInfo[]>([]);
const selectedPeer = ref<PeerInfo | null>(null);
const detailPopup = ref<{ x: number; y: number; peer: PeerInfo } | null>(null);

interface SimNode extends d3.SimulationNodeDatum {
  id: string;
  peer: PeerInfo;
  deviceType: string;
  isDiscovered: boolean;
  isOnline: boolean;
  cpuUsage: number | null;
  memoryUsage: number | null;
}

interface SimLink extends d3.SimulationLinkDatum<SimNode> {
  id: string;
  latencyMs: number | null;
  linkType: "subnet" | "discovered";
}

let simNodes: SimNode[] = [];
let simLinks: SimLink[] = [];
let simulation: d3.Simulation<SimNode, SimLink> | null = null;
let svg: d3.Selection<SVGSVGElement, unknown, null, undefined> | null = null;
let zoomGroup: d3.Selection<SVGGElement, unknown, null, undefined> | null = null;
let gNodes: d3.Selection<SVGGElement, SimNode, SVGGElement, unknown> | null = null;
let gLinks: d3.Selection<SVGGElement, SimLink, SVGGElement, unknown> | null = null;
let gLabels: d3.Selection<SVGGElement, SimNode, SVGGElement, unknown> | null = null;
let gSubLabels: d3.Selection<SVGGElement, SimNode, SVGGElement, unknown> | null = null;
let unlistenPeerList: UnlistenFn | null = null;
let unlistenPeerOffline: UnlistenFn | null = null;

// ============= Auto Discovery State =============
const subnet = ref("192.168.1.0/24");
const discovering = ref(false);
const showDiscoveryPanel = ref(false);
const discoverProgress = ref<DiscoverProgress | null>(null);
const discoveredLinks = ref<{ source: string; target: string; latencyMs: number | null }[]>([]);
let unlistenDiscoverProgress: UnlistenFn | null = null;
let unlistenDiscoverResult: UnlistenFn | null = null;
let unlistenDiscoverError: UnlistenFn | null = null;

// ============= Enhanced Features =============
const showSnapshotPanel = ref(false);
const snapshotName = ref("");
const algorithmOptions = [
  { value: "forceDirected", label: "力导向布局" },
  { value: "hierarchical", label: "层级布局" },
  { value: "circular", label: "环形布局" },
  { value: "grid", label: "网格布局" },
];

// ============= D3 Force Simulation =============
function detectDeviceType(ip: string): string {
  const last = parseInt(ip.split(".")[3] || "0");
  if (last === 1 || last === 254) return "router";
  if (last >= 2 && last <= 10) return "switch";
  return "other";
}

function buildSimData() {
  const discIpSet = new Set(discoveredLinks.value.flatMap((l) => [l.source, l.target]));
  simNodes = peers.value.map((p) => ({
    id: p.id,
    peer: p,
    deviceType: p.os === "__discovered__" ? "other" : detectDeviceType(p.ip),
    isDiscovered: p.os === "__discovered__" || discIpSet.has(p.ip),
    isOnline: p.status === "online",
    cpuUsage: null,
    memoryUsage: null,
  }));

  // Build links: subnet connections
  const linkMap = new Map<string, SimLink>();
  const subnets = new Map<string, PeerInfo[]>();
  for (const peer of peers.value) {
    const parts = peer.ip.split(".");
    if (parts.length === 4) {
      const sn = parts.slice(0, 3).join(".");
      if (!subnets.has(sn)) subnets.set(sn, []);
      subnets.get(sn)!.push(peer);
    }
  }
  const nodeIdByIp = new Map(simNodes.map((n) => [n.peer.ip, n.id]));
  for (const [, snPeers] of subnets) {
    for (let i = 0; i < snPeers.length; i++) {
      for (let j = i + 1; j < snPeers.length; j++) {
        const sId = nodeIdByIp.get(snPeers[i].ip);
        const tId = nodeIdByIp.get(snPeers[j].ip);
        if (sId && tId) {
          const k = [sId, tId].sort().join("-");
          if (!linkMap.has(k)) {
            linkMap.set(k, { id: k, source: sId, target: tId, latencyMs: null, linkType: "subnet" });
          }
        }
      }
    }
  }

  // Discovered links
  for (const dl of discoveredLinks.value) {
    const sId = nodeIdByIp.get(dl.source);
    const tId = nodeIdByIp.get(dl.target);
    if (sId && tId) {
      const k = [sId, tId].sort().join("-");
      linkMap.set(k, { id: k, source: sId, target: tId, latencyMs: dl.latencyMs, linkType: "discovered" });
    }
  }

  simLinks = Array.from(linkMap.values());
}

function initD3() {
  if (!svgRef.value) return;

  // Clear previous
  d3.select(svgRef.value).selectAll("*").remove();

  svg = d3.select(svgRef.value);

  // Zoom behavior: pan on empty drag, scroll to zoom
  const zoomBehavior = d3.zoom<SVGSVGElement, unknown>()
    .scaleExtent([0.2, 5])
    .on("zoom", (event) => {
      zoomGroup?.attr("transform", event.transform.toString());
    });

  svg.call(zoomBehavior);

  zoomGroup = svg.append("g");

  // Create groups for different layers
  gLinks = zoomGroup.append("g").attr("class", "links") as any;
  gNodes = zoomGroup.append("g").attr("class", "nodes") as any;
  gLabels = zoomGroup.append("g").attr("class", "labels") as any;
  gSubLabels = zoomGroup.append("g").attr("class", "sub-labels") as any;

  buildSimData();

  // Create force simulation
  simulation = d3.forceSimulation<SimNode>(simNodes)
    .force("link", d3.forceLink<SimNode, SimLink>(simLinks).id((d) => d.id).distance(120))
    .force("charge", d3.forceManyBody().strength(-400))
    .force("center", d3.forceCenter(400, 300))
    .force("collision", d3.forceCollide(30))
    .alphaDecay(0.02)
    .on("tick", ticked);

  // Draw links
  gLinks?.selectAll("line")
    .data(simLinks)
    .join("line")
    .attr("stroke", (d) => d.linkType === "discovered" ? "rgba(59, 130, 246, 0.4)" : "rgba(100, 116, 139, 0.15)")
    .attr("stroke-width", (d) => d.linkType === "discovered" ? 2 : 1)
    .attr("stroke-dasharray", (d) => d.linkType === "discovered" ? "5,4" : "none");

  // Draw nodes with shapes
  gNodes?.selectAll("g")
    .data(simNodes)
    .join("g")
    .attr("class", "topo-node")
    .each(function (d) {
      const el = d3.select(this);
      const shape = getDeviceShape(d.deviceType);
      const color = getDeviceColor(d.deviceType, d.isDiscovered);

      if (shape === "diamond") {
        el.append("path")
          .attr("d", d3.symbol<unknown, unknown>().type(d3.symbolDiamond).size(900)())
          .attr("fill", color)
          .attr("stroke", d.isOnline ? "#22c55e" : "none")
          .attr("stroke-width", d.isOnline ? 2.5 : 0);
      } else if (shape === "roundedRect") {
        el.append("rect")
          .attr("x", -18).attr("y", -13)
          .attr("width", 36).attr("height", 26)
          .attr("rx", 6).attr("ry", 6)
          .attr("fill", color)
          .attr("stroke", d.isOnline ? "#22c55e" : "none")
          .attr("stroke-width", d.isOnline ? 2.5 : 0);
      } else {
        el.append("circle")
          .attr("r", 20)
          .attr("fill", color)
          .attr("stroke", d.isOnline ? "#22c55e" : "none")
          .attr("stroke-width", d.isOnline ? 2.5 : 0);
      }

      // CPU usage ring
      if (d.cpuUsage !== null) {
        el.append("circle")
          .attr("class", "status-ring")
          .attr("r", 24)
          .attr("fill", "none")
          .attr("stroke", d.cpuUsage! > 80 ? "#ef4444" : d.cpuUsage! > 50 ? "#f59e0b" : "#22c55e")
          .attr("stroke-width", 2.5);
      }
    })
    // Drag behavior
    .call(d3.drag<SVGGElement, SimNode>()
      .on("start", function (event, d) {
        if (!event.active) simulation?.alphaTarget(0.3).restart();
        d.fx = d.x;
        d.fy = d.y;
      })
      .on("drag", function (event, d) {
        d.fx = event.x;
        d.fy = event.y;
      })
      .on("end", function (event, d) {
        if (!event.active) simulation?.alphaTarget(0);
        d.fx = null;
        d.fy = null;
      }) as any)
    // Click to select
    .on("click", function (event, d) {
      event.stopPropagation();
      selectedPeer.value = d.peer;
      detailPopup.value = {
        x: event.clientX || event.sourceEvent?.clientX,
        y: (event.clientY || event.sourceEvent?.clientY) - 10,
        peer: d.peer,
      };
    });

  // Draw labels
  gLabels?.selectAll("text")
    .data(simNodes)
    .join("text")
    .text((d) => {
      const label = d.peer.hostname || d.peer.ip;
      return label.length > 14 ? label.slice(0, 14) + "..." : label;
    })
    .attr("text-anchor", "middle")
    .attr("dy", 36)
    .attr("fill", "currentColor")
    .attr("font-size", "10px")
    .attr("font-family", "monospace")
    .style("pointer-events", "none");

  // Sub-labels (device type)
  gSubLabels?.selectAll("text")
    .data(simNodes)
    .join("text")
    .text((d) => d.deviceType)
    .attr("text-anchor", "middle")
    .attr("dy", 48)
    .attr("fill", "#64748b")
    .attr("font-size", "8px")
    .style("pointer-events", "none");

  function ticked() {
    gLinks?.selectAll<SVGLineElement, SimLink>("line")
      .attr("x1", (d) => (d.source as SimNode).x!)
      .attr("y1", (d) => (d.source as SimNode).y!)
      .attr("x2", (d) => (d.target as SimNode).x!)
      .attr("y2", (d) => (d.target as SimNode).y!);

    gNodes?.selectAll<SVGGElement, SimNode>("g")
      .attr("transform", (d) => `translate(${d.x},${d.y})`);

    gLabels?.selectAll<SVGTextElement, SimNode>("text")
      .attr("x", (d) => d.x!)
      .attr("y", (d) => d.y!);

    gSubLabels?.selectAll<SVGTextElement, SimNode>("text")
      .attr("x", (d) => d.x!)
      .attr("y", (d) => d.y!);
  }
}

function getDeviceShape(deviceType: string): string {
  switch (deviceType) {
    case "router": case "firewall": return "diamond";
    case "switch": case "ap": return "roundedRect";
    default: return "circle";
  }
}

function getDeviceColor(deviceType: string, isDiscovered: boolean): string {
  if (isDiscovered) return "#3b82f6";
  switch (deviceType) {
    case "router": return "#7c3aed";
    case "switch": return "#0891b2";
    case "firewall": return "#dc2626";
    case "server": return "#2563eb";
    case "camera": return "#059669";
    default: return "#475569";
  }
}

function closeDetail() {
  detailPopup.value = null;
  selectedPeer.value = null;
}

function viewportWidth(): number {
  return window.innerWidth;
}

function goToChat() {
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
  initD3();
}

function handlePeerList(updatedPeers: PeerInfo[]) {
  peers.value = updatedPeers;
  initD3();
}

function handlePeerOffline(payload: { id: string }) {
  peers.value = peers.value.filter((p) => p.id !== payload.id);
  initD3();
  if (detailPopup.value?.peer.id === payload.id) {
    closeDetail();
  }
}

// ============= Layout Switching & Snapshots =============
async function switchLayout(algo: string) {
  topoStore.layoutAlgorithm = algo;
  await topoStore.computeLayout();
  // Sync d3 node positions
  for (const topoNode of topoStore.nodes) {
    const simNode = simNodes.find((n) => n.id === topoNode.id || n.peer.ip === topoNode.ip);
    if (simNode) {
      simNode.x = topoNode.x;
      simNode.y = topoNode.y;
      simNode.fx = topoNode.x;
      simNode.fy = topoNode.y;
    }
  }
  simulation?.alpha(0).tick();
  ticked();
  // Release fixed positions after a moment
  setTimeout(() => {
    for (const n of simNodes) { n.fx = null; n.fy = null; }
  }, 100);
}

function ticked() {
  gLinks?.selectAll<SVGLineElement, SimLink>("line")
    .attr("x1", (d) => (d.source as SimNode).x!)
    .attr("y1", (d) => (d.source as SimNode).y!)
    .attr("x2", (d) => (d.target as SimNode).x!)
    .attr("y2", (d) => (d.target as SimNode).y!);

  gNodes?.selectAll<SVGGElement, SimNode>("g")
    .attr("transform", (d) => `translate(${d.x},${d.y})`);

  gLabels?.selectAll<SVGTextElement, SimNode>("text")
    .attr("x", (d) => d.x!)
    .attr("y", (d) => d.y!);

  gSubLabels?.selectAll<SVGTextElement, SimNode>("text")
    .attr("x", (d) => d.x!)
    .attr("y", (d) => d.y!);
}

async function saveCurrentSnapshot() {
  if (!snapshotName.value) return;
  try {
    const id = await topoStore.saveSnapshot(snapshotName.value);
    snapshotName.value = "";
    alert(`拓扑快照已保存: ${id.slice(0, 8)}`);
  } catch (e: any) {
    alert(`保存失败: ${String(e)}`);
  }
}

async function loadSnapshotById(snapshotId: string) {
  await topoStore.loadSnapshot(snapshotId);
  peers.value = topoStore.nodes.map((n) => ({
    id: n.id,
    hostname: n.hostname,
    ip: n.ip,
    os: n.os || n.deviceType,
    listen_port: 0,
    last_seen: new Date().toISOString(),
    status: n.status === "online" ? "online" : "offline",
  }));
  initD3();
}

// ============= Auto Discovery =============
async function startDiscovery() {
  discovering.value = true;
  discoverProgress.value = null;
  discoveredLinks.value = [];
  try {
    await discoverTopology(subnet.value);
  } catch {
    discovering.value = false;
  }
}

function cancelDiscovery() {
  cancelTopologyDiscovery();
}

function handleDiscoverProgress(payload: DiscoverProgress) {
  discoverProgress.value = payload;
  if (payload.phase === "complete") {
    discovering.value = false;
  }
}

function handleDiscoverResult(payload: TopologyResult) {
  discovering.value = false;
  discoveredLinks.value = payload.links;

  const existingIps = new Set(peers.value.map((p) => p.ip));
  for (const node of payload.nodes) {
    if (!existingIps.has(node.ip)) {
      peers.value.push({
        id: `discovered-${node.ip}`,
        hostname: node.hostname || node.ip,
        ip: node.ip,
        os: "__discovered__",
        listen_port: 0,
        last_seen: new Date().toISOString(),
        status: "online",
      });
      existingIps.add(node.ip);
    }
  }
  initD3();
}

function handleDiscoverError() {
  discovering.value = false;
}

// Watch for layout algorithm changes from store (when loaded from snapshot)
watch(() => topoStore.layoutAlgorithm, (algo) => {
  if (simNodes.length > 0) switchLayout(algo);
});

onMounted(async () => {
  unlistenPeerList = await onPeerList(handlePeerList);
  unlistenPeerOffline = await onPeerOffline(handlePeerOffline);
  unlistenDiscoverProgress = await onTopologyProgress(handleDiscoverProgress);
  unlistenDiscoverResult = await onTopologyResult(handleDiscoverResult);
  unlistenDiscoverError = await onTopologyError(handleDiscoverError);
  await loadPeers();
  topoStore.loadSnapshots();
  await nextTick();
});

onUnmounted(() => {
  simulation?.stop();
  unlistenPeerList?.();
  unlistenPeerOffline?.();
  unlistenDiscoverProgress?.();
  unlistenDiscoverResult?.();
  unlistenDiscoverError?.();
});
</script>

<template>
  <div class="flex h-full flex-col animate-view-fade">
    <!-- Header -->
    <div class="shrink-0 px-6 pt-6 pb-4">
      <div class="flex items-center justify-between">
        <div>
          <h1 class="text-2xl font-display font-bold text-ink">网络拓扑</h1>
          <p class="mt-0.5 text-sm text-ink-faint">可视化局域网发现设备</p>
        </div>
        <button
          class="rounded-lg px-3 py-1.5 text-xs font-medium transition-colors"
          :class="showDiscoveryPanel ? 'bg-paper-deep text-ink' : 'bg-bamboo/10 text-bamboo hover:bg-bamboo/15'"
          @click="showDiscoveryPanel = !showDiscoveryPanel"
        >
          {{ showDiscoveryPanel ? '关闭面板' : '自动发现' }}
        </button>
      </div>

      <!-- Auto Discovery Panel -->
      <div
        v-if="showDiscoveryPanel"
        class="mt-4 rounded-xl border border-paper-deep/60 bg-paper/90 p-4 shadow-sm backdrop-blur"
      >
        <div class="flex items-end gap-3">
          <div class="flex-1">
            <label class="mb-1 block text-xs text-ink-faint">子网 (CIDR)</label>
            <input
              v-model="subnet"
              type="text"
              class="w-full rounded-lg border border-paper-deep/60 bg-paper-deep/50 px-3 py-1.5 text-xs font-mono text-ink outline-none transition-colors focus:border-bamboo/50"
              placeholder="192.168.1.0/24"
              :disabled="discovering"
            />
          </div>
          <button
            v-if="!discovering"
            class="rounded-lg bg-bamboo px-4 py-1.5 text-xs font-medium text-white transition-colors hover:bg-bamboo/90 disabled:opacity-50"
            @click="startDiscovery"
          >
            开始发现
          </button>
          <button
            v-else
            class="rounded-lg bg-red-500 px-4 py-1.5 text-xs font-medium text-white transition-colors hover:bg-red-600"
            @click="cancelDiscovery"
          >
            取消
          </button>
        </div>

        <!-- Progress -->
        <div v-if="discovering || discoverProgress" class="mt-3">
          <div class="flex items-center justify-between text-xs text-ink-faint">
            <span>{{ discoverProgress?.message || '准备中...' }}</span>
            <span>{{ discoverProgress ? discoverProgress.progress.toFixed(0) : '0' }}%</span>
          </div>
          <div class="mt-1 h-1.5 w-full overflow-hidden rounded-full bg-paper-deep">
            <div
              class="h-full rounded-full bg-bamboo transition-all duration-300 ease-out"
              :style="{ width: (discoverProgress?.progress || 0) + '%' }"
            />
          </div>
          <div class="mt-1 text-xs text-ink-faint">
            <template v-if="discoverProgress?.phase === 'scan'">
              扫描主机中，已发现 {{ discoverProgress.nodesFound }} 个节点
            </template>
            <template v-else-if="discoverProgress?.phase === 'trace'">
              测量延迟中，{{ discoverProgress.currentIp }}
            </template>
            <template v-else-if="discoverProgress?.phase === 'complete'">
              发现完成！
            </template>
          </div>
        </div>
      </div>
    </div>

    <!-- Search & Controls Bar -->
    <div class="shrink-0 px-6 pb-2">
      <div class="mt-1 flex items-center gap-2">
        <!-- Search -->
        <div class="relative flex-1">
          <input
            v-model="topoStore.searchQuery"
            type="text"
            placeholder="搜索 IP / 主机名 / 厂商..."
            class="w-full rounded-lg border border-paper-deep/60 bg-paper-deep/50 px-3 py-1.5 pl-8 text-xs text-ink outline-none transition-colors focus:border-bamboo/50"
          />
          <svg class="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-ink-faint" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
          </svg>
        </div>

        <!-- Layout selector -->
        <select
          v-model="topoStore.layoutAlgorithm"
          @change="switchLayout(topoStore.layoutAlgorithm)"
          class="rounded-lg border border-paper-deep/60 bg-paper-deep/50 px-2 py-1.5 text-xs text-ink outline-none"
        >
          <option v-for="algo in algorithmOptions" :key="algo.value" :value="algo.value">
            {{ algo.label }}
          </option>
        </select>

        <!-- Snapshot button -->
        <button
          class="rounded-lg px-3 py-1.5 text-xs font-medium transition-colors"
          :class="showSnapshotPanel ? 'bg-paper-deep text-ink' : 'bg-paper-deep/30 text-ink-faint hover:bg-paper-deep/60'"
          @click="showSnapshotPanel = !showSnapshotPanel"
        >
          快照
        </button>
      </div>

      <!-- Snapshot Panel -->
      <div v-if="showSnapshotPanel" class="mt-2 rounded-xl border border-paper-deep/60 bg-paper/90 p-3 shadow-sm backdrop-blur">
        <div class="mb-2 flex gap-2">
          <input
            v-model="snapshotName"
            type="text"
            placeholder="快照名称"
            class="flex-1 rounded-lg border border-paper-deep/60 bg-paper-deep/50 px-2 py-1.5 text-xs text-ink outline-none focus:border-bamboo/50"
          />
          <button
            class="rounded-lg bg-bamboo px-3 py-1.5 text-xs font-medium text-white hover:bg-bamboo/90 disabled:opacity-50"
            :disabled="!snapshotName"
            @click="saveCurrentSnapshot"
          >
            保存
          </button>
        </div>
        <div v-if="topoStore.snapshots.length > 0" class="mt-2 max-h-40 overflow-y-auto">
          <div
            v-for="snap in topoStore.snapshots"
            :key="snap.id"
            class="flex items-center justify-between rounded-lg px-2 py-1.5 hover:bg-paper-deep/50"
          >
            <div>
              <span class="text-xs text-ink">{{ snap.name }}</span>
              <span class="ml-2 text-[10px] text-ink-faint">{{ new Date(snap.createdAt).toLocaleString() }}</span>
            </div>
            <div class="flex gap-1">
              <button class="rounded px-2 py-0.5 text-[10px] text-bamboo hover:bg-bamboo/10" @click="loadSnapshotById(snap.id)">加载</button>
              <button class="rounded px-2 py-0.5 text-[10px] text-red-500 hover:bg-red-50" @click="topoStore.deleteSnapshot(snap.id)">删除</button>
            </div>
          </div>
        </div>
        <div v-else class="py-2 text-center text-xs text-ink-faint">暂无快照</div>
      </div>

      <!-- Filter chips -->
      <div v-if="topoStore.nodes.length > 0" class="mt-2 flex flex-wrap gap-1">
        <button
          v-for="type in ['router', 'switch', 'server', 'firewall', 'camera', 'other']"
          :key="type"
          class="rounded-full px-2 py-0.5 text-[10px] transition-colors"
          :class="topoStore.deviceTypeFilter.includes(type)
            ? 'bg-bamboo/20 text-bamboo'
            : 'bg-paper-deep/30 text-ink-faint hover:bg-paper-deep/60'"
          @click="topoStore.toggleDeviceTypeFilter(type)"
        >
          {{ {router:'路由器', switch:'交换机', server:'服务器', firewall:'防火墙', camera:'摄像头', other:'其他'}[type] }}
        </button>
      </div>
    </div>

    <!-- SVG Canvas -->
    <div class="relative flex-1 overflow-hidden">
      <svg
        ref="svgRef"
        class="h-full w-full"
        style="cursor: grab;"
        @click.self="closeDetail"
      ></svg>

      <!-- Zoom controls -->
      <div
        class="absolute bottom-4 right-4 flex items-center gap-2 rounded-xl border border-paper-deep/60 bg-paper/90 px-3 py-2 shadow-sm backdrop-blur"
      >
        <button
          class="rounded-lg px-2 py-1 text-xs text-ink-soft transition-colors hover:bg-paper-deep hover:text-ink"
          @click="d3.select(svgRef!).transition().duration(300).call(d3.zoom().transform as any, d3.zoomIdentity)"
        >
          重置
        </button>
      </div>

      <!-- Empty state -->
      <div
        v-if="peers.length === 0"
        class="absolute inset-0 flex items-center justify-center pointer-events-none"
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
            <span class="text-xs text-ink-faint">{{ detailPopup.peer.os === '__discovered__' ? '发现方式' : '操作系统' }}</span>
            <span class="text-xs text-ink">{{ detailPopup.peer.os === '__discovered__' ? 'Ping 扫描' : detailPopup.peer.os }}</span>
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
            @click="goToChat"
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
          <span class="inline-block h-2.5 w-2.5 rounded-sm bg-purple-600" />
          路由器
        </span>
        <span class="flex items-center gap-1.5">
          <span class="inline-block h-2.5 w-2.5 rounded bg-cyan-600" />
          交换机
        </span>
        <span class="flex items-center gap-1.5">
          <span class="inline-block h-2.5 w-2.5 rounded-full bg-blue-600" />
          服务器
        </span>
        <span class="flex items-center gap-1.5">
          <span class="inline-block h-2.5 w-2.5 rounded-full bg-slate-600" />
          其他
        </span>
        <span class="mx-1 h-4 w-px bg-paper-deep" />
        <span class="flex items-center gap-1.5">
          <span class="inline-block h-2.5 w-2.5 rounded-full bg-blue-500" />
          自动发现节点
        </span>
        <span class="flex items-center gap-1.5">
          <span class="inline-block h-px w-6 bg-slate-400/20" />
          子网连接
        </span>
        <span class="flex items-center gap-1.5">
          <span class="inline-block h-0.5 w-6 border-t-2 border-dashed border-blue-400/40" />
          发现连接
        </span>
        <span class="ml-auto">滚轮缩放 | 空白拖拽平移 | 拖拽节点移动</span>
      </div>
    </div>
  </div>
</template>
