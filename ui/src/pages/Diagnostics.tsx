import { useEffect, useState } from "react";
import {
  apiClient,
  type DiagnosticsHealth,
  type LogEntry,
  type SubsystemHealth,
  type TcpConnectionSnapshot,
  type TemplateResolutionTrace,
  type ValidationReport,
  type TrafficRoute,
  type VpnGatewayCompatHandshakeProxyProfile,
} from "../api/client";
import { routeLabel as formatRouteLabel } from "../lib/routeLabels";

const SUBSYSTEMS: Array<{ key: keyof DiagnosticsHealth; label: string }> = [
  { key: "wfp", label: "WFP Engine" },
  { key: "vpn", label: "VPN" },
  { key: "dns", label: "DNS" },
  { key: "transport", label: "Transport" },
  { key: "database", label: "Database" },
  { key: "disk", label: "Disk" },
];

function statusClass(status: string) {
  if (status === "ok") return "text-sentinel-success";
  if (status === "error") return "text-sentinel-danger";
  if (status === "disabled") return "text-sentinel-muted";
  return "text-yellow-400";
}

function validationStatusClass(status: string) {
  if (status === "pass") return "text-sentinel-success";
  if (status === "fail") return "text-sentinel-danger";
  return "text-yellow-400";
}

function routeLabel(route: TrafficRoute | null | undefined): string {
  if (!route) return "—";
  return formatRouteLabel(route);
}

function HealthCard({ label, health }: { label: string; health: SubsystemHealth }) {
  return (
    <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
      <div className="flex items-center justify-between mb-2">
        <h3 className="font-medium">{label}</h3>
        <span className={`text-sm font-semibold uppercase ${statusClass(health.status)}`}>
          {health.status}
        </span>
      </div>
      {health.message && (
        <p className="text-xs text-sentinel-muted break-all">{health.message}</p>
      )}
    </div>
  );
}

function TcpSessionRow({ conn }: { conn: TcpConnectionSnapshot }) {
  return (
    <li className="p-2 bg-slate-800/50 rounded text-sm">
      <div className="flex justify-between gap-4">
        <span className="font-medium">{conn.exe_name}</span>
        <span className="text-xs text-sentinel-muted">PID {conn.pid}</span>
      </div>
      <p className="text-xs text-sentinel-muted mt-1 font-mono">
        {conn.local_addr} → {conn.remote_addr}
        {conn.remote_domain ? ` (${conn.remote_domain})` : ""}
      </p>
      <p className="text-xs text-sentinel-muted">
        {conn.protocol} · {conn.state}
      </p>
    </li>
  );
}

function HandshakeProxyRow({ entry }: { entry: VpnGatewayCompatHandshakeProxyProfile }) {
  const settings = entry.settings;
  const enabled = settings?.enabled ?? false;
  return (
    <li className="p-3 bg-slate-800/50 rounded flex flex-wrap justify-between gap-2 text-sm">
      <div>
        <p className="font-medium">{entry.profile_name || entry.profile_id.slice(0, 8)}</p>
        <p className="text-xs text-sentinel-muted">
          {enabled && settings?.host
            ? `${settings.host}:${settings.port ?? "—"}`
            : enabled
              ? "Enabled (no host)"
              : "Disabled"}
        </p>
      </div>
      <span
        className={
          enabled ? "text-yellow-400 text-xs font-semibold uppercase" : "text-sentinel-muted text-xs uppercase"
        }
      >
        {enabled ? "Configured" : "Off"}
      </span>
    </li>
  );
}

export function Diagnostics() {
  const [health, setHealth] = useState<DiagnosticsHealth | null>(null);
  const [validation, setValidation] = useState<ValidationReport | null>(null);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [tcpSessions, setTcpSessions] = useState<TcpConnectionSnapshot[]>([]);
  const [handshakeProfiles, setHandshakeProfiles] = useState<VpnGatewayCompatHandshakeProxyProfile[]>([]);
  const [templateTrace, setTemplateTrace] = useState<TemplateResolutionTrace | null>(null);
  const [loading, setLoading] = useState(true);
  const [exporting, setExporting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = async () => {
    setLoading(true);
    setError(null);
    try {
      const [diag, recentLogs, validationReport, gatewayCompat] = await Promise.all([
        apiClient.diagnostics(),
        apiClient.logs({ limit: 30 }),
        apiClient.validation(),
        apiClient.diagnosticsVpnGatewayCompat(),
      ]);
      setHealth(diag);
      setValidation(validationReport);
      setLogs(recentLogs);
      setTcpSessions(gatewayCompat.tcp_sessions);
      setHandshakeProfiles(gatewayCompat.handshake_proxy_profiles);
      setTemplateTrace(gatewayCompat.template_trace ?? null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load diagnostics");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    refresh();
  }, []);

  const runTemplateTrace = async () => {
    setError(null);
    try {
      const trace = await apiClient.runVpnGatewayCompatTemplateTrace();
      setTemplateTrace(trace);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Template trace failed");
    }
  };

  const exportZip = async () => {
    setExporting(true);
    setError(null);
    try {
      const blob = await apiClient.exportDiagnostics();
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `wiresentinel-diagnostics-${new Date().toISOString().slice(0, 10)}.zip`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Export failed");
    } finally {
      setExporting(false);
    }
  };

  if (loading && !health) {
    return <p className="text-sentinel-muted">Loading diagnostics...</p>;
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-semibold">Diagnostics</h2>
        <div className="flex gap-2">
          <button
            onClick={refresh}
            className="px-4 py-2 bg-slate-700 rounded text-sm hover:bg-slate-600"
          >
            Refresh
          </button>
          <button
            onClick={exportZip}
            disabled={exporting}
            className="px-4 py-2 bg-sentinel-accent rounded text-sm hover:bg-blue-600 disabled:opacity-50"
          >
            {exporting ? "Exporting..." : "Export diagnostics ZIP"}
          </button>
        </div>
      </div>

      {error && (
        <div className="p-3 bg-red-900/30 border border-red-700 rounded text-sm">{error}</div>
      )}

      {health && (
        <div className="grid grid-cols-3 gap-4">
          {SUBSYSTEMS.map(({ key, label }) => (
            <HealthCard key={key} label={label} health={health[key]} />
          ))}
        </div>
      )}

      {validation && (
        <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
          <div className="flex items-center justify-between mb-3">
            <h3 className="font-medium">Native validation</h3>
            <span
              className={`text-sm font-semibold uppercase ${validationStatusClass(validation.overall_status)}`}
            >
              {validation.overall_status}
            </span>
          </div>
          <ul className="space-y-2 text-sm">
            {validation.checks.map((check) => (
              <li key={check.id} className="flex items-start justify-between gap-4">
                <span className="font-mono text-slate-300">{check.check_name}</span>
                <span className={`uppercase text-xs font-semibold ${validationStatusClass(check.status)}`}>
                  {check.status}
                </span>
                {check.message && (
                  <span className="text-xs text-sentinel-muted flex-1 text-right break-all">
                    {check.message}
                  </span>
                )}
              </li>
            ))}
          </ul>
        </div>
      )}

      <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
        <h3 className="font-medium mb-3">TCP Sessions</h3>
        {tcpSessions.length === 0 ? (
          <p className="text-sentinel-muted text-sm">No active TCP sessions tracked</p>
        ) : (
          <ul className="space-y-2 max-h-48 overflow-auto">
            {tcpSessions.map((conn, i) => (
              <TcpSessionRow key={`${conn.pid}-${conn.local_addr}-${i}`} conn={conn} />
            ))}
          </ul>
        )}
      </div>

      <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
        <h3 className="font-medium mb-3">Handshake Proxy Profiles</h3>
        {handshakeProfiles.length === 0 ? (
          <p className="text-sentinel-muted text-sm">No VPN profiles configured</p>
        ) : (
          <ul className="space-y-2">
            {handshakeProfiles.map((entry) => (
              <HandshakeProxyRow key={entry.profile_id} entry={entry} />
            ))}
          </ul>
        )}
      </div>

      <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4 space-y-4">
        <div className="flex items-center justify-between">
          <h3 className="font-medium">Template Resolution Trace</h3>
          <button
            onClick={runTemplateTrace}
            className="px-4 py-2 bg-sentinel-accent rounded text-sm hover:bg-blue-600"
          >
            Run trace
          </button>
        </div>
        {!templateTrace ? (
          <p className="text-sentinel-muted text-sm">Run a trace to inspect active split-tunnel template resolution</p>
        ) : (
          <div className="space-y-3 text-sm">
            <div className="flex gap-4 text-xs text-sentinel-muted">
              <span>Mode: {templateTrace.mode}</span>
              {templateTrace.template_id && (
                <span>Template: {templateTrace.template_id.slice(0, 8)}…</span>
              )}
              {templateTrace.final_route && (
                <span>Final: {routeLabel(templateTrace.final_route)}</span>
              )}
            </div>
            <ul className="space-y-2">
              {templateTrace.steps.map((step, i) => (
                <li key={i} className="p-2 bg-slate-800/50 rounded font-mono text-xs">
                  <span className="text-sentinel-accent">{step.stage}</span>
                  <span className="text-sentinel-muted mx-2">→</span>
                  {step.detail}
                  {step.route && (
                    <span className="text-sentinel-muted ml-2">({routeLabel(step.route)})</span>
                  )}
                </li>
              ))}
            </ul>
          </div>
        )}
      </div>

      <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
        <h3 className="font-medium mb-3">Recent service logs</h3>
        {logs.length === 0 ? (
          <p className="text-sentinel-muted text-sm">No log entries</p>
        ) : (
          <ul className="space-y-1 text-xs font-mono max-h-80 overflow-auto">
            {logs.map((entry, i) => (
              <li key={i} className="text-sentinel-muted">
                <span className="text-slate-400">
                  {entry.timestamp.slice(11, 19)}
                </span>{" "}
                <span
                  className={
                    entry.level === "ERROR"
                      ? "text-sentinel-danger"
                      : entry.level === "WARN"
                        ? "text-yellow-400"
                        : ""
                  }
                >
                  [{entry.level}]
                </span>{" "}
                {entry.message}
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
