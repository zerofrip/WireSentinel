const DEFAULT_PORT = 8170;
const BASE = `http://127.0.0.1:${import.meta.env.VITE_API_PORT ?? DEFAULT_PORT}`;
const WS_BASE = `ws://127.0.0.1:${import.meta.env.VITE_API_PORT ?? DEFAULT_PORT}`;

let authToken: string | null = null;

export async function initAuth(): Promise<void> {
  if (authToken) return;
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    authToken = await invoke<string>("read_api_token");
  } catch {
    authToken = import.meta.env.VITE_API_TOKEN ?? null;
  }
  if (!authToken) {
    throw new Error("API token unavailable — start WireSentinel service first");
  }
}

export function getAuthToken(): string | null {
  return authToken;
}

function headers(): HeadersInit {
  const h: HeadersInit = { "Content-Type": "application/json" };
  if (authToken) h["Authorization"] = `Bearer ${authToken}`;
  return h;
}

async function api<T>(path: string, init?: RequestInit): Promise<T> {
  await initAuth();
  const res = await fetch(`${BASE}${path}`, {
    ...init,
    headers: { ...headers(), ...init?.headers },
  });
  if (!res.ok) throw new Error(`${res.status} ${await res.text()}`);
  return res.json();
}

export interface ServiceStatus {
  running: boolean;
  kill_switch_active: boolean;
  policy_mode: "blacklist" | "whitelist";
  active_vpn_count: number;
  monitored_app_count: number;
  connection_count: number;
  api_port: number;
}

export type AnonymousRoute =
  | { type: "tor"; value: string }
  | { type: "tor_bridge"; value: string }
  | { type: "multi_hop"; value: string[] }
  | { type: "future_mixnet"; value: string }
  | { type: "katzenpost"; value: string }
  | { type: "loopix"; value: string }
  | { type: "federated_mixnet"; value: { profile_id: string; federation_id?: string } };

export type TrafficRoute =
  | { type: "direct" }
  | { type: "wire_guard"; value: string }
  | { type: "amnezia_wg"; value: string }
  | { type: "blocked" }
  | { type: "tailnet"; value: string }
  | { type: "tor"; value: string }
  | { type: "anonymous"; value: AnonymousRoute }
  | { type: "proxy"; value: string }
  | { type: "proxy_chain"; value: string }
  | { type: "chain"; value: string }
  | { type: "katzenpost"; value: string }
  | { type: "loopix"; value: string }
  | { type: "federated_mixnet"; value: string };

export type ExitOnExhaustion = "kill_switch" | "blocked" | "direct";

export interface AppExitConfig {
  routes: TrafficRoute[];
  on_exhaustion?: ExitOnExhaustion;
}

export interface AppSummary {
  id: string;
  pid?: number | null;
  display_name: string;
  exe_path: string;
  publisher?: string | null;
  sha256?: string | null;
  default_route?: TrafficRoute | null;
  exit_config?: AppExitConfig | null;
  bytes_in: number;
  bytes_out: number;
  connection_count: number;
}

export interface VpnProfile {
  id: string;
  name: string;
  backend: string;
  auto_connect: boolean;
  handshake_proxy?: HandshakeProxySettings | null;
}

export interface VpnState {
  profile_id: string;
  profile_name: string;
  status: string;
  stats: { rx_bytes: number; tx_bytes: number };
}

export interface VpnListEntry {
  profile: VpnProfile;
  status: VpnState | string;
}

export interface Rule {
  id: string;
  priority: number;
  scope: { type: string; value?: unknown };
  action: { type: string; value?: unknown };
  enabled: boolean;
}

export interface DnsSettings {
  enabled: boolean;
  transport: string;
  provider: string;
  upstream_url: string;
  listen_addr?: string;
  dot_enabled?: boolean;
  filter_mode?: "blacklist" | "whitelist";
  dns_block_mode?: "null" | "nxdomain";
}

export interface FilterListRecord {
  id: string;
  name: string;
  url?: string | null;
  list_type: "hosts" | "easylist";
  enabled: boolean;
  update_interval_secs?: number | null;
  last_updated?: string | null;
}

export interface TopDomainEntry {
  domain: string;
  query_count: number;
  blocked_count: number;
}

export interface DomainCorrelation {
  id: string;
  app_id?: string | null;
  domain: string;
  ip_address?: string | null;
  query_count: number;
  traffic_count: number;
}

export interface RouteStatisticsRecord {
  id: string;
  app_id?: string | null;
  profile_id?: string | null;
  domain?: string | null;
  route_type: string;
  bytes_in: number;
  bytes_out: number;
  connection_count: number;
  window_start: string;
  window_end: string;
  updated_at: string;
}

export interface AuditLogEntry {
  id: string;
  event_type: string;
  actor?: string | null;
  target_type?: string | null;
  target_id?: string | null;
  detail_json?: string | null;
  timestamp: string;
}

export interface DnsQueryLog {
  id: string;
  qname: string;
  qtype: string;
  blocked: boolean;
  latency_ms?: number;
  timestamp?: string;
}

export interface BandwidthSnapshot {
  app_id: string;
  exe_name: string;
  bytes_in_per_sec: number;
  bytes_out_per_sec: number;
  total_bytes_in?: number;
  total_bytes_out?: number;
}

export interface TrafficEvent {
  id?: string;
  timestamp?: string;
  app?: { display_name?: string; pid?: number };
  app_id?: string;
  exe_name?: string;
  protocol: string;
  remote_addr?: string;
  remote_domain?: string | null;
  route: TrafficRoute;
  bytes_in?: number;
  bytes_out?: number;
}

export type TransportProfileKind = "sing_box" | "xray";
export type ObfuscationPreset = "disabled" | "basic" | "balanced" | "aggressive" | "lwo";
export type TransportKind =
  | "direct"
  | "wire_guard"
  | "amnezia_wg"
  | "sing_box"
  | "xray"
  | "tor"
  | "tls_tunnel"
  | "websocket_tunnel";
export type TransportState = "stopped" | "starting" | "running" | "stopping" | "error";
export type LeakType = "dns" | "route" | "vpn_disconnect";
export type DnsTransport = "plain" | "doh" | "dot" | "doq";

export interface PrivacyScoreComponents {
  encrypted_dns: number;
  blocked_trackers: number;
  vpn_coverage: number;
  route_leakage: number;
  dns_leakage: number;
}

export interface PrivacyScoreSnapshot {
  id: string;
  score: number;
  components: PrivacyScoreComponents;
  timestamp: string;
}

export interface TransportProfile {
  id: string;
  name: string;
  transport_kind: TransportProfileKind;
  config_json?: string | null;
  config_path?: string | null;
  binary_path?: string | null;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface TransportStatusRecord {
  id: string;
  name: string;
  kind: TransportProfileKind;
  state: TransportState;
  message?: string | null;
}

export interface ChainHop {
  kind: TransportKind;
  profile_id?: string | null;
  transport_profile_id?: string | null;
}

export interface ChainProfile {
  id: string;
  name: string;
  hops: ChainHop[];
  obfuscation_profile_id?: string | null;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface DnsProviderRecord {
  id: string;
  name: string;
  transport: DnsTransport;
  endpoint: string;
  priority: number;
  enabled: boolean;
  latency_ms?: number | null;
  last_check?: string | null;
  failure_count: number;
  created_at: string;
  updated_at: string;
}

export interface LeakIncident {
  id: string;
  leak_type: LeakType;
  app_id?: string | null;
  detail_json?: string | null;
  severity: string;
  detected_at: string;
  resolved_at?: string | null;
}

export interface DnsProvidersUpdate {
  providers: DnsProviderRecord[];
  failover?: boolean;
}

export type LogLevel = "info" | "warn" | "error" | "debug" | "trace";

export interface LogEntry {
  timestamp: string;
  level: string;
  target: string;
  message: string;
}

export interface SubsystemHealth {
  status: string;
  message?: string | null;
}

export interface DiagnosticsHealth {
  wfp: SubsystemHealth;
  vpn: SubsystemHealth;
  dns: SubsystemHealth;
  transport: SubsystemHealth;
  database: SubsystemHealth;
  disk: SubsystemHealth;
}

export type ValidationStatus = "pass" | "fail" | "warn";

export interface ValidationCheck {
  id: string;
  check_name: string;
  status: ValidationStatus;
  message?: string | null;
  checked_at: string;
}

export interface ValidationReport {
  overall_status: ValidationStatus;
  checks: ValidationCheck[];
  generated_at: string;
}

export interface PerformanceSnapshot {
  id: string;
  cpu_percent: number;
  memory_bytes: number;
  api_latency_ms: number;
  wfp_latency_ms: number;
  event_throughput: number;
  timestamp: string;
}

export interface PerformanceResponse {
  latest?: PerformanceSnapshot | null;
  snapshots: PerformanceSnapshot[];
}

export interface MetricsSnapshot {
  active_tunnels: number;
  active_transports: number;
  blocked_requests: number;
  dns_queries: number;
  open_leak_incidents: number;
  route_changes_24h: number;
  timestamp: string;
}

export interface UpdateInfo {
  current_version: string;
  latest_version?: string | null;
  channel: string;
  staged_percent: number;
  download_url?: string | null;
  update_available: boolean;
}

export interface EnterprisePolicy {
  id: string;
  version: number;
  policy_json: Record<string, unknown>;
  locked_keys: string[];
  updated_at: string;
}

export interface PluginManifest {
  id: string;
  name: string;
  version: string;
  format: "wasm" | "native";
  path: string;
}

export interface PluginRecord {
  id: string;
  manifest: PluginManifest;
  state: "installed" | "loaded" | "failed" | "unloaded";
  error_message?: string | null;
  installed_at: string;
  loaded_at?: string | null;
}

export interface TailnetProfile {
  id: string;
  name: string;
  auth_key?: string | null;
  exit_node?: string | null;
  subnet_router?: boolean;
  magic_dns?: boolean;
  hostname?: string | null;
  tailnet_ip?: string | null;
  connected: boolean;
}

export interface TailscaleStatus {
  connected: boolean;
  hostname?: string | null;
  tailnet_ip?: string | null;
  exit_node?: string | null;
  magic_dns: boolean;
  profiles: TailnetProfile[];
}

export interface TorProfile {
  id: string;
  name: string;
  control_port: number;
  socks_port: number;
  data_dir: string;
  bridge_ids: string[];
  enabled: boolean;
  bootstrap_progress: number;
  circuit_count: number;
}

export interface TorStatus {
  running: boolean;
  bootstrap_progress: number;
  circuit_count: number;
  socks_port: number;
  profile?: TorProfile | null;
}

export interface BridgeProfile {
  id: string;
  name: string;
  bridge_type: "obfs4" | "snowflake" | "meek" | "webtunnel";
  config_json?: { line?: string };
  enabled: boolean;
}

export interface BridgeTestResult {
  bridge_id: string;
  success: boolean;
  latency_ms?: number | null;
  error?: string | null;
}

export interface ProxyProfile {
  id: string;
  name: string;
  kind: "socks5" | "http" | "https";
  host: string;
  port: number;
  enabled: boolean;
  active?: boolean;
  latency_ms?: number | null;
  last_health_at?: string | null;
  last_error?: string | null;
}

export interface ProxyChainHop {
  kind: "socks5" | "http" | "https" | "tor" | "tls_tunnel";
  profile_id: string;
  order: number;
}

export interface ProxyChain {
  id: string;
  name: string;
  hops: ProxyChainHop[];
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface MixnetProfile {
  id: string;
  name: string;
  provider: { type: string; value?: string };
  gateway_id?: string | null;
  enabled: boolean;
  active: boolean;
  latency_ms?: number | null;
  last_health_at?: string | null;
  last_error?: string | null;
}

export interface MixnetStatus {
  running: boolean;
  profile_id?: string | null;
  gateway_id?: string | null;
  latency_ms?: number | null;
  active_sessions: number;
}

export type EnforcementBackend = "signed" | "custom_kernel";

export interface EnforcementComponentsHealth {
  wfp: string;
  wireguard: string;
  windivert: string;
  singbox: string;
  guardian: string;
  ndis: string;
}

export interface EnforcementSettings {
  enforcement_backend: EnforcementBackend;
  guardian_mode: string;
  wfp_engine_impl: string;
  components: EnforcementComponentsHealth;
  restart_required: boolean;
}

export interface KernelStatus {
  guardian_mode: string;
  driver_connected: boolean;
  lifecycle_state: string;
  wfp_engine: string;
  filter_count: number;
  provider_registered: boolean;
  kill_switch_active: boolean;
  ndis_enabled: boolean;
  ndis_lifecycle_state: string;
  healthy: boolean;
}

export interface KernelTelemetry {
  classify_count: number;
  block_count: number;
  route_count: number;
  permit_count: number;
  observe_count: number;
  error_count: number;
  avg_classify_latency_ns: number;
  max_classify_latency_ns: number;
  packets_per_sec: number;
}

export interface KernelRouteEntry {
  route_id: string;
  app_id: string;
  route_kind: string;
  label: string;
  active: boolean;
}

export interface KernelPacketEntry {
  flow_id: string;
  process_id: number;
  protocol: number;
  bytes: number;
  direction: string;
}

export interface NdisStatus {
  enabled: boolean;
  driver_connected: boolean;
  lifecycle_state: string;
  classify_count: number;
  redirect_count: number;
  transform_count: number;
  cover_traffic_count: number;
  error_count: number;
  pending_events: number;
}

export interface AnonymousChain {
  id: string;
  name: string;
  hops: Array<{ kind: string; profile_id: string; order: number }>;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export type CoverTrafficProfile =
  | "disabled"
  | "low"
  | "medium"
  | "high"
  | "maximum";

export interface CoverTrafficSettings {
  id: string;
  mixnet_profile_id?: string | null;
  profile: CoverTrafficProfile;
  enabled: boolean;
  rate_bps?: number | null;
  created_at: string;
  updated_at: string;
}

export interface PrivacyAnalyticsSnapshot {
  id: string;
  anonymity_score: number;
  route_entropy: number;
  path_diversity: number;
  cover_traffic_effectiveness: number;
  federation_peer_count?: number;
  entropy_bits?: number;
  active_route_count?: number;
  timestamp: string;
}

export interface AnonymityHealthSummary {
  connected_devices: number;
  healthy_devices: number;
  stub_devices: number;
  total_active_routes: number;
  avg_anonymity_score: number;
  federation: {
    total_peers: number;
    healthy_peers: number;
    devices_with_federation: number;
  };
  entropy: {
    avg_entropy_bits: number;
    avg_route_entropy: number;
    avg_path_diversity: number;
    devices_reporting: number;
  };
}

export interface AnonymityPrivacySnapshot {
  anonymity_score: number;
  route_entropy: number;
  path_diversity: number;
  cover_traffic_effectiveness: number;
  federation_peer_count: number;
  entropy_bits: number;
  active_route_count: number;
  federation_peers_total?: number;
  timestamp: string;
}

export interface SecurityAuditEntry {
  action: string;
  actor?: string | null;
  detail?: string | null;
  timestamp: string;
}

export type ProxyType = "socks5" | "http" | "https";

export type TcpTerminationMode =
  | "disabled"
  | "on_vpn_connect"
  | "on_vpn_disconnect"
  | "on_route_change"
  | "always";

export type TemplateMode = "disabled" | "merge" | "override";

export interface HandshakeProxySettings {
  enabled: boolean;
  proxy_type?: ProxyType;
  host: string;
  port: number;
  username?: string | null;
  password?: string | null;
}

export interface TcpTerminationRule {
  id: string;
  process_path?: string | null;
  process_name?: string | null;
  profile_id?: string | null;
  route?: TrafficRoute | null;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface TcpTerminationSettings {
  mode: TcpTerminationMode;
  updated_at: string;
}

export interface TcpTerminationPolicy {
  mode: TcpTerminationMode;
  rules: TcpTerminationRule[];
}

export interface AppRule {
  id: string;
  app_id: string;
  route: TrafficRoute;
  enabled: boolean;
  description?: string | null;
}

export interface DomainRule {
  id: string;
  pattern: string;
  route: TrafficRoute;
  enabled: boolean;
  description?: string | null;
}

export interface SplitTunnelTemplate {
  id: string;
  name: string;
  description: string;
  default_route: TrafficRoute;
  app_rules: AppRule[];
  domain_rules: DomainRule[];
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface SplitTemplateModeSettings {
  mode: TemplateMode;
  active_template_id?: string | null;
  updated_at: string;
}

export interface TemplateTraceStep {
  stage: string;
  detail: string;
  route?: TrafficRoute | null;
}

export interface TemplateResolutionTrace {
  template_id?: string | null;
  mode: TemplateMode;
  steps: TemplateTraceStep[];
  final_route?: TrafficRoute | null;
}

export interface TcpSessionEvent {
  kind: "connections_terminated" | "termination_failed";
  count?: number;
  profile_id?: string | null;
  mode?: TcpTerminationMode;
  error?: string;
  timestamp: string;
}

export interface HandshakeProxyStatusEntry {
  profile_id: string;
  profile_name?: string | null;
  enabled: boolean;
  connected: boolean;
  proxy_host?: string | null;
  proxy_port?: number | null;
  last_error?: string | null;
  last_event_at?: string | null;
}

export interface HandshakeProxyStatus {
  profiles: HandshakeProxyStatusEntry[];
}

export interface TcpConnectionSnapshot {
  pid: number;
  app_id?: string | null;
  exe_name: string;
  exe_path?: string | null;
  protocol: string;
  local_addr: string;
  remote_addr: string;
  state: string;
  remote_domain?: string | null;
}

export interface WiresockHandshakeProxyProfile {
  profile_id: string;
  profile_name: string;
  settings?: HandshakeProxySettings | null;
}

export interface WiresockDiagnostics {
  tcp_sessions: TcpConnectionSnapshot[];
  template_trace?: TemplateResolutionTrace | null;
  handshake_proxy_profiles: WiresockHandshakeProxyProfile[];
}

export type SecuritySeverity = "info" | "low" | "medium" | "high" | "critical";

export interface SecurityFinding {
  id: string;
  severity: SecuritySeverity;
  category: string;
  title: string;
  detail_json: Record<string, unknown>;
  resolved: boolean;
  created_at: string;
  resolved_at?: string | null;
}

export interface RuntimeSettings {
  api_port?: number;
  log_level?: LogLevel;
  recovery_enabled?: boolean;
  metrics_interval_secs?: number;
  update_channel?: string;
}

export interface BackupBundle {
  version: number;
  exported_at: string;
  settings: RuntimeSettings;
  vpn_profiles: unknown[];
  rules: unknown[];
  apps: unknown[];
  dns_settings: unknown;
  dns_providers: unknown[];
  filter_lists: unknown[];
  transport_profiles: unknown[];
  chain_profiles: unknown[];
  obfuscation_profiles: unknown[];
  enterprise_policy?: EnterprisePolicy | null;
}

async function apiBlob(path: string, init?: RequestInit): Promise<Blob> {
  await initAuth();
  const res = await fetch(`${BASE}${path}`, {
    ...init,
    headers: { ...headers(), ...init?.headers },
  });
  if (!res.ok) throw new Error(`${res.status} ${await res.text()}`);
  return res.blob();
}

async function apiText(path: string, init?: RequestInit): Promise<string> {
  await initAuth();
  const res = await fetch(`${BASE}${path}`, {
    ...init,
    headers: { ...headers(), ...init?.headers },
  });
  if (!res.ok) throw new Error(`${res.status} ${await res.text()}`);
  return res.text();
}

export const apiClient = {
  status: () => api<ServiceStatus>("/api/v1/status"),
  apps: () => api<AppSummary[]>("/api/v1/apps"),
  setAppRoute: (app_id: string, route: TrafficRoute | null) =>
    api<AppSummary>("/api/v1/apps", {
      method: "POST",
      body: JSON.stringify({ app_id, route }),
    }),
  setAppExitConfig: (app_id: string, exit_config: AppExitConfig | null) =>
    api<AppSummary>("/api/v1/apps", {
      method: "POST",
      body: JSON.stringify({ app_id, exit_config }),
    }),
  vpnList: () => api<VpnListEntry[]>("/api/v1/vpn"),
  addVpn: (name: string, config_plaintext: string) =>
    api<VpnProfile>("/api/v1/vpn", {
      method: "POST",
      body: JSON.stringify({ name, config_plaintext }),
    }),
  connectVpn: (profile_id: string) =>
    api<{ ok: boolean }>(`/api/v1/vpn/${profile_id}/connect`, { method: "POST" }),
  disconnectVpn: (profile_id: string) =>
    api<{ ok: boolean }>(`/api/v1/vpn/${profile_id}/disconnect`, { method: "POST" }),
  vpnStatus: (profile_id: string) => api<VpnState>(`/api/v1/vpn/${profile_id}/status`),
  rules: () => api<Rule[]>("/api/v1/rules"),
  addRule: (rule: Rule) =>
    api<{ ok: boolean }>("/api/v1/rules", { method: "POST", body: JSON.stringify(rule) }),
  policyMode: () => api<{ mode: ServiceStatus["policy_mode"] }>("/api/v1/rules/mode"),
  setPolicyMode: (mode: ServiceStatus["policy_mode"]) =>
    api<{ ok: boolean }>("/api/v1/rules/mode", {
      method: "PUT",
      body: JSON.stringify({ mode }),
    }),
  updateRule: (id: string, rule: Rule) =>
    api<{ ok: boolean }>(`/api/v1/rules/${id}`, {
      method: "PUT",
      body: JSON.stringify(rule),
    }),
  deleteRule: (id: string) =>
    api<{ ok: boolean }>(`/api/v1/rules/${id}`, { method: "DELETE" }),
  setKillSwitch: (active: boolean) =>
    api<{ ok: boolean; active: boolean }>("/api/v1/rules/kill-switch", {
      method: "POST",
      body: JSON.stringify({ active }),
    }),
  routeStatistics: (params?: {
    app_id?: string;
    domain?: string;
    route_type?: string;
    limit?: number;
  }) => {
    const q = new URLSearchParams();
    if (params?.app_id) q.set("app_id", params.app_id);
    if (params?.domain) q.set("domain", params.domain);
    if (params?.route_type) q.set("route_type", params.route_type);
    if (params?.limit) q.set("limit", String(params.limit));
    const qs = q.toString();
    return api<RouteStatisticsRecord[]>(
      `/api/v1/statistics/routes${qs ? `?${qs}` : ""}`
    );
  },
  blockedStatistics: (limit = 50) =>
    api<RouteStatisticsRecord[]>(`/api/v1/statistics/blocked?limit=${limit}`),
  auditLog: (params?: { event_type?: string; limit?: number; offset?: number }) => {
    const q = new URLSearchParams();
    if (params?.event_type) q.set("event_type", params.event_type);
    if (params?.limit) q.set("limit", String(params.limit));
    if (params?.offset) q.set("offset", String(params.offset));
    const qs = q.toString();
    return api<AuditLogEntry[]>(`/api/v1/audit${qs ? `?${qs}` : ""}`);
  },
  traffic: (limit = 50) => api<BandwidthSnapshot[]>(`/api/v1/traffic?limit=${limit}`),
  trafficLogs: (params?: {
    limit?: number;
    offset?: number;
    app_id?: string;
    sort?: "timestamp" | "bytes";
    order?: "asc" | "desc";
  }) => {
    const q = new URLSearchParams();
    if (params?.limit) q.set("limit", String(params.limit));
    if (params?.offset) q.set("offset", String(params.offset));
    if (params?.app_id) q.set("app_id", params.app_id);
    if (params?.sort) q.set("sort", params.sort);
    if (params?.order) q.set("order", params.order);
    const qs = q.toString();
    return api<TrafficEvent[]>(`/api/v1/traffic/logs${qs ? `?${qs}` : ""}`);
  },
  topDomains: (limit = 20) =>
    api<TopDomainEntry[]>(`/api/v1/traffic/top-domains?limit=${limit}`),
  exportTraffic: (format: "json" | "csv" = "json", source?: "logs" | "live") => {
    const q = new URLSearchParams({ format });
    if (source) q.set("source", source);
    return api<BandwidthSnapshot[] | TrafficEvent[]>(
      `/api/v1/traffic/export?${q.toString()}`
    );
  },
  dns: () => api<DnsSettings>("/api/v1/dns"),
  dnsSettings: () => api<DnsSettings>("/api/v1/dns/settings"),
  setDns: (settings: DnsSettings) =>
    api<{ ok: boolean }>("/api/v1/dns", { method: "PUT", body: JSON.stringify(settings) }),
  dnsLogs: (params?: {
    limit?: number;
    offset?: number;
    qname?: string;
    blocked?: boolean;
  }) => {
    const q = new URLSearchParams();
    if (params?.limit) q.set("limit", String(params.limit));
    if (params?.offset) q.set("offset", String(params.offset));
    if (params?.qname) q.set("qname", params.qname);
    if (params?.blocked !== undefined) q.set("blocked", String(params.blocked));
    const qs = q.toString();
    return api<DnsQueryLog[]>(`/api/v1/dns/logs${qs ? `?${qs}` : ""}`);
  },
  filterLists: () => api<FilterListRecord[]>("/api/v1/filter-lists"),
  addFilterList: (record: FilterListRecord) =>
    api<FilterListRecord>("/api/v1/filter-lists", {
      method: "POST",
      body: JSON.stringify(record),
    }),
  updateFilterList: (id: string, record: FilterListRecord) =>
    api<FilterListRecord>(`/api/v1/filter-lists/${id}`, {
      method: "PUT",
      body: JSON.stringify(record),
    }),
  deleteFilterList: (id: string) =>
    api<{ ok: boolean }>(`/api/v1/filter-lists/${id}`, { method: "DELETE" }),
  refreshFilterList: (id: string) =>
    api<{ ok: boolean; entry_count: number }>(`/api/v1/filter-lists/${id}/update`, {
      method: "POST",
    }),
  correlations: (params?: { limit?: number; app_id?: string; domain?: string }) => {
    const q = new URLSearchParams();
    if (params?.limit) q.set("limit", String(params.limit));
    if (params?.app_id) q.set("app_id", params.app_id);
    if (params?.domain) q.set("domain", params.domain);
    const qs = q.toString();
    return api<DomainCorrelation[]>(`/api/v1/correlations${qs ? `?${qs}` : ""}`);
  },
  privacy: () => api<PrivacyScoreSnapshot | null>("/api/v1/privacy"),
  leaks: (limit = 50) => api<LeakIncident[]>(`/api/v1/leaks?limit=${limit}`),
  transports: () => api<TransportProfile[]>("/api/v1/transports"),
  transportStatus: () => api<TransportStatusRecord[]>("/api/v1/transports/status"),
  chains: () => api<ChainProfile[]>("/api/v1/chains"),
  createChain: (chain: {
    name: string;
    hops: ChainHop[];
    obfuscation_profile_id?: string | null;
    enabled?: boolean;
  }) =>
    api<ChainProfile>("/api/v1/chains", {
      method: "POST",
      body: JSON.stringify(chain),
    }),
  startChain: (id: string) =>
    api<{ ok: boolean }>(`/api/v1/chains/${id}/start`, { method: "POST" }),
  stopChain: (id: string) =>
    api<{ ok: boolean }>(`/api/v1/chains/${id}/stop`, { method: "POST" }),
  plugins: () => api<PluginRecord[]>("/api/v1/plugins"),
  tailnetProfiles: () => api<TailnetProfile[]>("/api/v1/tailnet"),
  tailnetStatus: () => api<TailscaleStatus>("/api/v1/tailnet/status"),
  upsertTailnetProfile: (profile: TailnetProfile) =>
    api<TailnetProfile>("/api/v1/tailnet", {
      method: "POST",
      body: JSON.stringify(profile),
    }),
  tailnetJoin: (id: string) =>
    api<TailnetProfile>(`/api/v1/tailnet/profiles/${id}/join`, { method: "POST" }),
  tailnetLeave: (id: string) =>
    api<TailscaleStatus>(`/api/v1/tailnet/profiles/${id}/leave`, { method: "POST" }),
  torProfiles: () => api<TorProfile[]>("/api/v1/tor"),
  torStatus: () => api<TorStatus>("/api/v1/tor/status"),
  torStart: (id: string) =>
    api<TorProfile>(`/api/v1/tor/profiles/${id}/start`, { method: "POST" }),
  torStop: (id: string) =>
    api<TorStatus>(`/api/v1/tor/profiles/${id}/stop`, { method: "POST" }),
  upsertTorProfile: (profile: TorProfile) =>
    api<TorProfile>("/api/v1/tor", {
      method: "POST",
      body: JSON.stringify(profile),
    }),
  bridges: () => api<BridgeProfile[]>("/api/v1/bridges"),
  createBridge: (bridge: BridgeProfile) =>
    api<BridgeProfile>("/api/v1/bridges", {
      method: "POST",
      body: JSON.stringify(bridge),
    }),
  testBridge: (bridgeId: string) =>
    api<BridgeTestResult>("/api/v1/bridges/test", {
      method: "POST",
      body: JSON.stringify({ bridge_id: bridgeId }),
    }),
  proxies: () => api<ProxyProfile[]>("/api/v1/proxies"),
  getProxy: (id: string) => api<ProxyProfile>(`/api/v1/proxies/${id}`),
  createProxy: (profile: ProxyProfile) =>
    api<ProxyProfile>("/api/v1/proxies", {
      method: "POST",
      body: JSON.stringify(profile),
    }),
  updateProxy: (id: string, profile: ProxyProfile) =>
    api<ProxyProfile>(`/api/v1/proxies/${id}`, {
      method: "PUT",
      body: JSON.stringify(profile),
    }),
  deleteProxy: (id: string) =>
    api<{ ok: boolean }>(`/api/v1/proxies/${id}`, { method: "DELETE" }),
  connectProxy: (id: string) =>
    api<ProxyProfile>(`/api/v1/proxies/${id}/connect`, { method: "POST" }),
  disconnectProxy: (id: string) =>
    api<{ ok: boolean }>(`/api/v1/proxies/${id}/disconnect`, { method: "POST" }),
  healthProxy: (id: string) =>
    api<{ healthy: boolean; message?: string | null }>(
      `/api/v1/proxies/${id}/health`,
      { method: "POST" }
    ),
  latencyProxy: (id: string) =>
    api<{ latency_ms: number }>(`/api/v1/proxies/${id}/latency`, { method: "POST" }),
  proxyChains: () => api<ProxyChain[]>("/api/v1/proxy-chains"),
  getProxyChain: (id: string) => api<ProxyChain>(`/api/v1/proxy-chains/${id}`),
  createProxyChain: (chain: Omit<ProxyChain, "id" | "created_at" | "updated_at">) =>
    api<ProxyChain>("/api/v1/proxy-chains", {
      method: "POST",
      body: JSON.stringify(chain),
    }),
  updateProxyChain: (id: string, chain: ProxyChain) =>
    api<ProxyChain>(`/api/v1/proxy-chains/${id}`, {
      method: "PUT",
      body: JSON.stringify(chain),
    }),
  deleteProxyChain: (id: string) =>
    api<{ ok: boolean }>(`/api/v1/proxy-chains/${id}`, { method: "DELETE" }),
  startProxyChain: (id: string) =>
    api<ProxyChain>(`/api/v1/proxy-chains/${id}/start`, { method: "POST" }),
  stopProxyChain: (id: string) =>
    api<{ ok: boolean }>(`/api/v1/proxy-chains/${id}/stop`, { method: "POST" }),
  dnsProviders: () => api<DnsProviderRecord[]>("/api/v1/dns/providers"),
  updateDnsProviders: (body: DnsProvidersUpdate) =>
    api<{ ok: boolean }>("/api/v1/dns/providers", {
      method: "PUT",
      body: JSON.stringify(body),
    }),
  logs: (params?: { limit?: number; level?: string }) => {
    const q = new URLSearchParams();
    if (params?.limit) q.set("limit", String(params.limit));
    if (params?.level) q.set("level", params.level);
    const qs = q.toString();
    return api<LogEntry[]>(`/api/v1/logs${qs ? `?${qs}` : ""}`);
  },
  downloadLogs: () => apiBlob("/api/v1/logs/download"),
  setLogLevel: (level: LogLevel) =>
    api<{ ok: boolean }>("/api/v1/settings/log-level", {
      method: "PUT",
      body: JSON.stringify({ level }),
    }),
  diagnostics: () => api<DiagnosticsHealth>("/api/v1/diagnostics"),
  validation: () => api<ValidationReport>("/api/v1/validation"),
  exportDiagnostics: () =>
    apiBlob("/api/v1/diagnostics/export", { method: "POST" }),
  backupExport: (format: "json" | "encrypted" = "json") => {
    const q = new URLSearchParams({ format });
    if (format === "encrypted") {
      return apiBlob(`/api/v1/backup/export?${q.toString()}`);
    }
    return api<BackupBundle>(`/api/v1/backup/export?${q.toString()}`);
  },
  backupImport: (data: string, format: "json" | "encrypted" = "json") =>
    api<{ ok: boolean }>("/api/v1/backup/import", {
      method: "POST",
      body: JSON.stringify({ format, data }),
    }),
  performance: (limit = 20) =>
    api<PerformanceResponse>(`/api/v1/performance?limit=${limit}`),
  metrics: (format: "json" | "prometheus" = "json") => {
    const q = new URLSearchParams({ format });
    if (format === "prometheus") {
      return apiText(`/api/v1/metrics?${q.toString()}`);
    }
    return api<MetricsSnapshot>(`/api/v1/metrics?${q.toString()}`);
  },
  updateInfo: () => api<UpdateInfo>("/api/v1/update"),
  checkUpdate: () => api<UpdateInfo>("/api/v1/update/check", { method: "POST" }),
  enterprisePolicy: () => api<EnterprisePolicy>("/api/v1/enterprise/policy"),
  setEnterprisePolicy: (policy: EnterprisePolicy) =>
    api<{ ok: boolean }>("/api/v1/enterprise/policy", {
      method: "PUT",
      body: JSON.stringify(policy),
    }),
  mixnet: () => api<MixnetProfile[]>("/api/v1/mixnet").catch(() => [] as MixnetProfile[]),
  mixnetStatus: () =>
    api<MixnetStatus | null>("/api/v1/mixnet/status").catch(() => null),
  enforcementSettings: () =>
    api<EnforcementSettings>("/api/v1/settings/enforcement"),
  setEnforcementBackend: (backend: EnforcementBackend) =>
    api<EnforcementSettings>("/api/v1/settings/enforcement", {
      method: "PUT",
      body: JSON.stringify({ enforcement_backend: backend }),
    }),
  kernelStatus: () =>
    api<KernelStatus | null>("/api/v1/kernel/status").catch(() => null),
  kernelTelemetry: () =>
    api<KernelTelemetry | null>("/api/v1/kernel/telemetry").catch(() => null),
  kernelRoutes: () =>
    api<KernelRouteEntry[]>("/api/v1/kernel/routes").catch(() => [] as KernelRouteEntry[]),
  kernelPackets: () =>
    api<KernelPacketEntry[]>("/api/v1/kernel/packets").catch(() => [] as KernelPacketEntry[]),
  ndisStatus: () =>
    api<NdisStatus | null>("/api/v1/kernel/ndis/status").catch(() => null),
  anonymousRoutes: () =>
    api<AnonymousChain[]>("/api/v1/anonymous-routes").catch(() => [] as AnonymousChain[]),
  coverTraffic: () =>
    api<CoverTrafficSettings>("/api/v1/cover-traffic/settings").catch(() => null),
  setCoverTrafficSettings: (settings: CoverTrafficSettings) =>
    api<CoverTrafficSettings>("/api/v1/cover-traffic/settings", {
      method: "PUT",
      body: JSON.stringify(settings),
    }),
  privacyAnalytics: () =>
    api<PrivacyAnalyticsSnapshot | null>("/api/v1/privacy/analytics").catch(() => null),
  privacyAnonymity: () =>
    api<AnonymityPrivacySnapshot | null>("/api/v1/privacy/anonymity").catch(() => null),
  anonymityHealth: () =>
    api<AnonymityHealthSummary | null>("/api/v1/anonymity").catch(() => null),
  mixnetStart: () =>
    api<{ ok: boolean }>("/api/v1/mixnet/start", { method: "POST" }),
  mixnetStop: () =>
    api<{ ok: boolean }>("/api/v1/mixnet/stop", { method: "POST" }),
  startAnonymousRoute: (id: string) =>
    api<AnonymousChain>(`/api/v1/anonymous-routes/${id}/start`, { method: "POST" }),
  stopAnonymousRoute: (id: string) =>
    api<AnonymousChain>(`/api/v1/anonymous-routes/${id}/stop`, { method: "POST" }),
  upsertAnonymousRoute: (route: AnonymousChain) =>
    api<AnonymousChain>("/api/v1/anonymous-routes", {
      method: "POST",
      body: JSON.stringify(route),
    }),
  updateAnonymousRoute: (id: string, route: AnonymousChain) =>
    api<AnonymousChain>(`/api/v1/anonymous-routes/${id}`, {
      method: "PUT",
      body: JSON.stringify(route),
    }),
  loadPlugin: (id: string) =>
    api<PluginRecord>("/api/v1/plugins/load", {
      method: "POST",
      body: JSON.stringify({ id }),
    }),
  unloadPlugin: (id: string) =>
    api<{ ok: boolean }>(`/api/v1/plugins/unload`, {
      method: "POST",
      body: JSON.stringify({ id }),
    }),
  securityAudit: () =>
    api<SecurityFinding[]>("/api/v1/security/audit").catch(() => [] as SecurityFinding[]),
  runSecurityAudit: () =>
    api<SecurityFinding[]>("/api/v1/security/audit/run", { method: "POST" }),

  tcpTerminationSettings: () =>
    api<TcpTerminationSettings>("/api/v1/tcp-termination/settings"),
  setTcpTerminationSettings: (body: { mode: TcpTerminationMode }) =>
    api<TcpTerminationSettings>("/api/v1/tcp-termination/settings", {
      method: "PUT",
      body: JSON.stringify(body),
    }),
  tcpTerminationRules: () => api<TcpTerminationRule[]>("/api/v1/tcp-termination/rules"),
  tcpTerminationPolicy: () => api<TcpTerminationPolicy>("/api/v1/tcp-termination"),
  addTcpTerminationRule: (rule: TcpTerminationRule) =>
    api<TcpTerminationRule>("/api/v1/tcp-termination/rules", {
      method: "POST",
      body: JSON.stringify(rule),
    }),
  updateTcpTerminationRule: (id: string, rule: TcpTerminationRule) =>
    api<TcpTerminationRule>(`/api/v1/tcp-termination/rules/${id}`, {
      method: "PUT",
      body: JSON.stringify(rule),
    }),
  deleteTcpTerminationRule: (id: string) =>
    api<{ ok: boolean }>(`/api/v1/tcp-termination/rules/${id}`, { method: "DELETE" }),

  splitTemplates: () => api<SplitTunnelTemplate[]>("/api/v1/split-templates"),
  createSplitTemplate: (template: SplitTunnelTemplate) =>
    api<SplitTunnelTemplate>("/api/v1/split-templates", {
      method: "POST",
      body: JSON.stringify(template),
    }),
  updateSplitTemplate: (id: string, template: SplitTunnelTemplate) =>
    api<SplitTunnelTemplate>(`/api/v1/split-templates/${id}`, {
      method: "PUT",
      body: JSON.stringify(template),
    }),
  deleteSplitTemplate: (id: string) =>
    api<{ ok: boolean }>(`/api/v1/split-templates/${id}`, { method: "DELETE" }),
  cloneSplitTemplate: (id: string, name?: string) =>
    api<SplitTunnelTemplate>(`/api/v1/split-templates/${id}/clone`, {
      method: "POST",
      body: JSON.stringify(name ? { name } : {}),
    }),
  splitTemplateMode: () => api<SplitTemplateModeSettings>("/api/v1/split-templates/mode"),
  setSplitTemplateMode: (settings: {
    mode: TemplateMode;
    active_template_id?: string | null;
  }) =>
    api<SplitTemplateModeSettings>("/api/v1/split-templates/mode", {
      method: "PUT",
      body: JSON.stringify(settings),
    }),

  getVpnHandshakeProxy: (profileId: string) =>
    api<HandshakeProxySettings>(`/api/v1/vpn/${profileId}/handshake-proxy`),
  setVpnHandshakeProxy: (profileId: string, settings: HandshakeProxySettings) =>
    api<HandshakeProxySettings>(`/api/v1/vpn/${profileId}/handshake-proxy`, {
      method: "PUT",
      body: JSON.stringify(settings),
    }),

  diagnosticsWiresock: () =>
    api<WiresockDiagnostics>("/api/v1/diagnostics/wiresock"),
  runWiresockTemplateTrace: () =>
    api<TemplateResolutionTrace>("/api/v1/diagnostics/wiresock/template-trace", {
      method: "POST",
    }),
};

export function connectEvents(onEvent: (data: unknown) => void): WebSocket {
  if (!authToken) throw new Error("auth not initialized");
  const ws = new WebSocket(
    `${WS_BASE}/api/v1/events?token=${encodeURIComponent(authToken)}`
  );
  ws.onmessage = (ev) => {
    try {
      onEvent(JSON.parse(ev.data));
    } catch {
      /* ignore malformed */
    }
  };
  return ws;
}

export { BASE as API_BASE };
