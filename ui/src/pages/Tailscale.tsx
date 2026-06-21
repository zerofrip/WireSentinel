import { useEffect, useState } from "react";
import { apiClient, type TailnetProfile, type TailscaleStatus } from "../api/client";

export function Tailscale() {
  const [status, setStatus] = useState<TailscaleStatus | null>(null);
  const [profiles, setProfiles] = useState<TailnetProfile[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    Promise.all([apiClient.tailnetStatus(), apiClient.tailnetProfiles()])
      .then(([s, p]) => {
        setStatus(s);
        setProfiles(p);
      })
      .catch((e) => setError(String(e)));
  }, []);

  return (
    <div className="space-y-4">
      <h2 className="text-2xl font-semibold">Tailscale / Tailnet</h2>
      {error && <p className="text-sentinel-danger text-sm">{error}</p>}
      <div className="grid grid-cols-3 gap-4">
        <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
          <p className="text-sentinel-muted text-sm">Connection</p>
          <p className={`text-lg font-semibold ${status?.connected ? "text-sentinel-success" : "text-sentinel-muted"}`}>
            {status?.connected ? "Connected" : "Disconnected"}
          </p>
        </div>
        <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
          <p className="text-sentinel-muted text-sm">Tailnet IP</p>
          <p className="font-mono text-sm">{status?.tailnet_ip ?? "—"}</p>
        </div>
        <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
          <p className="text-sentinel-muted text-sm">Exit node</p>
          <p className="text-sm">{status?.exit_node ?? "None"}</p>
        </div>
      </div>
      <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
        <h3 className="font-medium mb-3">Profiles ({profiles.length})</h3>
        {profiles.length === 0 ? (
          <p className="text-sentinel-muted text-sm">No tailnet profiles configured</p>
        ) : (
          <ul className="space-y-2 text-sm">
            {profiles.map((p) => (
              <li key={p.id} className="flex justify-between p-2 bg-slate-800/50 rounded">
                <span>{p.name}</span>
                <span className={p.connected ? "text-sentinel-success" : "text-sentinel-muted"}>
                  {p.connected ? "connected" : "idle"}
                </span>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
