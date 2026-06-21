import { useEffect, useState } from "react";
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  Legend,
} from "recharts";
import { useEvents } from "../contexts/ServiceContext";
import { apiClient, type MetricsSnapshot, type PerformanceSnapshot } from "../api/client";

function formatBytes(bytes: number) {
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function MetricCard({ label, value, suffix = "" }: { label: string; value: string; suffix?: string }) {
  return (
    <div className="bg-sentinel-panel rounded-lg p-4 border border-slate-700">
      <p className="text-sentinel-muted text-sm">{label}</p>
      <p className="text-2xl font-bold mt-1">
        {value}
        {suffix && <span className="text-base font-normal text-sentinel-muted">{suffix}</span>}
      </p>
    </div>
  );
}

export function Performance() {
  const { performanceSnapshots, securityAudit } = useEvents();
  const [latest, setLatest] = useState<PerformanceSnapshot | null>(null);
  const [snapshots, setSnapshots] = useState<PerformanceSnapshot[]>([]);
  const [metrics, setMetrics] = useState<MetricsSnapshot | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = async () => {
    setLoading(true);
    try {
      const [perf, m] = await Promise.all([
        apiClient.performance(30),
        apiClient.metrics("json") as Promise<MetricsSnapshot>,
      ]);
      setLatest(perf.latest ?? perf.snapshots[0] ?? null);
      setSnapshots(perf.snapshots);
      setMetrics(m);
    } catch {
      setLatest(performanceSnapshots[0] ?? null);
      setSnapshots(performanceSnapshots);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    refresh();
    const id = setInterval(refresh, 15000);
    return () => clearInterval(id);
  }, []);

  const chartData = [...snapshots]
    .reverse()
    .slice(-20)
    .map((s) => ({
      time: s.timestamp.slice(11, 19),
      cpu: Number(s.cpu_percent.toFixed(1)),
      apiMs: Number(s.api_latency_ms.toFixed(1)),
      wfpMs: Number(s.wfp_latency_ms.toFixed(1)),
    }));

  const display = latest ?? performanceSnapshots[0] ?? null;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-semibold">Performance</h2>
        <button
          onClick={refresh}
          className="px-4 py-2 bg-slate-700 rounded text-sm hover:bg-slate-600"
        >
          Refresh
        </button>
      </div>

      {loading && !display ? (
        <p className="text-sentinel-muted">Loading performance data...</p>
      ) : (
        <>
          <div className="grid grid-cols-4 gap-4">
            <MetricCard
              label="CPU"
              value={display ? display.cpu_percent.toFixed(1) : "—"}
              suffix={display ? "%" : ""}
            />
            <MetricCard
              label="Memory"
              value={display ? formatBytes(display.memory_bytes) : "—"}
            />
            <MetricCard
              label="API latency"
              value={display ? display.api_latency_ms.toFixed(1) : "—"}
              suffix={display ? " ms" : ""}
            />
            <MetricCard
              label="WFP latency"
              value={display ? display.wfp_latency_ms.toFixed(1) : "—"}
              suffix={display ? " ms" : ""}
            />
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div className="bg-sentinel-panel rounded-lg p-4 border border-slate-700">
              <h3 className="text-sm font-medium text-sentinel-muted mb-4">Latency trend</h3>
              {chartData.length === 0 ? (
                <p className="text-sentinel-muted text-sm">No snapshots yet</p>
              ) : (
                <ResponsiveContainer width="100%" height={240}>
                  <LineChart data={chartData}>
                    <XAxis dataKey="time" stroke="#64748b" fontSize={11} />
                    <YAxis stroke="#64748b" fontSize={11} />
                    <Tooltip contentStyle={{ background: "#1a2332", border: "1px solid #334155" }} />
                    <Legend />
                    <Line type="monotone" dataKey="apiMs" stroke="#3b82f6" name="API ms" dot={false} />
                    <Line type="monotone" dataKey="wfpMs" stroke="#22c55e" name="WFP ms" dot={false} />
                  </LineChart>
                </ResponsiveContainer>
              )}
            </div>

            <div className="bg-sentinel-panel rounded-lg p-4 border border-slate-700">
              <h3 className="text-sm font-medium text-sentinel-muted mb-4">CPU trend</h3>
              {chartData.length === 0 ? (
                <p className="text-sentinel-muted text-sm">No snapshots yet</p>
              ) : (
                <ResponsiveContainer width="100%" height={240}>
                  <LineChart data={chartData}>
                    <XAxis dataKey="time" stroke="#64748b" fontSize={11} />
                    <YAxis stroke="#64748b" fontSize={11} />
                    <Tooltip contentStyle={{ background: "#1a2332", border: "1px solid #334155" }} />
                    <Line type="monotone" dataKey="cpu" stroke="#eab308" name="CPU %" dot={false} />
                  </LineChart>
                </ResponsiveContainer>
              )}
            </div>
          </div>

          {metrics && (
            <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
              <h3 className="font-medium mb-3">Service metrics</h3>
              <div className="grid grid-cols-3 gap-4 text-sm">
                <div className="flex justify-between">
                  <span className="text-sentinel-muted">Active tunnels</span>
                  <span>{metrics.active_tunnels}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-sentinel-muted">Active transports</span>
                  <span>{metrics.active_transports}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-sentinel-muted">Blocked requests</span>
                  <span>{metrics.blocked_requests}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-sentinel-muted">DNS queries</span>
                  <span>{metrics.dns_queries}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-sentinel-muted">Open leak incidents</span>
                  <span>{metrics.open_leak_incidents}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-sentinel-muted">Route changes (24h)</span>
                  <span>{metrics.route_changes_24h}</span>
                </div>
              </div>
            </div>
          )}

          {securityAudit.length > 0 && (
            <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
              <h3 className="font-medium mb-3">Security audit (live)</h3>
              <ul className="space-y-1 text-xs font-mono max-h-32 overflow-auto">
                {securityAudit.slice(0, 10).map((entry, i) => (
                  <li key={i} className="text-sentinel-muted">
                    {entry.timestamp.slice(11, 19)} · {entry.action}
                    {entry.detail ? ` — ${entry.detail}` : ""}
                  </li>
                ))}
              </ul>
            </div>
          )}
        </>
      )}
    </div>
  );
}
