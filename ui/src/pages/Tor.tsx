import { useEffect, useState } from "react";
import { apiClient, type TorProfile, type TorStatus } from "../api/client";

export function Tor() {
  const [status, setStatus] = useState<TorStatus | null>(null);
  const [profiles, setProfiles] = useState<TorProfile[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    Promise.all([apiClient.torStatus(), apiClient.torProfiles()])
      .then(([s, p]) => {
        setStatus(s);
        setProfiles(p);
      })
      .catch((e) => setError(String(e)));
  }, []);

  return (
    <div className="space-y-4">
      <h2 className="text-2xl font-semibold">Tor</h2>
      {error && <p className="text-sentinel-danger text-sm">{error}</p>}
      <div className="grid grid-cols-4 gap-4">
        <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
          <p className="text-sentinel-muted text-sm">Status</p>
          <p className={`text-lg font-semibold ${status?.running ? "text-sentinel-success" : "text-sentinel-muted"}`}>
            {status?.running ? "Running" : "Stopped"}
          </p>
        </div>
        <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
          <p className="text-sentinel-muted text-sm">Bootstrap</p>
          <p className="text-lg font-semibold">{status?.bootstrap_progress ?? 0}%</p>
        </div>
        <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
          <p className="text-sentinel-muted text-sm">Circuits</p>
          <p className="text-lg font-semibold">{status?.circuit_count ?? 0}</p>
        </div>
        <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
          <p className="text-sentinel-muted text-sm">SOCKS port</p>
          <p className="font-mono">{status?.socks_port ?? 9050}</p>
        </div>
      </div>
      <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
        <h3 className="font-medium mb-3">Tor profiles</h3>
        {profiles.length === 0 ? (
          <p className="text-sentinel-muted text-sm">No Tor profiles configured</p>
        ) : (
          <ul className="space-y-2 text-sm">
            {profiles.map((p) => (
              <li key={p.id} className="flex justify-between p-2 bg-slate-800/50 rounded">
                <span>{p.name}</span>
                <span className="text-sentinel-muted">{p.socks_port}</span>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
