import type {
  AppSummary,
  AuditLogEntry,
  BandwidthSnapshot,
  ChainProfile,
  DnsProviderRecord,
  DnsQueryLog,
  FilterListRecord,
  LeakIncident,
  PrivacyScoreSnapshot,
  RouteStatisticsRecord,
  Rule,
  ServiceStatus,
  TopDomainEntry,
  TrafficEvent,
  TrafficRoute,
  TransportProfile,
  TransportStatusRecord,
  VpnProfile,
  VpnState,
  PerformanceSnapshot,
  SecurityAuditEntry,
  TorStatus,
} from "../api/client";

export type AppRecordWire = {
  app_id: string;
  display_name: string;
  exe_path: string;
  publisher?: string | null;
  sha256?: string | null;
  default_route?: TrafficRoute | null;
};

export type ServiceEvent =
  | { kind: "vpn_connected"; profile_id: string; profile_name: string; timestamp: string }
  | { kind: "vpn_disconnected"; profile_id: string; reason: string; timestamp: string }
  | { kind: "vpn_error"; profile_id: string; message: string; timestamp: string }
  | { kind: "traffic_observed"; event: TrafficEvent; timestamp: string }
  | { kind: "traffic_allowed"; event: TrafficEvent; timestamp: string }
  | { kind: "traffic_blocked"; event: TrafficEvent; route: TrafficRoute; timestamp: string }
  | { kind: "rule_created"; rule: Rule; timestamp: string }
  | { kind: "rule_updated"; rule: Rule; timestamp: string }
  | { kind: "rule_deleted"; rule_id: string; timestamp: string }
  | { kind: "dns_query_observed"; log: DnsQueryLog; timestamp: string }
  | { kind: "dns_query_blocked"; log: DnsQueryLog; timestamp: string }
  | { kind: "filter_list_updated"; list_id: string; name: string; entry_count: number; timestamp: string }
  | { kind: "filter_list_failed"; list_id: string; error: string; timestamp: string }
  | { kind: "app_discovered"; app: AppRecordWire; timestamp: string }
  | { kind: "app_updated"; app: AppRecordWire; timestamp: string }
  | { kind: "bandwidth_updated"; snapshot: BandwidthSnapshot; timestamp: string }
  | { kind: "route_usage_updated"; stats: RouteStatisticsRecord; timestamp: string }
  | { kind: "policy_changed"; field: string; old_value?: string | null; new_value?: string | null; timestamp: string }
  | { kind: "route_changed"; app_id: string; old_route?: TrafficRoute | null; new_route?: TrafficRoute | null; timestamp: string }
  | { kind: "exit_failover"; app_id: string; from_index: number; to_index: number; route: TrafficRoute; timestamp: string }
  | { kind: "exit_exhausted"; app_id: string; action: string; timestamp: string }
  | { kind: "system_warning"; message: string; timestamp: string }
  | { kind: "system_error"; message: string; timestamp: string }
  | { kind: "service_status"; status: ServiceStatus; timestamp: string }
  | { kind: "transport_started"; transport_id: string; name: string; timestamp: string }
  | { kind: "transport_stopped"; transport_id: string; reason: string; timestamp: string }
  | { kind: "transport_error"; transport_id: string; message: string; timestamp: string }
  | { kind: "dns_provider_changed"; provider_id: string; provider_name: string; timestamp: string }
  | {
      kind: "dns_provider_failed";
      provider_id: string;
      provider_name: string;
      error: string;
      timestamp: string;
    }
  | { kind: "privacy_score_updated"; snapshot: PrivacyScoreSnapshot; timestamp: string }
  | { kind: "leak_detected"; incident: LeakIncident; timestamp: string }
  | { kind: "recovery_started"; scope: string; timestamp: string }
  | { kind: "recovery_completed"; restored_count: number; timestamp: string }
  | { kind: "recovery_failed"; scope: string; error: string; timestamp: string }
  | { kind: "performance_snapshot"; snapshot: PerformanceSnapshot; timestamp: string }
  | { kind: "security_audit"; entry: SecurityAuditEntry; timestamp: string }
  | { kind: "plugin_loaded"; plugin_id: string; name: string; timestamp: string }
  | { kind: "plugin_unloaded"; plugin_id: string; reason: string; timestamp: string }
  | { kind: "plugin_failed"; plugin_id: string; error: string; timestamp: string }
  | { kind: "tailnet_joined"; profile_id: string; hostname?: string | null; timestamp: string }
  | { kind: "tailnet_left"; profile_id: string; reason: string; timestamp: string }
  | { kind: "tor_started"; profile_id: string; timestamp: string }
  | { kind: "tor_stopped"; profile_id: string; reason: string; timestamp: string }
  | { kind: "tor_circuit_changed"; profile_id: string; circuit_count: number; timestamp: string }
  | { kind: "transport_chain_updated"; chain: ChainProfile; timestamp: string }
  | { kind: "transport_chain_started"; chain_id: string; name: string; timestamp: string }
  | { kind: "transport_chain_stopped"; chain_id: string; reason: string; timestamp: string }
  | { kind: "proxy_connected"; profile_id: string; listen_port: number; timestamp: string }
  | { kind: "proxy_disconnected"; profile_id: string; reason: string; timestamp: string }
  | { kind: "proxy_failed"; profile_id: string; error: string; timestamp: string }
  | { kind: "proxy_chain_started"; chain_id: string; name: string; timestamp: string }
  | { kind: "proxy_chain_stopped"; chain_id: string; reason: string; timestamp: string }
  | { kind: "agent_enrolled"; agent_id: string; name: string; timestamp: string }
  | { kind: "agent_revoked"; agent_id: string; reason: string; timestamp: string }
  | { kind: "obfuscation_profile_applied"; chain_id: string; profile_id: string; preset: string; timestamp: string };

export interface RecoveryState {
  status: "idle" | "running" | "completed" | "failed";
  scope?: string;
  restoredCount?: number | null;
  lastError?: string | null;
}

export interface EventState {
  status: ServiceStatus | null;
  bandwidth: BandwidthSnapshot[];
  apps: AppSummary[];
  vpnProfiles: Array<{ profile: VpnProfile; status: string | VpnState }>;
  rules: Rule[];
  dnsLogs: DnsQueryLog[];
  topDomains: TopDomainEntry[];
  filterLists: FilterListRecord[];
  routeStats: RouteStatisticsRecord[];
  blockedStats: RouteStatisticsRecord[];
  auditLog: AuditLogEntry[];
  privacySnapshot: PrivacyScoreSnapshot | null;
  transports: TransportProfile[];
  transportStatus: TransportStatusRecord[];
  chains: ChainProfile[];
  dnsProviders: DnsProviderRecord[];
  leakIncidents: LeakIncident[];
  performanceSnapshots: PerformanceSnapshot[];
  securityAudit: SecurityAuditEntry[];
  recovery: RecoveryState;
  torStatus: TorStatus | null;
  recentEvents: ServiceEvent[];
  lastEvent: ServiceEvent | null;
}

export const initialEventState: EventState = {
  status: null,
  bandwidth: [],
  apps: [],
  vpnProfiles: [],
  rules: [],
  dnsLogs: [],
  topDomains: [],
  filterLists: [],
  routeStats: [],
  blockedStats: [],
  auditLog: [],
  privacySnapshot: null,
  transports: [],
  transportStatus: [],
  chains: [],
  dnsProviders: [],
  leakIncidents: [],
  performanceSnapshots: [],
  securityAudit: [],
  recovery: { status: "idle", restoredCount: null, lastError: null },
  torStatus: null,
  recentEvents: [],
  lastEvent: null,
};

function wireToSummary(app: AppRecordWire): AppSummary {
  return {
    id: app.app_id,
    display_name: app.display_name,
    exe_path: app.exe_path,
    publisher: app.publisher,
    sha256: app.sha256,
    default_route: app.default_route,
    bytes_in: 0,
    bytes_out: 0,
    connection_count: 0,
  };
}

function pushRecent(state: EventState, event: ServiceEvent): ServiceEvent[] {
  return [event, ...state.recentEvents].slice(0, 20);
}

function upsertRouteStat(stats: RouteStatisticsRecord[], row: RouteStatisticsRecord) {
  const idx = stats.findIndex((s) => s.id === row.id);
  if (idx >= 0) {
    return stats.map((s, i) => (i === idx ? row : s));
  }
  return [row, ...stats].slice(0, 100);
}

function upsertTransportStatus(
  status: TransportStatusRecord[],
  row: TransportStatusRecord
): TransportStatusRecord[] {
  const idx = status.findIndex((s) => s.id === row.id);
  if (idx >= 0) {
    return status.map((s, i) => (i === idx ? { ...s, ...row } : s));
  }
  return [row, ...status];
}

export function reduceEvent(state: EventState, event: ServiceEvent): EventState {
  switch (event.kind) {
    case "service_status":
      return { ...state, status: event.status, lastEvent: event, recentEvents: pushRecent(state, event) };
    case "bandwidth_updated": {
      const idx = state.bandwidth.findIndex((b) => b.app_id === event.snapshot.app_id);
      const bandwidth =
        idx >= 0
          ? state.bandwidth.map((b, i) => (i === idx ? event.snapshot : b))
          : [...state.bandwidth, event.snapshot].slice(-100);
      return { ...state, bandwidth, lastEvent: event, recentEvents: pushRecent(state, event) };
    }
    case "app_discovered": {
      const summary = wireToSummary(event.app);
      return {
        ...state,
        apps: [...state.apps.filter((a) => a.id !== summary.id), summary],
        lastEvent: event,
        recentEvents: pushRecent(state, event),
      };
    }
    case "app_updated": {
      const summary = wireToSummary(event.app);
      return {
        ...state,
        apps: state.apps.map((a) => (a.id === summary.id ? summary : a)),
        lastEvent: event,
        recentEvents: pushRecent(state, event),
      };
    }
    case "vpn_connected":
    case "vpn_disconnected":
    case "vpn_error":
      return {
        ...state,
        vpnProfiles: state.vpnProfiles.map((entry) =>
          entry.profile.id === event.profile_id
            ? {
                ...entry,
                status:
                  event.kind === "vpn_connected"
                    ? "connected"
                    : event.kind === "vpn_disconnected"
                      ? "disconnected"
                      : `error: ${"message" in event ? event.message : "unknown"}`,
              }
            : entry
        ),
        lastEvent: event,
        recentEvents: pushRecent(state, event),
      };
    case "rule_created":
      return {
        ...state,
        rules: [...state.rules, event.rule],
        lastEvent: event,
        recentEvents: pushRecent(state, event),
      };
    case "rule_updated":
      return {
        ...state,
        rules: state.rules.map((r) => (r.id === event.rule.id ? event.rule : r)),
        lastEvent: event,
        recentEvents: pushRecent(state, event),
      };
    case "rule_deleted":
      return {
        ...state,
        rules: state.rules.filter((r) => r.id !== event.rule_id),
        lastEvent: event,
        recentEvents: pushRecent(state, event),
      };
    case "dns_query_observed":
    case "dns_query_blocked":
      return {
        ...state,
        dnsLogs: [event.log, ...state.dnsLogs].slice(0, 100),
        lastEvent: event,
        recentEvents: pushRecent(state, event),
      };
    case "route_usage_updated": {
      const routeStats = upsertRouteStat(state.routeStats, event.stats);
      const blockedStats =
        event.stats.route_type === "blocked"
          ? upsertRouteStat(state.blockedStats, event.stats)
          : state.blockedStats;
      return {
        ...state,
        routeStats,
        blockedStats,
        lastEvent: event,
        recentEvents: pushRecent(state, event),
      };
    }
    case "policy_changed":
      return {
        ...state,
        status: state.status
          ? {
              ...state.status,
              kill_switch_active:
                event.field === "kill_switch_active"
                  ? event.new_value === "true"
                  : state.status.kill_switch_active,
              policy_mode:
                event.field === "policy_mode" && event.new_value?.includes("Whitelist")
                  ? "whitelist"
                  : event.field === "policy_mode" && event.new_value?.includes("Blacklist")
                    ? "blacklist"
                    : state.status.policy_mode,
            }
          : state.status,
        lastEvent: event,
        recentEvents: pushRecent(state, event),
      };
    case "route_changed":
      return {
        ...state,
        apps: state.apps.map((a) =>
          a.id === event.app_id ? { ...a, default_route: event.new_route ?? null } : a
        ),
        lastEvent: event,
        recentEvents: pushRecent(state, event),
      };
    case "exit_failover":
      return {
        ...state,
        apps: state.apps.map((a) =>
          a.id === event.app_id ? { ...a, default_route: event.route } : a,
        ),
        lastEvent: event,
        recentEvents: pushRecent(state, event),
      };
    case "exit_exhausted":
      return {
        ...state,
        lastEvent: event,
        recentEvents: pushRecent(state, event),
      };
    case "filter_list_updated":
      return {
        ...state,
        filterLists: state.filterLists.map((f) =>
          f.id === event.list_id ? { ...f, name: event.name } : f
        ),
        lastEvent: event,
        recentEvents: pushRecent(state, event),
      };
    case "filter_list_failed":
      return { ...state, lastEvent: event, recentEvents: pushRecent(state, event) };
    case "transport_started":
      return {
        ...state,
        transportStatus: upsertTransportStatus(state.transportStatus, {
          id: event.transport_id,
          name: event.name,
          kind: "sing_box",
          state: "running",
          message: null,
        }),
        lastEvent: event,
        recentEvents: pushRecent(state, event),
      };
    case "transport_stopped":
      return {
        ...state,
        transportStatus: upsertTransportStatus(state.transportStatus, {
          id: event.transport_id,
          name:
            state.transportStatus.find((t) => t.id === event.transport_id)?.name ?? event.transport_id,
          kind:
            state.transportStatus.find((t) => t.id === event.transport_id)?.kind ?? "sing_box",
          state: "stopped",
          message: event.reason,
        }),
        lastEvent: event,
        recentEvents: pushRecent(state, event),
      };
    case "transport_error":
      return {
        ...state,
        transportStatus: upsertTransportStatus(state.transportStatus, {
          id: event.transport_id,
          name:
            state.transportStatus.find((t) => t.id === event.transport_id)?.name ?? event.transport_id,
          kind:
            state.transportStatus.find((t) => t.id === event.transport_id)?.kind ?? "sing_box",
          state: "error",
          message: event.message,
        }),
        lastEvent: event,
        recentEvents: pushRecent(state, event),
      };
    case "dns_provider_changed":
      return {
        ...state,
        dnsProviders: state.dnsProviders.map((p) =>
          p.id === event.provider_id ? { ...p, name: event.provider_name, failure_count: 0 } : p
        ),
        lastEvent: event,
        recentEvents: pushRecent(state, event),
      };
    case "dns_provider_failed":
      return {
        ...state,
        dnsProviders: state.dnsProviders.map((p) =>
          p.id === event.provider_id
            ? { ...p, failure_count: p.failure_count + 1 }
            : p
        ),
        lastEvent: event,
        recentEvents: pushRecent(state, event),
      };
    case "privacy_score_updated":
      return {
        ...state,
        privacySnapshot: event.snapshot,
        lastEvent: event,
        recentEvents: pushRecent(state, event),
      };
    case "leak_detected":
      return {
        ...state,
        leakIncidents: [event.incident, ...state.leakIncidents].slice(0, 100),
        lastEvent: event,
        recentEvents: pushRecent(state, event),
      };
    case "recovery_started":
      return {
        ...state,
        recovery: {
          status: "running",
          scope: event.scope,
          restoredCount: null,
          lastError: null,
        },
        lastEvent: event,
        recentEvents: pushRecent(state, event),
      };
    case "recovery_completed":
      return {
        ...state,
        recovery: {
          status: "completed",
          scope: state.recovery.scope,
          restoredCount: event.restored_count,
          lastError: null,
        },
        lastEvent: event,
        recentEvents: pushRecent(state, event),
      };
    case "recovery_failed":
      return {
        ...state,
        recovery: {
          status: "failed",
          scope: event.scope,
          restoredCount: state.recovery.restoredCount ?? null,
          lastError: event.error,
        },
        lastEvent: event,
        recentEvents: pushRecent(state, event),
      };
    case "performance_snapshot": {
      const performanceSnapshots = [event.snapshot, ...state.performanceSnapshots].slice(0, 50);
      return {
        ...state,
        performanceSnapshots,
        lastEvent: event,
        recentEvents: pushRecent(state, event),
      };
    }
    case "security_audit":
      return {
        ...state,
        securityAudit: [event.entry, ...state.securityAudit].slice(0, 100),
        lastEvent: event,
        recentEvents: pushRecent(state, event),
      };
    case "transport_chain_updated":
      return {
        ...state,
        chains: [...state.chains.filter((c) => c.id !== event.chain.id), event.chain],
        lastEvent: event,
        recentEvents: pushRecent(state, event),
      };
    case "tor_started":
      return {
        ...state,
        torStatus: state.torStatus
          ? { ...state.torStatus, running: true, bootstrap_progress: 100 }
          : state.torStatus,
        lastEvent: event,
        recentEvents: pushRecent(state, event),
      };
    case "tor_stopped":
      return {
        ...state,
        torStatus: state.torStatus
          ? {
              ...state.torStatus,
              running: false,
              bootstrap_progress: 0,
              circuit_count: 0,
            }
          : state.torStatus,
        lastEvent: event,
        recentEvents: pushRecent(state, event),
      };
    case "tor_circuit_changed":
      return {
        ...state,
        torStatus: state.torStatus
          ? { ...state.torStatus, circuit_count: event.circuit_count }
          : state.torStatus,
        lastEvent: event,
        recentEvents: pushRecent(state, event),
      };
    case "transport_chain_started":
      return {
        ...state,
        transportStatus: upsertTransportStatus(state.transportStatus, { id: event.chain_id, name: event.name, kind: "sing_box", state: "running" }),
        lastEvent: event,
        recentEvents: pushRecent(state, event),
      };
    case "transport_chain_stopped":
      return {
        ...state,
        transportStatus: upsertTransportStatus(state.transportStatus, { id: event.chain_id, name: event.chain_id.slice(0,8), kind: "sing_box", state: "stopped" }),
        lastEvent: event,
        recentEvents: pushRecent(state, event),
      };
    case "proxy_connected":
    case "proxy_disconnected":
    case "proxy_failed":
      return {
        ...state,
        transportStatus: upsertTransportStatus(state.transportStatus, { id: event.profile_id, name: event.profile_id.slice(0, 8), kind: "sing_box", state: event.kind === "proxy_connected" ? "running" : "stopped" }),
        lastEvent: event,
        recentEvents: pushRecent(state, event),
      };
    case "proxy_chain_started":
    case "proxy_chain_stopped":
      return {
        ...state,
        lastEvent: event,
        recentEvents: pushRecent(state, event),
      };
    case "plugin_loaded":
    case "plugin_unloaded":
    case "plugin_failed":
    case "tailnet_joined":
    case "tailnet_left":
      return { ...state, lastEvent: event, recentEvents: pushRecent(state, event) };
    case "traffic_observed":
    case "traffic_allowed":
    case "traffic_blocked":
    case "obfuscation_profile_applied":
    case "agent_enrolled":
    case "agent_revoked":
      return { ...state, lastEvent: event, recentEvents: pushRecent(state, event) };

    default:
      return { ...state, lastEvent: event, recentEvents: pushRecent(state, event) };
  }
}
