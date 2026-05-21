// src/stores/topology.ts

import { defineStore } from "pinia";
import { ref, computed } from "vue";
import {
  computeTopologyLayout,
  topologySaveSnapshot,
  topologyListSnapshots,
  topologyLoadSnapshot,
  topologyDeleteSnapshot,
  topologyCompareSnapshots,
  type TopologyNode,
  type TopologyLink,
  type TopologySnapshot,
  type SnapshotDetail,
  type SnapshotDiff,
} from "@/lib/tauri";

export const useTopologyStore = defineStore("topology", () => {
  // Device nodes
  const nodes = ref<TopologyNode[]>([]);
  const links = ref<TopologyLink[]>([]);
  const selectedNodeId = ref<string | null>(null);

  // Layout
  const layoutAlgorithm = ref("forceDirected");
  const canvasWidth = ref(800);
  const canvasHeight = ref(600);

  // Search & filter
  const searchQuery = ref("");
  const deviceTypeFilter = ref<string[]>([]);
  const statusFilter = ref<string[]>([]);

  const filteredNodes = computed(() => {
    let result = nodes.value;
    if (searchQuery.value) {
      const q = searchQuery.value.toLowerCase();
      result = result.filter(
        (n) =>
          n.ip.toLowerCase().includes(q) ||
          n.hostname.toLowerCase().includes(q) ||
          n.vendor.toLowerCase().includes(q) ||
          n.model.toLowerCase().includes(q),
      );
    }
    if (deviceTypeFilter.value.length > 0) {
      result = result.filter((n) => deviceTypeFilter.value.includes(n.deviceType));
    }
    if (statusFilter.value.length > 0) {
      result = result.filter((n) => statusFilter.value.includes(n.status));
    }
    return result;
  });

  // Snapshots
  const snapshots = ref<TopologySnapshot[]>([]);
  const isLoadingSnapshots = ref(false);

  async function computeLayout() {
    const linkPairs: [string, string][] = links.value.map((l) => [l.sourceId, l.targetId]);
    try {
      const updated = await computeTopologyLayout(
        nodes.value,
        linkPairs,
        layoutAlgorithm.value,
        canvasWidth.value,
        canvasHeight.value,
      );
      nodes.value = updated;
    } catch (e) {
      console.error("Layout computation failed:", e);
    }
  }

  async function loadSnapshots() {
    isLoadingSnapshots.value = true;
    try {
      snapshots.value = await topologyListSnapshots();
    } finally {
      isLoadingSnapshots.value = false;
    }
  }

  async function saveSnapshot(name: string) {
    const id = await topologySaveSnapshot(name, layoutAlgorithm.value, nodes.value, links.value);
    await loadSnapshots();
    return id;
  }

  async function loadSnapshot(id: string) {
    const detail: SnapshotDetail = await topologyLoadSnapshot(id);
    nodes.value = detail.nodes;
    links.value = detail.links;
    layoutAlgorithm.value = detail.layoutAlgorithm;
  }

  async function deleteSnapshot(id: string) {
    await topologyDeleteSnapshot(id);
    await loadSnapshots();
  }

  async function compareSnapshots(idA: string, idB: string): Promise<SnapshotDiff> {
    return await topologyCompareSnapshots(idA, idB);
  }

  function updateNodePosition(nodeId: string, x: number, y: number) {
    const node = nodes.value.find((n) => n.id === nodeId);
    if (node) {
      node.x = x;
      node.y = y;
    }
  }

  function toggleDeviceTypeFilter(type: string) {
    const idx = deviceTypeFilter.value.indexOf(type);
    if (idx >= 0) {
      deviceTypeFilter.value.splice(idx, 1);
    } else {
      deviceTypeFilter.value.push(type);
    }
  }

  return {
    nodes,
    links,
    selectedNodeId,
    layoutAlgorithm,
    canvasWidth,
    canvasHeight,
    searchQuery,
    deviceTypeFilter,
    statusFilter,
    filteredNodes,
    snapshots,
    isLoadingSnapshots,
    computeLayout,
    loadSnapshots,
    saveSnapshot,
    loadSnapshot,
    deleteSnapshot,
    compareSnapshots,
    updateNodePosition,
    toggleDeviceTypeFilter,
  };
});
