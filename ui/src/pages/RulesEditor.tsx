import { useState } from "react";
import { useEvents } from "../contexts/ServiceContext";
import { apiClient, Rule, VpnListEntry } from "../api/client";

export function RulesEditor() {
  const { rules, status, vpnProfiles, refresh } = useEvents();
  const mode = status?.policy_mode ?? "blacklist";
  const killSwitch = status?.kill_switch_active ?? false;

  const [domainInput, setDomainInput] = useState("");
  const [vpnProfileId, setVpnProfileId] = useState("");
  const [priority, setPriority] = useState(50);

  const toggleMode = async () => {
    const next = mode === "blacklist" ? "whitelist" : "blacklist";
    await apiClient.setPolicyMode(next);
    await refresh();
  };

  const toggleKillSwitch = async () => {
    await apiClient.setKillSwitch(!killSwitch);
    await refresh();
  };

  const addDomainRoute = async () => {
    if (!domainInput.trim() || !vpnProfileId) return;
    const profile = vpnProfiles.find((e: VpnListEntry) => e.profile.id === vpnProfileId)?.profile;
    if (!profile) return;
    const rule: Rule = {
      id: crypto.randomUUID(),
      priority,
      scope: { type: "domain", value: domainInput.trim() },
      action: { type: "route_via_vpn", value: vpnProfileId },
      enabled: true,
    };
    await apiClient.addRule(rule);
    setDomainInput("");
    await refresh();
  };

  const toggleRule = async (rule: Rule) => {
    await apiClient.updateRule(rule.id, { ...rule, enabled: !rule.enabled });
    await refresh();
  };

  const removeRule = async (id: string) => {
    await apiClient.deleteRule(id);
    await refresh();
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between flex-wrap gap-2">
        <h2 className="text-2xl font-semibold">Rules Editor</h2>
        <div className="flex gap-2">
          <button
            onClick={toggleKillSwitch}
            className={`px-4 py-2 rounded text-sm ${
              killSwitch ? "bg-red-700 hover:bg-red-600" : "bg-slate-700 hover:bg-slate-600"
            }`}
          >
            Kill Switch: {killSwitch ? "ON" : "OFF"}
          </button>
          <button
            onClick={toggleMode}
            className="px-4 py-2 bg-slate-700 rounded text-sm hover:bg-slate-600"
          >
            Mode: {mode === "blacklist" ? "Blacklist" : "Whitelist"}
          </button>
        </div>
      </div>
      <p className="text-sentinel-muted text-sm">
        Global, per-app, and domain rules (priority-ordered).
      </p>
      <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4 space-y-3">
        <h3 className="font-medium">Domain routing</h3>
        <div className="flex flex-wrap gap-2 items-end">
          <div>
            <label className="block text-xs text-sentinel-muted mb-1">Domain</label>
            <input
              value={domainInput}
              onChange={(e) => setDomainInput(e.target.value)}
              placeholder="example.com"
              className="px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
            />
          </div>
          <div>
            <label className="block text-xs text-sentinel-muted mb-1">VPN profile</label>
            <select
              value={vpnProfileId}
              onChange={(e) => setVpnProfileId(e.target.value)}
              className="px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
            >
              <option value="">Select…</option>
              {vpnProfiles.map(({ profile }: VpnListEntry) => (
                <option key={profile.id} value={profile.id}>
                  {profile.name}
                </option>
              ))}
            </select>
          </div>
          <div>
            <label className="block text-xs text-sentinel-muted mb-1">Priority</label>
            <input
              type="number"
              value={priority}
              onChange={(e) => setPriority(Number(e.target.value))}
              className="w-20 px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
            />
          </div>
          <button
            onClick={addDomainRoute}
            className="px-4 py-2 bg-sentinel-accent rounded text-sm hover:bg-blue-600"
          >
            Add domain rule
          </button>
        </div>
      </div>
      <div className="bg-sentinel-panel rounded-lg border border-slate-700 overflow-hidden">
        <table className="w-full text-sm">
          <thead className="bg-slate-800/50 text-sentinel-muted">
            <tr>
              <th className="text-left p-3">Priority</th>
              <th className="text-left p-3">Scope</th>
              <th className="text-left p-3">Action</th>
              <th className="text-left p-3">Enabled</th>
              <th className="text-right p-3">Actions</th>
            </tr>
          </thead>
          <tbody>
            {rules.length === 0 && (
              <tr>
                <td colSpan={5} className="p-4 text-sentinel-muted">
                  No rules configured — default {mode} mode applies
                </td>
              </tr>
            )}
            {rules.map((r) => (
              <tr key={r.id} className="border-t border-slate-700">
                <td className="p-3">{r.priority}</td>
                <td className="p-3">
                  {r.scope.type}
                  {"value" in r.scope && r.scope.value ? `: ${String(r.scope.value).slice(0, 24)}` : ""}
                </td>
                <td className="p-3">{r.action.type}</td>
                <td className="p-3">{r.enabled ? "Yes" : "No"}</td>
                <td className="p-3 text-right space-x-2">
                  <button
                    onClick={() => toggleRule(r)}
                    className="px-2 py-1 text-xs bg-slate-700 rounded"
                  >
                    {r.enabled ? "Disable" : "Enable"}
                  </button>
                  <button
                    onClick={() => removeRule(r.id)}
                    className="px-2 py-1 text-xs bg-red-800/50 rounded"
                  >
                    Delete
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
