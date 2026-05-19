<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { Search, Wifi, Copy } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import ReportButton from "@/components/ReportButton.vue";
import { useToastStore } from "@/stores/toast";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

const toast = useToastStore();

interface MdnsService {
  serviceType: string;
  hostname: string;
  ip: string;
  port: number;
  txt: Record<string, string>;
}

interface MdnsProgress {
  status: string;
  message?: string;
  count?: number;
}

const loading = ref(false);
const services = ref<MdnsService[]>([]);
const progress = ref("");
const error = ref("");

let unlistenProgress: UnlistenFn | null = null;

function copyValue(val: string) {
  navigator.clipboard.writeText(val).then(() => {
    toast.add("success", "已复制");
  });
}

function serviceBadgeClass(type: string): string {
  if (type.includes("http")) return "bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300";
  if (type.includes("smb")) return "bg-orange-100 text-orange-700 dark:bg-orange-900/30 dark:text-orange-300";
  if (type.includes("ssh")) return "bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-300";
  if (type.includes("ftp")) return "bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-300";
  if (type.includes("rdp")) return "bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-300";
  if (type.includes("vnc")) return "bg-pink-100 text-pink-700 dark:bg-pink-900/30 dark:text-pink-300";
  return "bg-gray-100 text-gray-700 dark:bg-gray-800 dark:text-gray-300";
}

function serviceLabel(type: string): string {
  const labels: Record<string, string> = {
    "_http._tcp": "HTTP",
    "_https._tcp": "HTTPS",
    "_smb._tcp": "SMB",
    "_ssh._tcp": "SSH",
    "_ftp._tcp": "FTP",
    "_afpovertcp._tcp": "AFP",
    "_rdp._tcp": "RDP",
    "_teamviewer._tcp": "TeamViewer",
    "_vnc._tcp": "VNC",
  };
  return labels[type] || type;
}

// Group services by type
const groupedServices = ref<Map<string, MdnsService[]>>(new Map());

function updateGroupedServices() {
  const map = new Map<string, MdnsService[]>();
  for (const s of services.value) {
    const list = map.get(s.serviceType) || [];
    list.push(s);
    map.set(s.serviceType, list);
  }
  groupedServices.value = map;
}

async function scan() {
  loading.value = true;
  error.value = "";
  services.value = [];
  progress.value = "正在扫描...";

  try {
    const result = await invoke<MdnsService[]>("mdns_discover");
    services.value = result;
    updateGroupedServices();
    progress.value = `发现 ${result.length} 个服务`;
  } catch (e) {
    error.value = String(e);
    progress.value = "扫描失败";
  } finally {
    loading.value = false;
  }
}

function handleProgress(payload: MdnsProgress) {
  if (payload.message) {
    progress.value = payload.message;
  }
}

onMounted(async () => {
  unlistenProgress = await listen<MdnsProgress>("mdns:progress", (event) => {
    handleProgress(event.payload);
  });
});

onUnmounted(() => {
  unlistenProgress?.();
});
</script>

<template>
  <div class="flex h-full flex-col p-4 md:p-6 space-y-4 md:space-y-6 animate-view-fade">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-display font-bold text-ink">mDNS 服务发现</h1>
        <p class="mt-0.5 text-sm text-ink-faint">发现局域网中的 Bonjour/mDNS 服务</p>
      </div>
      <div class="flex items-center gap-2">
        <ReportButton
          v-if="services.length > 0"
          title="mDNS 服务发现"
          :columns="[
            { key: 'serviceType', label: '服务类型' },
            { key: 'hostname', label: '主机名' },
            { key: 'ip', label: 'IP 地址' },
            { key: 'port', label: '端口' },
            { key: 'txt', label: 'TXT 信息' },
          ]"
          :rows="services"
        />
        <Button :disabled="loading" @click="scan">
          <Wifi class="mr-1.5 h-3.5 w-3.5" />
          {{ loading ? "扫描中..." : "扫描" }}
        </Button>
      </div>
    </div>

    <!-- Progress -->
    <div
      v-if="progress"
      class="rounded-lg border border-paper-deep/60 bg-paper-warm/50 px-4 py-2 text-sm text-ink-soft"
    >
      {{ progress }}
    </div>

    <!-- Error -->
    <div
      v-if="error"
      class="rounded-lg border border-red-200 bg-red-50 px-4 py-2 text-sm text-red-600 dark:border-red-900/30 dark:bg-red-900/20 dark:text-red-400"
    >
      {{ error }}
    </div>

    <!-- Loading -->
    <div
      v-if="loading && services.length === 0"
      class="flex flex-col items-center justify-center py-16 text-ink-faint"
    >
      <Wifi class="mb-3 h-10 w-10 animate-pulse" />
      <p class="text-sm">正在扫描局域网 mDNS 服务...</p>
    </div>

    <!-- Empty state -->
    <div
      v-if="!loading && services.length === 0 && !error && progress"
      class="flex flex-col items-center justify-center py-16 text-ink-faint"
    >
      <Search class="mb-3 h-10 w-10" />
      <p class="text-sm">未发现 mDNS 服务</p>
      <p class="mt-1 text-xs">请确保网络中有支持 mDNS/Bonjour 的设备</p>
    </div>

    <!-- Results grouped by service type -->
    <div v-if="services.length > 0" class="space-y-4">
      <div
        v-for="[type, typeServices] in groupedServices"
        :key="type"
        class="overflow-hidden rounded-xl border border-paper-deep/60 bg-paper shadow-sm"
      >
        <!-- Group header -->
        <div class="flex items-center gap-2 border-b border-paper-deep/50 bg-paper-warm/50 px-5 py-3">
          <span
            class="rounded-full px-2.5 py-0.5 text-xs font-medium"
            :class="serviceBadgeClass(type)"
          >
            {{ serviceLabel(type) }}
          </span>
          <span class="text-xs text-ink-faint">{{ typeServices.length }} 个实例</span>
        </div>

        <!-- Service table -->
        <div class="overflow-x-auto">
          <table class="w-full text-sm">
            <thead>
              <tr class="border-b border-paper-deep/30 bg-paper-warm/30">
                <th class="px-5 py-2.5 text-left text-xs font-medium text-ink-soft">主机名</th>
                <th class="px-5 py-2.5 text-left text-xs font-medium text-ink-soft">IP 地址</th>
                <th class="px-5 py-2.5 text-left text-xs font-medium text-ink-soft">端口</th>
                <th class="px-5 py-2.5 text-left text-xs font-medium text-ink-soft">TXT 信息</th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="(svc, idx) in typeServices"
                :key="idx"
                class="border-b border-paper-deep/20 transition-colors hover:bg-paper-warm/50"
              >
                <td class="px-5 py-3">
                  <button
                    class="flex items-center gap-1 text-ink hover:text-bamboo transition-colors"
                    @click="copyValue(svc.hostname)"
                    :title="'点击复制: ' + svc.hostname"
                  >
                    {{ svc.hostname || "-" }}
                    <Copy class="h-3 w-3 shrink-0 opacity-0 group-hover:opacity-100" />
                  </button>
                </td>
                <td class="px-5 py-3 font-mono text-xs text-ink-soft">
                  <button
                    class="hover:text-bamboo transition-colors"
                    @click="copyValue(svc.ip)"
                    :title="'点击复制: ' + svc.ip"
                  >
                    {{ svc.ip || "-" }}
                  </button>
                </td>
                <td class="px-5 py-3 text-ink-soft">{{ svc.port || "-" }}</td>
                <td class="px-5 py-3">
                  <div v-if="Object.keys(svc.txt).length > 0" class="flex flex-wrap gap-1">
                    <span
                      v-for="(val, key) in svc.txt"
                      :key="key"
                      class="inline-flex items-center rounded bg-paper-deep/30 px-1.5 py-0.5 text-xs text-ink-soft"
                      :title="`${key}=${val}`"
                    >
                      {{ key }}{{ val ? `=${val}` : "" }}
                    </span>
                  </div>
                  <span v-else class="text-ink-faint">-</span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
  </div>
</template>
