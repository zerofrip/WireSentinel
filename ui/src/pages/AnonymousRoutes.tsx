import { useEffect, useState } from "react";
import { apiClient, type AnonymousChain } from "../api/client";

export function AnonymousRoutes() {
  const [chains, setChains] = useState<AnonymousChain[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);

  const refresh = () => {
    apiClient
      .anonymousRoutes()
      .then(setChains)
      .catch((e) => setError(String(e)));
  };

  useEffect(() => {
    refresh();
  }, []);

  const run = async (id: string, action: "start" | "stop") => {
    setBusy(id);
    setError(null);
    try {
      if (action === "start") await apiClient.startAnonymousRoute(id);
      else await apiClient.stopAnonymousRoute(id);
      refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(null);
    }
  };

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
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <div>
                    <p className="font-medium">{c.name}</p>
                    <p className="text-xs text-sentinel-muted mt-1">
                      {c.hops.map((h) => h.kind).join(" → ")}
                    </p>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className={c.enabled ? "text-sentinel-success" : "text-sentinel-muted"}>
                      {c.enabled ? "enabled" : "disabled"}
                    </span>
                    <button
                      className="px-2 py-1 rounded bg-sentinel-accent text-xs disabled:opacity-50"
                      disabled={busy === c.id}
                      onClick={() => run(c.id, "start")}
                    >
                      Start
                    </button>
                    <button
                      className="px-2 py-1 rounded border border-slate-600 text-xs disabled:opacity-50"
                      disabled={busy === c.id}
                      onClick={() => run(c.id, "stop")}
                    >
                      Stop
                    </button>
                  </div>
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
