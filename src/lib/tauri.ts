import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// ============================================================
// TypeScript interfaces matching Rust backend types
// ============================================================

export interface PingOptions {
  count: number;
  intervalMs: number;
  timeoutMs: number;
  payloadSize: number;
}

export interface TraceOptions {
  maxHops: number;
  timeoutMs: number;
  probesPerHop: number;
}

export interface PortRange {
  start: number;
  end: number;
}

export interface ScanOptions {
  concurrency: number;
  timeoutMs: number;
}

export type RecordType = "a" | "aaaa" | "cname" | "mx" | "ns" | "soa" | "txt" | "all";

// ============================================================
// Event payload types (snake_case matches Rust serialization)
// ============================================================

export interface PingProgressPayload {
  task_id: string;
  seq: number;
  ttl: number;
  latency_ms: number | null;
  status: string;
}

export interface PingCompletePayload {
  task_id: string;
  sent: number;
  received: number;
  loss_percent: number;
  min_ms: number;
  avg_ms: number;
  max_ms: number;
}

export interface PingErrorPayload {
  task_id: string;
  error: string;
}

export interface TraceHop {
  hop: number;
  addr: string | null;
  hostname: string | null;
  latencies: (number | null)[];
}

export interface TraceHopPayload {
  hop: number;
  addr: string | null;
  hostname: string | null;
  latencies: (number | null)[];
}

export interface TraceCompletePayload {
  task_id: string;
  target: string;
  hops: TraceHop[];
}

export interface TraceErrorPayload {
  task_id: string;
  error: string;
}

// ============================================================
// MTR (My TraceRoute) — combines traceroute + continuous ping
// ============================================================

export interface MtrOptions {
  target: string;
  maxHops: number;
  intervalMs: number;
  timeoutMs: number;
}

export interface MtrHopStats {
  hop: number;
  addr: string | null;
  hostname: string | null;
  sent: number;
  received: number;
  lossPercent: number;
  minMs: number;
  avgMs: number;
  maxMs: number;
  jitterMs: number;
  lastMs: number | null;
}

export interface MtrProgressPayload {
  target: string;
  totalHops: number;
  round: number;
  hops: MtrHopStats[];
}

export interface MtrCompletePayload {
  target: string;
  totalRounds: number;
  hops: MtrHopStats[];
}

export interface MtrErrorPayload {
  task_id: string;
  error: string;
}

export function mtrStart(options: MtrOptions): Promise<string> {
  return invoke<string>("mtr_start", { options });
}

export function mtrStop(taskId: string): Promise<void> {
  return invoke<void>("mtr_stop", { taskId });
}

export function onMtrProgress(cb: (payload: MtrProgressPayload) => void): Promise<UnlistenFn> {
  return listen<MtrProgressPayload>("mtr:progress", (event) => cb(event.payload));
}

export function onMtrComplete(cb: (payload: MtrCompletePayload) => void): Promise<UnlistenFn> {
  return listen<MtrCompletePayload>("mtr:complete", (event) => cb(event.payload));
}

export function onMtrError(cb: (payload: MtrErrorPayload) => void): Promise<UnlistenFn> {
  return listen<MtrErrorPayload>("mtr:error", (event) => cb(event.payload));
}

// ============================================================
// Phase 2 — LAN Discovery + Chat + File Transfer
// ============================================================

export interface PortErrorPayload {
  task_id: string;
  error: string;
}

export interface PortProgressPayload {
  task_id: string;
  scanned: number;
  total: number;
  open: number;
}

export interface PortFoundPayload {
  task_id: string;
  port: number;
  service: string | null;
}

export interface PortCompletePayload {
  task_id: string;
  target: string;
  open_ports: { port: number; service: string | null }[];
}

export interface DnsRecord {
  name: string;
  type: string;
  value: string;
  ttl: number;
}

export interface DnsResultPayload {
  task_id: string;
  target: string;
  records: DnsRecord[];
}

export interface DnsErrorPayload {
  task_id: string;
  target: string;
  error: string;
}

// ============================================================
// Invoke wrappers
// ============================================================

/**
 * Start a ping task against the given target.
 * Returns the task_id UUID.
 */
export function pingStart(target: string, options?: PingOptions): Promise<string> {
  return invoke<string>("ping_start", {
    target,
    ...(options !== undefined ? { options } : {}),
  });
}

/**
 * Stop a running ping task.
 */
export function pingStop(taskId: string): Promise<void> {
  return invoke<void>("ping_stop", { taskId });
}

/**
 * Start a traceroute task.
 * Returns the task_id UUID.
 */
export function tracerouteStart(target: string, options?: TraceOptions): Promise<string> {
  return invoke<string>("traceroute_start", {
    target,
    ...(options !== undefined ? { options } : {}),
  });
}

/**
 * Stop a running traceroute task.
 */
export function tracerouteStop(taskId: string): Promise<void> {
  return invoke<void>("traceroute_stop", { taskId });
}

/**
 * Start a port scan task.
 * Returns the task_id UUID.
 */
export function portScanStart(target: string, portRange: PortRange, options?: ScanOptions): Promise<string> {
  return invoke<string>("port_scan_start", {
    target,
    portRange,
    ...(options !== undefined ? { options } : {}),
  });
}

/**
 * Stop a running port scan task.
 */
export function portScanStop(taskId: string): Promise<void> {
  return invoke<void>("port_scan_stop", { taskId });
}

/**
 * Perform a DNS lookup.
 * Returns the parsed DNS records directly.
 * Optionally specify a custom DNS server (ip:port format, defaults to 8.8.8.8:53).
 */
export function dnsLookup(target: string, recordType: RecordType, dnsServer?: string): Promise<DnsRecord[]> {
  return invoke<string>("dns_lookup", {
    target,
    recordType,
    ...(dnsServer !== undefined ? { dnsServer } : {}),
  }).then(
    (result) => JSON.parse(result) as DnsRecord[],
  );
}

// ============================================================
// Event listener wrappers
// ============================================================

export function onPingProgress(cb: (payload: PingProgressPayload) => void): Promise<UnlistenFn> {
  return listen<PingProgressPayload>("ping:progress", (event) => cb(event.payload));
}

export function onPingComplete(cb: (payload: PingCompletePayload) => void): Promise<UnlistenFn> {
  return listen<PingCompletePayload>("ping:complete", (event) => cb(event.payload));
}

export function onPingError(cb: (payload: PingErrorPayload) => void): Promise<UnlistenFn> {
  return listen<PingErrorPayload>("ping:error", (event) => cb(event.payload));
}

export function onTraceHop(cb: (payload: TraceHopPayload) => void): Promise<UnlistenFn> {
  return listen<TraceHopPayload>("trace:hop", (event) => cb(event.payload));
}

export function onTraceComplete(cb: (payload: TraceCompletePayload) => void): Promise<UnlistenFn> {
  return listen<TraceCompletePayload>("trace:complete", (event) => cb(event.payload));
}

export function onTraceError(cb: (payload: TraceErrorPayload) => void): Promise<UnlistenFn> {
  return listen<TraceErrorPayload>("trace:error", (event) => cb(event.payload));
}

export function onPortProgress(cb: (payload: PortProgressPayload) => void): Promise<UnlistenFn> {
  return listen<PortProgressPayload>("port:progress", (event) => cb(event.payload));
}

export function onPortFound(cb: (payload: PortFoundPayload) => void): Promise<UnlistenFn> {
  return listen<PortFoundPayload>("port:found", (event) => cb(event.payload));
}

export function onPortComplete(cb: (payload: PortCompletePayload) => void): Promise<UnlistenFn> {
  return listen<PortCompletePayload>("port:complete", (event) => cb(event.payload));
}

export function onPortError(cb: (payload: PortErrorPayload) => void): Promise<UnlistenFn> {
  return listen<PortErrorPayload>("port:error", (event) => cb(event.payload));
}

export function onDnsResult(cb: (payload: DnsResultPayload) => void): Promise<UnlistenFn> {
  return listen<DnsResultPayload>("dns:result", (event) => cb(event.payload));
}

export function onDnsError(cb: (payload: DnsErrorPayload) => void): Promise<UnlistenFn> {
  return listen<DnsErrorPayload>("dns:error", (event) => cb(event.payload));
}

// ============================================================
// Phase 2 — LAN Discovery + Chat + File Transfer
// ============================================================

export interface PeerInfo {
  id: string;
  hostname: string;
  ip: string;
  os: string;
  listen_port: number;
  last_seen: string;
  status: string;
}

export interface StoredMessage {
  id: string;
  peer_id: string;
  peer_name: string;
  peer_ip: string;
  peer_os: string | null;
  content: string;
  is_broadcast: boolean;
  is_incoming: boolean;
  file_ref: string | null;
  created_at: string;
}

export interface FileTransfer {
  id: string;
  filename: string;
  path: string | null;
  size: number;
  received: number;
  status: string;
  peer_id: string;
  is_incoming: boolean;
  created_at: string;
  download_url?: string;
}

/** Initialize all LAN services. */
export function lanInit(): Promise<void> {
  return invoke<void>("lan_init");
}

/** Shutdown all LAN services. */
export function lanShutdown(): Promise<void> {
  return invoke<void>("lan_shutdown");
}

/** Get discovered peers. */
export function discoveryPeers(): Promise<PeerInfo[]> {
  return invoke<PeerInfo[]>("discovery_peers");
}

/** Send a chat message to a specific peer. */
export function chatSend(target: string, content: string): Promise<StoredMessage> {
  return invoke<StoredMessage>("chat_send", { target, content });
}

/** Broadcast a chat message to all connected peers. */
export function chatBroadcast(content: string): Promise<StoredMessage> {
  return invoke<StoredMessage>("chat_broadcast", { content });
}

/** Get messages from chat history, optionally filtered by peer. */
export function chatMessages(peerId?: string): Promise<StoredMessage[]> {
  return invoke<StoredMessage[]>("chat_messages", { peerId });
}

/** Get recent chat history. */
export function chatHistory(limit?: number): Promise<StoredMessage[]> {
  return invoke<StoredMessage[]>("chat_history", { limit });
}

export interface FileSendResult {
  file_id: string;
  file_size: number;
  download_url?: string;
}

/** Send a file to a peer. Returns the file transfer ID and size. */
export function fileSend(target: string, path: string): Promise<FileSendResult> {
  return invoke<FileSendResult>("file_send", { target, path });
}

/** Broadcast a file to all connected peers. Returns the broadcast file ID and size. */
export function fileBroadcast(path: string): Promise<FileSendResult> {
  return invoke<FileSendResult>("file_broadcast", { path });
}

/** Accept an incoming file transfer. */
export function fileAccept(fileId: string): Promise<void> {
  return invoke<void>("file_accept", { fileId });
}

/** Reject an incoming file transfer. */
export function fileReject(fileId: string): Promise<void> {
  return invoke<void>("file_reject", { fileId });
}

/** List all file transfers. */
export function fileList(): Promise<FileTransfer[]> {
  return invoke<FileTransfer[]>("file_list");
}

/** Get download URL for a completed file transfer. */
export function getFileDownloadUrl(fileId: string): Promise<string> {
  return invoke<string>("get_file_download_url", { fileId });
}

// Event listeners
export function onPeerList(cb: (peers: PeerInfo[]) => void): Promise<UnlistenFn> {
  return listen<PeerInfo[]>("peer:list", (event) => cb(event.payload));
}

export function onPeerOffline(cb: (payload: { id: string }) => void): Promise<UnlistenFn> {
  return listen<{ id: string }>("peer:offline", (event) => cb(event.payload));
}

export function onChatMessage(cb: (msg: StoredMessage) => void): Promise<UnlistenFn> {
  return listen<StoredMessage>("chat:message", (event) => cb(event.payload));
}

export function onFileRequest(cb: (payload: { fileId: string; filename: string; size: number; from: string }) => void): Promise<UnlistenFn> {
  return listen<{ fileId: string; filename: string; size: number; from: string }>("file:request", (event) => cb(event.payload));
}

export function onFileProgress(cb: (payload: { fileId: string; received: number; total: number; speed: number }) => void): Promise<UnlistenFn> {
  return listen<{ fileId: string; received: number; total: number; speed: number }>("file:progress", (event) => cb(event.payload));
}

export function onFileComplete(cb: (payload: { fileId: string; path: string; downloadUrl?: string }) => void): Promise<UnlistenFn> {
  return listen<{ fileId: string; path: string; downloadUrl?: string }>("file:complete", (event) => cb(event.payload));
}

export function onFileError(cb: (payload: { fileId: string; error: string }) => void): Promise<UnlistenFn> {
  return listen<{ fileId: string; error: string }>("file:error", (event) => cb(event.payload));
}

// ============================================================
// Clipboard Manager
// ============================================================

export interface ClipboardEntry {
  id: string;
  content_type: string;   // "text" | "image" | "file"
  text_content: string | null;
  image_path: string | null;
  file_paths: string[] | null;
  content_hash: string;
  is_favorite: boolean;
  created_at: string;
}

export function clipboardStart(): Promise<void> {
  return invoke<void>("clipboard_start");
}

export function clipboardStop(): Promise<void> {
  return invoke<void>("clipboard_stop");
}

export function clipboardList(search?: string, limit?: number): Promise<ClipboardEntry[]> {
  return invoke<ClipboardEntry[]>("clipboard_list", {
    ...(search !== undefined ? { search } : {}),
    ...(limit !== undefined ? { limit } : {}),
  });
}

export function clipboardDelete(id: string): Promise<void> {
  return invoke<void>("clipboard_delete", { id });
}

export function clipboardToggleFavorite(id: string): Promise<boolean> {
  return invoke<boolean>("clipboard_toggle_favorite", { id });
}

export function clipboardCopy(id: string): Promise<void> {
  return invoke<void>("clipboard_copy", { id });
}

export function clipboardClear(): Promise<void> {
  return invoke<void>("clipboard_clear");
}

export function onClipboardNew(cb: (entry: ClipboardEntry) => void): Promise<UnlistenFn> {
  return listen<ClipboardEntry>("clipboard:new", (event) => cb(event.payload));
}

export function clipboardGetInterval(): Promise<number> {
  return invoke<number>("clipboard_get_interval");
}

export function clipboardSetInterval(ms: number): Promise<void> {
  return invoke<void>("clipboard_set_interval", { ms });
}

// ============================================================
// Network Sniffer
// ============================================================

export interface SnifferOptions {
  targets: string[];
  ports: number[];
  mode: string;
  concurrencyHosts: number;
  concurrencyPorts: number;
  timeoutMs: number;
  probeServices: boolean;
}

export interface PortResult {
  port: number;
  protocol: string;
  state: string;
  service: string | null;
  version: string | null;
  banner: string | null;
  confidence: number;
  probeMethod: string;
}

export interface DeviceResult {
  ip: string;
  hostname: string | null;
  mac: string | null;
  vendor: string | null;
  os: string | null;
  openPorts: PortResult[];
  isAlive: boolean;
  scanMode: string;
  scanCompleted: boolean;
}

export interface SnifferProgress {
  totalHosts: number;
  scannedHosts: number;
  servicesFound: number;
  currentTarget: string;
}

export interface PortPreset {
  name: string;
  label: string;
  ports: number[];
}

export function snifferStart(options: SnifferOptions): Promise<string> {
  return invoke<string>("sniffer_start", { options });
}

export function snifferStop(taskId: string): Promise<void> {
  return invoke<void>("sniffer_stop", { taskId });
}

export function snifferList(): Promise<DeviceResult[]> {
  return invoke<DeviceResult[]>("sniffer_list");
}

export function snifferExport(taskId: string, format: string): Promise<string> {
  return invoke<string>("sniffer_export", { taskId, format });
}

export function snifferPresets(): Promise<PortPreset[]> {
  return invoke<PortPreset[]>("sniffer_presets");
}

export function onSnifferProgress(cb: (p: SnifferProgress) => void): Promise<UnlistenFn> {
  return listen<SnifferProgress>("sniffer:progress", (e) => cb(e.payload));
}

export function onSnifferDevice(cb: (d: DeviceResult) => void): Promise<UnlistenFn> {
  return listen<DeviceResult>("sniffer:device", (e) => cb(e.payload));
}

export function onSnifferPort(cb: (p: PortResult & { ip: string }) => void): Promise<UnlistenFn> {
  return listen<PortResult & { ip: string }>("sniffer:port", (e) => cb(e.payload));
}

export function onSnifferComplete(cb: (p: { taskId: string }) => void): Promise<UnlistenFn> {
  return listen<{ taskId: string }>("sniffer:complete", (e) => cb(e.payload));
}

export function onSnifferError(cb: (p: { taskId: string; error: string }) => void): Promise<UnlistenFn> {
  return listen<{ taskId: string; error: string }>("sniffer:error", (e) => cb(e.payload));
}

// ============================================================
// Chat History Management
// ============================================================

/** Search chat history by keyword and date range. */
export function chatSearch(keyword: string, dateFrom?: string, dateTo?: string): Promise<StoredMessage[]> {
  return invoke<StoredMessage[]>("chat_search", { keyword, dateFrom, dateTo });
}

/** Delete specific chat messages by IDs. */
export function chatDelete(ids: number[]): Promise<void> {
  return invoke<void>("chat_delete", { ids });
}

/** Clear all chat history. */
export function chatClear(): Promise<void> {
  return invoke<void>("chat_clear");
}

// ============================================================
// Clipboard Batch Operations
// ============================================================

/** Delete multiple clipboard entries by IDs. */
export function clipboardDeleteBatch(ids: string[]): Promise<void> {
  return invoke<void>("clipboard_delete", { ids });
}

/** Export clipboard entries to a file. */
export function clipboardExport(ids: string[], format: string): Promise<string> {
  return invoke<string>("clipboard_export", { ids, format });
}

/** Set the maximum number of clipboard entries to keep. */
export function clipboardSetLimit(limit: number): Promise<void> {
  return invoke<void>("clipboard_set_limit", { limit });
}

// ============================================================
// Speedtest (Phase 4 Direction A)
// ============================================================

export interface SpeedtestResult {
  downloadMbps: number;
  uploadMbps: number;
  latencyMs: number;
  jitterMs: number;
  peerIp: string;
}

export interface SpeedtestProgress {
  phase: string;
  percent: number;
  currentValue: number;
}

export function startSpeedtest(
  peerIp: string,
  port: number,
  durationSecs: number,
  mode: string,
): Promise<string> {
  return invoke<string>("start_speedtest", { peerIp, port, durationSecs, mode });
}

export function onSpeedtestProgress(
  cb: (payload: SpeedtestProgress) => void,
): Promise<UnlistenFn> {
  return listen<SpeedtestProgress>("speedtest:progress", (e) => cb(e.payload));
}

export function onSpeedtestComplete(
  cb: (payload: SpeedtestResult) => void,
): Promise<UnlistenFn> {
  return listen<SpeedtestResult>("speedtest:complete", (e) => cb(e.payload));
}

// ============================================================
// Presets (Phase 4 Direction A)
// ============================================================

export interface Preset {
  id: string;
  name: string;
  feature: string;
  params: Record<string, unknown>;
  createdAt: string;
  updatedAt: string;
}

export function savePreset(
  name: string,
  feature: string,
  params: string,
): Promise<Preset> {
  return invoke<Preset>("save_preset", { name, feature, params });
}

export function loadPresets(feature?: string): Promise<Preset[]> {
  return invoke<Preset[]>("load_presets", {
    ...(feature !== undefined ? { feature } : {}),
  });
}

export function deletePreset(id: string): Promise<void> {
  return invoke<void>("delete_preset", { id });
}

// ============================================================
// Wake-on-LAN (WOL)
// ============================================================

export interface WolRecord {
  id: string;
  mac: string;
  broadcastIp: string;
  port: number;
  label: string;
  lastUsed: string;
}

export interface WolResult {
  success: boolean;
  message: string;
}

export function wolSend(mac: string, broadcastIp: string, port: number): Promise<WolResult> {
  return invoke<WolResult>("wol_send", { mac, broadcastIp, port });
}

export function wolSave(mac: string, broadcastIp: string, label: string): Promise<WolRecord> {
  return invoke<WolRecord>("wol_save", { mac, broadcastIp, label });
}

export function wolList(): Promise<WolRecord[]> {
  return invoke<WolRecord[]>("wol_list");
}

export function wolDelete(id: string): Promise<void> {
  return invoke<void>("wol_delete", { id });
}

// ============================================================
// Network Performance Monitor
// ============================================================

export interface MonitorTarget {
  id: string;
  host: string;
  label: string;
  intervalSecs: number;
  enabled: boolean;
}

export interface PingRecord {
  id: number;
  targetId: string;
  targetHost: string;
  timestamp: string;
  latencyMs: number | null;
  lossRate: number;
}

export interface MonitorUpdate {
  targetId: string;
  targetHost: string;
  label: string;
  timestamp: string;
  latencyMs: number | null;
  lossRate: number;
  minMs: number;
  avgMs: number;
  maxMs: number;
  sent: number;
  received: number;
}

export function monitorStart(): Promise<void> {
  return invoke<void>("monitor_start");
}

export function monitorStop(): Promise<void> {
  return invoke<void>("monitor_stop");
}

export function monitorStatus(): Promise<boolean> {
  return invoke<boolean>("monitor_status");
}

export function monitorAddTarget(
  host: string,
  label: string,
  intervalSecs: number,
): Promise<MonitorTarget> {
  return invoke<MonitorTarget>("monitor_add_target", { host, label, intervalSecs });
}

export function monitorListTargets(): Promise<MonitorTarget[]> {
  return invoke<MonitorTarget[]>("monitor_list_targets");
}

export function monitorDeleteTarget(id: string): Promise<void> {
  return invoke<void>("monitor_delete_target", { id });
}

export function monitorGetHistory(targetId: string, sinceDays: number): Promise<PingRecord[]> {
  return invoke<PingRecord[]>("monitor_get_history", { targetId, sinceDays });
}

export function monitorGetAllRecentHistory(sinceDays: number): Promise<PingRecord[]> {
  return invoke<PingRecord[]>("monitor_get_all_recent_history", { sinceDays });
}

export function onMonitorUpdate(
  cb: (payload: MonitorUpdate) => void,
): Promise<UnlistenFn> {
  return listen<MonitorUpdate>("monitor:update", (event) => cb(event.payload));
}

// ============================================================
// mDNS Service Discovery
// ============================================================

export interface MdnsService {
  serviceType: string;
  hostname: string;
  ip: string;
  port: number;
  txt: Record<string, string>;
}

export interface MdnsProgress {
  status: string;
  message?: string;
  count?: number;
}

export function mdnsDiscover(): Promise<MdnsService[]> {
  return invoke<MdnsService[]>("mdns_discover");
}

export function onMdnsProgress(cb: (payload: MdnsProgress) => void): Promise<UnlistenFn> {
  return listen<MdnsProgress>("mdns:progress", (event) => cb(event.payload));
}

// ============================================================
// Bandwidth Monitor
// ============================================================

export interface InterfaceInfo {
  name: string;
  friendlyName: string;
  ip: string;
}

export interface BandwidthSample {
  interface: string;
  downloadBps: number;
  uploadBps: number;
  totalRx: number;
  totalTx: number;
  timestamp: string;
}

export function getInterfaces(): Promise<InterfaceInfo[]> {
  return invoke<InterfaceInfo[]>("get_interfaces");
}

export function startBandwidthMonitor(): Promise<void> {
  return invoke<void>("start_bandwidth_monitor");
}

export function stopBandwidthMonitor(): Promise<void> {
  return invoke<void>("stop_bandwidth_monitor");
}

export function onBandwidthData(cb: (payload: BandwidthSample[]) => void): Promise<UnlistenFn> {
  return listen<BandwidthSample[]>("bandwidth:data", (event) => cb(event.payload));
}

export function onBandwidthError(cb: (payload: { error: string }) => void): Promise<UnlistenFn> {
  return listen<{ error: string }>("bandwidth:error", (event) => cb(event.payload));
}

// ============================================================
// Log Viewer
// ============================================================

export interface LogEntry {
  timestamp: string;
  level: string;
  target: string;
  message: string;
}

export async function getLogs(count?: number): Promise<LogEntry[]> {
  return invoke("get_logs", { count });
}

export async function clearLogs(): Promise<void> {
  return invoke("clear_logs");
}

// ============================================================
// Backup & Restore
// ============================================================

export interface BackupInfo {
  name: string;
  size: number;
  created: string | null;
  path: string;
}

export async function backupAllData(): Promise<string> {
  return invoke("backup_all_data");
}

export async function listBackups(): Promise<BackupInfo[]> {
  return invoke("list_backups");
}

export async function restoreBackup(path: string): Promise<string> {
  return invoke("restore_backup", { path });
}

export async function deleteBackup(path: string): Promise<void> {
  return invoke("delete_backup", { path });
}

// ============================================================
// Topology Auto Discovery
// ============================================================

export interface DiscoveredNode {
  ip: string;
  hostname: string | null;
  latencyMs: number | null;
  isGateway: boolean;
}

export interface DiscoveredLink {
  source: string;
  target: string;
  hopCount: number;
  latencyMs: number | null;
}

export interface TopologyResult {
  nodes: DiscoveredNode[];
  links: DiscoveredLink[];
}

export interface DiscoverProgress {
  phase: string;
  progress: number;
  currentIp: string;
  nodesFound: number;
  message: string;
}

/**
 * Start automatic network topology discovery.
 * Scans the specified subnet (CIDR) and discovers alive hosts, gateway, and links.
 */
export function discoverTopology(subnet?: string): Promise<void> {
  return invoke("discover_topology", {
    ...(subnet !== undefined ? { subnet } : {}),
  });
}

/**
 * Cancel a running topology discovery.
 */
export function cancelTopologyDiscovery(): Promise<void> {
  return invoke("cancel_topology_discovery");
}

/**
 * Listen for topology discovery progress updates.
 */
export function onTopologyProgress(
  cb: (payload: DiscoverProgress) => void,
): Promise<UnlistenFn> {
  return listen<DiscoverProgress>("topology:progress", (event) =>
    cb(event.payload),
  );
}

/**
 * Listen for topology discovery completion with final results.
 */
export function onTopologyResult(
  cb: (payload: TopologyResult) => void,
): Promise<UnlistenFn> {
  return listen<TopologyResult>("topology:result", (event) =>
    cb(event.payload),
  );
}

/**
 * Listen for topology discovery errors.
 */
export function onTopologyError(
  cb: (payload: { error: string }) => void,
): Promise<UnlistenFn> {
  return listen<{ error: string }>("topology:error", (event) =>
    cb(event.payload),
  );
}

// ============================================================
// API Test Tool
// ============================================================

export interface ApiRequest {
  method: string;
  url: string;
  headers: string[][];
  body: string | null;
  bodyType: string | null;
}

export interface ApiResponse {
  status: number;
  statusText: string;
  headers: string[][];
  body: string;
  durationMs: number;
  bodySize: number;
}

export interface SavedRequest {
  id: string;
  name: string;
  request: ApiRequest;
  createdAt: string;
  updatedAt: string;
}

export function sendApiRequest(request: ApiRequest): Promise<ApiResponse> {
  return invoke("send_api_request", { request });
}

export function listApiRequests(): Promise<SavedRequest[]> {
  return invoke("list_api_requests");
}

export function saveApiRequest(id: string | null, name: string, request: ApiRequest): Promise<SavedRequest> {
  return invoke("save_api_request", { id, name, request });
}

export function deleteApiRequest(id: string): Promise<void> {
  return invoke("delete_api_request", { id });
}

// ============================================================
// Bookmarks / Favorites
// ============================================================

export interface Bookmark {
  id: string;
  label: string;
  target: string;
  tags: string[];
  createdAt: string;
}

export function listBookmarks(): Promise<Bookmark[]> {
  return invoke("list_bookmarks");
}

export function addBookmark(label: string, target: string, tags?: string[]): Promise<Bookmark> {
  return invoke("add_bookmark", { label, target, tags: tags || [] });
}

export function deleteBookmark(id: string): Promise<void> {
  return invoke("delete_bookmark", { id });
}

// ============================================================
// Target Group Management
// ============================================================

export interface TargetGroup {
  id: string;
  name: string;
  targets: string[];
  createdAt: string;
  updatedAt: string;
}

export function listTargetGroups(): Promise<TargetGroup[]> {
  return invoke<TargetGroup[]>("list_target_groups");
}

export function getTargetGroup(id: string): Promise<TargetGroup | null> {
  return invoke<TargetGroup | null>("get_target_group", { id });
}

export function saveTargetGroup(
  id: string | null,
  name: string,
  targets: string[],
): Promise<TargetGroup> {
  return invoke<TargetGroup>("save_target_group", { id, name, targets });
}

export function deleteTargetGroup(id: string): Promise<void> {
  return invoke<void>("delete_target_group", { id });
}

// ============================================================
// Subnet Calculator
// ============================================================

export interface IpClassification {
  isPrivate: boolean;
  isLoopback: boolean;
  isLinkLocal: boolean;
  isMulticast: boolean;
  isPublic: boolean;
  description: string;
}

export interface SubnetResult {
  networkAddress: string;
  broadcastAddress: string;
  subnetMask: string;
  wildcardMask: string;
  usableHosts: number;
  ipRange: string;
  cidr: number;
  ipVersion: string;
  classification: IpClassification;
}

export interface SubnetSplitResult {
  subnets: SubnetResult[];
  totalUsable: number;
}

export function calculateSubnet(address: string, cidr: number): Promise<SubnetResult> {
  return invoke<SubnetResult>("calculate_subnet", { address, cidr });
}

export function splitSubnet(network: string, targetPrefix: number): Promise<SubnetSplitResult> {
  return invoke<SubnetSplitResult>("split_subnet", { network, targetPrefix });
}

// ============================================================
// Remote Shell — Session Management
// ============================================================

export interface RemoteSession {
  id: string;
  environment: string;
  name: string;
  protocol: "ssh" | "telnet";
  host: string;
  port: number;
  username: string;
  encoding: string;
  keepaliveSecs: number;
  createdAt: string;
  updatedAt: string;
}

export interface SessionInput {
  name: string;
  protocol: "ssh" | "telnet";
  host: string;
  port: number;
  username: string;
  encoding?: string;
  keepaliveSecs?: number;
  environment?: string;
}

export interface SessionSummary {
  id: string;
  environment: string;
  name: string;
  protocol: "ssh" | "telnet";
  host: string;
  port: number;
  username: string;
  isConnected: boolean;
}

export function remoteShellInit(): Promise<void> {
  return invoke<void>("remote_shell_init");
}

export function remoteShellListSessions(): Promise<RemoteSession[]> {
  return invoke<RemoteSession[]>("remote_shell_list_sessions");
}

export function remoteShellGetSession(id: string): Promise<RemoteSession> {
  return invoke<RemoteSession>("remote_shell_get_session", { id });
}

export function remoteShellCreateSession(input: SessionInput, password: string): Promise<RemoteSession> {
  return invoke<RemoteSession>("remote_shell_create_session", { input, password });
}

export function remoteShellUpdateSession(id: string, input: SessionInput): Promise<RemoteSession> {
  return invoke<RemoteSession>("remote_shell_update_session", { id, input });
}

export function remoteShellDeleteSession(id: string): Promise<void> {
  return invoke<void>("remote_shell_delete_session", { id });
}

export function remoteShellConnect(id: string): Promise<void> {
  return invoke<void>("remote_shell_connect", { id });
}

export function remoteShellDisconnect(id: string): Promise<void> {
  return invoke<void>("remote_shell_disconnect", { id });
}

export function remoteShellSendInput(id: string, data: string): Promise<void> {
  return invoke<void>("remote_shell_send_input", { id, data });
}

export function remoteShellPullOutput(id: string): Promise<string> {
  return invoke<string>("remote_shell_pull_output", { id });
}

export function remoteShellResize(id: string, cols: number, rows: number): Promise<void> {
  return invoke<void>("remote_shell_resize", { id, cols, rows });
}

export function remoteShellListSummaries(): Promise<SessionSummary[]> {
  return invoke<SessionSummary[]>("remote_shell_list_summaries");
}

// ============================================================
// Remote Shell — SFTP
// ============================================================

export interface SftpEntry {
  name: string;
  path: string;
  isDir: boolean;
  size: number;
  mtime: number;
}

export function remoteShellListSftp(sessionId: string, path: string): Promise<SftpEntry[]> {
  return invoke<SftpEntry[]>("remote_shell_list_sftp", { sessionId, path });
}

export function remoteShellReadSftpText(sessionId: string, path: string): Promise<{ content: string; encoding: string }> {
  return invoke<{ content: string; encoding: string }>("remote_shell_read_sftp_text", { sessionId, path });
}

export function remoteShellSaveSftpText(sessionId: string, path: string, content: string): Promise<void> {
  return invoke<void>("remote_shell_save_sftp_text", { sessionId, path, content });
}

// ============================================================
// Remote Shell — Host Metrics
// ============================================================

export interface HostMetrics {
  cpuPercent: number;
  memoryUsedBytes: number;
  memoryTotalBytes: number;
  memoryPercent: number;
  diskUsedBytes: number;
  diskTotalBytes: number;
  diskPercent: number;
  collectedAt: string;
}

export function remoteShellGetMetrics(sessionId: string): Promise<HostMetrics> {
  return invoke<HostMetrics>("remote_shell_get_metrics", { sessionId });
}

// ============================================================
// Remote Shell — Environments
// ============================================================

export function remoteShellListEnvironments(): Promise<string[]> {
  return invoke<string[]>("remote_shell_list_environments");
}

export function remoteShellCreateEnvironment(name: string): Promise<void> {
  return invoke<void>("remote_shell_create_environment", { name });
}

// ============================================================
// Remote Shell — Database Connections
// ============================================================

export interface DbConnection {
  id: string;
  environment: string;
  name: string;
  dbType: "mysql" | "postgresql" | "redis" | "zookeeper" | "etcd";
  host: string;
  port: number;
  username: string;
  defaultDatabase: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface DbConnectionInput {
  name: string;
  dbType: "mysql" | "postgresql" | "redis" | "zookeeper" | "etcd";
  host: string;
  port: number;
  username: string;
  defaultDatabase?: string;
  environment?: string;
}

export function remoteShellListDbConnections(dbType?: string): Promise<DbConnection[]> {
  return invoke<DbConnection[]>("remote_shell_list_db_connections", { dbType: dbType || null });
}

export function remoteShellCreateDbConnection(input: DbConnectionInput, password: string): Promise<DbConnection> {
  return invoke<DbConnection>("remote_shell_create_db_connection", { input, password });
}

export function remoteShellDeleteDbConnection(id: string): Promise<void> {
  return invoke<void>("remote_shell_delete_db_connection", { id });
}

export function remoteShellTestDbConnection(id: string): Promise<string> {
  return invoke<string>("remote_shell_test_db_connection", { id });
}

// ============================================================
// Remote Shell — MySQL
// ============================================================

export interface MySqlColumnInfo {
  field: string;
  dbType: string;
  nullable: boolean;
  key: string;
  default: string | null;
  extra: string;
}

export interface MySqlQueryResult {
  columns: string[];
  rows: unknown[][];
  affectedRows: number;
  elapsedMs: number;
}

export function remoteShellMysqlListDatabases(connId: string): Promise<string[]> {
  return invoke<string[]>("remote_shell_mysql_list_databases", { connId });
}

export function remoteShellMysqlListTables(connId: string, database: string): Promise<string[]> {
  return invoke<string[]>("remote_shell_mysql_list_tables", { connId, database });
}

export function remoteShellMysqlDescribeTable(connId: string, database: string, table: string): Promise<MySqlColumnInfo[]> {
  return invoke<MySqlColumnInfo[]>("remote_shell_mysql_describe_table", { connId, database, table });
}

export function remoteShellMysqlExecuteQuery(connId: string, database: string, query: string): Promise<MySqlQueryResult> {
  return invoke<MySqlQueryResult>("remote_shell_mysql_execute_query", { connId, database, query });
}

// ============================================================
// Remote Shell — PostgreSQL
// ============================================================

export function remoteShellPgListDatabases(connId: string): Promise<string[]> {
  return invoke<string[]>("remote_shell_pg_list_databases", { connId });
}

export function remoteShellPgListTables(connId: string, database: string): Promise<string[]> {
  return invoke<string[]>("remote_shell_pg_list_tables", { connId, database });
}

export function remoteShellPgExecuteQuery(connId: string, database: string, query: string): Promise<MySqlQueryResult> {
  return invoke<MySqlQueryResult>("remote_shell_pg_execute_query", { connId, database, query });
}

// ============================================================
// Remote Shell — Redis
// ============================================================

export interface RedisKeyEntry {
  key: string;
  keyType: string;
  ttl: number;
  size: number;
}

export function remoteShellRedisListKeys(connId: string, pattern?: string): Promise<RedisKeyEntry[]> {
  return invoke<RedisKeyEntry[]>("remote_shell_redis_list_keys", { connId, pattern: pattern || null });
}

export function remoteShellRedisGetValue(connId: string, key: string): Promise<string> {
  return invoke<string>("remote_shell_redis_get_value", { connId, key });
}

export function remoteShellRedisSetValue(connId: string, key: string, value: string): Promise<void> {
  return invoke<void>("remote_shell_redis_set_value", { connId, key, value });
}

export function remoteShellRedisSetTtl(connId: string, key: string, ttl: number): Promise<void> {
  return invoke<void>("remote_shell_redis_set_ttl", { connId, key, ttl });
}

// ============================================================
// Remote Desktop — VNC
// ============================================================

export interface DesktopSession {
  id: string;
  name: string;
  protocol: "rdp" | "vnc";
  host: string;
  port: number;
  username: string;
  quality: number;
  createdAt: string;
  updatedAt: string;
}

export interface DesktopSessionInput {
  name: string;
  protocol: "rdp" | "vnc";
  host: string;
  port: number;
  username: string;
  quality?: number;
}

export interface DesktopFrame {
  sessionId: string;
  x: number;
  y: number;
  width: number;
  height: number;
  data: number[];
  encoding: string;
}

export interface MouseEvent {
  x: number;
  y: number;
  button: number;
  pressed: boolean;
}

export interface KeyEvent {
  keyCode: number;
  pressed: boolean;
}

export function rdListSessions(): Promise<DesktopSession[]> {
  return invoke<DesktopSession[]>("rd_list_sessions");
}

export function rdCreateSession(input: DesktopSessionInput, password: string): Promise<DesktopSession> {
  return invoke<DesktopSession>("rd_create_session", { input, password });
}

export function rdUpdateSession(id: string, input: DesktopSessionInput): Promise<DesktopSession> {
  return invoke<DesktopSession>("rd_update_session", { id, input });
}

export function rdDeleteSession(id: string): Promise<void> {
  return invoke<void>("rd_delete_session", { id });
}

export function rdConnect(sessionId: string, password: string): Promise<void> {
  return invoke<void>("rd_connect", { sessionId, password });
}

export function rdDisconnect(sessionId: string): Promise<void> {
  return invoke<void>("rd_disconnect", { sessionId });
}

export function rdResize(sessionId: string, width: number, height: number): Promise<void> {
  return invoke<void>("rd_resize", { sessionId, width, height });
}

export function rdSendKey(sessionId: string, event: KeyEvent): Promise<void> {
  return invoke<void>("rd_send_key", { sessionId, event });
}

export function rdSendMouse(sessionId: string, event: MouseEvent): Promise<void> {
  return invoke<void>("rd_send_mouse", { sessionId, event });
}

export function onRdFrame(cb: (frame: DesktopFrame) => void): Promise<UnlistenFn> {
  return listen<DesktopFrame>("rd:frame", (event) => cb(event.payload));
}

// ── System Info ──

export interface LocalNetworkInfo {
  ipv4: string[];
  ipv6: string[];
  hostname: string;
}

export function getLocalNetworkInfo(): Promise<LocalNetworkInfo> {
  return invoke<LocalNetworkInfo>("get_local_network_info");
}
