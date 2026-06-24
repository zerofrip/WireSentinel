import { useEffect, useState } from "react";
import { apiClient, type PluginRecord } from "../api/client";

export function Plugins() {
  const [plugins, setPlugins] = useState<PluginRecord[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);

  const refresh = () => {
    apiClient
      .plugins()
      .then(setPlugins)
      .catch((e) => setError(String(e)));
  };

  useEffect(() => {
    refresh();
  }, []);

  const run = async (id: string, action: "load" | "unload") => {
    setBusy(id);
    setError(null);
    try {
      if (action === "load") await apiClient.loadPlugin(id);
      else await apiClient.unloadPlugin(id);
      refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(null);
    }
  };

  const loaded = plugins.filter((p) => p.state === "loaded");

  return (
    <div className="space-y-4">
      <h2 className="text-2xl font-semibold">Plugins</h2>
      {error && <p className="text-sentinel-danger text-sm">{error}</p>}
      <div className="grid grid-cols-3 gap-4">
        <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
          <p className="text-sentinel-muted text-sm">Installed</p>
          <p className="text-2xl font-bold">{plugins.length}</p>
        </div>
        <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
          <p className="text-sentinel-muted text-sm">Loaded</p>
          <p className="text-2xl font-bold text-sentinel-success">{loaded.length}</p>
        </div>
        <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
          <p className="text-sentinel-muted text-sm">Failed</p>
          <p className="text-2xl font-bold text-sentinel-danger">
            {plugins.filter((p) => p.state === "failed").length}
          </p>
        </div>
      </div>
      <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
        <h3 className="font-medium mb-3">Plugin registry</h3>
        {plugins.length === 0 ? (
          <p className="text-sentinel-muted text-sm">No plugins installed</p>
        ) : (
          <ul className="space-y-2 text-sm">
            {plugins.map((p) => (
              <li
                key={p.id}
                className="flex flex-wrap justify-between items-center gap-2 p-2 bg-slate-800/50 rounded"
              >
                <div>
                  <p className="font-medium">{p.manifest.name}</p>
                  <p className="text-xs text-sentinel-muted">
                    {p.manifest.version} · {p.manifest.format}
                  </p>
                </div>
                <div className="flex items-center gap-2">
                  <span
                    className={
                      p.state === "loaded"
                        ? "text-sentinel-success"
                        : p.state === "failed"
                          ? "text-sentinel-danger"
                          : "text-sentinel-muted"
                    }
                  >
                    {p.state}
                  </span>
                  {p.state !== "loaded" ? (
                    <button
                      className="px-2 py-1 rounded bg-sentinel-accent text-xs disabled:opacity-50"
                      disabled={busy === p.id}
                      onClick={() => run(p.id, "load")}
                    >
                      Load
                    </button>
                  ) : (
                    <button
                      className="px-2 py-1 rounded border border-slate-600 text-xs disabled:opacity-50"
                      disabled={busy === p.id}
                      onClick={() => run(p.id, "unload")}
                    >
                      Unload
                    </button>
                  )}
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
