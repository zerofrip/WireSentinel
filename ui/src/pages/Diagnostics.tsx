import { useEffect, useState } from "react";
import {
  apiClient,
  type DiagnosticsHealth,
  type HandshakeProxyStatus,
  type LogEntry,
  type SubsystemHealth,
  type TcpSessionEvent,
  type TemplateResolutionTrace,
  type ValidationReport,
  type TrafficRoute,
} from "../api/client";

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
  switch (route.type) {
    case "direct":
      return "Direct";
    case "blocked":
      return "Blocked";
    case "wire_guard":
    case "amnezia_wg":
      return `VPN (${route.value.slice(0, 8)}…)`;
  }
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

export function Diagnostics() {
  const [health, setHealth] = useState<DiagnosticsHealth | null>(null);
  const [validation, setValidation] = useState<ValidationReport | null>(null);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [tcpEvents, setTcpEvents] = useState<TcpSessionEvent[]>([]);
  const [handshakeStatus, setHandshakeStatus] = useState<HandshakeProxyStatus | null>(null);
  const [templateTrace, setTemplateTrace] = useState<TemplateResolutionTrace | null>(null);
  const [traceAppId, setTraceAppId] = useState("");
  const [traceDomain, setTraceDomain] = useState("");
  const [tracePid, setTracePid] = useState("");
  const [loading, setLoading] = useState(true);
  const [exporting, setExporting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = async () => {
    setLoading(true);
    setError(null);
    try {
      const [diag, recentLogs, validationReport, tcp, proxy, trace] = await Promise.all([
        apiClient.diagnostics(),
        apiClient.logs({ limit: 30 }),
        apiClient.validation(),
        apiClient.diagnosticsTcpEvents(50).catch(() => [] as TcpSessionEvent[]),
        apiClient.diagnosticsHandshakeProxy().catch(() => ({ profiles: [] })),
        apiClient.diagnosticsTemplateTrace().catch(() => null),
      ]);
      setHealth(diag);
      setValidation(validationReport);
      setLogs(recentLogs);
      setTcpEvents(tcp);
      setHandshakeStatus(proxy);
      setTemplateTrace(trace);
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
      const trace = await apiClient.diagnosticsTemplateTrace({
        app_id: traceAppId.trim() || undefined,
        domain: traceDomain.trim() || undefined,
        pid: tracePid.trim() ? Number(tracePid) : undefined,
      });
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
        <h3 className="font-medium mb-3">TCP Session Events</h3>
        {tcpEvents.length === 0 ? (
          <p className="text-sentinel-muted text-sm">No recent TCP termination events</p>
        ) : (
          <ul className="space-y-2 text-sm max-h-48 overflow-auto">
            {tcpEvents.map((ev, i) => (
              <li key={i} className="p-2 bg-slate-800/50 rounded flex justify-between gap-4">
                <div>
                  <span
                    className={
                      ev.kind === "termination_failed"
                        ? "text-sentinel-danger font-medium"
                        : "text-sentinel-success font-medium"
                    }
                  >
                    {ev.kind === "termination_failed" ? "Failed" : "Terminated"}
                  </span>
                  {ev.count != null && (
                    <span className="text-sentinel-muted ml-2">{ev.count} session(s)</span>
                  )}
                  {ev.mode && (
                    <span className="text-xs text-sentinel-muted ml-2">mode: {ev.mode}</span>
                  )}
                  {ev.error && (
                    <p className="text-xs text-sentinel-danger mt-1">{ev.error}</p>
                  )}
                </div>
                <span className="text-xs text-sentinel-muted whitespace-nowrap">
                  {new Date(ev.timestamp).toLocaleString()}
                </span>
              </li>
            ))}
          </ul>
        )}
      </div>

      <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
        <h3 className="font-medium mb-3">Handshake Proxy Status</h3>
        {!handshakeStatus || handshakeStatus.profiles.length === 0 ? (
          <p className="text-sentinel-muted text-sm">No handshake proxy status available</p>
        ) : (
          <ul className="space-y-2 text-sm">
            {handshakeStatus.profiles.map((entry) => (
              <li
                key={entry.profile_id}
                className="p-3 bg-slate-800/50 rounded flex flex-wrap justify-between gap-2"
              >
                <div>
                  <p className="font-medium">{entry.profile_name ?? entry.profile_id.slice(0, 8)}</p>
                  <p className="text-xs text-sentinel-muted">
                    {entry.enabled
                      ? entry.proxy_host
                        ? `${entry.proxy_host}:${entry.proxy_port ?? "—"}`
                        : "Enabled (no host)"
                      : "Disabled"}
                  </p>
                  {entry.last_error && (
                    <p className="text-xs text-sentinel-danger mt-1">{entry.last_error}</p>
                  )}
                </div>
                <span
                  className={
                    entry.connected
                      ? "text-sentinel-success text-xs font-semibold uppercase"
                      : entry.enabled
                        ? "text-yellow-400 text-xs font-semibold uppercase"
                        : "text-sentinel-muted text-xs uppercase"
                  }
                >
                  {entry.connected ? "Connected" : entry.enabled ? "Idle" : "Off"}
                </span>
              </li>
            ))}
          </ul>
        )}
      </div>

      <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4 space-y-4">
        <h3 className="font-medium">Template Resolution Trace</h3>
        <div className="flex flex-wrap gap-2 items-end">
          <div>
            <label className="block text-xs text-sentinel-muted mb-1">App ID</label>
            <input
              value={traceAppId}
              onChange={(e) => setTraceAppId(e.target.value)}
              className="px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
            />
          </div>
          <div>
            <label className="block text-xs text-sentinel-muted mb-1">Domain</label>
            <input
              value={traceDomain}
              onChange={(e) => setTraceDomain(e.target.value)}
              placeholder="example.com"
              className="px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
            />
          </div>
          <div>
            <label className="block text-xs text-sentinel-muted mb-1">PID</label>
            <input
              value={tracePid}
              onChange={(e) => setTracePid(e.target.value)}
              type="number"
              className="w-24 px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
            />
          </div>
          <button
            onClick={runTemplateTrace}
            className="px-4 py-2 bg-sentinel-accent rounded text-sm hover:bg-blue-600"
          >
            Run trace
          </button>
        </div>
        {!templateTrace ? (
          <p className="text-sentinel-muted text-sm">Run a trace to inspect template resolution steps</p>
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
