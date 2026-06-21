import { useEffect, useState } from "react";
import { apiClient, type ProxyChain, type ProxyProfile } from "../api/client";

export function Proxies() {
  const [proxies, setProxies] = useState<ProxyProfile[]>([]);
  const [chains, setChains] = useState<ProxyChain[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);

  const refresh = () => {
    Promise.all([apiClient.proxies(), apiClient.proxyChains()])
      .then(([p, c]) => {
        setProxies(p);
        setChains(c);
      })
      .catch((e) => setError(String(e)));
  };

  useEffect(() => {
    refresh();
  }, []);

  const run = async (id: string, action: () => Promise<unknown>) => {
    setBusy(id);
    setError(null);
    try {
      await action();
      refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-semibold">Proxies</h2>
      {error && <p className="text-sentinel-danger text-sm">{error}</p>}

      <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
        <h3 className="font-medium mb-3">{proxies.length} proxy profile(s)</h3>
        {proxies.length === 0 ? (
          <p className="text-sentinel-muted text-sm">No proxies configured</p>
        ) : (
          <ul className="space-y-2 text-sm">
            {proxies.map((p) => (
              <li
                key={p.id}
                className="p-3 bg-slate-800/50 rounded flex flex-wrap items-center justify-between gap-2"
              >
                <div>
                  <p className="font-medium">{p.name}</p>
                  <p className="text-xs text-sentinel-muted">
                    {p.kind} · {p.host}:{p.port}
                    {p.latency_ms != null ? ` · ${p.latency_ms} ms` : ""}
                  </p>
                  {p.last_error && (
                    <p className="text-xs text-sentinel-danger mt-1">{p.last_error}</p>
                  )}
                </div>
                <div className="flex items-center gap-2">
                  <span
                    className={
                      p.active ? "text-sentinel-success" : "text-sentinel-muted"
                    }
                  >
                    {p.active ? "active" : "inactive"}
                  </span>
                  <button
                    className="px-2 py-1 rounded bg-sentinel-accent text-xs"
                    disabled={busy === p.id}
                    onClick={() =>
                      run(p.id, () =>
                        p.active
                          ? apiClient.disconnectProxy(p.id)
                          : apiClient.connectProxy(p.id)
                      )
                    }
                  >
                    {p.active ? "Disconnect" : "Connect"}
                  </button>
                  <button
                    className="px-2 py-1 rounded border border-slate-600 text-xs"
                    disabled={busy === p.id}
                    onClick={() => run(p.id, () => apiClient.healthProxy(p.id))}
                  >
                    Health
                  </button>
                  <button
                    className="px-2 py-1 rounded border border-slate-600 text-xs"
                    disabled={busy === p.id}
                    onClick={() => run(p.id, () => apiClient.latencyProxy(p.id))}
                  >
                    Latency
                  </button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>

      <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
        <h3 className="font-medium mb-3">{chains.length} proxy chain(s)</h3>
        {chains.length === 0 ? (
          <p className="text-sentinel-muted text-sm">No proxy chains configured</p>
        ) : (
          <ul className="space-y-2 text-sm">
            {chains.map((c) => (
              <li
                key={c.id}
                className="p-3 bg-slate-800/50 rounded flex flex-wrap items-center justify-between gap-2"
              >
                <div>
                  <p className="font-medium">{c.name}</p>
                  <p className="text-xs text-sentinel-muted">{c.hops.length} hop(s)</p>
                </div>
                <div className="flex gap-2">
                  <button
                    className="px-2 py-1 rounded bg-sentinel-accent text-xs"
                    disabled={busy === c.id}
                    onClick={() => run(c.id, () => apiClient.startProxyChain(c.id))}
                  >
                    Start
                  </button>
                  <button
                    className="px-2 py-1 rounded border border-slate-600 text-xs"
                    disabled={busy === c.id}
                    onClick={() => run(c.id, () => apiClient.stopProxyChain(c.id))}
                  >
                    Stop
                  </button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
