import { Link } from "react-router-dom";
import { useEvents } from "../contexts/ServiceContext";
import type { PrivacyScoreComponents } from "../api/client";

function scoreColor(score: number) {
  if (score >= 80) return "text-sentinel-success";
  if (score >= 50) return "text-yellow-400";
  return "text-sentinel-danger";
}

function gaugeColor(score: number) {
  if (score >= 80) return "#22c55e";
  if (score >= 50) return "#eab308";
  return "#ef4444";
}

function PrivacyGauge({ score }: { score: number }) {
  const pct = Math.min(100, Math.max(0, score));
  return (
    <div className="flex flex-col items-center">
      <div
        className="relative w-40 h-40 rounded-full flex items-center justify-center"
        style={{
          background: `conic-gradient(${gaugeColor(score)} ${pct * 3.6}deg, #334155 ${pct * 3.6}deg)`,
        }}
      >
        <div className="w-32 h-32 rounded-full bg-sentinel-panel flex flex-col items-center justify-center">
          <span className={`text-4xl font-bold ${scoreColor(score)}`}>{score}</span>
          <span className="text-xs text-sentinel-muted">/ 100</span>
        </div>
      </div>
    </div>
  );
}

function ComponentBar({ label, value }: { label: string; value: number }) {
  return (
    <div>
      <div className="flex justify-between text-sm mb-1">
        <span className="text-sentinel-muted">{label}</span>
        <span>{value}%</span>
      </div>
      <div className="h-2 bg-slate-800 rounded overflow-hidden">
        <div
          className="h-full bg-sentinel-accent rounded transition-all"
          style={{ width: `${value}%` }}
        />
      </div>
    </div>
  );
}

function componentRows(components: PrivacyScoreComponents) {
  return [
    { label: "Encrypted DNS", value: components.encrypted_dns },
    { label: "Blocked Trackers", value: components.blocked_trackers },
    { label: "VPN Coverage", value: components.vpn_coverage },
    { label: "Route Integrity", value: 100 - components.route_leakage },
    { label: "DNS Integrity", value: 100 - components.dns_leakage },
  ];
}

export function Privacy() {
  const {
    privacySnapshot,
    vpnProfiles,
    dnsProviders,
    transports,
    transportStatus,
    leakIncidents,
    status,
  } = useEvents();

  const score = privacySnapshot?.score ?? 0;
  const components = privacySnapshot?.components;
  const activeVpn = vpnProfiles.filter((e) => {
    if (typeof e.status === "string") return e.status === "connected";
    return e.status.status === "connected";
  });
  const enabledProviders = dnsProviders.filter((p) => p.enabled);
  const runningTransports = transportStatus.filter((t) => t.state === "running");
  const openLeaks = leakIncidents.filter((l) => !l.resolved_at);

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-semibold">Privacy</h2>

      <div className="grid grid-cols-3 gap-4">
        <div className="col-span-1 bg-sentinel-panel rounded-lg border border-slate-700 p-6 flex flex-col items-center justify-center">
          <h3 className="text-sm font-medium text-sentinel-muted mb-4">Privacy Score</h3>
          <PrivacyGauge score={score} />
          {privacySnapshot && (
            <p className="text-xs text-sentinel-muted mt-4">
              Updated {new Date(privacySnapshot.timestamp).toLocaleString()}
            </p>
          )}
        </div>

        <div className="col-span-2 bg-sentinel-panel rounded-lg border border-slate-700 p-4">
          <h3 className="text-sm font-medium text-sentinel-muted mb-4">Score Breakdown</h3>
          {components ? (
            <div className="space-y-3">
              {componentRows(components).map((row) => (
                <ComponentBar key={row.label} label={row.label} value={row.value} />
              ))}
            </div>
          ) : (
            <p className="text-sentinel-muted text-sm">No privacy score calculated yet</p>
          )}
        </div>
      </div>

      <div className="grid grid-cols-2 gap-4">
        <section className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
          <h3 className="font-medium mb-3">VPN Privacy</h3>
          <ul className="space-y-2 text-sm">
            <li className="flex justify-between">
              <span className="text-sentinel-muted">Active VPNs</span>
              <span>{activeVpn.length}</span>
            </li>
            <li className="flex justify-between">
              <span className="text-sentinel-muted">Kill switch</span>
              <span className={status?.kill_switch_active ? "text-sentinel-success" : "text-sentinel-danger"}>
                {status?.kill_switch_active ? "Active" : "Inactive"}
              </span>
            </li>
            <li className="flex justify-between">
              <span className="text-sentinel-muted">Coverage score</span>
              <span>{components?.vpn_coverage ?? "—"}%</span>
            </li>
          </ul>
          {activeVpn.length > 0 && (
            <ul className="mt-3 pt-3 border-t border-slate-700 space-y-1 text-xs text-sentinel-muted">
              {activeVpn.map(({ profile }) => (
                <li key={profile.id}>{profile.name}</li>
              ))}
            </ul>
          )}
        </section>

        <section className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
          <h3 className="font-medium mb-3">DNS Privacy</h3>
          <ul className="space-y-2 text-sm">
            <li className="flex justify-between">
              <span className="text-sentinel-muted">Encrypted DNS score</span>
              <span>{components?.encrypted_dns ?? "—"}%</span>
            </li>
            <li className="flex justify-between">
              <span className="text-sentinel-muted">Providers enabled</span>
              <span>
                {enabledProviders.length} / {dnsProviders.length || "—"}
              </span>
            </li>
            <li className="flex justify-between">
              <span className="text-sentinel-muted">DNS leakage risk</span>
              <span className={components && components.dns_leakage > 20 ? "text-sentinel-danger" : ""}>
                {components ? `${components.dns_leakage}%` : "—"}
              </span>
            </li>
          </ul>
          <Link to="/dns" className="inline-block mt-3 text-xs text-sentinel-accent hover:underline">
            Manage DNS providers →
          </Link>
        </section>

        <section className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
          <h3 className="font-medium mb-3">Transport Privacy</h3>
          <ul className="space-y-2 text-sm">
            <li className="flex justify-between">
              <span className="text-sentinel-muted">Transport profiles</span>
              <span>{transports.length}</span>
            </li>
            <li className="flex justify-between">
              <span className="text-sentinel-muted">Running</span>
              <span>{runningTransports.length}</span>
            </li>
            <li className="flex justify-between">
              <span className="text-sentinel-muted">Route leakage risk</span>
              <span className={components && components.route_leakage > 20 ? "text-sentinel-danger" : ""}>
                {components ? `${components.route_leakage}%` : "—"}
              </span>
            </li>
          </ul>
          {transportStatus.length > 0 && (
            <ul className="mt-3 pt-3 border-t border-slate-700 space-y-1 text-xs">
              {transportStatus.slice(0, 5).map((t) => (
                <li key={t.id} className="flex justify-between text-sentinel-muted">
                  <span>{t.name}</span>
                  <span className={t.state === "running" ? "text-sentinel-success" : ""}>{t.state}</span>
                </li>
              ))}
            </ul>
          )}
          <Link to="/connect/vpn" className="inline-block mt-3 text-xs text-sentinel-accent hover:underline">
            Configure transports →
          </Link>
        </section>

        <section className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
          <h3 className="font-medium mb-3">Leak Detection</h3>
          <ul className="space-y-2 text-sm mb-3">
            <li className="flex justify-between">
              <span className="text-sentinel-muted">Open incidents</span>
              <span className={openLeaks.length > 0 ? "text-sentinel-danger font-medium" : "text-sentinel-success"}>
                {openLeaks.length}
              </span>
            </li>
            <li className="flex justify-between">
              <span className="text-sentinel-muted">Total recorded</span>
              <span>{leakIncidents.length}</span>
            </li>
          </ul>
          {leakIncidents.length === 0 ? (
            <p className="text-sentinel-muted text-sm">No leaks detected</p>
          ) : (
            <ul className="space-y-2 text-xs max-h-40 overflow-auto">
              {leakIncidents.slice(0, 10).map((incident) => (
                <li
                  key={incident.id}
                  className="p-2 bg-slate-800/50 rounded flex justify-between gap-2"
                >
                  <span>
                    <span className="font-medium">{incident.leak_type}</span>
                    <span className="text-sentinel-muted ml-2">{incident.severity}</span>
                  </span>
                  <span className="text-sentinel-muted shrink-0">
                    {incident.resolved_at ? "resolved" : "open"}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </section>
      </div>
    </div>
  );
}
