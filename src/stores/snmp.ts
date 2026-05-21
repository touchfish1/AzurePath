import { defineStore } from "pinia";
import { ref } from "vue";
import {
  snmpDiscover,
  snmpListDevices,
  snmpDeleteDevice,
  snmpGetInterfaces,
  snmpGetArpTable,
  snmpGetHistory,
  snmpStartCollect,
  snmpStopCollect,
  onSnmpProgress,
  onSnmpSample,
  type SnmpDevice,
  type SnmpInterface,
  type SnmpSample,
  type SnmpArpEntry,
  type SnmpDiscoverProgress,
} from "@/lib/tauri";

export const useSnmpStore = defineStore("snmp", () => {
  const devices = ref<SnmpDevice[]>([]);
  const interfaces = ref<SnmpInterface[]>([]);
  const arpTable = ref<SnmpArpEntry[]>([]);
  const samples = ref<SnmpSample[]>([]);
  const discoverProgress = ref<SnmpDiscoverProgress | null>(null);
  const isLoading = ref(false);
  const isCollecting = ref(false);

  async function loadDevices() {
    isLoading.value = true;
    try {
      devices.value = await snmpListDevices();
    } finally {
      isLoading.value = false;
    }
  }

  async function discover(cidr: string, community: string) {
    const unlisten = await onSnmpProgress((p) => {
      discoverProgress.value = p;
    });
    try {
      const result = await snmpDiscover(cidr, community);
      devices.value = result;
      return result;
    } finally {
      unlisten();
      discoverProgress.value = null;
    }
  }

  async function deleteDevice(id: string) {
    await snmpDeleteDevice(id);
    devices.value = devices.value.filter((d) => d.id !== id);
  }

  async function fetchInterfaces(host: string, community: string) {
    interfaces.value = await snmpGetInterfaces(host, community);
  }

  async function fetchArpTable(host: string, community: string) {
    arpTable.value = await snmpGetArpTable(host, community);
  }

  async function startCollection(host: string, community: string) {
    isCollecting.value = true;
    const unlisten = await onSnmpSample((sample) => {
      samples.value.push(sample);
      if (samples.value.length > 1000) {
        samples.value = samples.value.slice(-500);
      }
    });
    await snmpStartCollect(host, community);
  }

  async function stopCollection(host: string) {
    await snmpStopCollect(host);
    isCollecting.value = false;
  }

  return {
    devices,
    interfaces,
    arpTable,
    samples,
    discoverProgress,
    isLoading,
    isCollecting,
    loadDevices,
    discover,
    deleteDevice,
    fetchInterfaces,
    fetchArpTable,
    startCollection,
    stopCollection,
  };
});
