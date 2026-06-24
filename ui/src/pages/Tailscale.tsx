import { useCallback, useEffect, useState } from "react";
import { useEvents } from "../contexts/ServiceContext";
import { apiClient, type TailnetProfile, type TailscaleStatus } from "../api/client";

const NIL_ID = "00000000-0000-0000-0000-000000000000";

function emptyProfile(): TailnetProfile {
  return {
    id: NIL_ID,
    name: "",
    auth_key: null,
    exit_node: null,
    subnet_router: false,
    magic_dns: true,
    hostname: null,
    tailnet_ip: null,
    connected: false,
  };
}

export function Tailscale() {
  const { lastEvent } = useEvents();
  const [status, setStatus] = useState<TailscaleStatus | null>(null);
  const [profiles, setProfiles] = useState<TailnetProfile[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);
  const [editing, setEditing] = useState<TailnetProfile>(emptyProfile());

  const refresh = useCallback(() => {
    return Promise.all([apiClient.tailnetStatus(), apiClient.tailnetProfiles()])
      .then(([s, p]) => {
        setStatus(s);
        setProfiles(p);
      })
      .catch((e) => setError(String(e)));
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  useEffect(() => {
    if (!lastEvent) return;
    if (lastEvent.kind === "tailnet_joined" || lastEvent.kind === "tailnet_left") {
      refresh();
    }
  }, [lastEvent, refresh]);

  const run = async (id: string, action: () => Promise<unknown>) => {
    setBusy(id);
    setError(null);
    try {
      await action();
      await refresh();
    } catch (e) {
      const msg = String(e);
      setError(
        msg.includes("tailscale")
          ? `${msg} — ensure tailscale CLI is installed and on PATH.`
          : msg
      );
    } finally {
      setBusy(null);
    }
  };

  const saveProfile = async () => {
    if (!editing.name.trim()) {
      setError("Profile name is required");
      return;
    }
    setBusy("save");
    try {
      await apiClient.upsertTailnetProfile(editing);
      setShowForm(false);
      setEditing(emptyProfile());
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h2 className="text-2xl font-semibold">Tailscale / Tailnet</h2>
        <button
          type="button"
          className="px-3 py-1.5 rounded bg-sentinel-accent text-sm"
          onClick={() => {
            setEditing(emptyProfile());
            setShowForm(true);
          }}
        >
          New profile
        </button>
      </div>

      {error && <p className="text-sentinel-danger text-sm">{error}</p>}

      <div className="grid grid-cols-3 gap-4">
        <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
          <p className="text-sentinel-muted text-sm">Connection</p>
          <p
            className={`text-lg font-semibold ${
              status?.connected ? "text-sentinel-success" : "text-sentinel-muted"
            }`}
          >
            {status?.connected ? "Connected" : "Disconnected"}
          </p>
        </div>
        <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
          <p className="text-sentinel-muted text-sm">Tailnet IP</p>
          <p className="font-mono text-sm">{status?.tailnet_ip ?? "—"}</p>
        </div>
        <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
          <p className="text-sentinel-muted text-sm">Exit node</p>
          <p className="text-sm">{status?.exit_node ?? "None"}</p>
        </div>
      </div>

      {showForm && (
        <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4 space-y-3">
          <h3 className="font-medium">Create tailnet profile</h3>
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
              <span className="text-sentinel-muted">Auth key (optional)</span>
              <input
                type="password"
                className="w-full rounded border border-slate-600 bg-slate-900 px-2 py-1 font-mono text-xs"
                value={editing.auth_key ?? ""}
                onChange={(e) =>
                  setEditing({ ...editing, auth_key: e.target.value || null })
                }
              />
            </label>
            <label className="text-sm space-y-1">
              <span className="text-sentinel-muted">Exit node (optional)</span>
              <input
                className="w-full rounded border border-slate-600 bg-slate-900 px-2 py-1 font-mono text-xs"
                value={editing.exit_node ?? ""}
                onChange={(e) =>
                  setEditing({ ...editing, exit_node: e.target.value || null })
                }
              />
            </label>
          </div>
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={editing.magic_dns ?? true}
              onChange={(e) => setEditing({ ...editing, magic_dns: e.target.checked })}
            />
            Magic DNS
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
              onClick={() => setShowForm(false)}
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
        <h3 className="font-medium mb-3">Profiles ({profiles.length})</h3>
        {profiles.length === 0 ? (
          <p className="text-sentinel-muted text-sm">No tailnet profiles configured</p>
        ) : (
          <ul className="space-y-2 text-sm">
            {profiles.map((p) => (
              <li
                key={p.id}
                className="flex flex-wrap items-center justify-between gap-2 p-3 bg-slate-800/50 rounded"
              >
                <div>
                  <p className="font-medium">{p.name}</p>
                  <p className="text-xs text-sentinel-muted">
                    {p.tailnet_ip ?? "no IP"}
                    {p.exit_node ? ` · exit ${p.exit_node}` : ""}
                  </p>
                </div>
                <div className="flex items-center gap-2">
                  <span
                    className={
                      p.connected ? "text-sentinel-success" : "text-sentinel-muted"
                    }
                  >
                    {p.connected ? "connected" : "idle"}
                  </span>
                  <button
                    type="button"
                    className="px-2 py-1 rounded bg-sentinel-accent text-xs"
                    disabled={busy === p.id}
                    onClick={() =>
                      run(p.id, () =>
                        p.connected
                          ? apiClient.tailnetLeave(p.id)
                          : apiClient.tailnetJoin(p.id)
                      )
                    }
                  >
                    {p.connected ? "Leave" : "Join"}
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
