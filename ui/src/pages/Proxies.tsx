import { useEffect, useState } from "react";
import { apiClient, type ProxyChain, type ProxyProfile } from "../api/client";

function newProxyProfile(
  name: string,
  host: string,
  port: number,
  kind: ProxyProfile["kind"]
): ProxyProfile {
  return {
    id: crypto.randomUUID(),
    name: name || `proxy-${host}`,
    kind,
    host,
    port,
    enabled: true,
    active: false,
  };
}

export function Proxies() {
  const [proxies, setProxies] = useState<ProxyProfile[]>([]);
  const [chains, setChains] = useState<ProxyChain[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);
  const [form, setForm] = useState({
    name: "",
    host: "127.0.0.1",
    port: "1080",
    kind: "socks5" as ProxyProfile["kind"],
  });

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

  const createProxy = async () => {
    setBusy("create");
    setError(null);
    try {
      await apiClient.createProxy(
        newProxyProfile(form.name, form.host, Number(form.port), form.kind)
      );
      setShowForm(false);
      setForm({ name: "", host: "127.0.0.1", port: "1080", kind: "socks5" });
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
        <h2 className="text-2xl font-semibold">Proxies</h2>
        <button
          className="px-3 py-1.5 bg-sentinel-accent rounded text-sm"
          onClick={() => setShowForm((v) => !v)}
        >
          {showForm ? "Cancel" : "New proxy"}
        </button>
      </div>
      {error && <p className="text-sentinel-danger text-sm">{error}</p>}

      {showForm && (
        <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4 grid grid-cols-2 gap-3 max-w-xl">
          <input
            placeholder="Name"
            value={form.name}
            onChange={(e) => setForm({ ...form, name: e.target.value })}
            className="px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
          />
          <select
            value={form.kind}
            onChange={(e) =>
              setForm({ ...form, kind: e.target.value as ProxyProfile["kind"] })
            }
            className="px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
          >
            <option value="socks5">SOCKS5</option>
            <option value="http">HTTP</option>
            <option value="https">HTTPS</option>
          </select>
          <input
            placeholder="Host"
            value={form.host}
            onChange={(e) => setForm({ ...form, host: e.target.value })}
            className="px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
          />
          <input
            placeholder="Port"
            type="number"
            value={form.port}
            onChange={(e) => setForm({ ...form, port: e.target.value })}
            className="px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
          />
          <button
            className="col-span-2 px-4 py-2 bg-sentinel-accent rounded text-sm disabled:opacity-50"
            disabled={busy === "create"}
            onClick={createProxy}
          >
            {busy === "create" ? "Creating..." : "Create proxy"}
          </button>
        </div>
      )}

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
                  <span className={p.active ? "text-sentinel-success" : "text-sentinel-muted"}>
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
