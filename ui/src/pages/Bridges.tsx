import { useEffect, useState } from "react";
import {
  apiClient,
  type BridgeProfile,
  type BridgeTestResult,
} from "../api/client";

const NIL_ID = "00000000-0000-0000-0000-000000000000";

type BridgeType = BridgeProfile["bridge_type"];

export function Bridges() {
  const [bridges, setBridges] = useState<BridgeProfile[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [testResults, setTestResults] = useState<Record<string, BridgeTestResult>>({});
  const [showForm, setShowForm] = useState(false);
  const [name, setName] = useState("");
  const [bridgeType, setBridgeType] = useState<BridgeType>("obfs4");
  const [line, setLine] = useState("");
  const [enabled, setEnabled] = useState(true);

  const refresh = () => {
    apiClient
      .bridges()
      .then(setBridges)
      .catch((e) => setError(String(e)));
  };

  useEffect(() => {
    refresh();
  }, []);

  const resetForm = () => {
    setName("");
    setBridgeType("obfs4");
    setLine("");
    setEnabled(true);
    setShowForm(false);
  };

  const createBridge = async () => {
    if (!name.trim() || !line.trim()) {
      setError("Name and bridge line are required");
      return;
    }
    setBusy("create");
    setError(null);
    try {
      const profile: BridgeProfile = {
        id: NIL_ID,
        name: name.trim(),
        bridge_type: bridgeType,
        config_json: { line: line.trim() },
        enabled,
      };
      await apiClient.createBridge(profile);
      resetForm();
      refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(null);
    }
  };

  const testBridge = async (bridgeId: string) => {
    setBusy(bridgeId);
    setError(null);
    try {
      const result = await apiClient.testBridge(bridgeId);
      setTestResults((prev) => ({ ...prev, [bridgeId]: result }));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h2 className="text-2xl font-semibold">Tor Bridges</h2>
        <button
          type="button"
          className="px-3 py-1.5 rounded bg-sentinel-accent text-sm"
          onClick={() => setShowForm(true)}
        >
          Add bridge
        </button>
      </div>

      {error && <p className="text-sentinel-danger text-sm">{error}</p>}

      {showForm && (
        <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4 space-y-3">
          <h3 className="font-medium">New bridge</h3>
          <div className="grid gap-3 md:grid-cols-2">
            <label className="text-sm space-y-1">
              <span className="text-sentinel-muted">Name</span>
              <input
                className="w-full rounded border border-slate-600 bg-slate-900 px-2 py-1"
                value={name}
                onChange={(e) => setName(e.target.value)}
              />
            </label>
            <label className="text-sm space-y-1">
              <span className="text-sentinel-muted">Type</span>
              <select
                className="w-full rounded border border-slate-600 bg-slate-900 px-2 py-1"
                value={bridgeType}
                onChange={(e) => setBridgeType(e.target.value as BridgeType)}
              >
                <option value="obfs4">obfs4</option>
                <option value="snowflake">snowflake</option>
                <option value="meek">meek</option>
                <option value="webtunnel">webtunnel</option>
              </select>
            </label>
            <label className="text-sm space-y-1 md:col-span-2">
              <span className="text-sentinel-muted">Bridge line (torrc)</span>
              <textarea
                className="w-full rounded border border-slate-600 bg-slate-900 px-2 py-1 font-mono text-xs min-h-[4rem]"
                placeholder="obfs4 1.2.3.4:443 ..."
                value={line}
                onChange={(e) => setLine(e.target.value)}
              />
            </label>
          </div>
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={enabled}
              onChange={(e) => setEnabled(e.target.checked)}
            />
            Enabled
          </label>
          <div className="flex gap-2">
            <button
              type="button"
              className="px-3 py-1.5 rounded bg-sentinel-accent text-sm"
              disabled={busy === "create"}
              onClick={createBridge}
            >
              Save
            </button>
            <button
              type="button"
              className="px-3 py-1.5 rounded border border-slate-600 text-sm"
              onClick={resetForm}
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
        <h3 className="font-medium mb-3">{bridges.length} bridge profile(s)</h3>
        {bridges.length === 0 ? (
          <p className="text-sentinel-muted text-sm">No bridges configured</p>
        ) : (
          <ul className="space-y-2 text-sm">
            {bridges.map((b) => {
              const result = testResults[b.id];
              return (
                <li
                  key={b.id}
                  className="p-3 bg-slate-800/50 rounded flex flex-wrap items-start justify-between gap-2"
                >
                  <div className="min-w-0 flex-1">
                    <p className="font-medium">{b.name}</p>
                    <p className="text-xs text-sentinel-muted">{b.bridge_type}</p>
                    {b.config_json?.line && (
                      <p className="text-xs font-mono text-sentinel-muted mt-1 truncate">
                        {b.config_json.line}
                      </p>
                    )}
                    {result && (
                      <p
                        className={`text-xs mt-1 ${
                          result.success ? "text-sentinel-success" : "text-sentinel-danger"
                        }`}
                      >
                        {result.success
                          ? `Reachable${result.latency_ms != null ? ` (${result.latency_ms} ms)` : ""}`
                          : result.error ?? "Unreachable"}
                      </p>
                    )}
                  </div>
                  <div className="flex items-center gap-2 shrink-0">
                    <span
                      className={
                        b.enabled ? "text-sentinel-success" : "text-sentinel-muted"
                      }
                    >
                      {b.enabled ? "enabled" : "disabled"}
                    </span>
                    <button
                      type="button"
                      className="px-2 py-1 rounded border border-slate-600 text-xs"
                      disabled={busy === b.id}
                      onClick={() => testBridge(b.id)}
                    >
                      Test
                    </button>
                  </div>
                </li>
              );
            })}
          </ul>
        )}
      </div>
    </div>
  );
}
