import { useEffect, useState } from "react";
import {
  apiClient,
  type CoverTrafficProfile,
  type CoverTrafficSettings,
} from "../api/client";

const PROFILES: CoverTrafficProfile[] = [
  "disabled",
  "low",
  "medium",
  "high",
  "maximum",
];

export function CoverTraffic() {
  const [settings, setSettings] = useState<CoverTrafficSettings | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  const load = async () => {
    setError(null);
    try {
      const data = await apiClient.coverTraffic();
      setSettings(data);
    } catch (e) {
      setError(String(e));
    }
  };

  useEffect(() => {
    load();
  }, []);

  const save = async () => {
    if (!settings) return;
    setSaving(true);
    setError(null);
    setMessage(null);
    try {
      const updated = await apiClient.setCoverTrafficSettings(settings);
      setSettings(updated);
      setMessage("Settings saved");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Save failed");
    } finally {
      setSaving(false);
    }
  };

  if (!settings) {
    return (
      <div className="space-y-6">
        <h2 className="text-2xl font-semibold">Cover Traffic</h2>
        {error ? (
          <p className="text-sentinel-danger text-sm">{error}</p>
        ) : (
          <p className="text-sentinel-muted text-sm">Loading...</p>
        )}
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-semibold">Cover Traffic</h2>
      {error && <p className="text-sentinel-danger text-sm">{error}</p>}
      {message && <p className="text-sentinel-success text-sm">{message}</p>}

      <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4 space-y-4 max-w-lg">
        <label className="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={settings.enabled}
            onChange={(e) => setSettings({ ...settings, enabled: e.target.checked })}
          />
          Enable cover traffic
        </label>

        <div>
          <label className="block text-xs text-sentinel-muted mb-1">Profile</label>
          <select
            value={settings.profile}
            onChange={(e) =>
              setSettings({
                ...settings,
                profile: e.target.value as CoverTrafficProfile,
              })
            }
            className="w-full px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
          >
            {PROFILES.map((p) => (
              <option key={p} value={p}>
                {p}
              </option>
            ))}
          </select>
        </div>

        <div>
          <label className="block text-xs text-sentinel-muted mb-1">Rate (bps, optional)</label>
          <input
            type="number"
            min={0}
            value={settings.rate_bps ?? ""}
            onChange={(e) =>
              setSettings({
                ...settings,
                rate_bps: e.target.value ? Number(e.target.value) : null,
              })
            }
            className="w-full px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
            placeholder="Auto from profile"
          />
        </div>

        <button
          onClick={save}
          disabled={saving}
          className="px-4 py-2 bg-sentinel-accent rounded text-sm hover:bg-blue-600 disabled:opacity-50"
        >
          {saving ? "Saving..." : "Save settings"}
        </button>
      </div>
    </div>
  );
}
