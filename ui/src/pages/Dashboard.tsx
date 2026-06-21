import { useEffect, useState } from "react";
import { LineChart, Line, XAxis, YAxis, Tooltip, ResponsiveContainer } from "recharts";
import { useEvents } from "../contexts/ServiceContext";
import {
  apiClient,
  type DiagnosticsHealth,
  type KernelStatus,
  type KernelTelemetry,
  type MixnetStatus,
  type PluginRecord,
  type PrivacyAnalyticsSnapshot,
  type TailscaleStatus,
  type TorStatus,
} from "../api/client";

function overallHealth(health: DiagnosticsHealth): "healthy" | "degraded" | "error" {
  const subs = [health.wfp, health.vpn, health.dns, health.transport, health.database, health.disk];
  if (subs.some((s) => s.status === "error")) return "error";
  if (subs.some((s) => s.status !== "ok")) return "degraded";
  return "healthy";
}

function HealthBadge({ health }: { health: DiagnosticsHealth | null }) {
  if (!health) {
    return (
      <span className="inline-flex items-center gap-2 px-3 py-1 rounded-full text-xs bg-slate-800 text-sentinel-muted">
        Service health: unknown
      </span>
    );
  }
  const status = overallHealth(health);
  const styles = {
    healthy: "bg-green-900/40 text-sentinel-success border-green-800",
    degraded: "bg-yellow-900/30 text-yellow-400 border-yellow-800",
    error: "bg-red-900/30 text-sentinel-danger border-red-800",
  };
  const labels = { healthy: "Healthy", degraded: "Degraded", error: "Unhealthy" };
  return (
    <span
      className={`inline-flex items-center gap-2 px-3 py-1 rounded-full text-xs border ${styles[status]}`}
    >
      <span className="w-2 h-2 rounded-full bg-current" />
      Service health: {labels[status]}
    </span>
  );
}

export function Dashboard() {
  const {
    status,
    bandwidth,
    topDomains,
    vpnProfiles,
    recentEvents,
    routeStats,
    blockedStats,
    privacySnapshot,
    dnsProviders,
    transportStatus,
    chains,
  } = useEvents();
  const [health, setHealth] = useState<DiagnosticsHealth | null>(null);
  const [plugins, setPlugins] = useState<PluginRecord[]>([]);
  const [tailnet, setTailnet] = useState<TailscaleStatus | null>(null);
  const [tor, setTor] = useState<TorStatus | null>(null);
  const [mixnetStatus, setMixnetStatus] = useState<MixnetStatus | null>(null);
  const [kernelStatus, setKernelStatus] = useState<KernelStatus | null>(null);
  const [kernelTelemetry, setKernelTelemetry] = useState<KernelTelemetry | null>(null);
  const [privacyAnalytics, setPrivacyAnalytics] = useState<PrivacyAnalyticsSnapshot | null>(null);

  useEffect(() => {
    apiClient.diagnostics().then(setHealth).catch(() => setHealth(null));
    apiClient.plugins().then(setPlugins).catch(() => setPlugins([]));
    apiClient.tailnetStatus().then(setTailnet).catch(() => setTailnet(null));
    apiClient.torStatus().then(setTor).catch(() => setTor(null));
    apiClient.mixnetStatus().then(setMixnetStatus).catch(() => setMixnetStatus(null));
    apiClient.kernelStatus().then(setKernelStatus).catch(() => setKernelStatus(null));
    apiClient.kernelTelemetry().then(setKernelTelemetry).catch(() => setKernelTelemetry(null));
    apiClient.privacyAnalytics().then(setPrivacyAnalytics).catch(() => setPrivacyAnalytics(null));
    const id = setInterval(() => {
      apiClient.diagnostics().then(setHealth).catch(() => setHealth(null));
      apiClient.plugins().then(setPlugins).catch(() => setPlugins([]));
      apiClient.tailnetStatus().then(setTailnet).catch(() => setTailnet(null));
      apiClient.torStatus().then(setTor).catch(() => setTor(null));
      apiClient.mixnetStatus().then(setMixnetStatus).catch(() => setMixnetStatus(null));
    apiClient.kernelStatus().then(setKernelStatus).catch(() => setKernelStatus(null));
    apiClient.kernelTelemetry().then(setKernelTelemetry).catch(() => setKernelTelemetry(null));
      apiClient.privacyAnalytics().then(setPrivacyAnalytics).catch(() => setPrivacyAnalytics(null));
    }, 30000);
    return () => clearInterval(id);
  }, []);

  const chartData = bandwidth.slice(0, 10).map((b, i) => ({
    name: b.exe_name.slice(0, 12),
    down: Math.round(b.bytes_in_per_sec / 1024),
    up: Math.round(b.bytes_out_per_sec / 1024),
    idx: i,
  }));

  const activeVpn = vpnProfiles.filter((e) => {
    if (typeof e.status === "string") return e.status === "connected";
    return e.status.status === "connected";
  });

  const routeSummary = routeStats.slice(0, 5);
  const blockedSummary = blockedStats.slice(0, 5);
  const privacyScore = privacySnapshot?.score ?? privacyAnalytics?.anonymity_score ?? null;
  const encryptedDnsOk =
    (privacySnapshot?.components.encrypted_dns ?? 0) >= 70 ||
    dnsProviders.some((p) => p.enabled && p.transport !== "plain");
  const runningTransports = transportStatus.filter((t) => t.state === "running");
  const enabledChains = chains.filter((c) => c.enabled);
  const activePlugins = plugins.filter((p) => p.state === "loaded");

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-semibold">Dashboard</h2>
        <HealthBadge health={health} />
      </div>
      <div className="grid grid-cols-4 gap-4">
        <StatCard label="Connections" value={status?.connection_count ?? 0} />
        <StatCard label="Monitored Apps" value={status?.monitored_app_count ?? 0} />
        <StatCard label="Active VPNs" value={status?.active_vpn_count ?? 0} />
        <StatCard
          label="Kill Switch"
          value={status?.kill_switch_active ? 1 : 0}
          suffix={status?.kill_switch_active ? " ON" : " off"}
        />
      </div>
      <div className="grid grid-cols-4 gap-4">
        <div className="bg-sentinel-panel rounded-lg p-4 border border-slate-700">
          <p className="text-sentinel-muted text-sm">Privacy Score</p>
          <p className="text-3xl font-bold mt-1">
            {privacyScore !== null ? (
              <span
                className={
                  privacyScore >= 80
                    ? "text-sentinel-success"
                    : privacyScore >= 50
                      ? "text-yellow-400"
                      : "text-sentinel-danger"
                }
              >
                {privacyScore}
              </span>
            ) : (
              <span className="text-sentinel-muted text-xl">—</span>
            )}
          </p>
        </div>
        <div className="bg-sentinel-panel rounded-lg p-4 border border-slate-700">
          <p className="text-sentinel-muted text-sm">Encrypted DNS</p>
          <p className="text-lg font-semibold mt-2">
            <span className={encryptedDnsOk ? "text-sentinel-success" : "text-sentinel-danger"}>
              {encryptedDnsOk ? "Protected" : "At risk"}
            </span>
          </p>
          <p className="text-xs text-sentinel-muted mt-1">
            {dnsProviders.filter((p) => p.enabled).length} provider(s) active
          </p>
        </div>
        <div className="bg-sentinel-panel rounded-lg p-4 border border-slate-700">
          <p className="text-sentinel-muted text-sm">Transports</p>
          <p className="text-lg font-semibold mt-2">
            {runningTransports.length} running
          </p>
          <p className="text-xs text-sentinel-muted mt-1">
            {transportStatus.length} profile(s) configured
          </p>
        </div>
        <div className="bg-sentinel-panel rounded-lg p-4 border border-slate-700">
          <p className="text-sentinel-muted text-sm">Chains</p>
          <p className="text-lg font-semibold mt-2">
            {enabledChains.length} enabled
          </p>
          <p className="text-xs text-sentinel-muted mt-1">
            {chains.length} chain profile(s)
          </p>
        </div>
      </div>
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <div className="bg-sentinel-panel rounded-lg p-4 border border-slate-700">
          <p className="text-sentinel-muted text-sm">Guardian mode</p>
          <p className="text-lg font-semibold mt-2">{kernelStatus?.guardian_mode ?? "—"}</p>
          <p className="text-xs text-sentinel-muted mt-1">
            {kernelStatus?.healthy ? "healthy" : "degraded"}
          </p>
        </div>
        <StatCard label="Kernel flows" value={kernelTelemetry?.route_count ?? 0} />
        <StatCard label="Kernel latency" value={kernelTelemetry?.avg_classify_latency_ns ?? 0} suffix=" ns" />
        <StatCard label="Packet rate" value={kernelTelemetry?.packets_per_sec ?? 0} suffix="/s" />

        <div className="bg-sentinel-panel rounded-lg p-4 border border-slate-700">
          <p className="text-sentinel-muted text-sm">Active Plugins</p>
          <p className="text-lg font-semibold mt-2">{activePlugins.length} loaded</p>
          <p className="text-xs text-sentinel-muted mt-1">{plugins.length} installed</p>
        </div>
        <div className="bg-sentinel-panel rounded-lg p-4 border border-slate-700">
          <p className="text-sentinel-muted text-sm">Mixnet</p>
          <p
            className={`text-lg font-semibold mt-2 ${
              mixnetStatus?.running ? "text-sentinel-success" : "text-sentinel-muted"
            }`}
          >
            {mixnetStatus?.running ? "Running" : "Stopped"}
          </p>
          <p className="text-xs text-sentinel-muted mt-1">
            {mixnetStatus?.active_sessions ?? 0} session(s)
            {mixnetStatus?.latency_ms != null ? ` · ${mixnetStatus.latency_ms} ms` : ""}
          </p>
        </div>
        <div className="bg-sentinel-panel rounded-lg p-4 border border-slate-700">
          <p className="text-sentinel-muted text-sm">Tailnet</p>
          <p className={`text-lg font-semibold mt-2 ${tailnet?.connected ? "text-sentinel-success" : "text-sentinel-muted"}`}>
            {tailnet?.connected ? "Connected" : "Offline"}
          </p>
          <p className="text-xs text-sentinel-muted mt-1 font-mono truncate">
            {tailnet?.tailnet_ip ?? "—"}
          </p>
        </div>
        <div className="bg-sentinel-panel rounded-lg p-4 border border-slate-700">
          <p className="text-sentinel-muted text-sm">Tor</p>
          <p className={`text-lg font-semibold mt-2 ${tor?.running ? "text-sentinel-success" : "text-sentinel-muted"}`}>
            {tor?.running ? "Running" : "Stopped"}
          </p>
          <p className="text-xs text-sentinel-muted mt-1">
            {tor?.circuit_count ?? 0} circuits · {tor?.bootstrap_progress ?? 0}% boot
          </p>
        </div>
      </div>
      <div className="grid grid-cols-2 gap-4">
        <div className="bg-sentinel-panel rounded-lg p-4 border border-slate-700">
          <h3 className="text-sm font-medium text-sentinel-muted mb-4">Bandwidth (KB/s)</h3>
          {chartData.length === 0 ? (
            <p className="text-sentinel-muted text-sm">No traffic data yet</p>
          ) : (
            <ResponsiveContainer width="100%" height={240}>
              <LineChart data={chartData}>
                <XAxis dataKey="name" stroke="#64748b" fontSize={11} />
                <YAxis stroke="#64748b" fontSize={11} />
                <Tooltip contentStyle={{ background: "#1a2332", border: "1px solid #334155" }} />
                <Line type="monotone" dataKey="down" stroke="#3b82f6" name="Download" dot={false} />
                <Line type="monotone" dataKey="up" stroke="#22c55e" name="Upload" dot={false} />
              </LineChart>
            </ResponsiveContainer>
          )}
        </div>
        <div className="bg-sentinel-panel rounded-lg p-4 border border-slate-700">
          <h3 className="text-sm font-medium text-sentinel-muted mb-4">Top Domains</h3>
          {topDomains.length === 0 ? (
            <p className="text-sentinel-muted text-sm">No DNS queries recorded</p>
          ) : (
            <ul className="space-y-2 text-sm">
              {topDomains.map((d) => (
                <li key={d.domain} className="flex justify-between">
                  <span className="truncate">{d.domain}</span>
                  <span className="text-sentinel-muted">
                    {d.query_count}
                    {d.blocked_count > 0 && (
                      <span className="text-red-400 ml-2">({d.blocked_count} blocked)</span>
                    )}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
      <div className="grid grid-cols-3 gap-4">
        <div className="bg-sentinel-panel rounded-lg p-4 border border-slate-700">
          <h3 className="text-sm font-medium text-sentinel-muted mb-3">Route Usage</h3>
          {routeSummary.length === 0 ? (
            <p className="text-sentinel-muted text-sm">No route statistics yet</p>
          ) : (
            <ul className="space-y-2 text-sm">
              {routeSummary.map((r) => (
                <li key={r.id} className="flex justify-between">
                  <span>{r.route_type}</span>
                  <span className="text-sentinel-muted font-mono text-xs">
                    ↓{r.bytes_in} ↑{r.bytes_out}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </div>
        <div className="bg-sentinel-panel rounded-lg p-4 border border-slate-700">
          <h3 className="text-sm font-medium text-sentinel-muted mb-3">Blocked Traffic</h3>
          {blockedSummary.length === 0 ? (
            <p className="text-sentinel-muted text-sm">No blocked flows recorded</p>
          ) : (
            <ul className="space-y-2 text-sm">
              {blockedSummary.map((r) => (
                <li key={r.id} className="flex justify-between">
                  <span className="truncate">{r.domain ?? "unknown"}</span>
                  <span className="text-red-400">{r.connection_count} conn</span>
                </li>
              ))}
            </ul>
          )}
        </div>
        <div className="bg-sentinel-panel rounded-lg p-4 border border-slate-700">
          <h3 className="text-sm font-medium text-sentinel-muted mb-3">Active VPN</h3>
          {activeVpn.length === 0 ? (
            <p className="text-sentinel-muted text-sm">No VPN connected</p>
          ) : (
            <ul className="space-y-2 text-sm">
              {activeVpn.map(({ profile }) => (
                <li key={profile.id}>{profile.name}</li>
              ))}
            </ul>
          )}
        </div>
      </div>
      <div className="bg-sentinel-panel rounded-lg p-4 border border-slate-700">
        <h3 className="text-sm font-medium text-sentinel-muted mb-3">Recent Events</h3>
        {recentEvents.length === 0 ? (
          <p className="text-sentinel-muted text-sm">Waiting for events…</p>
        ) : (
          <ul className="space-y-1 text-xs font-mono max-h-40 overflow-auto">
            {recentEvents.map((ev, i) => (
              <li key={i} className="text-sentinel-muted">
                {ev.timestamp.slice(11, 19)} · {ev.kind}
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}

function StatCard({
  label,
  value,
  suffix = "",
}: {
  label: string;
  value: number;
  suffix?: string;
}) {
  return (
    <div className="bg-sentinel-panel rounded-lg p-4 border border-slate-700">
      <p className="text-sentinel-muted text-sm">{label}</p>
      <p className="text-3xl font-bold mt-1">
        {value}
        {suffix && <span className="text-base font-normal text-sentinel-muted">{suffix}</span>}
      </p>
    </div>
  );
}
