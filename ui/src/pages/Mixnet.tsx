import { useEffect, useState } from "react";
import { apiClient, type MixnetProfile, type MixnetStatus } from "../api/client";

export function Mixnet() {
  const [profiles, setProfiles] = useState<MixnetProfile[]>([]);
  const [status, setStatus] = useState<MixnetStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);

  const refresh = () => {
    Promise.all([apiClient.mixnet(), apiClient.mixnetStatus()])
      .then(([p, s]) => {
        setProfiles(p);
        setStatus(s);
        setError(null);
      })
      .catch((e) => setError(String(e)));
  };

  useEffect(() => {
    refresh();
  }, []);

  const runGlobal = async (action: "start" | "stop") => {
    setBusy(action);
    setError(null);
    try {
      if (action === "start") await apiClient.mixnetStart();
      else await apiClient.mixnetStop();
      refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-semibold">Mixnet</h2>
        <div className="flex gap-2">
          <button
            className="px-3 py-1.5 bg-sentinel-accent rounded text-sm disabled:opacity-50"
            disabled={busy !== null}
            onClick={() => runGlobal("start")}
          >
            {busy === "start" ? "Starting..." : "Start active"}
          </button>
          <button
            className="px-3 py-1.5 bg-slate-700 rounded text-sm disabled:opacity-50"
            disabled={busy !== null}
            onClick={() => runGlobal("stop")}
          >
            {busy === "stop" ? "Stopping..." : "Stop"}
          </button>
        </div>
      </div>
      {error && <p className="text-sentinel-danger text-sm">{error}</p>}

      <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
        <h3 className="font-medium mb-3">Status</h3>
        {status ? (
          <p className="text-sm">
            <span className={status.running ? "text-sentinel-success" : "text-sentinel-muted"}>
              {status.running ? "Running" : "Stopped"}
            </span>
            {status.gateway_id && (
              <span className="text-sentinel-muted ml-2">· gateway {status.gateway_id}</span>
            )}
            {status.latency_ms != null && (
              <span className="text-sentinel-muted ml-2">· {status.latency_ms} ms</span>
            )}
          </p>
        ) : (
          <p className="text-sentinel-muted text-sm">No mixnet status available</p>
        )}
      </div>

      <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
        <h3 className="font-medium mb-3">{profiles.length} profile(s)</h3>
        {profiles.length === 0 ? (
          <p className="text-sentinel-muted text-sm">No mixnet profiles configured</p>
        ) : (
          <ul className="space-y-2 text-sm">
            {profiles.map((p) => (
              <li
                key={p.id}
                className="p-3 bg-slate-800/50 rounded flex flex-wrap items-center justify-between gap-2"
              >
                <div>
                  <p className="font-medium">{p.name}</p>
                  <p className="text-xs text-sentinel-muted">
                    {typeof p.provider === "object" ? p.provider.type : String(p.provider)}
                    {p.gateway_id ? ` · ${p.gateway_id}` : ""}
                    {p.latency_ms != null ? ` · ${p.latency_ms} ms` : ""}
                  </p>
                  {p.last_error && (
                    <p className="text-xs text-sentinel-danger mt-1">{p.last_error}</p>
                  )}
                </div>
                <span className={p.active ? "text-sentinel-success" : "text-sentinel-muted"}>
                  {p.active ? "active" : "inactive"}
                </span>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
