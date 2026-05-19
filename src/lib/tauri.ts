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
 */
export function dnsLookup(target: string, recordType: RecordType): Promise<DnsRecord[]> {
  return invoke<string>("dns_lookup", { target, recordType }).then(
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
  return listen("file:request", (event) => cb(event.payload as any));
}

export function onFileProgress(cb: (payload: { fileId: string; received: number; total: number; speed: number }) => void): Promise<UnlistenFn> {
  return listen("file:progress", (event) => cb(event.payload as any));
}

export function onFileComplete(cb: (payload: { fileId: string; path: string; downloadUrl?: string }) => void): Promise<UnlistenFn> {
  return listen("file:complete", (event) => cb(event.payload as any));
}

export function onFileError(cb: (payload: { fileId: string; error: string }) => void): Promise<UnlistenFn> {
  return listen("file:error", (event) => cb(event.payload as any));
}
