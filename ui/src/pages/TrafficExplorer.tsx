import { useEffect, useState } from "react";
import { useEvents } from "../contexts/ServiceContext";
import { apiClient, getAuthToken, initAuth, TrafficEvent } from "../api/client";

export function TrafficExplorer() {
  const { bandwidth, blockedStats, auditLog } = useEvents();
  const [filter, setFilter] = useState("");
  const [logs, setLogs] = useState<TrafficEvent[]>([]);
  const [loading, setLoading] = useState(false);

  const loadLogs = async () => {
    setLoading(true);
    try {
      const data = await apiClient.trafficLogs({ limit: 100, sort: "timestamp", order: "desc" });
      setLogs(data);
    } catch {
      setLogs([]);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadLogs();
  }, []);

  const filteredLive = bandwidth.filter((t) =>
    t.exe_name.toLowerCase().includes(filter.toLowerCase())
  );

  const exportJson = async () => {
    const data = await apiClient.exportTraffic("json", "logs");
    downloadBlob(JSON.stringify(data, null, 2), "application/json", "traffic-logs.json");
  };

  const exportCsv = async () => {
    await initAuth();
    const port = import.meta.env.VITE_API_PORT ?? 8170;
    const res = await fetch(
      `http://127.0.0.1:${port}/api/v1/traffic/export?format=csv&source=logs`,
      { headers: { Authorization: `Bearer ${getAuthToken()}` } }
    );
    const text = await res.text();
    downloadBlob(text, "text/csv", "traffic-logs.csv");
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-semibold">Traffic Explorer</h2>
        <div className="space-x-2">
          <button
            onClick={loadLogs}
            className="px-4 py-2 bg-slate-700 rounded text-sm hover:bg-slate-600"
          >
            Refresh logs
          </button>
          <button
            onClick={exportJson}
            className="px-4 py-2 bg-sentinel-accent rounded text-sm hover:bg-blue-600"
          >
            Export JSON
          </button>
          <button
            onClick={exportCsv}
            className="px-4 py-2 bg-slate-700 rounded text-sm hover:bg-slate-600"
          >
            Export CSV
          </button>
        </div>
      </div>
      <input
        type="search"
        placeholder="Filter by app name..."
        value={filter}
        onChange={(e) => setFilter(e.target.value)}
        className="w-full max-w-md px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
      />
      <div className="bg-sentinel-panel rounded-lg border border-slate-700 overflow-hidden">
        <h3 className="p-3 text-sm font-medium border-b border-slate-700">Live bandwidth</h3>
        <table className="w-full text-sm">
          <thead className="bg-slate-800/50 text-sentinel-muted">
            <tr>
              <th className="text-left p-3">Application</th>
              <th className="text-right p-3">Down (B/s)</th>
              <th className="text-right p-3">Up (B/s)</th>
            </tr>
          </thead>
          <tbody>
            {filteredLive.map((t) => (
              <tr key={t.app_id} className="border-t border-slate-700">
                <td className="p-3">{t.exe_name}</td>
                <td className="p-3 text-right text-sentinel-accent">{t.bytes_in_per_sec}</td>
                <td className="p-3 text-right text-sentinel-success">{t.bytes_out_per_sec}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="bg-sentinel-panel rounded-lg border border-slate-700 overflow-hidden">
        <h3 className="p-3 text-sm font-medium border-b border-slate-700">
          Traffic log {loading ? "(loading…)" : ""}
        </h3>
        <table className="w-full text-sm">
          <thead className="bg-slate-800/50 text-sentinel-muted">
            <tr>
              <th className="text-left p-3">Time</th>
              <th className="text-left p-3">Protocol</th>
              <th className="text-left p-3">Remote</th>
              <th className="text-left p-3">Domain</th>
              <th className="text-left p-3">Route</th>
            </tr>
          </thead>
          <tbody>
            {logs.length === 0 && (
              <tr>
                <td colSpan={5} className="p-4 text-sentinel-muted">
                  No persisted traffic logs
                </td>
              </tr>
            )}
            {logs.map((log) => (
              <tr key={log.id ?? log.timestamp} className="border-t border-slate-700">
                <td className="p-3 text-xs">{log.timestamp?.slice(11, 19) ?? "—"}</td>
                <td className="p-3">{log.protocol}</td>
                <td className="p-3 font-mono text-xs">{log.remote_addr ?? "—"}</td>
                <td className="p-3 text-sentinel-muted">{log.remote_domain ?? "—"}</td>
                <td className="p-3">{log.route?.type ?? "—"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="grid grid-cols-2 gap-4">
        <div className="bg-sentinel-panel rounded-lg border border-slate-700 overflow-hidden">
          <h3 className="p-3 text-sm font-medium border-b border-slate-700">Blocked statistics</h3>
          <table className="w-full text-sm">
            <thead className="bg-slate-800/50 text-sentinel-muted">
              <tr>
                <th className="text-left p-3">Domain</th>
                <th className="text-right p-3">Connections</th>
                <th className="text-right p-3">Bytes</th>
              </tr>
            </thead>
            <tbody>
              {blockedStats.length === 0 && (
                <tr>
                  <td colSpan={3} className="p-4 text-sentinel-muted">
                    No blocked traffic stats
                  </td>
                </tr>
              )}
              {blockedStats.map((r) => (
                <tr key={r.id} className="border-t border-slate-700">
                  <td className="p-3">{r.domain ?? "—"}</td>
                  <td className="p-3 text-right">{r.connection_count}</td>
                  <td className="p-3 text-right font-mono text-xs">
                    {r.bytes_in + r.bytes_out}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <div className="bg-sentinel-panel rounded-lg border border-slate-700 overflow-hidden">
          <h3 className="p-3 text-sm font-medium border-b border-slate-700">Audit log</h3>
          <ul className="max-h-48 overflow-auto text-xs">
            {auditLog.length === 0 && (
              <li className="p-4 text-sentinel-muted">No audit entries</li>
            )}
            {auditLog.map((entry) => (
              <li key={entry.id} className="p-2 border-t border-slate-700 font-mono">
                {entry.timestamp.slice(11, 19)} · {entry.event_type}
                {entry.target_id && ` · ${entry.target_id.slice(0, 8)}`}
              </li>
            ))}
          </ul>
        </div>
      </div>
    </div>
  );
}

function downloadBlob(content: string, type: string, filename: string) {
  const blob = new Blob([content], { type });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}
