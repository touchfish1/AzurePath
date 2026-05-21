<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useSnmpStore } from "@/stores/snmp";
import { useToastStore } from "@/stores/toast";
import type { SnmpDevice } from "@/lib/tauri";

const store = useSnmpStore();
const toast = useToastStore();

const cidr = ref("192.168.1.0/24");
const community = ref("public");
const selectedDevice = ref<SnmpDevice | null>(null);
const activeTab = ref<"interfaces" | "arp" | "traffic">("interfaces");
const isScanning = ref(false);
const intervalSecs = ref(10);

onMounted(() => {
  store.loadDevices();
});

async function startScan() {
  isScanning.value = true;
  try {
    await store.discover(cidr.value, community.value);
    toast.add("success", `发现 ${store.devices.length} 个设备`);
  } catch (e: any) {
    toast.error(String(e));
  } finally {
    isScanning.value = false;
  }
}

async function selectDevice(device: SnmpDevice) {
  selectedDevice.value = device;
  activeTab.value = "interfaces";
  store.fetchInterfaces(device.ip, device.community);
  store.fetchArpTable(device.ip, device.community);
}

async function handleDelete(id: string) {
  await store.deleteDevice(id);
  if (selectedDevice.value?.id === id) selectedDevice.value = null;
  toast.add("info", "设备已删除");
}

async function toggleCollection() {
  if (!selectedDevice.value) return;
  if (store.isCollecting) {
    await store.stopCollection(selectedDevice.value.ip);
  } else {
    await store.startCollection(selectedDevice.value.ip, selectedDevice.value.community);
  }
}

function switchTab(tab: "interfaces" | "arp" | "traffic") {
  activeTab.value = tab;
  if (!selectedDevice.value) return;
  if (tab === "arp") {
    store.fetchArpTable(selectedDevice.value.ip, selectedDevice.value.community);
  }
}
</script>

<template>
  <div class="flex h-full gap-4 p-4">
    <!-- Left panel: device list + scanner -->
    <div class="flex w-80 shrink-0 flex-col gap-3">
      <!-- Scanner -->
      <div class="rounded-xl border border-paper-deep/60 bg-paper p-3">
        <h3 class="mb-2 text-sm font-semibold text-ink">SNMP 扫描</h3>
        <div class="flex flex-col gap-2">
          <div class="flex gap-2">
            <input v-model="cidr" type="text" placeholder="192.168.1.0/24"
              class="flex-1 rounded-lg border border-paper-deep/60 bg-paper-warm/50 px-2 py-1.5 text-xs text-ink outline-none focus:border-bamboo/50" />
            <input v-model="community" type="text" placeholder="public"
              class="w-20 rounded-lg border border-paper-deep/60 bg-paper-warm/50 px-2 py-1.5 text-xs text-ink outline-none focus:border-bamboo/50" />
          </div>
          <button @click="startScan" :disabled="isScanning"
            class="rounded-lg bg-bamboo px-3 py-1.5 text-xs font-medium text-white transition-colors hover:bg-bamboo/90 disabled:opacity-50">
            {{ isScanning ? '扫描中...' : '扫描' }}
          </button>
          <!-- Progress -->
          <div v-if="store.discoverProgress" class="text-xs text-ink-faint">
            已扫描 {{ store.discoverProgress.scanned }}/{{ store.discoverProgress.total }}，发现 {{ store.discoverProgress.found }} 个设备
          </div>
        </div>
      </div>

      <!-- Device list -->
      <div class="flex-1 overflow-y-auto rounded-xl border border-paper-deep/60 bg-paper p-2">
        <div v-if="store.devices.length === 0" class="py-8 text-center text-xs text-ink-faint">
          暂无设备，请先扫描
        </div>
        <div v-for="device in store.devices" :key="device.id"
          @click="selectDevice(device)"
          class="cursor-pointer rounded-lg p-2.5 transition-colors hover:bg-paper-deep/50"
          :class="{ 'bg-bamboo/10': selectedDevice?.id === device.id }">
          <div class="flex items-center justify-between">
            <span class="text-xs font-medium text-ink">{{ device.hostname || device.ip }}</span>
            <span class="rounded bg-paper-deep/40 px-1.5 py-0.5 text-[10px] text-ink-faint">{{ device.vendor }}</span>
          </div>
          <div class="mt-0.5 text-[10px] text-ink-faint">{{ device.ip }} · {{ device.model }}</div>
        </div>
      </div>
    </div>

    <!-- Right panel: device detail -->
    <div v-if="selectedDevice" class="flex-1 rounded-xl border border-paper-deep/60 bg-paper p-4">
      <!-- Device header -->
      <div class="mb-4 flex items-center justify-between">
        <div>
          <h2 class="text-base font-semibold text-ink">{{ selectedDevice.hostname || selectedDevice.ip }}</h2>
          <p class="text-xs text-ink-faint">{{ selectedDevice.vendor }} {{ selectedDevice.model }} · {{ selectedDevice.ip }}</p>
        </div>
        <div class="flex items-center gap-2">
          <button @click="toggleCollection"
            class="rounded-lg px-3 py-1.5 text-xs font-medium transition-colors"
            :class="store.isCollecting
              ? 'bg-red-50 text-red-600 hover:bg-red-100 dark:bg-red-900/20 dark:text-red-400'
              : 'bg-bamboo/10 text-bamboo hover:bg-bamboo/20'">
            {{ store.isCollecting ? '停止采集' : '开始采集' }}
          </button>
          <button @click="handleDelete(selectedDevice.id)"
            class="rounded-lg px-3 py-1.5 text-xs font-medium text-red-500 transition-colors hover:bg-red-50 dark:hover:bg-red-900/20">
            删除
          </button>
        </div>
      </div>

      <!-- Tabs -->
      <div class="mb-4 flex gap-1 border-b border-paper-deep/60">
        <button v-for="tab in ([{k:'interfaces',l:'接口'},{k:'arp',l:'ARP表'},{k:'traffic',l:'流量'}])" :key="tab.k"
          @click="switchTab(tab.k as any)"
          class="border-b-2 px-3 py-2 text-xs font-medium transition-colors"
          :class="activeTab === tab.k
            ? 'border-bamboo text-bamboo'
            : 'border-transparent text-ink-faint hover:text-ink'">
          {{ tab.l }}
        </button>
      </div>

      <!-- Interfaces tab -->
      <div v-if="activeTab === 'interfaces'" class="overflow-y-auto" style="max-height: calc(100vh - 320px)">
        <table class="w-full text-xs">
          <thead>
            <tr class="text-ink-faint">
              <th class="px-2 py-1 text-left">接口</th>
              <th class="px-2 py-1 text-left">MAC</th>
              <th class="px-2 py-1 text-right">速率</th>
              <th class="px-2 py-1 text-center">状态</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="iface in store.interfaces" :key="iface.index"
              class="border-t border-paper-deep/30">
              <td class="px-2 py-1.5 font-medium text-ink">{{ iface.name }}</td>
              <td class="px-2 py-1.5 font-mono text-ink-faint">{{ iface.mac }}</td>
              <td class="px-2 py-1.5 text-right text-ink-faint">
                {{ iface.speed >= 1_000_000_000 ? (iface.speed / 1_000_000_000).toFixed(1) + ' Gbps' : (iface.speed / 1_000_000).toFixed(0) + ' Mbps' }}
              </td>
              <td class="px-2 py-1.5 text-center">
                <span class="inline-block h-2 w-2 rounded-full"
                  :class="iface.operStatus === 1 ? 'bg-green-500' : 'bg-red-500'"></span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <!-- ARP tab -->
      <div v-if="activeTab === 'arp'" class="overflow-y-auto" style="max-height: calc(100vh - 320px)">
        <table class="w-full text-xs">
          <thead>
            <tr class="text-ink-faint">
              <th class="px-2 py-1 text-left">IP</th>
              <th class="px-2 py-1 text-left">MAC</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(entry, i) in store.arpTable" :key="i"
              class="border-t border-paper-deep/30">
              <td class="px-2 py-1.5 font-mono text-ink">{{ entry.ip }}</td>
              <td class="px-2 py-1.5 font-mono text-ink-faint">{{ entry.mac }}</td>
            </tr>
          </tbody>
        </table>
      </div>

      <!-- Traffic tab -->
      <div v-if="activeTab === 'traffic'" class="flex flex-col items-center justify-center py-12 text-ink-faint">
        <p class="text-sm">流量趋势图</p>
        <p class="mt-1 text-xs">（启动采集后显示数据）</p>
        <div v-if="store.samples.length > 0" class="mt-4 text-xs text-ink">
          已采集 {{ store.samples.length }} 个样本
        </div>
      </div>
    </div>

    <!-- Empty state -->
    <div v-else class="flex flex-1 items-center justify-center text-sm text-ink-faint">
      选择一个设备查看详情
    </div>
  </div>
</template>
