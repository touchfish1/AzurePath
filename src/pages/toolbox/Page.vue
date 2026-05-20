<script setup lang="ts">
import { ref, computed } from "vue";
import {
  Network,
  Terminal,
  Link,
  Fingerprint,
  Search,
  Copy,
  Check,
  Wifi,
  FileJson,
  Key,
  Clock,
} from "lucide-vue-next";
import Button from "@/components/ui/button/Button.vue";
import QRCode from "qrcode";
import { calculateSubnet as tauriCalculateSubnet, splitSubnet as tauriSplitSubnet } from "@/lib/tauri";
import type { SubnetResult, SubnetSplitResult } from "@/lib/tauri";

// ─── Tab System ───────────────────────────────────────────────
type ToolTab = "subnet" | "base64" | "url" | "hash" | "port" | "wifi" | "json" | "jwt" | "timestamp";

interface ToolTabDef {
  key: ToolTab;
  label: string;
  icon: object;
}

const tabs: ToolTabDef[] = [
  { key: "subnet", label: "子网计算", icon: Network },
  { key: "base64", label: "Base64", icon: Terminal },
  { key: "url", label: "URL", icon: Link },
  { key: "hash", label: "Hash", icon: Fingerprint },
  { key: "port", label: "端口速查", icon: Search },
  { key: "wifi", label: "WiFi QR", icon: Wifi },
  { key: "json", label: "JSON", icon: FileJson },
  { key: "jwt", label: "JWT", icon: Key },
  { key: "timestamp", label: "时间戳", icon: Clock },
];

const activeTab = ref<ToolTab>("subnet");

// ─── 1. Subnet Calculator ────────────────────────────────────
const subnetIpVersion = ref<"IPv4" | "IPv6">("IPv4");
const subnetIp = ref("192.168.1.0");
const subnetCidr = ref(24);
const subnetResult = ref<SubnetResult | null>(null);
const subnetError = ref("");

// Subnet splitting
const subnetSplitOpen = ref(false);
const subnetTargetPrefix = ref(26);
const subnetSplitResult = ref<SubnetSplitResult | null>(null);
const subnetSplitError = ref("");

const cidrMax = computed(() => (subnetIpVersion.value === "IPv4" ? 32 : 128));
const cidrHint = computed(() =>
  subnetIpVersion.value === "IPv4" ? "(0-32)" : "(0-128)"
);

const classificationBadgeClass = computed(() => {
  const c = subnetResult.value?.classification;
  if (!c) return "";
  if (c.isPrivate) return "bg-amber/10 text-amber ring-1 ring-amber/30";
  if (c.isLoopback) return "bg-blue/10 text-blue ring-1 ring-blue/30";
  if (c.isLinkLocal) return "bg-cyan/10 text-cyan ring-1 ring-cyan/30";
  if (c.isMulticast) return "bg-purple/10 text-purple ring-1 ring-purple/30";
  if (c.isPublic) return "bg-emerald/10 text-emerald ring-1 ring-emerald/30";
  return "bg-gray/10 text-gray ring-1 ring-gray/30";
});

function formatUsableHosts(hosts: number): string {
  if (hosts > Number.MAX_SAFE_INTEGER) return "大量";
  return hosts.toLocaleString();
}

async function doCalculateSubnet() {
  subnetError.value = "";
  subnetResult.value = null;
  subnetSplitResult.value = null;

  const cidr = subnetCidr.value;
  if (cidr < 0 || cidr > (subnetIpVersion.value === "IPv4" ? 32 : 128)) {
    subnetError.value = `CIDR 必须介于 0 和 ${subnetIpVersion.value === "IPv4" ? 32 : 128} 之间`;
    return;
  }

  try {
    const result = await tauriCalculateSubnet(subnetIp.value, cidr);
    subnetResult.value = result;
  } catch (e) {
    subnetError.value = String(e);
  }
}

async function doSplitSubnet() {
  subnetSplitError.value = "";
  subnetSplitResult.value = null;

  if (!subnetResult.value) {
    subnetSplitError.value = "请先计算子网";
    return;
  }

  const target = subnetTargetPrefix.value;
  if (target <= subnetCidr.value) {
    subnetSplitError.value = "目标前缀必须大于当前前缀";
    return;
  }

  try {
    const networkStr = `${subnetResult.value.networkAddress}/${subnetResult.value.cidr}`;
    const result = await tauriSplitSubnet(networkStr, target);
    subnetSplitResult.value = result;
  } catch (e) {
    subnetSplitError.value = String(e);
  }
}

// ─── 2. Base64 ───────────────────────────────────────────────
const base64Input = ref("");
const base64Output = ref("");
const base64Mode = ref<"encode" | "decode">("encode");
const base64Copied = ref(false);
const base64Error = ref("");

function base64Encode() {
  base64Error.value = "";
  try {
    base64Output.value = btoa(unescape(encodeURIComponent(base64Input.value)));
    base64Mode.value = "encode";
  } catch {
    base64Error.value = "编码失败";
  }
}

function base64Decode() {
  base64Error.value = "";
  try {
    base64Output.value = decodeURIComponent(escape(atob(base64Input.value)));
    base64Mode.value = "decode";
  } catch {
    base64Error.value = "解码失败：输入不是有效的 Base64 编码";
  }
}

function detectAndConvertBase64() {
  base64Error.value = "";
  const input = base64Input.value.trim();
  if (!input) {
    base64Error.value = "请输入内容";
    return;
  }
  // Try to detect if it looks like base64 (alphanumeric + +/=)
  const base64Regex = /^[A-Za-z0-9+/]*={0,2}$/;
  if (base64Regex.test(input) && input.length % 4 === 0 && input.length > 0) {
    base64Decode();
  } else {
    base64Encode();
  }
}

function copyBase64() {
  if (!base64Output.value) return;
  navigator.clipboard.writeText(base64Output.value).then(() => {
    base64Copied.value = true;
    setTimeout(() => { base64Copied.value = false; }, 2000);
  });
}

// ─── 3. URL ──────────────────────────────────────────────────
const urlInput = ref("");
const urlOutput = ref("");
const urlMode = ref<"encode" | "decode">("encode");
const urlCopied = ref(false);
const urlError = ref("");

function urlEncode() {
  urlError.value = "";
  try {
    urlOutput.value = encodeURIComponent(urlInput.value);
    urlMode.value = "encode";
  } catch {
    urlError.value = "编码失败";
  }
}

function urlDecode() {
  urlError.value = "";
  try {
    urlOutput.value = decodeURIComponent(urlInput.value);
    urlMode.value = "decode";
  } catch {
    urlError.value = "解码失败：输入不是有效的 URL 编码";
  }
}

function detectAndConvertUrl() {
  urlError.value = "";
  const input = urlInput.value.trim();
  if (!input) {
    urlError.value = "请输入内容";
    return;
  }
  // Check if contains percent-encoded sequences
  if (/%[0-9A-Fa-f]{2}/.test(input)) {
    urlDecode();
  } else {
    urlEncode();
  }
}

function copyUrl() {
  if (!urlOutput.value) return;
  navigator.clipboard.writeText(urlOutput.value).then(() => {
    urlCopied.value = true;
    setTimeout(() => { urlCopied.value = false; }, 2000);
  });
}

// ─── 4. Hash Generator ───────────────────────────────────────
const hashInput = ref("");
const hashAlgorithm = ref<"MD5" | "SHA-1" | "SHA-256" | "SHA-512">("SHA-256");
const hashResult = ref("");
const hashCopied = ref(false);
const hashError = ref("");
const hashComputing = ref(false);

const algorithmOptions = [
  { value: "MD5" as const, label: "MD5" },
  { value: "SHA-1" as const, label: "SHA1" },
  { value: "SHA-256" as const, label: "SHA256" },
  { value: "SHA-512" as const, label: "SHA512" },
];

const algDisplayName = computed(() => {
  const map: Record<string, string> = {
    MD5: "MD5",
    "SHA-1": "SHA1",
    "SHA-256": "SHA256",
    "SHA-512": "SHA512",
  };
  return map[hashAlgorithm.value] || hashAlgorithm.value;
});

async function computeHash() {
  hashError.value = "";
  hashResult.value = "";
  if (!hashInput.value.trim()) {
    hashError.value = "请输入要计算哈希的文本";
    return;
  }

  hashComputing.value = true;
  try {
    const encoder = new TextEncoder();
    const data = encoder.encode(hashInput.value);

    if (hashAlgorithm.value === "MD5") {
      hashResult.value = md5Hex(data);
    } else {
      const hashBuffer = await crypto.subtle.digest(hashAlgorithm.value, data);
      hashResult.value = bufferToHex(hashBuffer);
    }
  } catch (e) {
    hashError.value = "哈希计算失败：" + String(e);
  } finally {
    hashComputing.value = false;
  }
}

function bufferToHex(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer);
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

// Pure JS MD5 implementation (Web Crypto API does not support MD5 natively)
function md5Hex(data: Uint8Array): string {
  const n = data.length;
  const bits: number[] = [];
  for (let i = 0; i < n; i++) {
    bits.push(data[i]);
  }
  // Append padding
  bits.push(0x80);
  while (bits.length % 64 !== 56) {
    bits.push(0x00);
  }
  // Append length in bits (as 64-bit little-endian)
  const lenBits = (n * 8) >>> 0;
  bits.push(lenBits & 0xff);
  bits.push((lenBits >>> 8) & 0xff);
  bits.push((lenBits >>> 16) & 0xff);
  bits.push((lenBits >>> 24) & 0xff);
  // Upper 32 bits of length (always 0 for short messages)
  bits.push(0, 0, 0, 0);

  let a0 = 0x67452301;
  let b0 = 0xefcdab89;
  let c0 = 0x98badcfe;
  let d0 = 0x10325476;

  const K: number[] = [];
  for (let i = 0; i < 64; i++) {
    K[i] = Math.floor(Math.abs(Math.sin(i + 1)) * 0x100000000);
  }

  function leftRotate(x: number, c: number): number {
    return (x << c) | (x >>> (32 - c));
  }

  for (let offset = 0; offset < bits.length; offset += 64) {
    const M = bits.slice(offset, offset + 64);
    const w: number[] = [];
    for (let i = 0; i < 16; i++) {
      w[i] = M[i * 4] | (M[i * 4 + 1] << 8) | (M[i * 4 + 2] << 16) | (M[i * 4 + 3] << 24);
    }

    let A = a0;
    let B = b0;
    let C = c0;
    let D = d0;

    for (let i = 0; i < 64; i++) {
      let F: number;
      let g: number;
      if (i < 16) {
        F = (B & C) | (~B & D);
        g = i;
      } else if (i < 32) {
        F = (D & B) | (~D & C);
        g = (5 * i + 1) % 16;
      } else if (i < 48) {
        F = B ^ C ^ D;
        g = (3 * i + 5) % 16;
      } else {
        F = C ^ (B | ~D);
        g = (7 * i) % 16;
      }

      const temp = D;
      D = C;
      C = B;
      const shiftAmounts = [7, 12, 17, 22, 5, 9, 14, 20, 4, 11, 16, 23, 6, 10, 15, 21];
      const shiftIdx = Math.floor(i / 4) % 4 + ((i < 16) ? 0 : (i < 32) ? 4 : (i < 48) ? 8 : 12);
      B = (B + leftRotate((A + F + K[i] + w[g]) >>> 0, shiftAmounts[shiftIdx])) >>> 0;
      A = temp;
    }

    a0 = (a0 + A) >>> 0;
    b0 = (b0 + B) >>> 0;
    c0 = (c0 + C) >>> 0;
    d0 = (d0 + D) >>> 0;
  }

  // Convert to hex (little-endian output)
  function wordToHexLE(n: number): string {
    const b1 = n & 0xff;
    const b2 = (n >>> 8) & 0xff;
    const b3 = (n >>> 16) & 0xff;
    const b4 = (n >>> 24) & 0xff;
    return [b1, b2, b3, b4].map((b) => b.toString(16).padStart(2, "0")).join("");
  }

  return wordToHexLE(a0) + wordToHexLE(b0) + wordToHexLE(c0) + wordToHexLE(d0);
}

function copyHash() {
  if (!hashResult.value) return;
  navigator.clipboard.writeText(hashResult.value).then(() => {
    hashCopied.value = true;
    setTimeout(() => { hashCopied.value = false; }, 2000);
  });
}

// ─── 5. Port Lookup ──────────────────────────────────────────
const portInput = ref<number | null>(null);
const portResult = ref<PortInfo | null>(null);
const portError = ref("");

interface PortInfo {
  port: number;
  service: string;
  protocol: string;
  description: string;
}

const portDb: PortInfo[] = [
  { port: 20, service: "FTP-DATA", protocol: "TCP", description: "文件传输协议（数据连接）" },
  { port: 21, service: "FTP", protocol: "TCP", description: "文件传输协议（控制连接）" },
  { port: 22, service: "SSH", protocol: "TCP", description: "安全外壳协议 / SFTP / SCP" },
  { port: 23, service: "Telnet", protocol: "TCP", description: "远程登录协议（不加密）" },
  { port: 25, service: "SMTP", protocol: "TCP", description: "简单邮件传输协议" },
  { port: 53, service: "DNS", protocol: "UDP/TCP", description: "域名系统" },
  { port: 67, service: "DHCP", protocol: "UDP", description: "动态主机配置协议（服务端）" },
  { port: 68, service: "DHCP", protocol: "UDP", description: "动态主机配置协议（客户端）" },
  { port: 69, service: "TFTP", protocol: "UDP", description: "简单文件传输协议" },
  { port: 80, service: "HTTP", protocol: "TCP", description: "超文本传输协议" },
  { port: 110, service: "POP3", protocol: "TCP", description: "邮局协议第3版" },
  { port: 123, service: "NTP", protocol: "UDP", description: "网络时间协议" },
  { port: 135, service: "RPC", protocol: "TCP/UDP", description: "远程过程调用" },
  { port: 137, service: "NetBIOS-NS", protocol: "UDP", description: "NetBIOS 名称服务" },
  { port: 138, service: "NetBIOS-DGM", protocol: "UDP", description: "NetBIOS 数据报服务" },
  { port: 139, service: "NetBIOS-SSN", protocol: "TCP", description: "NetBIOS 会话服务" },
  { port: 143, service: "IMAP", protocol: "TCP", description: "互联网消息访问协议" },
  { port: 161, service: "SNMP", protocol: "UDP", description: "简单网络管理协议" },
  { port: 162, service: "SNMP-TRAP", protocol: "UDP", description: "SNMP 陷阱通知" },
  { port: 389, service: "LDAP", protocol: "TCP/UDP", description: "轻量级目录访问协议" },
  { port: 443, service: "HTTPS", protocol: "TCP", description: "超文本传输安全协议" },
  { port: 445, service: "SMB", protocol: "TCP", description: "SMB 文件共享协议" },
  { port: 465, service: "SMTPS", protocol: "TCP", description: "SMTP over SSL" },
  { port: 500, service: "IKE", protocol: "UDP", description: "Internet 密钥交换" },
  { port: 514, service: "Syslog", protocol: "UDP", description: "系统日志协议" },
  { port: 554, service: "RTSP", protocol: "TCP/UDP", description: "实时流协议" },
  { port: 587, service: "SMTP-Sub", protocol: "TCP", description: "SMTP 邮件提交" },
  { port: 631, service: "IPP", protocol: "TCP", description: "互联网打印协议" },
  { port: 636, service: "LDAPS", protocol: "TCP", description: "LDAP over SSL" },
  { port: 993, service: "IMAPS", protocol: "TCP", description: "IMAP over SSL" },
  { port: 995, service: "POP3S", protocol: "TCP", description: "POP3 over SSL" },
  { port: 1080, service: "SOCKS", protocol: "TCP", description: "SOCKS 代理协议" },
  { port: 1194, service: "OpenVPN", protocol: "UDP/TCP", description: "OpenVPN 虚拟专用网络" },
  { port: 1433, service: "MSSQL", protocol: "TCP", description: "Microsoft SQL Server" },
  { port: 1521, service: "Oracle", protocol: "TCP", description: "Oracle 数据库" },
  { port: 1701, service: "L2TP", protocol: "UDP", description: "第二层隧道协议" },
  { port: 1723, service: "PPTP", protocol: "TCP", description: "点对点隧道协议" },
  { port: 1812, service: "RADIUS", protocol: "UDP", description: "RADIUS 认证" },
  { port: 1813, service: "RADIUS-ACCT", protocol: "UDP", description: "RADIUS 计费" },
  { port: 1883, service: "MQTT", protocol: "TCP", description: "MQTT 物联网消息协议" },
  { port: 2082, service: "cPanel", protocol: "TCP", description: "cPanel 管理面板" },
  { port: 2083, service: "cPanel-SSL", protocol: "TCP", description: "cPanel 管理面板（SSL）" },
  { port: 2375, service: "Docker", protocol: "TCP", description: "Docker REST API（未加密）" },
  { port: 2376, service: "Docker-SSL", protocol: "TCP", description: "Docker REST API（SSL）" },
  { port: 3000, service: "Dev-HTTP", protocol: "TCP", description: "开发服务器 / Node.js / Grafana" },
  { port: 3306, service: "MySQL", protocol: "TCP", description: "MySQL 数据库" },
  { port: 3389, service: "RDP", protocol: "TCP", description: "远程桌面协议" },
  { port: 3443, service: "Alt-HTTPS", protocol: "TCP", description: "备用 HTTPS 端口" },
  { port: 3689, service: "DAAP", protocol: "TCP", description: "Apple 数字音频访问协议" },
  { port: 4000, service: "Dev-HTTP", protocol: "TCP", description: "开发服务器" },
  { port: 4242, service: "GOGS", protocol: "TCP", description: "Gogs Git 服务" },
  { port: 4567, service: "Sinatra", protocol: "TCP", description: "Sinatra 开发服务器" },
  { port: 5000, service: "Dev-HTTP", protocol: "TCP", description: "Flask / Node.js 开发服务器" },
  { port: 5222, service: "XMPP", protocol: "TCP", description: "XMPP 即时通讯" },
  { port: 5223, service: "XMPP-SSL", protocol: "TCP", description: "XMPP over SSL" },
  { port: 5432, service: "PostgreSQL", protocol: "TCP", description: "PostgreSQL 数据库" },
  { port: 5672, service: "AMQP", protocol: "TCP", description: "AMQP 消息队列" },
  { port: 5900, service: "VNC", protocol: "TCP", description: "虚拟网络计算（远程桌面）" },
  { port: 5901, service: "VNC-1", protocol: "TCP", description: "VNC 显示 :1" },
  { port: 5984, service: "CouchDB", protocol: "TCP", description: "CouchDB 数据库" },
  { port: 6379, service: "Redis", protocol: "TCP", description: "Redis 缓存数据库" },
  { port: 6443, service: "K8s-API", protocol: "TCP", description: "Kubernetes API Server (HTTPS)" },
  { port: 6660, service: "IRC", protocol: "TCP", description: "Internet 中继聊天" },
  { port: 6667, service: "IRC", protocol: "TCP", description: "IRC 默认端口" },
  { port: 6881, service: "BitTorrent", protocol: "UDP/TCP", description: "BitTorrent 文件共享" },
  { port: 7001, service: "WebLogic", protocol: "TCP", description: "Oracle WebLogic 管理" },
  { port: 7777, service: "Terraria", protocol: "TCP", description: "Terraria 游戏服务器" },
  { port: 8000, service: "Alt-HTTP", protocol: "TCP", description: "备用 HTTP / Django" },
  { port: 8080, service: "HTTP-Proxy", protocol: "TCP", description: "HTTP 代理 / Tomcat" },
  { port: 8443, service: "HTTPS-Alt", protocol: "TCP", description: "备用 HTTPS / Tomcat SSL" },
  { port: 8888, service: "Dev-Proxy", protocol: "TCP", description: "Jupyter Notebook / 开发代理" },
  { port: 9000, service: "Dev-Alt", protocol: "TCP", description: "PHP-FPM / SonarQube" },
  { port: 9090, service: "Prometheus", protocol: "TCP", description: "Prometheus 监控" },
  { port: 9100, service: "Node-Exporter", protocol: "TCP", description: "Prometheus Node Exporter" },
  { port: 9200, service: "Elasticsearch", protocol: "TCP", description: "Elasticsearch API" },
  { port: 9300, service: "Elasticsearch", protocol: "TCP", description: "Elasticsearch 集群通信" },
  { port: 9418, service: "Git", protocol: "TCP", description: "Git 协议" },
  { port: 9600, service: "Logstash", protocol: "TCP", description: "Logstash 监控" },
  { port: 9999, service: "Abyss", protocol: "TCP", description: "Abyss Web Server" },
  { port: 10000, service: "Webmin", protocol: "TCP", description: "Webmin 管理面板" },
  { port: 11211, service: "Memcached", protocol: "TCP/UDP", description: "Memcached 缓存系统" },
  { port: 15672, service: "RabbitMQ", protocol: "TCP", description: "RabbitMQ 管理界面" },
  { port: 17000, service: "Gradle", protocol: "TCP", description: "Gradle 构建缓存" },
  { port: 20000, service: "DNP", protocol: "TCP", description: "DNP (Distributed Network Protocol)" },
  { port: 25565, service: "Minecraft", protocol: "TCP", description: "Minecraft Java 版游戏服务器" },
  { port: 27015, service: "SRCDS", protocol: "UDP/TCP", description: "Source 引擎游戏服务器" },
  { port: 27017, service: "MongoDB", protocol: "TCP", description: "MongoDB 数据库" },
  { port: 32400, service: "Plex", protocol: "TCP", description: "Plex 媒体服务器" },
  { port: 33434, service: "Traceroute", protocol: "UDP", description: "Traceroute 默认起始端口" },
  { port: 37777, service: "RTSP-Alt", protocol: "TCP", description: "大华摄像头 RTSP" },
  { port: 50070, service: "HDFS", protocol: "TCP", description: "Hadoop HDFS NameNode" },
  { port: 60000, service: "D-Link", protocol: "TCP", description: "D-Link 设备管理" },
  { port: 65535, service: "Reserved", protocol: "TCP", description: "系统保留端口上限" },
];

function lookupPort() {
  portError.value = "";
  portResult.value = null;

  const p = portInput.value;
  if (p === null || p === undefined) {
    portError.value = "请输入端口号";
    return;
  }
  if (isNaN(p) || p < 1 || p > 65535) {
    portError.value = "端口号必须在 1-65535 之间";
    return;
  }

  const found = portDb.find((item) => item.port === p);
  if (found) {
    portResult.value = found;
  } else {
    portResult.value = {
      port: p,
      service: "未知",
      protocol: "-",
      description: "该端口在常用端口表中未找到。建议查询 IANA 官方端口登记表。",
    };
  }
}

// ─── 6. WiFi QR Generator ─────────────────────────────────────
const wifiSsid = ref("");
const wifiPassword = ref("");
const wifiEncryption = ref<"WPA" | "WPA2" | "WPA3" | "none">("WPA2");
const wifiQrDataUrl = ref("");
const wifiQrError = ref("");
const wifiQrGenerating = ref(false);

const encryptionOptions = [
  { value: "WPA" as const, label: "WPA" },
  { value: "WPA2" as const, label: "WPA2" },
  { value: "WPA3" as const, label: "WPA3" },
  { value: "none" as const, label: "无密码" },
];

async function generateWifiQr() {
  wifiQrError.value = "";
  wifiQrDataUrl.value = "";

  if (!wifiSsid.value.trim()) {
    wifiQrError.value = "请输入 WiFi 名称 (SSID)";
    return;
  }

  const ssid = wifiSsid.value.trim();
  const password = wifiPassword.value;
  const enc = wifiEncryption.value;

  let wifiString: string;
  if (enc === "none") {
    wifiString = `WIFI:T:nopass;S:${ssid};P:${password};;`;
  } else {
    wifiString = `WIFI:T:${enc};S:${ssid};P:${password};;`;
  }

  wifiQrGenerating.value = true;
  try {
    wifiQrDataUrl.value = await QRCode.toDataURL(wifiString, {
      width: 256,
      margin: 2,
      color: {
        dark: "#1e1e2e",
        light: "#ffffff",
      },
    });
  } catch (e) {
    wifiQrError.value = "QR 码生成失败：" + String(e);
  } finally {
    wifiQrGenerating.value = false;
  }
}

// ─── 7. JSON Formatter ─────────────────────────────────────────
const jsonInput = ref("");
const jsonOutput = ref("");
const jsonError = ref("");
const jsonCopied = ref(false);

function formatJson() {
  jsonError.value = "";
  jsonOutput.value = "";
  const input = jsonInput.value.trim();
  if (!input) {
    jsonError.value = "请输入 JSON";
    return;
  }
  try {
    const parsed = JSON.parse(input);
    jsonOutput.value = JSON.stringify(parsed, null, 2);
  } catch (e) {
    jsonError.value = `无效 JSON: ${e}`;
  }
}

function compressJson() {
  jsonError.value = "";
  jsonOutput.value = "";
  const input = jsonInput.value.trim();
  if (!input) {
    jsonError.value = "请输入 JSON";
    return;
  }
  try {
    const parsed = JSON.parse(input);
    jsonOutput.value = JSON.stringify(parsed);
  } catch (e) {
    jsonError.value = `无效 JSON: ${e}`;
  }
}

function copyJson() {
  if (!jsonOutput.value) return;
  navigator.clipboard.writeText(jsonOutput.value).then(() => {
    jsonCopied.value = true;
    setTimeout(() => { jsonCopied.value = false; }, 2000);
  });
}

// ─── 8. JWT Decoder ────────────────────────────────────────────
const jwtInput = ref("");
const jwtHeader = ref("");
const jwtPayload = ref("");
const jwtError = ref("");
const jwtCopied = ref(false);

function decodeJwt() {
  jwtError.value = "";
  jwtHeader.value = "";
  jwtPayload.value = "";
  const token = jwtInput.value.trim();
  if (!token) {
    jwtError.value = "请输入 JWT Token";
    return;
  }
  const parts = token.split(".");
  if (parts.length !== 3) {
    jwtError.value = "无效 JWT：需要三部分（header.payload.signature）";
    return;
  }
  try {
    // Decode header
    const headerJson = atob(base64UrlDecode(parts[0]));
    const headerParsed = JSON.parse(headerJson);
    jwtHeader.value = JSON.stringify(headerParsed, null, 2);

    // Decode payload
    const payloadJson = atob(base64UrlDecode(parts[1]));
    const payloadParsed = JSON.parse(payloadJson);
    jwtPayload.value = JSON.stringify(payloadParsed, null, 2);
  } catch (e) {
    jwtError.value = `解码失败: ${e}`;
  }
}

function base64UrlDecode(str: string): string {
  // Replace URL-safe chars and pad
  let base64 = str.replace(/-/g, "+").replace(/_/g, "/");
  while (base64.length % 4 !== 0) {
    base64 += "=";
  }
  return base64;
}

function jwtHasExpiry(): string | null {
  try {
    if (!jwtPayload.value) return null;
    const payload = JSON.parse(jwtPayload.value);
    if (payload.exp) {
      const expDate = new Date(payload.exp * 1000);
      const now = new Date();
      const diff = expDate.getTime() - now.getTime();
      const isExpired = diff < 0;
      const relative = isExpired
        ? `已过期 ${Math.abs(Math.round(diff / 1000 / 60))} 分钟前`
        : `剩余 ${Math.round(diff / 1000 / 60)} 分钟`;
      return `${expDate.toLocaleString()} (${relative})`;
    }
    return "无过期时间 (exp)";
  } catch {
    return null;
  }
}

function copyJwt() {
  const header = jwtHeader.value;
  const payload = jwtPayload.value;
  if (!header && !payload) return;
  const text = header && payload
    ? `Header:\n${header}\n\nPayload:\n${payload}`
    : header
      ? `Header:\n${header}`
      : `Payload:\n${payload}`;
  navigator.clipboard.writeText(text).then(() => {
    jwtCopied.value = true;
    setTimeout(() => { jwtCopied.value = false; }, 2000);
  });
}

// ─── 9. Timestamp Converter ────────────────────────────────────
const tsInput = ref<number | null>(null);
const tsResult = ref("");
const tsRelative = ref("");
const tsError = ref("");

function convertTimestamp() {
  tsError.value = "";
  tsResult.value = "";
  tsRelative.value = "";

  const val = tsInput.value;
  if (val === null || val === undefined || (typeof val === 'number' && isNaN(val))) {
    tsError.value = "请输入有效的时间戳";
    return;
  }

  let ms: number;
  // Auto-detect: if > 1e11, treat as milliseconds
  if (val > 1e11) {
    ms = val;
  } else {
    ms = val * 1000;
  }

  const date = new Date(ms);
  if (isNaN(date.getTime())) {
    tsError.value = "无效的时间戳";
    return;
  }

  // Format: YYYY-MM-DD HH:mm:ss in local timezone
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  const hours = String(date.getHours()).padStart(2, "0");
  const minutes = String(date.getMinutes()).padStart(2, "0");
  const seconds = String(date.getSeconds()).padStart(2, "0");
  tsResult.value = `${year}-${month}-${day} ${hours}:${minutes}:${seconds}`;

  // Relative time
  const now = Date.now();
  const diffMs = ms - now;
  const absDiffMs = Math.abs(diffMs);
  const secondsDiff = Math.round(absDiffMs / 1000);
  const minutesDiff = Math.round(secondsDiff / 60);
  const hoursDiff = Math.round(minutesDiff / 60);
  const daysDiff = Math.round(hoursDiff / 24);

  let relative = "";
  if (diffMs > 0) {
    if (daysDiff > 0) relative = `${daysDiff} 天后`;
    else if (hoursDiff > 0) relative = `${hoursDiff} 小时后`;
    else if (minutesDiff > 0) relative = `${minutesDiff} 分钟后`;
    else relative = `${secondsDiff} 秒后`;
  } else {
    if (daysDiff > 0) relative = `${daysDiff} 天前`;
    else if (hoursDiff > 0) relative = `${hoursDiff} 小时前`;
    else if (minutesDiff > 0) relative = `${minutesDiff} 分钟前`;
    else relative = `${secondsDiff} 秒前`;
  }
  tsRelative.value = relative;
}

</script>

<template>
  <div class="flex h-full animate-view-fade">
    <!-- Left: Tool tabs -->
    <div class="flex w-44 shrink-0 flex-col border-r border-paper-deep/50 bg-paper-warm/20 p-2">
      <div class="mb-3 px-2 pt-2">
        <h2 class="text-xs font-semibold uppercase tracking-wider text-ink-faint">工具箱</h2>
      </div>
      <nav class="flex flex-col gap-0.5">
        <button
          v-for="tab in tabs"
          :key="tab.key"
          class="flex items-center gap-2.5 rounded-lg px-3 py-2 text-sm font-medium transition-colors text-left"
          :class="
            activeTab === tab.key
              ? 'bg-bamboo/10 text-bamboo'
              : 'text-ink-soft hover:bg-paper-deep/50 hover:text-ink'
          "
          @click="activeTab = tab.key"
        >
          <component :is="tab.icon" class="h-4 w-4 shrink-0" />
          <span>{{ tab.label }}</span>
        </button>
      </nav>
    </div>

    <!-- Right: Tool content -->
    <div class="flex-1 overflow-y-auto p-6">
      <!-- Subnet Calculator -->
      <div v-if="activeTab === 'subnet'" class="max-w-xl">
        <h3 class="text-lg font-display font-bold text-ink">子网计算器</h3>
        <p class="mt-1 text-sm text-ink-faint">输入 IP 地址和 CIDR 前缀长度，计算子网信息。支持 IPv4/IPv6。</p>

        <div class="mt-5 space-y-4">
          <!-- IP Version Toggle -->
          <div class="flex w-fit rounded-lg border border-paper-deep/30 bg-paper-warm/30 p-1">
            <button
              class="rounded-md px-4 py-1.5 text-sm font-medium transition-colors"
              :class="subnetIpVersion === 'IPv4' ? 'bg-bamboo/15 text-bamboo' : 'text-ink-soft hover:text-ink'"
              @click="subnetIpVersion = 'IPv4'"
            >
              IPv4
            </button>
            <button
              class="rounded-md px-4 py-1.5 text-sm font-medium transition-colors"
              :class="subnetIpVersion === 'IPv6' ? 'bg-bamboo/15 text-bamboo' : 'text-ink-soft hover:text-ink'"
              @click="subnetIpVersion = 'IPv6'"
            >
              IPv6
            </button>
          </div>

          <div>
            <label class="mb-1.5 block text-xs font-medium text-ink-soft">IP 地址</label>
            <input
              v-model="subnetIp"
              type="text"
              :placeholder="subnetIpVersion === 'IPv4' ? '如 192.168.1.0' : '如 2001:db8::'"
              class="w-full rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/80"
            />
          </div>
          <div>
            <label class="mb-1.5 block text-xs font-medium text-ink-soft">
              CIDR 前缀长度
              <span class="ml-1 text-ink-faint">{{ cidrHint }}</span>
            </label>
            <input
              v-model.number="subnetCidr"
              type="number"
              :min="0"
              :max="cidrMax"
              class="w-full rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/80"
            />
          </div>
          <Button @click="doCalculateSubnet">计算</Button>

          <p v-if="subnetError" class="text-sm text-red-500">{{ subnetError }}</p>

          <div
            v-if="subnetResult"
            class="mt-4 space-y-3 rounded-xl border border-paper-deep/20 bg-paper-warm/30 p-5"
          >
            <!-- Classification badge -->
            <div class="mb-3 flex items-center gap-2">
              <span class="text-xs font-medium text-ink-soft">分类：</span>
              <span
                :class="classificationBadgeClass"
                class="inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium"
              >
                {{ subnetResult.classification.description }}
              </span>
            </div>

            <div class="flex items-center justify-between border-b border-paper-deep/10 pb-2">
              <span class="text-xs font-medium text-ink-soft">网络地址</span>
              <span class="text-sm font-mono text-ink">{{ subnetResult.networkAddress }}/{{ subnetResult.cidr }}</span>
            </div>
            <div class="flex items-center justify-between">
              <span class="text-xs font-medium text-ink-soft">子网掩码</span>
              <span class="text-sm font-mono text-ink">{{ subnetResult.subnetMask }}</span>
            </div>
            <div v-if="subnetResult.broadcastAddress" class="flex items-center justify-between">
              <span class="text-xs font-medium text-ink-soft">广播地址</span>
              <span class="text-sm font-mono text-ink">{{ subnetResult.broadcastAddress }}</span>
            </div>
            <div class="flex items-center justify-between">
              <span class="text-xs font-medium text-ink-soft">通配符掩码</span>
              <span class="text-sm font-mono text-ink">{{ subnetResult.wildcardMask }}</span>
            </div>
            <div class="flex items-center justify-between">
              <span class="text-xs font-medium text-ink-soft">可用主机数</span>
              <span class="text-sm font-mono text-ink">{{ formatUsableHosts(subnetResult.usableHosts) }}</span>
            </div>
            <div class="flex items-center justify-between">
              <span class="text-xs font-medium text-ink-soft">IP 范围</span>
              <span class="text-sm font-mono text-ink break-all">{{ subnetResult.ipRange }}</span>
            </div>
          </div>

          <!-- Subnet Splitting Panel -->
          <div v-if="subnetResult" class="mt-6 border-t border-paper-deep/20 pt-4">
            <button
              class="flex items-center gap-2 text-sm font-medium text-ink-soft hover:text-ink transition-colors"
              @click="subnetSplitOpen = !subnetSplitOpen"
            >
              <span
                class="text-xs transition-transform"
                :class="subnetSplitOpen ? 'rotate-90' : ''"
              >▶</span>
              子网划分
            </button>

            <div v-if="subnetSplitOpen" class="mt-3 space-y-4">
              <div>
                <label class="mb-1.5 block text-xs font-medium text-ink-soft">
                  目标前缀长度
                </label>
                <input
                  v-model.number="subnetTargetPrefix"
                  type="number"
                  :min="subnetCidr + 1"
                  :max="cidrMax"
                  class="w-full rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/80"
                />
              </div>
              <Button variant="outline" @click="doSplitSubnet">划分子网</Button>

              <p v-if="subnetSplitError" class="text-sm text-red-500">{{ subnetSplitError }}</p>

              <div v-if="subnetSplitResult" class="space-y-3">
                <div class="text-sm text-ink-soft">
                  共 {{ subnetSplitResult.subnets.length }} 个子网，
                  总计 {{ formatUsableHosts(subnetSplitResult.totalUsable) }} 个可用地址
                </div>
                <div class="overflow-x-auto rounded-xl border border-paper-deep/20">
                  <table class="w-full text-sm">
                    <thead>
                      <tr class="bg-paper-deep/10 text-ink-soft text-xs uppercase tracking-wider">
                        <th class="px-3 py-2 text-left">子网</th>
                        <th class="px-3 py-2 text-left">掩码</th>
                        <th class="px-3 py-2 text-left">可用主机数</th>
                        <th class="px-3 py-2 text-left">IP 范围</th>
                      </tr>
                    </thead>
                    <tbody>
                      <tr
                        v-for="(sub, idx) in subnetSplitResult.subnets"
                        :key="idx"
                        class="border-t border-paper-deep/10"
                      >
                        <td class="px-3 py-2 font-mono text-ink">{{ sub.networkAddress }}/{{ sub.cidr }}</td>
                        <td class="px-3 py-2 font-mono text-ink">{{ sub.subnetMask }}</td>
                        <td class="px-3 py-2 font-mono text-ink">{{ formatUsableHosts(sub.usableHosts) }}</td>
                        <td class="px-3 py-2 font-mono text-ink break-all">{{ sub.ipRange }}</td>
                      </tr>
                    </tbody>
                  </table>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Base64 -->
      <div v-if="activeTab === 'base64'" class="max-w-xl">
        <h3 class="text-lg font-display font-bold text-ink">Base64 编解码</h3>
        <p class="mt-1 text-sm text-ink-faint">对文本进行 Base64 编码或解码。支持自动检测。</p>

        <div class="mt-5 space-y-4">
          <div>
            <label class="mb-1.5 block text-xs font-medium text-ink-soft">输入</label>
            <textarea
              v-model="base64Input"
              rows="4"
              placeholder="输入要编码或解码的文本..."
              class="w-full rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/80 resize-y"
            />
          </div>
          <div class="flex gap-2">
            <Button @click="detectAndConvertBase64">自动转换</Button>
            <Button variant="outline" @click="base64Encode">编码</Button>
            <Button variant="outline" @click="base64Decode">解码</Button>
          </div>

          <p v-if="base64Error" class="text-sm text-red-500">{{ base64Error }}</p>

          <div v-if="base64Output">
            <div class="mb-1.5 flex items-center justify-between">
              <label class="text-xs font-medium text-ink-soft">{{ base64Mode === 'encode' ? '编码结果' : '解码结果' }}</label>
              <button
                class="flex items-center gap-1 rounded-md px-2 py-1 text-xs transition-colors"
                :class="base64Copied ? 'text-bamboo bg-bamboo/10' : 'text-ink-faint hover:text-ink hover:bg-paper-deep/30'"
                @click="copyBase64"
              >
                <Copy v-if="!base64Copied" class="h-3 w-3" />
                <Check v-else class="h-3 w-3" />
                {{ base64Copied ? '已复制' : '复制' }}
              </button>
            </div>
            <div class="rounded-xl border border-paper-deep/20 bg-paper-warm/30 p-4">
              <pre class="whitespace-pre-wrap break-all text-sm text-ink font-mono">{{ base64Output }}</pre>
            </div>
          </div>
        </div>
      </div>

      <!-- URL -->
      <div v-if="activeTab === 'url'" class="max-w-xl">
        <h3 class="text-lg font-display font-bold text-ink">URL 编解码</h3>
        <p class="mt-1 text-sm text-ink-faint">对 URL 或文本进行百分比编码/解码。支持 UTF-8 中文。</p>

        <div class="mt-5 space-y-4">
          <div>
            <label class="mb-1.5 block text-xs font-medium text-ink-soft">输入</label>
            <textarea
              v-model="urlInput"
              rows="4"
              placeholder="输入 URL 或文本..."
              class="w-full rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/80 resize-y"
            />
          </div>
          <div class="flex gap-2">
            <Button @click="detectAndConvertUrl">自动转换</Button>
            <Button variant="outline" @click="urlEncode">编码</Button>
            <Button variant="outline" @click="urlDecode">解码</Button>
          </div>

          <p v-if="urlError" class="text-sm text-red-500">{{ urlError }}</p>

          <div v-if="urlOutput">
            <div class="mb-1.5 flex items-center justify-between">
              <label class="text-xs font-medium text-ink-soft">{{ urlMode === 'encode' ? '编码结果' : '解码结果' }}</label>
              <button
                class="flex items-center gap-1 rounded-md px-2 py-1 text-xs transition-colors"
                :class="urlCopied ? 'text-bamboo bg-bamboo/10' : 'text-ink-faint hover:text-ink hover:bg-paper-deep/30'"
                @click="copyUrl"
              >
                <Copy v-if="!urlCopied" class="h-3 w-3" />
                <Check v-else class="h-3 w-3" />
                {{ urlCopied ? '已复制' : '复制' }}
              </button>
            </div>
            <div class="rounded-xl border border-paper-deep/20 bg-paper-warm/30 p-4">
              <pre class="whitespace-pre-wrap break-all text-sm text-ink font-mono">{{ urlOutput }}</pre>
            </div>
          </div>
        </div>
      </div>

      <!-- Hash Generator -->
      <div v-if="activeTab === 'hash'" class="max-w-xl">
        <h3 class="text-lg font-display font-bold text-ink">Hash 生成器</h3>
        <p class="mt-1 text-sm text-ink-faint">使用 Web Crypto API 计算文本的哈希值。</p>

        <div class="mt-5 space-y-4">
          <div>
            <label class="mb-1.5 block text-xs font-medium text-ink-soft">输入文本</label>
            <textarea
              v-model="hashInput"
              rows="4"
              placeholder="输入要计算哈希的文本..."
              class="w-full rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/80 resize-y"
            />
          </div>
          <div>
            <label class="mb-1.5 block text-xs font-medium text-ink-soft">算法</label>
            <div class="flex flex-wrap gap-2">
              <button
                v-for="opt in algorithmOptions"
                :key="opt.value"
                class="rounded-lg px-3 py-1.5 text-sm font-medium transition-colors"
                :class="hashAlgorithm === opt.value ? 'bg-bamboo/15 text-bamboo ring-1 ring-bamboo/30' : 'bg-paper-deep/20 text-ink-soft hover:bg-paper-deep/40 hover:text-ink'"
                @click="hashAlgorithm = opt.value"
              >
                {{ opt.label }}
              </button>
            </div>
          </div>
          <Button :disabled="hashComputing" @click="computeHash">
            {{ hashComputing ? '计算中...' : '计算哈希' }}
          </Button>

          <p v-if="hashError" class="text-sm text-red-500">{{ hashError }}</p>

          <div v-if="hashResult">
            <div class="mb-1.5 flex items-center justify-between">
              <label class="text-xs font-medium text-ink-soft">{{ algDisplayName }} 结果</label>
              <button
                class="flex items-center gap-1 rounded-md px-2 py-1 text-xs transition-colors"
                :class="hashCopied ? 'text-bamboo bg-bamboo/10' : 'text-ink-faint hover:text-ink hover:bg-paper-deep/30'"
                @click="copyHash"
              >
                <Copy v-if="!hashCopied" class="h-3 w-3" />
                <Check v-else class="h-3 w-3" />
                {{ hashCopied ? '已复制' : '复制' }}
              </button>
            </div>
            <div class="rounded-xl border border-paper-deep/20 bg-paper-warm/30 p-4">
              <pre class="whitespace-pre-wrap break-all text-sm text-ink font-mono select-all">{{ hashResult }}</pre>
            </div>
          </div>
        </div>
      </div>

      <!-- WiFi QR Generator -->
      <div v-if="activeTab === 'wifi'" class="max-w-xl">
        <h3 class="text-lg font-display font-bold text-ink">WiFi QR 生成器</h3>
        <p class="mt-1 text-sm text-ink-faint">生成 WiFi 连接二维码，手机扫码即可连接。</p>

        <div class="mt-5 space-y-4">
          <div>
            <label class="mb-1.5 block text-xs font-medium text-ink-soft">WiFi 名称 (SSID)</label>
            <input
              v-model="wifiSsid"
              type="text"
              placeholder="输入 WiFi 名称..."
              class="w-full rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/80"
            />
          </div>
          <div>
            <label class="mb-1.5 block text-xs font-medium text-ink-soft">密码</label>
            <input
              v-model="wifiPassword"
              type="text"
              placeholder="输入 WiFi 密码（可选）"
              class="w-full rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/80"
            />
          </div>
          <div>
            <label class="mb-1.5 block text-xs font-medium text-ink-soft">加密类型</label>
            <div class="flex flex-wrap gap-2">
              <button
                v-for="opt in encryptionOptions"
                :key="opt.value"
                class="rounded-lg px-3 py-1.5 text-sm font-medium transition-colors"
                :class="wifiEncryption === opt.value ? 'bg-bamboo/15 text-bamboo ring-1 ring-bamboo/30' : 'bg-paper-deep/20 text-ink-soft hover:bg-paper-deep/40 hover:text-ink'"
                @click="wifiEncryption = opt.value"
              >
                {{ opt.label }}
              </button>
            </div>
          </div>
          <Button :disabled="wifiQrGenerating" @click="generateWifiQr">
            {{ wifiQrGenerating ? '生成中...' : '生成二维码' }}
          </Button>

          <p v-if="wifiQrError" class="text-sm text-red-500">{{ wifiQrError }}</p>

          <div v-if="wifiQrDataUrl" class="mt-4 space-y-4">
            <div class="flex justify-center">
              <div class="inline-block rounded-xl border border-paper-deep/20 bg-white p-4 shadow-sm">
                <img :src="wifiQrDataUrl" alt="WiFi QR Code" class="h-48 w-48" />
              </div>
            </div>
            <div class="rounded-xl border border-paper-deep/20 bg-paper-warm/30 p-4 text-sm text-ink-soft">
              <p>使用手机相机或 WiFi 扫码工具扫描上方二维码即可连接 WiFi。</p>
              <p class="mt-1 text-xs text-ink-faint">
                网络: {{ wifiSsid.trim() }}
                <template v-if="wifiEncryption !== 'none'"> · 加密: {{ wifiEncryption }}</template>
                <template v-else> · 无密码</template>
              </p>
            </div>
          </div>
        </div>
      </div>

      <!-- Port Lookup -->
      <div v-if="activeTab === 'port'" class="max-w-xl">
        <h3 class="text-lg font-display font-bold text-ink">端口号速查</h3>
        <p class="mt-1 text-sm text-ink-faint">查询常用端口号对应的服务名称、协议和描述信息。</p>

        <div class="mt-5 space-y-4">
          <div>
            <label class="mb-1.5 block text-xs font-medium text-ink-soft">端口号</label>
            <input
              v-model.number="portInput"
              type="number"
              min="1"
              max="65535"
              placeholder="如 443, 3306, 8080..."
              class="w-full rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/80"
              @keyup.enter="lookupPort"
            />
          </div>
          <Button @click="lookupPort">查询</Button>

          <p v-if="portError" class="text-sm text-red-500">{{ portError }}</p>

          <div
            v-if="portResult"
            class="mt-4 space-y-3 rounded-xl border border-paper-deep/20 bg-paper-warm/30 p-5"
          >
            <div class="flex items-center justify-between">
              <span class="text-xs font-medium text-ink-soft">端口</span>
              <span class="text-sm font-mono font-bold text-ink">{{ portResult.port }}</span>
            </div>
            <div class="flex items-center justify-between">
              <span class="text-xs font-medium text-ink-soft">服务</span>
              <span class="text-sm font-mono text-ink">{{ portResult.service }}</span>
            </div>
            <div class="flex items-center justify-between">
              <span class="text-xs font-medium text-ink-soft">协议</span>
              <span class="text-sm font-mono text-ink">{{ portResult.protocol }}</span>
            </div>
            <div class="flex items-center justify-between border-t border-paper-deep/10 pt-3">
              <span class="text-xs font-medium text-ink-soft">描述</span>
              <span class="text-sm text-right text-ink max-w-xs">{{ portResult.description }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- JSON Formatter -->
      <div v-if="activeTab === 'json'" class="max-w-xl">
        <h3 class="text-lg font-display font-bold text-ink">JSON 格式化</h3>
        <p class="mt-1 text-sm text-ink-faint">格式化、压缩和验证 JSON 数据。</p>

        <div class="mt-5 space-y-4">
          <div>
            <label class="mb-1.5 block text-xs font-medium text-ink-soft">输入 JSON</label>
            <textarea
              v-model="jsonInput"
              rows="6"
              placeholder='{"key": "value"}'
              class="w-full rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/80 resize-y font-mono"
            />
          </div>
          <div class="flex gap-2">
            <Button @click="formatJson">格式化</Button>
            <Button variant="outline" @click="compressJson">压缩</Button>
          </div>

          <p v-if="jsonError" class="text-sm text-red-500">{{ jsonError }}</p>

          <div v-if="jsonOutput">
            <div class="mb-1.5 flex items-center justify-between">
              <label class="text-xs font-medium text-ink-soft">结果</label>
              <button
                class="flex items-center gap-1 rounded-md px-2 py-1 text-xs transition-colors"
                :class="jsonCopied ? 'text-bamboo bg-bamboo/10' : 'text-ink-faint hover:text-ink hover:bg-paper-deep/30'"
                @click="copyJson"
              >
                <Copy v-if="!jsonCopied" class="h-3 w-3" />
                <Check v-else class="h-3 w-3" />
                {{ jsonCopied ? '已复制' : '复制' }}
              </button>
            </div>
            <div class="rounded-xl border border-paper-deep/20 bg-paper-warm/30 p-4 max-h-96 overflow-auto">
              <pre class="whitespace-pre-wrap break-all text-sm text-ink font-mono">{{ jsonOutput }}</pre>
            </div>
          </div>
        </div>
      </div>

      <!-- JWT Decoder -->
      <div v-if="activeTab === 'jwt'" class="max-w-xl">
        <h3 class="text-lg font-display font-bold text-ink">JWT 解码器</h3>
        <p class="mt-1 text-sm text-ink-faint">解码 JSON Web Token 的 Header 和 Payload。</p>

        <div class="mt-5 space-y-4">
          <div>
            <label class="mb-1.5 block text-xs font-medium text-ink-soft">JWT Token</label>
            <textarea
              v-model="jwtInput"
              rows="3"
              placeholder="eyJhbGciOiJIUzI1NiIs..."
              class="w-full rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/80 resize-y font-mono"
            />
          </div>
          <Button @click="decodeJwt">解码</Button>

          <p v-if="jwtError" class="text-sm text-red-500">{{ jwtError }}</p>

          <div v-if="jwtHeader">
            <div class="mb-1.5 flex items-center justify-between">
              <label class="text-xs font-medium text-ink-soft">Header</label>
              <button
                class="flex items-center gap-1 rounded-md px-2 py-1 text-xs transition-colors"
                :class="jwtCopied ? 'text-bamboo bg-bamboo/10' : 'text-ink-faint hover:text-ink hover:bg-paper-deep/30'"
                @click="copyJwt"
              >
                <Copy v-if="!jwtCopied" class="h-3 w-3" />
                <Check v-else class="h-3 w-3" />
                {{ jwtCopied ? '已复制' : '复制' }}
              </button>
            </div>
            <div class="rounded-xl border border-paper-deep/20 bg-paper-warm/30 p-4 mb-4">
              <pre class="whitespace-pre-wrap break-all text-sm text-ink font-mono">{{ jwtHeader }}</pre>
            </div>

            <div class="mb-1.5 flex items-center justify-between">
              <label class="text-xs font-medium text-ink-soft">Payload</label>
            </div>
            <div class="rounded-xl border border-paper-deep/20 bg-paper-warm/30 p-4">
              <pre class="whitespace-pre-wrap break-all text-sm text-ink font-mono">{{ jwtPayload }}</pre>
            </div>

            <div v-if="jwtPayload" class="mt-3 rounded-lg border border-paper-deep/20 bg-paper-warm/30 px-4 py-3 text-sm">
              <span class="text-xs font-medium text-ink-soft">过期信息：</span>
              <span class="text-ink">{{ jwtHasExpiry() || '解析失败' }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Timestamp Converter -->
      <div v-if="activeTab === 'timestamp'" class="max-w-xl">
        <h3 class="text-lg font-display font-bold text-ink">时间戳转换</h3>
        <p class="mt-1 text-sm text-ink-faint">将 Unix 时间戳转换为可读的日期时间格式。支持秒和毫秒。</p>

        <div class="mt-5 space-y-4">
          <div>
            <label class="mb-1.5 block text-xs font-medium text-ink-soft">Unix 时间戳</label>
            <input
              v-model.number="tsInput"
              type="number"
              placeholder="如 1700000000 或 1700000000000"
              class="w-full rounded-lg border border-paper-deep/40 bg-paper-warm/50 px-3 py-2 text-sm text-ink outline-none transition-colors placeholder:text-ink-faint/40 focus:border-bamboo/40 focus:bg-paper-warm/80 font-mono"
              @keyup.enter="convertTimestamp"
            />
          </div>
          <Button @click="convertTimestamp">转换</Button>

          <p v-if="tsError" class="text-sm text-red-500">{{ tsError }}</p>

          <div v-if="tsResult" class="space-y-3 rounded-xl border border-paper-deep/20 bg-paper-warm/30 p-5">
            <div class="flex items-center justify-between">
              <span class="text-xs font-medium text-ink-soft">本地时间</span>
              <span class="text-sm font-mono text-ink">{{ tsResult }}</span>
            </div>
            <div class="flex items-center justify-between">
              <span class="text-xs font-medium text-ink-soft">相对时间</span>
              <span class="text-sm text-ink">{{ tsRelative }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
