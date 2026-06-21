import { useEffect, useState } from "react";
import { useEvents } from "../contexts/ServiceContext";
import {
  apiClient,
  AppSummary,
  TcpTerminationRule,
  TrafficRoute,
  VpnListEntry,
} from "../api/client";

type Tab = "routes" | "tcp";

function routeLabel(route: TrafficRoute | null | undefined): string {
  if (!route) return "Default (policy)";
  switch (route.type) {
    case "direct":
      return "Direct";
    case "blocked":
      return "Blocked";
    case "wire_guard":
    case "amnezia_wg":
      return `VPN (${route.value.slice(0, 8)}…)`;
    default:
      return "Unknown";
  }
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

export function Applications() {
  const { apps, vpnProfiles, bandwidth, dnsLogs, refresh } = useEvents();
  const [tab, setTab] = useState<Tab>("routes");
  const [tcpRules, setTcpRules] = useState<TcpTerminationRule[]>([]);
  const [tcpLoading, setTcpLoading] = useState(false);
  const [tcpError, setTcpError] = useState<string | null>(null);
  const [processName, setProcessName] = useState("");
  const [processPath, setProcessPath] = useState("");
  const [profileId, setProfileId] = useState("");

  const loadTcpRules = async () => {
    setTcpLoading(true);
    setTcpError(null);
    try {
      const rules = await apiClient.tcpTerminationRules();
      setTcpRules(rules);
    } catch (e) {
      setTcpError(e instanceof Error ? e.message : "Failed to load TCP rules");
    } finally {
      setTcpLoading(false);
    }
  };

  useEffect(() => {
    if (tab === "tcp") {
      loadTcpRules();
    }
  }, [tab]);

  const setRoute = async (app: AppSummary, route: TrafficRoute | null) => {
    await apiClient.setAppRoute(app.id, route);
    await refresh();
  };

  const vpnOptions = vpnProfiles.map((e: VpnListEntry) => e.profile);

  const bytesFor = (appId: string) =>
    bandwidth.find((b) => b.app_id === appId) ?? {
      bytes_in_per_sec: 0,
      bytes_out_per_sec: 0,
      total_bytes_in: 0,
      total_bytes_out: 0,
    };

  const addTcpRule = async () => {
    if (!processName.trim() && !processPath.trim()) return;
    const now = new Date().toISOString();
    const rule: TcpTerminationRule = {
      id: crypto.randomUUID(),
      process_name: processName.trim() || null,
      process_path: processPath.trim() || null,
      profile_id: profileId || null,
      route: null,
      enabled: true,
      created_at: now,
      updated_at: now,
    };
    try {
      await apiClient.addTcpTerminationRule(rule);
      setProcessName("");
      setProcessPath("");
      setProfileId("");
      await loadTcpRules();
    } catch (e) {
      setTcpError(e instanceof Error ? e.message : "Failed to add rule");
    }
  };

  const toggleTcpRule = async (rule: TcpTerminationRule) => {
    try {
      await apiClient.updateTcpTerminationRule(rule.id, {
        ...rule,
        enabled: !rule.enabled,
        updated_at: new Date().toISOString(),
      });
      await loadTcpRules();
    } catch (e) {
      setTcpError(e instanceof Error ? e.message : "Update failed");
    }
  };

  const removeTcpRule = async (id: string) => {
    try {
      await apiClient.deleteTcpTerminationRule(id);
      await loadTcpRules();
    } catch (e) {
      setTcpError(e instanceof Error ? e.message : "Delete failed");
    }
  };

  const addRuleForApp = (app: AppSummary) => {
    setProcessName(app.display_name);
    setProcessPath(app.exe_path);
    setTab("tcp");
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between flex-wrap gap-2">
        <h2 className="text-2xl font-semibold">Applications</h2>
        <div className="flex gap-2">
          <button
            onClick={() => setTab("routes")}
            className={`px-4 py-2 rounded text-sm ${
              tab === "routes" ? "bg-sentinel-accent" : "bg-slate-700 hover:bg-slate-600"
            }`}
          >
            Route assignment
          </button>
          <button
            onClick={() => setTab("tcp")}
            className={`px-4 py-2 rounded text-sm ${
              tab === "tcp" ? "bg-sentinel-accent" : "bg-slate-700 hover:bg-slate-600"
            }`}
          >
            TCP reconnect policies
          </button>
        </div>
      </div>

      {tab === "routes" && (
        <>
          <p className="text-sentinel-muted text-sm">
            Per-app route assignment with live bandwidth and recent DNS activity
          </p>
          <div className="bg-sentinel-panel rounded-lg border border-slate-700 overflow-hidden">
            <table className="w-full text-sm">
              <thead className="bg-slate-800/50 text-sentinel-muted">
                <tr>
                  <th className="text-left p-3">Application</th>
                  <th className="text-left p-3">Route</th>
                  <th className="text-right p-3">Traffic</th>
                  <th className="text-right p-3">Assign</th>
                </tr>
              </thead>
              <tbody>
                {apps.length === 0 && (
                  <tr>
                    <td colSpan={4} className="p-4 text-sentinel-muted">
                      No applications detected yet — events will appear live
                    </td>
                  </tr>
                )}
                {apps.map((app) => {
                  const bw = bytesFor(app.id);
                  const recentDns = dnsLogs.find((l) => l.qname)?.qname;
                  return (
                    <tr key={app.id} className="border-t border-slate-700">
                      <td className="p-3">
                        <p className="font-medium">{app.display_name}</p>
                        {recentDns && (
                          <p className="text-xs text-sentinel-muted truncate">DNS: {recentDns}</p>
                        )}
                      </td>
                      <td className="p-3">{routeLabel(app.default_route)}</td>
                      <td className="p-3 text-right text-xs text-sentinel-muted">
                        ↓ {formatBytes(bw.total_bytes_in ?? bw.bytes_in_per_sec)} / ↑{" "}
                        {formatBytes(bw.total_bytes_out ?? bw.bytes_out_per_sec)}
                      </td>
                      <td className="p-3 text-right space-x-2">
                        <button
                          onClick={() => setRoute(app, { type: "direct" })}
                          className="px-3 py-1 rounded bg-green-800/50 hover:bg-green-700/50 text-xs"
                        >
                          Direct
                        </button>
                        <button
                          onClick={() => setRoute(app, { type: "blocked" })}
                          className="px-3 py-1 rounded bg-red-800/50 hover:bg-red-700/50 text-xs"
                        >
                          Block
                        </button>
                        {vpnOptions.map((p) => (
                          <button
                            key={p.id}
                            onClick={() =>
                              setRoute(app, {
                                type: p.backend === "amnezia_wg" ? "amnezia_wg" : "wire_guard",
                                value: p.id,
                              })
                            }
                            className="px-3 py-1 rounded bg-blue-800/50 hover:bg-blue-700/50 text-xs"
                          >
                            {p.name}
                          </button>
                        ))}
                        <button
                          onClick={() => setRoute(app, null)}
                          className="px-3 py-1 rounded bg-slate-700 hover:bg-slate-600 text-xs"
                        >
                          Clear
                        </button>
                        <button
                          onClick={() => addRuleForApp(app)}
                          className="px-3 py-1 rounded border border-slate-600 text-xs"
                          title="Add TCP reconnect rule for this app"
                        >
                          TCP rule
                        </button>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </>
      )}

      {tab === "tcp" && (
        <>
          <p className="text-sentinel-muted text-sm">
            Process-aware TCP reconnect rules. Global termination mode is configured in Settings.
          </p>
          {tcpError && (
            <div className="p-3 bg-red-900/30 border border-red-700 rounded text-sm">{tcpError}</div>
          )}
          <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4 space-y-3">
            <h3 className="font-medium">Add reconnect rule</h3>
            <div className="flex flex-wrap gap-2 items-end">
              <div>
                <label className="block text-xs text-sentinel-muted mb-1">Process name</label>
                <input
                  value={processName}
                  onChange={(e) => setProcessName(e.target.value)}
                  placeholder="chrome.exe"
                  className="px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
                />
              </div>
              <div>
                <label className="block text-xs text-sentinel-muted mb-1">Process path</label>
                <input
                  value={processPath}
                  onChange={(e) => setProcessPath(e.target.value)}
                  placeholder="C:\Program Files\..."
                  className="px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm min-w-[200px]"
                />
              </div>
              <div>
                <label className="block text-xs text-sentinel-muted mb-1">VPN profile (optional)</label>
                <select
                  value={profileId}
                  onChange={(e) => setProfileId(e.target.value)}
                  className="px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
                >
                  <option value="">Any profile</option>
                  {vpnOptions.map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.name}
                    </option>
                  ))}
                </select>
              </div>
              <button
                onClick={addTcpRule}
                className="px-4 py-2 bg-sentinel-accent rounded text-sm hover:bg-blue-600"
              >
                Add rule
              </button>
            </div>
          </div>
          <div className="bg-sentinel-panel rounded-lg border border-slate-700 overflow-hidden">
            <table className="w-full text-sm">
              <thead className="bg-slate-800/50 text-sentinel-muted">
                <tr>
                  <th className="text-left p-3">Process</th>
                  <th className="text-left p-3">Profile scope</th>
                  <th className="text-left p-3">Enabled</th>
                  <th className="text-right p-3">Actions</th>
                </tr>
              </thead>
              <tbody>
                {tcpLoading && (
                  <tr>
                    <td colSpan={4} className="p-4 text-sentinel-muted">
                      Loading TCP rules...
                    </td>
                  </tr>
                )}
                {!tcpLoading && tcpRules.length === 0 && (
                  <tr>
                    <td colSpan={4} className="p-4 text-sentinel-muted">
                      No TCP reconnect rules — all matching processes use global mode only
                    </td>
                  </tr>
                )}
                {tcpRules.map((rule) => (
                  <tr key={rule.id} className="border-t border-slate-700">
                    <td className="p-3">
                      <p className="font-medium">{rule.process_name ?? "—"}</p>
                      {rule.process_path && (
                        <p className="text-xs text-sentinel-muted truncate max-w-xs">
                          {rule.process_path}
                        </p>
                      )}
                    </td>
                    <td className="p-3 text-xs text-sentinel-muted">
                      {rule.profile_id
                        ? vpnOptions.find((p) => p.id === rule.profile_id)?.name ?? rule.profile_id.slice(0, 8)
                        : "Any"}
                    </td>
                    <td className="p-3">{rule.enabled ? "Yes" : "No"}</td>
                    <td className="p-3 text-right space-x-2">
                      <button
                        onClick={() => toggleTcpRule(rule)}
                        className="px-2 py-1 text-xs bg-slate-700 rounded"
                      >
                        {rule.enabled ? "Disable" : "Enable"}
                      </button>
                      <button
                        onClick={() => removeTcpRule(rule.id)}
                        className="px-2 py-1 text-xs bg-red-800/50 rounded"
                      >
                        Delete
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}
    </div>
  );
}
