import { useEffect, useState } from "react";
import { apiClient, type BridgeProfile } from "../api/client";

export function Bridges() {
  const [bridges, setBridges] = useState<BridgeProfile[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    apiClient
      .bridges()
      .then(setBridges)
      .catch((e) => setError(String(e)));
  }, []);

  return (
    <div className="space-y-4">
      <h2 className="text-2xl font-semibold">Tor Bridges</h2>
      {error && <p className="text-sentinel-danger text-sm">{error}</p>}
      <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
        <h3 className="font-medium mb-3">{bridges.length} bridge profile(s)</h3>
        {bridges.length === 0 ? (
          <p className="text-sentinel-muted text-sm">No bridges configured</p>
        ) : (
          <ul className="space-y-2 text-sm">
            {bridges.map((b) => (
              <li key={b.id} className="p-2 bg-slate-800/50 rounded flex justify-between">
                <div>
                  <p className="font-medium">{b.name}</p>
                  <p className="text-xs text-sentinel-muted">{b.bridge_type}</p>
                </div>
                <span className={b.enabled ? "text-sentinel-success" : "text-sentinel-muted"}>
                  {b.enabled ? "enabled" : "disabled"}
                </span>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
