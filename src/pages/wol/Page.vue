<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { Power, Plus, Trash2, Copy, Check, Send, Laptop } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";

interface WolRecord {
  id: string;
  mac: string;
  broadcastIp: string;
  port: number;
  label: string;
  lastUsed: string;
}

interface WolResult {
  success: boolean;
  message: string;
}

// ─── Form State ────────────────────────────────────────────────
const mac = ref("");
const broadcastIp = ref("192.168.1.255");
const port = ref(9);
const label = ref("");

const sending = ref(false);
const result = ref<WolResult | null>(null);
const error = ref("");

const records = ref<WolRecord[]>([]);
const loading = ref(false);
const copiedId = ref<string | null>(null);

// ─── Methods ────────────────────────────────────────────────────

async function loadRecords() {
  loading.value = true;
  try {
    records.value = await invoke<WolRecord[]>("wol_list");
  } catch {
    records.value = [];
  } finally {
    loading.value = false;
  }
}

async function sendWol(macAddr?: string, broadcastIpAddr?: string, portNum?: number) {
  sending.value = true;
  result.value = null;
  error.value = "";

  const targetMac = macAddr || mac.value.trim();
  const targetIp = broadcastIpAddr || broadcastIp.value.trim();
  const targetPort = portNum || port.value;

  if (!targetMac) {
    error.value = "请输入 MAC 地址";
    sending.value = false;
    return;
  }
  if (!isValidMac(targetMac)) {
    error.value = "MAC 地址格式无效，请使用 XX:XX:XX:XX:XX:XX 格式";
    sending.value = false;
    return;
  }

  try {
    const res = await invoke<WolResult>("wol_send", {
      mac: targetMac,
      broadcastIp: targetIp,
      port: targetPort,
    });
    result.value = res;
  } catch (e) {
    error.value = String(e);
  } finally {
    sending.value = false;
  }
}

function isValidMac(mac: string): boolean {
  return /^([0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}$/.test(mac.trim());
}

async function saveRecord() {
  if (!mac.value.trim() || !label.value.trim()) {
    error.value = "请填写 MAC 地址和标签";
    return;
  }
  if (!isValidMac(mac.value)) {
    error.value = "MAC 地址格式无效，请使用 XX:XX:XX:XX:XX:XX 格式";
    return;
  }

  try {
    await invoke<WolRecord>("wol_save", {
      mac: mac.value.trim(),
      broadcastIp: broadcastIp.value.trim() || "192.168.1.255",
      label: label.value.trim(),
    });
    // Reset form
    label.value = "";
    mac.value = "";
    broadcastIp.value = "192.168.1.255";
    port.value = 9;
    await loadRecords();
  } catch (e) {
    error.value = String(e);
  }
}

async function deleteRecord(id: string) {
  try {
    await invoke<string>("wol_delete", { id });
    records.value = records.value.filter((r) => r.id !== id);
  } catch (e) {
    error.value = String(e);
  }
}

function copyMac(macAddr: string) {
  navigator.clipboard.writeText(macAddr).then(() => {
    copiedId.value = macAddr;
    setTimeout(() => { copiedId.value = null; }, 2000);
  }).catch(() => {
    // Clipboard write might fail in some contexts
  });
}

function sendFromRecord(record: WolRecord) {
  sendWol(record.mac, record.broadcastIp, record.port);
}

onMounted(() => {
  loadRecords();
});
</script>

<template>
  <div class="flex h-full flex-col p-6 space-y-6 animate-view-fade overflow-y-auto">
    <!-- Header -->
    <div>
      <h1 class="text-2xl font-display font-bold text-ink">Wake-on-LAN</h1>
      <p class="mt-0.5 text-sm text-ink-faint">通过网络发送魔术包唤醒远程设备</p>
    </div>

    <!-- Send Form -->
    <div class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm">
      <h2 class="text-sm font-semibold text-ink mb-4 flex items-center gap-2">
        <Send class="h-4 w-4 text-bamboo" />
        发送魔术包
      </h2>
      <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div>
          <label class="mb-1.5 block text-xs font-medium text-ink-soft">MAC 地址</label>
          <input
            v-model="mac"
            type="text"
            placeholder="00:11:22:33:44:55"
            class="w-full rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/80 font-mono"
          />
        </div>
        <div>
          <label class="mb-1.5 block text-xs font-medium text-ink-soft">广播地址</label>
          <input
            v-model="broadcastIp"
            type="text"
            placeholder="192.168.1.255"
            class="w-full rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/80 font-mono"
          />
        </div>
        <div>
          <label class="mb-1.5 block text-xs font-medium text-ink-soft">端口</label>
          <input
            v-model.number="port"
            type="number"
            min="1"
            max="65535"
            class="w-full rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/80"
          />
        </div>
        <div>
          <label class="mb-1.5 block text-xs font-medium text-ink-soft">标签（保存用）</label>
          <input
            v-model="label"
            type="text"
            placeholder="如 办公室电脑"
            class="w-full rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/80"
          />
        </div>
      </div>

      <div class="mt-4 flex gap-2">
        <Button :disabled="sending" @click="sendWol()">
          <Power class="mr-1.5 h-3.5 w-3.5" />
          {{ sending ? "发送中..." : "发送魔术包" }}
        </Button>
        <Button variant="secondary" @click="saveRecord">
          <Plus class="mr-1.5 h-3.5 w-3.5" />
          保存设备
        </Button>
      </div>

      <p v-if="error" class="mt-3 text-sm text-red-500">{{ error }}</p>

      <div
        v-if="result"
        class="mt-3 rounded-lg px-4 py-3 text-sm"
        :class="result.success ? 'bg-bamboo/10 text-bamboo' : 'bg-red-500/10 text-red-500'"
      >
        {{ result.message }}
      </div>
    </div>

    <!-- Saved Devices -->
    <div class="noise-bg rounded-xl border border-paper-deep/60 bg-paper p-5 shadow-sm">
      <h2 class="text-sm font-semibold text-ink mb-4 flex items-center gap-2">
        <Laptop class="h-4 w-4 text-bamboo" />
        已保存的设备
        <span v-if="records.length" class="ml-auto text-xs font-normal text-ink-faint">{{ records.length }} 个设备</span>
      </h2>

      <div v-if="loading" class="py-8 text-center text-sm text-ink-faint">加载中...</div>

      <div v-else-if="records.length === 0" class="py-8 text-center text-sm text-ink-faint">
        暂无保存的设备。发送魔术包并点击"保存设备"将其添加到列表。
      </div>

      <div v-else class="space-y-2">
        <div
          v-for="record in records"
          :key="record.id"
          class="flex items-center justify-between rounded-lg border border-paper-deep/30 bg-paper-warm/30 px-4 py-3 transition-colors hover:border-paper-deep/60"
        >
          <div class="flex-1 min-w-0">
            <div class="flex items-center gap-2">
              <span class="text-sm font-medium text-ink">{{ record.label }}</span>
            </div>
            <div class="mt-1 flex items-center gap-3 text-xs text-ink-faint">
              <span class="font-mono">{{ record.mac }}</span>
              <span>{{ record.broadcastIp }}:{{ record.port }}</span>
            </div>
          </div>
          <div class="flex items-center gap-1 ml-4 shrink-0">
            <button
              class="rounded-lg p-2 text-ink-soft transition-colors hover:bg-bamboo/10 hover:text-bamboo"
              title="复制 MAC"
              @click="copyMac(record.mac)"
            >
              <Copy v-if="copiedId !== record.mac" class="h-3.5 w-3.5" />
              <Check v-else class="h-3.5 w-3.5 text-bamboo" />
            </button>
            <button
              class="rounded-lg p-2 text-ink-soft transition-colors hover:bg-bamboo/10 hover:text-bamboo"
              title="发送魔术包"
              @click="sendFromRecord(record)"
            >
              <Power class="h-3.5 w-3.5" />
            </button>
            <button
              class="rounded-lg p-2 text-ink-soft transition-colors hover:bg-red-500/10 hover:text-red-500"
              title="删除"
              @click="deleteRecord(record.id)"
            >
              <Trash2 class="h-3.5 w-3.5" />
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
