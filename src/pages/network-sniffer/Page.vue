<script setup lang="ts">
import { ref, computed, onUnmounted } from "vue";
import {
  Play,
  Square,
  Radio,
  Search,
  Download,
  ChevronDown,
} from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import PortModal from "@/components/network-sniffer/PortModal.vue";
import {
  snifferStart,
  snifferStop,
  snifferPresets,
  snifferExport,
  onSnifferProgress,
  onSnifferDevice,
  onSnifferPort,
  onSnifferComplete,
  onSnifferError,
  type DeviceResult,
  type SnifferProgress,
  type PortPreset,
} from "@/lib/tauri";
import type { UnlistenFn } from "@tauri-apps/api/event";

const targets = ref("192.168.1.0/24");
const scanMode = ref<"fast" | "deep">("fast");
const selectedPorts = ref<number[]>([]);
const scanState = ref<"idle" | "scanning" | "completed" | "error">("idle");
const taskId = ref<string | null>(null);
const devices = ref<DeviceResult[]>([]);
const progress = ref<SnifferProgress | null>(null);
const errorMsg = ref("");
const filterService = ref("all");
const searchQuery = ref("");
const showPortModal = ref(false);
const concurrencyHosts = ref(10);
const concurrencyPorts = ref(50);
const presets = ref<PortPreset[]>([]);
const activePreset = ref("top100");
const expandedDevices = ref<Set<string>>(new Set());

const unlisteners: UnlistenFn[] = [];

// Load presets on mount
snifferPresets().then((p) => {
  presets.value = p;
});

function selectPreset(name: string) {
  activePreset.value = name;
  const preset = presets.value.find((p) => p.name === name);
  if (preset) {
    selectedPorts.value = [...preset.ports];
  }
}

const serviceOptions = computed(() => {
  const services = new Set<string>();
  for (const d of devices.value) {
    for (const p of d.openPorts) {
      if (p.service) services.add(p.service);
    }
  }
  return ["all", ...Array.from(services).sort()];
});

const filteredDevices = computed(() => {
  return devices.value.filter((d) => {
    if (searchQuery.value) {
      const q = searchQuery.value.toLowerCase();
      if (
        !d.ip.toLowerCase().includes(q) &&
        !d.hostname?.toLowerCase().includes(q) &&
        !d.mac?.toLowerCase().includes(q)
      ) {
        return false;
      }
    }
    if (filterService.value !== "all") {
      if (!d.openPorts.some((p) => p.service === filterService.value)) {
        return false;
      }
    }
    return true;
  });
});

const summary = computed(() => {
  const totalPorts = devices.value.reduce((s, d) => s + d.openPorts.length, 0);
  const totalServices = devices.value.reduce(
    (s, d) => s + d.openPorts.filter((p) => p.service).length,
    0
  );
  return { hosts: devices.value.length, ports: totalPorts, services: totalServices };
});

function toggleDevice(ip: string) {
  if (expandedDevices.value.has(ip)) {
    expandedDevices.value.delete(ip);
  } else {
    expandedDevices.value.add(ip);
  }
}

function serviceClass(service: string | null): string {
  const map: Record<string, string> = {
    HTTP: "tag-http",
    HTTPS: "tag-https",
    "HTTP-Alt": "tag-http",
    "HTTPS-Alt": "tag-https",
    SSH: "tag-ssh",
    MySQL: "tag-mysql",
    PostgreSQL: "tag-mysql",
    Redis: "tag-redis",
    SMB: "tag-smb",
    DNS: "tag-dns",
  };
  return map[service || ""] || "tag-unknown";
}

async function startScan() {
  scanState.value = "scanning";
  errorMsg.value = "";
  devices.value = [];
  progress.value = null;

  try {
    const options = {
      targets: targets.value.split(",").map((s) => s.trim()).filter(Boolean),
      ports: selectedPorts.value.length > 0 ? selectedPorts.value : [],
      mode: scanMode.value,
      concurrencyHosts: concurrencyHosts.value,
      concurrencyPorts: concurrencyPorts.value,
      timeoutMs: 1000,
      probeServices: scanMode.value === "deep",
    };

    const unlistenProgress = await onSnifferProgress((p) => {
      progress.value = p;
    });
    unlisteners.push(unlistenProgress);

    const unlistenDevice = await onSnifferDevice((d) => {
      devices.value = [...devices.value.filter((x) => x.ip !== d.ip), d];
    });
    unlisteners.push(unlistenDevice);

    const unlistenPort = await onSnifferPort(() => {});
    unlisteners.push(unlistenPort);

    const unlistenComplete = await onSnifferComplete(() => {
      scanState.value = "completed";
    });
    unlisteners.push(unlistenComplete);

    const unlistenError = await onSnifferError((e) => {
      errorMsg.value = e.error;
      scanState.value = "error";
    });
    unlisteners.push(unlistenError);

    const tid = await snifferStart(options);
    taskId.value = tid;
  } catch (e: any) {
    errorMsg.value = typeof e === "string" ? e : "启动扫描失败";
    scanState.value = "error";
  }
}

async function stopScan() {
  if (taskId.value) {
    try {
      await snifferStop(taskId.value);
    } catch (e) {
      console.error("停止扫描失败:", e);
    }
  }
  scanState.value = "idle";
}

async function exportResults(format: "json" | "csv") {
  if (!taskId.value) return;
  try {
    const data = await snifferExport(taskId.value, format);
    const ext = format === "json" ? "json" : "csv";
    const blob = new Blob([data], { type: "text/plain;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `sniffer-results-${taskId.value.slice(0, 8)}.${ext}`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  } catch (e) {
    console.error("导出失败:", e);
  }
}

function cleanup() {
  for (const fn of unlisteners) {
    fn();
  }
  unlisteners.length = 0;
}

onUnmounted(cleanup);
</script>

<template>
  <div class="flex h-full flex-col animate-view-fade">
    <!-- Header -->
    <div class="border-b border-paper-deep/50 px-6 py-3">
      <h1 class="text-xl font-display font-bold text-ink">网络嗅探</h1>
      <p class="text-xs text-ink-faint">扫描局域网设备，检测开放端口与运行服务</p>
    </div>

    <div class="flex-1 overflow-y-auto p-4 space-y-4">
      <!-- Scan config card -->
      <div class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm">
        <!-- Target row -->
        <div class="flex flex-wrap items-center gap-3">
          <input
            v-model="targets"
            placeholder="IP / CIDR (e.g. 192.168.1.0/24)"
            :disabled="scanState === 'scanning'"
            class="flex-1 min-w-[200px] rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 font-mono text-sm text-ink outline-none transition-colors focus:border-bamboo/50 disabled:opacity-50"
          />

          <!-- Mode toggle -->
          <div class="flex gap-1 rounded-lg bg-paper-deep/30 p-0.5">
            <button
              class="rounded-md px-3 py-1.5 text-xs font-medium transition-all"
              :class="
                scanMode === 'fast'
                  ? 'bg-paper text-ink shadow-sm'
                  : 'text-ink-faint hover:text-ink'
              "
              :disabled="scanState === 'scanning'"
              @click="scanMode = 'fast'"
            >
              快速扫描
              <span class="block text-[10px] font-normal text-ink-faint/70">TOP 100 端口</span>
            </button>
            <button
              class="rounded-md px-3 py-1.5 text-xs font-medium transition-all"
              :class="
                scanMode === 'deep'
                  ? 'bg-paper text-ink shadow-sm'
                  : 'text-ink-faint hover:text-ink'
              "
              :disabled="scanState === 'scanning'"
              @click="scanMode = 'deep'"
            >
              深度扫描
              <span class="block text-[10px] font-normal text-ink-faint/70">服务指纹</span>
            </button>
          </div>

          <!-- Buttons -->
          <Button
            :disabled="scanState === 'scanning'"
            @click="startScan"
          >
            <Play class="mr-1.5 h-3.5 w-3.5" />
            开始扫描
          </Button>
          <Button
            variant="danger"
            :disabled="scanState !== 'scanning'"
            @click="stopScan"
          >
            <Square class="mr-1.5 h-3.5 w-3.5" />
            停止
          </Button>
        </div>

        <!-- Port selection -->
        <div class="mt-3 flex flex-wrap items-center gap-2">
          <div class="flex flex-wrap gap-1">
            <button
              v-for="preset in presets"
              :key="preset.name"
              class="rounded-md border px-2.5 py-1 text-xs transition-all"
              :class="
                activePreset === preset.name && selectedPorts.length > 0
                  ? 'border-bamboo/40 bg-bamboo/5 text-bamboo font-medium'
                  : 'border-paper-deep text-ink-faint hover:border-paper-deep hover:text-ink'
              "
              :disabled="scanState === 'scanning'"
              @click="selectPreset(preset.name)"
            >
              {{ preset.label }}
            </button>
          </div>

          <!-- Custom port tags -->
          <div class="flex flex-wrap gap-1">
            <span
              v-for="port in selectedPorts.slice(0, 15)"
              :key="port"
              class="inline-flex items-center gap-1 rounded-md border border-bamboo/20 bg-bamboo/5 px-2 py-0.5 font-mono text-xs text-bamboo"
            >
              {{ port }}
              <button
                class="text-bamboo/50 hover:text-bamboo"
                :disabled="scanState === 'scanning'"
                @click="
                  selectedPorts = selectedPorts.filter((p) => p !== port)
                "
              >
                ×
              </button>
            </span>
            <span
              v-if="selectedPorts.length > 15"
              class="text-xs text-ink-faint"
            >
              +{{ selectedPorts.length - 15 }}
            </span>
          </div>

          <button
            class="rounded-md border border-dashed border-paper-deep px-2.5 py-1 text-xs text-ink-faint transition-colors hover:border-bamboo/50 hover:text-bamboo"
            :disabled="scanState === 'scanning'"
            @click="showPortModal = true"
          >
            + 自定义端口
          </button>
        </div>

        <!-- Concurrency settings -->
        <div class="mt-3 flex flex-wrap items-center gap-4 border-t border-paper-deep/30 pt-3">
          <div class="flex items-center gap-2">
            <label class="text-xs text-ink-faint whitespace-nowrap">并发主机:</label>
            <input
              v-model.number="concurrencyHosts"
              type="number"
              min="1"
              max="100"
              :disabled="scanState === 'scanning'"
              class="w-16 rounded-md border border-paper-deep bg-paper-warm/50 px-2 py-1 text-xs font-mono text-ink outline-none transition-colors focus:border-bamboo/50 disabled:opacity-50"
            />
          </div>
          <div class="flex items-center gap-2">
            <label class="text-xs text-ink-faint whitespace-nowrap">并发端口:</label>
            <input
              v-model.number="concurrencyPorts"
              type="number"
              min="1"
              max="500"
              :disabled="scanState === 'scanning'"
              class="w-16 rounded-md border border-paper-deep bg-paper-warm/50 px-2 py-1 text-xs font-mono text-ink outline-none transition-colors focus:border-bamboo/50 disabled:opacity-50"
            />
          </div>
          <span class="text-[10px] text-ink-faint/60">并发数越高扫描越快，但可能增加网络负载</span>
        </div>

        <!-- Progress -->
        <div
          v-if="progress && scanState === 'scanning'"
          class="mt-4 flex items-center gap-3 border-t border-paper-deep/30 pt-3"
        >
          <span class="inline-flex items-center gap-1.5 rounded-full bg-bamboo/10 px-3 py-1 text-xs font-medium text-bamboo">
            <span class="h-1.5 w-1.5 animate-pulse rounded-full bg-bamboo" />
            扫描中{{ progress.currentTarget ? ' (' + progress.currentTarget + ')' : '' }}
          </span>
          <div class="flex-1 h-1.5 rounded-full bg-paper-deep/50 overflow-hidden">
            <div
              class="h-full rounded-full bg-bamboo transition-all duration-300"
              :style="{ width: progress.totalHosts > 0 ? (progress.scannedHosts / progress.totalHosts * 100) + '%' : '0%' }"
            />
          </div>
          <span class="shrink-0 text-xs text-ink-faint font-mono">
            {{ progress.scannedHosts }}/{{ progress.totalHosts }} · {{ progress.servicesFound }} 服务
          </span>
        </div>
      </div>

      <!-- Error banner -->
      <div
        v-if="errorMsg"
        class="rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-700"
      >
        {{ errorMsg }}
      </div>

      <!-- Results -->
      <div
        v-if="devices.length > 0"
        class="noise-bg rounded-xl border border-paper-deep/60 bg-paper shadow-sm"
      >
        <!-- Toolbar -->
        <div class="flex flex-wrap items-center justify-between gap-2 border-b border-paper-deep/50 px-5 py-3">
          <h2 class="text-sm font-semibold text-ink">扫描结果</h2>
          <div class="flex items-center gap-2">
            <select
              v-model="filterService"
              class="rounded-lg border border-paper-deep bg-paper-warm/50 px-2.5 py-1 text-xs text-ink outline-none"
            >
              <option value="all">全部服务</option>
              <option v-for="s in serviceOptions.filter((x) => x !== 'all')" :key="s" :value="s">
                {{ s }}
              </option>
            </select>
            <div class="relative">
              <Search class="absolute left-2 top-1/2 h-3 w-3 -translate-y-1/2 text-ink-faint" />
              <input
                v-model="searchQuery"
                placeholder="搜索 IP / 主机名"
                class="w-40 rounded-lg border border-paper-deep bg-paper-warm/50 py-1 pl-7 pr-2 text-xs text-ink outline-none"
              />
            </div>
            <Button variant="ghost" size="sm" @click="exportResults('json')">
              <Download class="mr-1 h-3 w-3" />
              导出
            </Button>
          </div>
        </div>

        <!-- Device list -->
        <div class="p-4 space-y-3">
          <div
            v-for="device in filteredDevices"
            :key="device.ip"
            class="rounded-xl border border-paper-deep/20 bg-paper-warm/30"
          >
            <!-- Device header -->
            <div
              class="flex cursor-pointer items-center gap-3 px-4 py-3 text-sm transition-colors hover:bg-paper-warm/50"
              @click="toggleDevice(device.ip)"
            >
              <ChevronDown
                class="h-3.5 w-3.5 text-ink-faint transition-transform"
                :class="{ '-rotate-90': !expandedDevices.has(device.ip) }"
              />
              <span class="font-mono font-semibold text-ink">{{ device.ip }}</span>
              <span v-if="device.hostname" class="text-ink-faint truncate max-w-[160px]">
                {{ device.hostname }}
              </span>
              <span
                class="rounded px-1.5 py-0.5 text-[10px] font-semibold uppercase"
                :class="device.scanMode === 'deep' ? 'bg-blue-100 text-blue-700' : 'bg-green-100 text-green-700'"
              >
                {{ device.scanMode }}
              </span>
              <span v-if="device.os" class="ml-auto text-xs text-ink-soft">{{ device.os }}</span>
              <span class="text-xs text-ink-faint">
                {{ device.openPorts.length }} 端口
              </span>
            </div>

            <!-- Port table -->
            <div v-if="expandedDevices.has(device.ip)" class="overflow-x-auto border-t border-paper-deep/20 animate-slide-up">
              <table class="w-full text-xs">
                <thead>
                  <tr class="border-b border-paper-deep/20 text-ink-faint uppercase tracking-wider">
                    <th class="px-4 py-2.5 text-left font-medium">端口</th>
                    <th class="px-4 py-2.5 text-left font-medium">协议</th>
                    <th class="px-4 py-2.5 text-left font-medium">服务</th>
                    <th class="px-4 py-2.5 text-left font-medium">版本</th>
                    <th class="px-4 py-2.5 text-left font-medium max-w-[200px]">Banner</th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="port in device.openPorts"
                    :key="port.port"
                    class="border-b border-paper-deep/10 last:border-0 hover:bg-paper-warm/30"
                  >
                    <td class="px-4 py-2 font-mono font-medium text-ink">{{ port.port }}</td>
                    <td class="px-4 py-2 text-ink-soft">{{ port.protocol.toUpperCase() }}</td>
                    <td class="px-4 py-2">
                      <span
                        class="inline-block rounded px-1.5 py-0.5 text-[11px] font-medium"
                        :class="serviceClass(port.service)"
                      >
                        {{ port.service || '未知' }}
                      </span>
                    </td>
                    <td class="px-4 py-2 text-ink-soft font-mono max-w-[160px] truncate" :title="port.version || ''">
                      {{ port.version || '-' }}
                    </td>
                    <td class="px-4 py-2 text-ink-faint font-mono max-w-[200px] truncate" :title="port.banner || ''">
                      {{ port.banner || '-' }}
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>

        <!-- Summary -->
        <div class="flex gap-6 border-t border-paper-deep/50 px-5 py-3">
          <div class="text-center">
            <div class="text-2xl font-bold text-ink">{{ summary.hosts }}</div>
            <div class="text-xs text-ink-faint">在线主机</div>
          </div>
          <div class="text-center">
            <div class="text-2xl font-bold text-ink">{{ summary.ports }}</div>
            <div class="text-xs text-ink-faint">开放端口</div>
          </div>
          <div class="text-center">
            <div class="text-2xl font-bold text-ink">{{ summary.services }}</div>
            <div class="text-xs text-ink-faint">服务识别</div>
          </div>
          <div class="text-center">
            <div class="text-2xl font-bold text-ink">
              {{ scanState === 'completed' ? '✓' : '-' }}
            </div>
            <div class="text-xs text-ink-faint">扫描状态</div>
          </div>
        </div>
      </div>

      <!-- Empty state -->
      <div
        v-if="devices.length === 0 && scanState !== 'scanning'"
        class="flex items-center justify-center py-16 text-sm text-ink-faint"
      >
        <div class="text-center">
          <Radio class="mx-auto h-10 w-10 mb-3 opacity-30" />
          <p>输入目标 CIDR 并点击"开始扫描"</p>
          <p class="mt-1 text-xs opacity-60">将自动发现在线设备并检测开放端口</p>
        </div>
      </div>

      <!-- Scanning empty state -->
      <div
        v-if="scanState === 'scanning' && devices.length === 0"
        class="flex items-center justify-center py-16 text-sm text-ink-faint"
      >
        <div class="text-center">
          <div class="mx-auto mb-3 h-8 w-8 animate-spin rounded-full border-2 border-bamboo border-t-transparent" />
          <p>正在扫描网络中...</p>
        </div>
      </div>
    </div>

    <!-- Port modal -->
    <PortModal
      v-if="showPortModal"
      :model-value="selectedPorts"
      @update:model-value="(ports) => { selectedPorts = ports; activePreset = ''; }"
      @close="showPortModal = false"
    />
  </div>
</template>

<style scoped>
.tag-http {
  background: #d4e8f7;
  color: #2a6a9e;
}
.tag-https {
  background: #d4e8f7;
  color: #2a6a9e;
}
.tag-ssh {
  background: #d4f0d4;
  color: #2a7a2a;
}
.tag-mysql {
  background: #f7e8d4;
  color: #9e6a2a;
}
.tag-redis {
  background: #f7d4d4;
  color: #9e2a2a;
}
.tag-smb {
  background: #f0f0d4;
  color: #7a7a2a;
}
.tag-dns {
  background: #d4f0f0;
  color: #2a7a7a;
}
.tag-unknown {
  background: #eee;
  color: #888;
}
</style>
