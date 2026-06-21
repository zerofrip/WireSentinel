import { useEffect, useState } from "react";
import { apiClient, type AnonymousChain } from "../api/client";

export function AnonymousRoutes() {
  const [chains, setChains] = useState<AnonymousChain[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    apiClient
      .anonymousRoutes()
      .then(setChains)
      .catch((e) => setError(String(e)));
  }, []);

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-semibold">Anonymous Routes</h2>
      {error && <p className="text-sentinel-danger text-sm">{error}</p>}

      <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
        <h3 className="font-medium mb-3">{chains.length} chain(s)</h3>
        {chains.length === 0 ? (
          <p className="text-sentinel-muted text-sm">No anonymous route chains configured</p>
        ) : (
          <ul className="space-y-2 text-sm">
            {chains.map((c) => (
              <li key={c.id} className="p-3 bg-slate-800/50 rounded">
                <div className="flex justify-between gap-2">
                  <span className="font-medium">{c.name}</span>
                  <span className={c.enabled ? "text-sentinel-success" : "text-sentinel-muted"}>
                    {c.enabled ? "enabled" : "disabled"}
                  </span>
                </div>
                <p className="text-xs text-sentinel-muted mt-1">
                  {c.hops.map((h) => h.kind).join(" → ")}
                </p>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
