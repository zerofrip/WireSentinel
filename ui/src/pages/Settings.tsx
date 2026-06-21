import { useEffect, useState } from "react";
import { apiClient, type BackupBundle, type EnterprisePolicy, type LogLevel, type RuntimeSettings, type TcpTerminationMode, type TcpTerminationSettings } from "../api/client";
import { useEvents } from "../contexts/ServiceContext";

const LOG_LEVELS: LogLevel[] = ["info", "warn", "error", "debug", "trace"];
const UPDATE_CHANNELS = ["stable", "beta", "dev"];

const TCP_MODES: { id: TcpTerminationMode; label: string }[] = [
  { id: "disabled", label: "Disabled" },
  { id: "on_vpn_connect", label: "On VPN connect" },
  { id: "on_vpn_disconnect", label: "On VPN disconnect" },
  { id: "on_route_change", label: "On route change" },
  { id: "always", label: "Always" },
];

export function Settings() {
  const { recovery } = useEvents();
  const [settings, setSettings] = useState<RuntimeSettings | null>(null);
  const [bundleJson, setBundleJson] = useState<string>("");
  const [policy, setPolicy] = useState<EnterprisePolicy | null>(null);
  const [tcpSettings, setTcpSettings] = useState<TcpTerminationSettings | null>(null);
  const [tcpSaving, setTcpSaving] = useState(false);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = async () => {
    try {
      const [bundle, enterprise, tcp] = await Promise.all([
        apiClient.backupExport("json") as Promise<BackupBundle>,
        apiClient.enterprisePolicy().catch(() => null),
        apiClient.tcpTerminationSettings().catch(() => null),
      ]);
      setSettings(bundle.settings ?? {});
      setBundleJson(JSON.stringify(bundle));
      setPolicy(enterprise);
      setTcpSettings(tcp);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load settings");
    }
  };

  useEffect(() => {
    load();
  }, []);

  const isLocked = (key: string) => policy?.locked_keys.includes(key) ?? false;

  const save = async () => {
    if (!settings || !bundleJson) return;
    setSaving(true);
    setMessage(null);
    setError(null);
    try {
      if (settings.log_level && !isLocked("log_level")) {
        await apiClient.setLogLevel(settings.log_level);
      }

      const bundle = JSON.parse(bundleJson) as BackupBundle;
      bundle.settings = {
        ...bundle.settings,
        log_level: settings.log_level,
        recovery_enabled: settings.recovery_enabled,
        metrics_interval_secs: settings.metrics_interval_secs,
        update_channel: settings.update_channel,
      };
      await apiClient.backupImport(JSON.stringify(bundle));
      setMessage("Settings saved");
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Save failed");
    } finally {
      setSaving(false);
    }
  };

  const saveTcpMode = async (mode: TcpTerminationMode) => {
    setTcpSaving(true);
    setError(null);
    setMessage(null);
    try {
      const updated = await apiClient.setTcpTerminationSettings({ mode });
      setTcpSettings(updated);
      setMessage("TCP session control updated");
    } catch (e) {
      setError(e instanceof Error ? e.message : "TCP settings save failed");
    } finally {
      setTcpSaving(false);
    }
  };

  if (!settings) {
    return <p className="text-sentinel-muted">Loading settings...</p>;
  }

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-semibold">Settings</h2>

      {error && (
        <div className="p-3 bg-red-900/30 border border-red-700 rounded text-sm">{error}</div>
      )}
      {message && (
        <div className="p-3 bg-green-900/30 border border-green-700 rounded text-sm">{message}</div>
      )}

      <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4 space-y-4 max-w-lg">
        <div>
          <label className="block text-sm text-sentinel-muted mb-1">Log level</label>
          <select
            value={settings.log_level ?? "info"}
            disabled={isLocked("log_level")}
            onChange={(e) =>
              setSettings({ ...settings, log_level: e.target.value as LogLevel })
            }
            className="w-full px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm disabled:opacity-50"
          >
            {LOG_LEVELS.map((level) => (
              <option key={level} value={level}>
                {level}
              </option>
            ))}
          </select>
          {isLocked("log_level") && (
            <p className="text-xs text-yellow-400 mt-1">Locked by enterprise policy</p>
          )}
        </div>

        <label className="flex items-center gap-3">
          <input
            type="checkbox"
            checked={settings.recovery_enabled ?? true}
            disabled={isLocked("recovery_enabled")}
            onChange={(e) =>
              setSettings({ ...settings, recovery_enabled: e.target.checked })
            }
          />
          <span>Crash recovery (restore VPN/chains on startup)</span>
        </label>

        <div>
          <label className="block text-sm text-sentinel-muted mb-1">
            Metrics collection interval (seconds)
          </label>
          <input
            type="number"
            min={5}
            max={3600}
            value={settings.metrics_interval_secs ?? 30}
            disabled={isLocked("metrics_interval_secs")}
            onChange={(e) =>
              setSettings({
                ...settings,
                metrics_interval_secs: Number(e.target.value),
              })
            }
            className="w-full px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm disabled:opacity-50"
          />
        </div>

        <div>
          <label className="block text-sm text-sentinel-muted mb-1">Update channel</label>
          <select
            value={settings.update_channel ?? "stable"}
            disabled={isLocked("update_channel")}
            onChange={(e) => setSettings({ ...settings, update_channel: e.target.value })}
            className="w-full px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm disabled:opacity-50"
          >
            {UPDATE_CHANNELS.map((channel) => (
              <option key={channel} value={channel}>
                {channel}
              </option>
            ))}
          </select>
        </div>

        <button
          onClick={save}
          disabled={saving}
          className="px-4 py-2 bg-sentinel-accent rounded text-sm hover:bg-blue-600 disabled:opacity-50"
        >
          {saving ? "Saving..." : "Save settings"}
        </button>
      </div>

      <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4 space-y-4 max-w-lg">
        <h3 className="font-medium">TCP Session Control</h3>
        <p className="text-xs text-sentinel-muted">
          Global policy for terminating existing TCP sessions when VPN or route state changes.
          Per-app reconnect rules are configured under Applications.
        </p>
        <div>
          <label className="block text-sm text-sentinel-muted mb-1">Termination mode</label>
          <select
            value={tcpSettings?.mode ?? "disabled"}
            disabled={tcpSaving}
            onChange={(e) => saveTcpMode(e.target.value as TcpTerminationMode)}
            className="w-full px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm disabled:opacity-50"
          >
            {TCP_MODES.map((m) => (
              <option key={m.id} value={m.id}>
                {m.label}
              </option>
            ))}
          </select>
        </div>
        {tcpSettings?.updated_at && (
          <p className="text-xs text-sentinel-muted">
            Last updated: {new Date(tcpSettings.updated_at).toLocaleString()}
          </p>
        )}
      </div>

      <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4 max-w-lg">
        <h3 className="font-medium mb-3">Recovery status</h3>
        <ul className="space-y-2 text-sm">
          <li className="flex justify-between">
            <span className="text-sentinel-muted">Status</span>
            <span>{recovery.status}</span>
          </li>
          {recovery.scope && (
            <li className="flex justify-between">
              <span className="text-sentinel-muted">Scope</span>
              <span>{recovery.scope}</span>
            </li>
          )}
          {recovery.restoredCount != null && (
            <li className="flex justify-between">
              <span className="text-sentinel-muted">Restored</span>
              <span>{recovery.restoredCount}</span>
            </li>
          )}
          {recovery.lastError && (
            <li className="text-sentinel-danger text-xs">{recovery.lastError}</li>
          )}
        </ul>
      </div>

      {policy && policy.locked_keys.length > 0 && (
        <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4 max-w-lg">
          <h3 className="font-medium mb-2">Enterprise policy</h3>
          <p className="text-xs text-sentinel-muted mb-2">
            Version {policy.version} · locked keys:
          </p>
          <ul className="text-xs font-mono space-y-1">
            {policy.locked_keys.map((key) => (
              <li key={key} className="text-yellow-400">
                {key}
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}
