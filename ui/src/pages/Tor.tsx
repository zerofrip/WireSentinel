import { useCallback, useEffect, useState } from "react";
import { useEvents } from "../contexts/ServiceContext";
import {
  apiClient,
  type BridgeProfile,
  type TorProfile,
  type TorStatus,
} from "../api/client";

const NIL_ID = "00000000-0000-0000-0000-000000000000";

function emptyTorProfile(): TorProfile {
  return {
    id: NIL_ID,
    name: "",
    control_port: 9051,
    socks_port: 9050,
    data_dir: "",
    bridge_ids: [],
    enabled: true,
    bootstrap_progress: 0,
    circuit_count: 0,
  };
}

export function Tor() {
  const { lastEvent } = useEvents();
  const [status, setStatus] = useState<TorStatus | null>(null);
  const [profiles, setProfiles] = useState<TorProfile[]>([]);
  const [bridges, setBridges] = useState<BridgeProfile[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [editing, setEditing] = useState<TorProfile | null>(null);
  const [showForm, setShowForm] = useState(false);

  const refresh = useCallback(() => {
    return Promise.all([
      apiClient.torStatus(),
      apiClient.torProfiles(),
      apiClient.bridges(),
    ])
      .then(([s, p, b]) => {
        setStatus(s);
        setProfiles(p);
        setBridges(b);
      })
      .catch((e) => setError(String(e)));
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  useEffect(() => {
    if (!lastEvent) return;
    if (
      lastEvent.kind === "tor_started" ||
      lastEvent.kind === "tor_stopped" ||
      lastEvent.kind === "tor_circuit_changed"
    ) {
      refresh();
    }
  }, [lastEvent, refresh]);

  useEffect(() => {
    if (!status?.running) return;
    const timer = window.setInterval(() => {
      apiClient.torStatus().then(setStatus).catch(() => undefined);
    }, 5000);
    return () => window.clearInterval(timer);
  }, [status?.running]);

  const run = async (id: string, action: () => Promise<unknown>) => {
    setBusy(id);
    setError(null);
    try {
      await action();
      await refresh();
    } catch (e) {
      const msg = String(e);
      setError(
        msg.includes("not found") || msg.includes("executable")
          ? `${msg} — ensure resources/sing-box.exe and resources/tor.exe are installed.`
          : msg
      );
    } finally {
      setBusy(null);
    }
  };

  const activeProfileId = status?.profile?.id ?? null;

  const saveProfile = async () => {
    if (!editing?.name.trim()) {
      setError("Profile name is required");
      return;
    }
    setBusy("save");
    setError(null);
    try {
      await apiClient.upsertTorProfile(editing);
      setShowForm(false);
      setEditing(null);
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(null);
    }
  };

  const openCreate = () => {
    setEditing(emptyTorProfile());
    setShowForm(true);
  };

  const openEdit = (profile: TorProfile) => {
    setEditing({ ...profile });
    setShowForm(true);
  };

  const toggleBridge = (bridgeId: string) => {
    if (!editing) return;
    const has = editing.bridge_ids.includes(bridgeId);
    setEditing({
      ...editing,
      bridge_ids: has
        ? editing.bridge_ids.filter((id) => id !== bridgeId)
        : [...editing.bridge_ids, bridgeId],
    });
  };

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h2 className="text-2xl font-semibold">Tor</h2>
        <button
          type="button"
          className="px-3 py-1.5 rounded bg-sentinel-accent text-sm"
          onClick={openCreate}
        >
          New profile
        </button>
      </div>

      {error && <p className="text-sentinel-danger text-sm">{error}</p>}

      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
          <p className="text-sentinel-muted text-sm">Status</p>
          <p
            className={`text-lg font-semibold ${
              status?.running ? "text-sentinel-success" : "text-sentinel-muted"
            }`}
          >
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

      {showForm && editing && (
        <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4 space-y-3">
          <h3 className="font-medium">
            {editing.id === NIL_ID ? "Create Tor profile" : "Edit Tor profile"}
          </h3>
          <div className="grid gap-3 md:grid-cols-2">
            <label className="text-sm space-y-1">
              <span className="text-sentinel-muted">Name</span>
              <input
                className="w-full rounded border border-slate-600 bg-slate-900 px-2 py-1"
                value={editing.name}
                onChange={(e) => setEditing({ ...editing, name: e.target.value })}
              />
            </label>
            <label className="text-sm space-y-1">
              <span className="text-sentinel-muted">SOCKS port</span>
              <input
                type="number"
                className="w-full rounded border border-slate-600 bg-slate-900 px-2 py-1"
                value={editing.socks_port}
                onChange={(e) =>
                  setEditing({ ...editing, socks_port: Number(e.target.value) })
                }
              />
            </label>
            <label className="text-sm space-y-1 md:col-span-2">
              <span className="text-sentinel-muted">Data directory (optional)</span>
              <input
                className="w-full rounded border border-slate-600 bg-slate-900 px-2 py-1 font-mono text-xs"
                placeholder="%ProgramData%/WireSentinel/tor/{profile_id}/"
                value={editing.data_dir}
                onChange={(e) => setEditing({ ...editing, data_dir: e.target.value })}
              />
            </label>
          </div>
          <div>
            <p className="text-sm text-sentinel-muted mb-2">Bridges</p>
            {bridges.length === 0 ? (
              <p className="text-xs text-sentinel-muted">
                No bridges — add some on the Bridges page.
              </p>
            ) : (
              <ul className="space-y-1 text-sm">
                {bridges.map((b) => (
                  <li key={b.id}>
                    <label className="flex items-center gap-2">
                      <input
                        type="checkbox"
                        checked={editing.bridge_ids.includes(b.id)}
                        onChange={() => toggleBridge(b.id)}
                      />
                      <span>{b.name}</span>
                      <span className="text-xs text-sentinel-muted">{b.bridge_type}</span>
                    </label>
                  </li>
                ))}
              </ul>
            )}
          </div>
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={editing.enabled}
              onChange={(e) => setEditing({ ...editing, enabled: e.target.checked })}
            />
            Enabled
          </label>
          <div className="flex gap-2">
            <button
              type="button"
              className="px-3 py-1.5 rounded bg-sentinel-accent text-sm"
              disabled={busy === "save"}
              onClick={saveProfile}
            >
              Save
            </button>
            <button
              type="button"
              className="px-3 py-1.5 rounded border border-slate-600 text-sm"
              onClick={() => {
                setShowForm(false);
                setEditing(null);
              }}
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
        <h3 className="font-medium mb-3">Tor profiles</h3>
        {profiles.length === 0 ? (
          <p className="text-sentinel-muted text-sm">No Tor profiles configured</p>
        ) : (
          <ul className="space-y-2 text-sm">
            {profiles.map((p) => {
              const isActive = activeProfileId === p.id && status?.running;
              return (
                <li
                  key={p.id}
                  className="flex flex-wrap items-center justify-between gap-2 p-3 bg-slate-800/50 rounded"
                >
                  <div>
                    <p className="font-medium">{p.name}</p>
                    <p className="text-xs text-sentinel-muted">
                      SOCKS {p.socks_port}
                      {p.bridge_ids.length > 0 ? ` · ${p.bridge_ids.length} bridge(s)` : ""}
                      {p.enabled ? "" : " · disabled"}
                    </p>
                  </div>
                  <div className="flex items-center gap-2">
                    <span
                      className={
                        isActive ? "text-sentinel-success" : "text-sentinel-muted"
                      }
                    >
                      {isActive ? "running" : "stopped"}
                    </span>
                    <button
                      type="button"
                      className="px-2 py-1 rounded border border-slate-600 text-xs"
                      onClick={() => openEdit(p)}
                    >
                      Edit
                    </button>
                    <button
                      type="button"
                      className="px-2 py-1 rounded bg-sentinel-accent text-xs"
                      disabled={busy === p.id}
                      onClick={() =>
                        run(p.id, () =>
                          isActive ? apiClient.torStop(p.id) : apiClient.torStart(p.id)
                        )
                      }
                    >
                      {isActive ? "Stop" : "Start"}
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
