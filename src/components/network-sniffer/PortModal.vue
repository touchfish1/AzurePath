<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { Search, X, ChevronRight } from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";

const props = defineProps<{
  modelValue: number[];
}>();

const emit = defineEmits<{
  "update:modelValue": [value: number[]];
  close: [];
}>();

const selectedPorts = ref<Set<number>>(new Set(props.modelValue));
const searchQuery = ref("");
const manualInput = ref("");

watch(
  () => props.modelValue,
  (val) => {
    selectedPorts.value = new Set(val);
  }
);

interface PortEntry {
  port: number;
  service: string;
  description: string;
}

interface Category {
  name: string;
  entries: PortEntry[];
}

const categories: Category[] = [
  {
    name: "Web 服务器",
    entries: [
      { port: 80, service: "HTTP", description: "Web 服务" },
      { port: 443, service: "HTTPS", description: "加密 Web" },
      { port: 8080, service: "HTTP-Alt", description: "HTTP 备用 / Tomcat" },
      { port: 8443, service: "HTTPS-Alt", description: "HTTPS 备用 / Tomcat SSL" },
      { port: 3000, service: "Dev", description: "Node.js / React 开发" },
      { port: 5000, service: "Flask", description: "Flask / Python Web" },
    ],
  },
  {
    name: "数据库",
    entries: [
      { port: 3306, service: "MySQL", description: "MySQL / MariaDB" },
      { port: 5432, service: "PostgreSQL", description: "PostgreSQL" },
      { port: 1433, service: "MSSQL", description: "Microsoft SQL Server" },
      { port: 1521, service: "Oracle", description: "Oracle Database" },
      { port: 27017, service: "MongoDB", description: "MongoDB" },
      { port: 6379, service: "Redis", description: "Redis" },
      { port: 9200, service: "Elasticsearch", description: "Elasticsearch" },
      { port: 11211, service: "Memcached", description: "Memcached" },
    ],
  },
  {
    name: "远程访问 & Shell",
    entries: [
      { port: 22, service: "SSH", description: "安全 Shell" },
      { port: 23, service: "Telnet", description: "远程登录" },
      { port: 3389, service: "RDP", description: "远程桌面" },
      { port: 5900, service: "VNC", description: "虚拟网络计算" },
    ],
  },
  {
    name: "文件传输",
    entries: [
      { port: 21, service: "FTP", description: "文件传输" },
      { port: 20, service: "FTP-Data", description: "FTP 数据传输" },
      { port: 69, service: "TFTP", description: "简单文件传输" },
      { port: 445, service: "SMB", description: "文件共享" },
    ],
  },
  {
    name: "邮件服务",
    entries: [
      { port: 25, service: "SMTP", description: "发送邮件" },
      { port: 465, service: "SMTPS", description: "加密发送" },
      { port: 587, service: "SMTP-Sub", description: "SMTP 提交端口" },
      { port: 110, service: "POP3", description: "接收邮件" },
      { port: 143, service: "IMAP", description: "邮件同步" },
      { port: 993, service: "IMAPS", description: "加密同步" },
    ],
  },
  {
    name: "网络基础设施",
    entries: [
      { port: 53, service: "DNS", description: "域名解析" },
      { port: 67, service: "DHCP", description: "地址分配" },
      { port: 123, service: "NTP", description: "网络时间" },
      { port: 161, service: "SNMP", description: "网络管理" },
      { port: 389, service: "LDAP", description: "目录服务" },
      { port: 514, service: "Syslog", description: "系统日志" },
    ],
  },
  {
    name: "Windows 服务",
    entries: [
      { port: 135, service: "RPC", description: "远程过程调用" },
      { port: 139, service: "NetBIOS", description: "会话服务" },
      { port: 445, service: "SMB", description: "文件打印机共享" },
      { port: 3389, service: "RDP", description: "远程桌面" },
    ],
  },
  {
    name: "消息 & 中间件",
    entries: [
      { port: 2181, service: "ZooKeeper", description: "ZooKeeper" },
      { port: 61616, service: "ActiveMQ", description: "ActiveMQ" },
      { port: 9092, service: "Kafka", description: "Kafka" },
    ],
  },
];

const openCategories = ref<Set<string>>(new Set(categories.map((c) => c.name)));

function toggleCategory(name: string) {
  if (openCategories.value.has(name)) {
    openCategories.value.delete(name);
  } else {
    openCategories.value.add(name);
  }
}

function togglePort(port: number) {
  if (selectedPorts.value.has(port)) {
    selectedPorts.value.delete(port);
  } else {
    selectedPorts.value.add(port);
  }
}

function addManualPorts() {
  const nums = manualInput.value
    .split(",")
    .map((s) => parseInt(s.trim()))
    .filter((n) => !isNaN(n) && n > 0 && n <= 65535);
  for (const p of nums) {
    selectedPorts.value.add(p);
  }
  manualInput.value = "";
}

const filteredCategories = computed(() => {
  const q = searchQuery.value.toLowerCase().trim();
  if (!q) return categories;

  return categories
    .map((cat) => ({
      ...cat,
      entries: cat.entries.filter(
        (e) =>
          e.port.toString().includes(q) ||
          e.service.toLowerCase().includes(q) ||
          e.description.toLowerCase().includes(q)
      ),
    }))
    .filter((cat) => cat.entries.length > 0);
});

const sortedSelected = computed(() =>
  Array.from(selectedPorts.value).sort((a, b) => a - b)
);

function confirm() {
  emit("update:modelValue", sortedSelected.value);
}
</script>

<template>
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/45" @click.self="$emit('close')">
    <div class="flex w-[640px] max-h-[80vh] flex-col rounded-2xl bg-paper shadow-2xl overflow-hidden">
      <!-- Header -->
      <div class="flex items-center justify-between border-b border-paper-deep/50 px-6 py-4">
        <h2 class="text-base font-semibold text-ink">选择扫描端口</h2>
        <button
          class="flex h-7 w-7 items-center justify-center rounded-md bg-paper-deep/30 text-ink-faint transition-colors hover:bg-paper-deep/60"
          @click="$emit('close')"
        >
          <X class="h-4 w-4" />
        </button>
      </div>

      <!-- Quick manual input -->
      <div class="flex items-center gap-2 border-b border-paper-deep/20 px-6 py-3">
        <input
          v-model="manualInput"
          type="text"
          placeholder="手动输入端口号，用逗号分隔 (e.g. 22,80,443,8080)"
          class="flex-1 rounded-lg border border-paper-deep bg-paper-warm/50 px-3 py-2 text-sm font-mono text-ink outline-none transition-colors focus:border-bamboo/50"
          @keydown.enter="addManualPorts"
        />
        <Button size="sm" @click="addManualPorts">添加</Button>
      </div>

      <!-- Body -->
      <div class="flex-1 overflow-y-auto px-6 py-4">
        <!-- Search -->
        <div class="relative mb-4">
          <Search class="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-ink-faint" />
          <input
            v-model="searchQuery"
            type="text"
            placeholder="搜索服务名称或端口号..."
            class="w-full rounded-lg border border-paper-deep bg-paper-warm/50 py-2 pl-9 pr-3 text-sm text-ink outline-none transition-colors focus:border-bamboo/50"
          />
        </div>

        <!-- Categories -->
        <div v-for="cat in filteredCategories" :key="cat.name" class="mb-2">
          <div
            class="flex cursor-pointer items-center gap-2 rounded-lg px-2.5 py-2 text-sm font-semibold text-ink-soft transition-colors hover:bg-paper-deep/30 select-none"
            @click="toggleCategory(cat.name)"
          >
            <ChevronRight
              class="h-3.5 w-3.5 text-ink-faint transition-transform duration-150"
              :class="{ 'rotate-90': openCategories.has(cat.name) }"
            />
            {{ cat.name }}
            <span class="ml-auto text-xs text-ink-faint font-normal">{{ cat.entries.length }}</span>
          </div>

          <div
            v-if="openCategories.has(cat.name)"
            class="grid grid-cols-[repeat(auto-fill,minmax(180px,1fr))] gap-1 py-1 pl-6"
          >
            <div
              v-for="entry in cat.entries"
              :key="entry.port"
              class="flex cursor-pointer items-center gap-2 rounded-lg border border-transparent px-2.5 py-1.5 text-sm transition-colors hover:border-paper-deep/50 hover:bg-paper-warm/50"
              :class="{
                'bg-bamboo/5 border-bamboo/20': selectedPorts.has(entry.port),
              }"
              @click="togglePort(entry.port)"
            >
              <div
                class="flex h-4 w-4 shrink-0 items-center justify-center rounded border-2 transition-colors"
                :class="
                  selectedPorts.has(entry.port)
                    ? 'border-bamboo bg-bamboo text-white'
                    : 'border-paper-deep'
                "
              >
                <span v-if="selectedPorts.has(entry.port)" class="text-[10px]">✓</span>
              </div>
              <span class="min-w-[40px] font-mono text-sm font-semibold text-ink">{{ entry.port }}</span>
              <span class="truncate text-xs text-ink-soft">{{ entry.description }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Footer -->
      <div class="flex items-center justify-between border-t border-paper-deep/50 px-6 py-3">
        <span class="text-xs text-ink-faint">
          已选择 <strong class="text-ink">{{ sortedSelected.length }}</strong> 个端口
        </span>
        <div class="flex gap-2">
          <Button variant="ghost" size="sm" @click="$emit('close')">取消</Button>
          <Button size="sm" @click="confirm">确认选择</Button>
        </div>
      </div>
    </div>
  </div>
</template>
