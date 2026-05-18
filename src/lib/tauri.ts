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

export function onPortProgress(cb: (payload: PortProgressPayload) => void): Promise<UnlistenFn> {
  return listen<PortProgressPayload>("port:progress", (event) => cb(event.payload));
}

export function onPortFound(cb: (payload: PortFoundPayload) => void): Promise<UnlistenFn> {
  return listen<PortFoundPayload>("port:found", (event) => cb(event.payload));
}

export function onPortComplete(cb: (payload: PortCompletePayload) => void): Promise<UnlistenFn> {
  return listen<PortCompletePayload>("port:complete", (event) => cb(event.payload));
}

export function onDnsResult(cb: (payload: DnsResultPayload) => void): Promise<UnlistenFn> {
  return listen<DnsResultPayload>("dns:result", (event) => cb(event.payload));
}

export function onDnsError(cb: (payload: DnsErrorPayload) => void): Promise<UnlistenFn> {
  return listen<DnsErrorPayload>("dns:error", (event) => cb(event.payload));
}
