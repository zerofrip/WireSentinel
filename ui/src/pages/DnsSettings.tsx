import { useEffect, useState } from "react";
import { useEvents } from "../contexts/ServiceContext";
import { apiClient, DnsSettings, FilterListRecord } from "../api/client";

const PROVIDERS = [
  { id: "cloudflare", label: "Cloudflare", url: "https://cloudflare-dns.com/dns-query" },
  { id: "quad9", label: "Quad9", url: "https://dns.quad9.net/dns-query" },
];

export function DnsSettingsPage() {
  const { dnsLogs, filterLists, dnsProviders, refresh } = useEvents();
  const [settings, setSettings] = useState<DnsSettings | null>(null);
  const [newListName, setNewListName] = useState("");
  const [newListUrl, setNewListUrl] = useState("");
  const [failover, setFailover] = useState(true);
  const [providers, setProviders] = useState(dnsProviders);

  useEffect(() => {
    apiClient.dnsSettings().then(setSettings).catch(() => apiClient.dns().then(setSettings));
  }, []);

  useEffect(() => {
    setProviders(dnsProviders);
  }, [dnsProviders]);

  const save = async () => {
    if (!settings) return;
    await apiClient.setDns(settings);
  };

  const saveProviders = async () => {
    await apiClient.updateDnsProviders({ providers, failover });
    await refresh();
  };

  const toggleProvider = (id: string) => {
    setProviders((prev) =>
      prev.map((p) => (p.id === id ? { ...p, enabled: !p.enabled } : p))
    );
  };

  const providerHealth = (p: (typeof providers)[number]) => {
    if (!p.enabled) return { label: "Disabled", className: "text-sentinel-muted" };
    if (p.failure_count > 0) return { label: "Degraded", className: "text-yellow-400" };
    if (p.latency_ms != null && p.latency_ms < 100) {
      return { label: "Healthy", className: "text-sentinel-success" };
    }
    if (p.latency_ms != null) return { label: "Slow", className: "text-yellow-400" };
    return { label: "Unknown", className: "text-sentinel-muted" };
  };

  const addList = async () => {
    if (!newListName || !newListUrl) return;
    const record: FilterListRecord = {
      id: crypto.randomUUID(),
      name: newListName,
      url: newListUrl,
      list_type: "hosts",
      enabled: true,
    };
    await apiClient.addFilterList(record);
    setNewListName("");
    setNewListUrl("");
    await refresh();
  };

  const toggleList = async (list: FilterListRecord) => {
    await apiClient.updateFilterList(list.id, { ...list, enabled: !list.enabled });
    await refresh();
  };

  const updateList = async (id: string) => {
    await apiClient.refreshFilterList(id);
    await refresh();
  };

  const removeList = async (id: string) => {
    await apiClient.deleteFilterList(id);
    await refresh();
  };

  if (!settings) {
    return <p className="text-sentinel-muted">Loading DNS settings...</p>;
  }

  return (
    <div className="space-y-4">
      <h2 className="text-2xl font-semibold">DNS Settings</h2>
      <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4 space-y-4 max-w-lg">
        <label className="flex items-center gap-3">
          <input
            type="checkbox"
            checked={settings.enabled}
            onChange={(e) => setSettings({ ...settings, enabled: e.target.checked })}
          />
          <span>Enable DNS proxy</span>
        </label>
        <label className="flex items-center gap-3">
          <input
            type="checkbox"
            checked={settings.dot_enabled ?? false}
            onChange={(e) =>
              setSettings({
                ...settings,
                dot_enabled: e.target.checked,
                transport: e.target.checked ? "dot" : "doh",
              })
            }
          />
          <span>Use DoT transport (DoH when off)</span>
        </label>
        <div>
          <label className="block text-sm text-sentinel-muted mb-1">Filter mode</label>
          <select
            value={settings.filter_mode ?? "blacklist"}
            onChange={(e) =>
              setSettings({
                ...settings,
                filter_mode: e.target.value as "blacklist" | "whitelist",
              })
            }
            className="w-full px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
          >
            <option value="blacklist">Blacklist</option>
            <option value="whitelist">Whitelist</option>
          </select>
        </div>
        <div>
          <label className="block text-sm text-sentinel-muted mb-1">Block mode</label>
          <select
            value={settings.dns_block_mode ?? "nxdomain"}
            onChange={(e) =>
              setSettings({
                ...settings,
                dns_block_mode: e.target.value as "null" | "nxdomain",
              })
            }
            className="w-full px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
          >
            <option value="nxdomain">NXDOMAIN</option>
            <option value="null">Null (0.0.0.0)</option>
          </select>
        </div>
        <div>
          <label className="block text-sm text-sentinel-muted mb-1">Listen address</label>
          <input
            value={settings.listen_addr ?? "127.0.0.1:5353"}
            onChange={(e) => setSettings({ ...settings, listen_addr: e.target.value })}
            className="w-full px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm font-mono"
          />
        </div>
        <div>
          <label className="block text-sm text-sentinel-muted mb-1">Provider</label>
          <select
            value={settings.provider}
            onChange={(e) => {
              const p = PROVIDERS.find((x) => x.id === e.target.value);
              setSettings({
                ...settings,
                provider: e.target.value,
                upstream_url: p?.url ?? settings.upstream_url,
              });
            }}
            className="w-full px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
          >
            {PROVIDERS.map((p) => (
              <option key={p.id} value={p.id}>
                {p.label}
              </option>
            ))}
          </select>
        </div>
        <button
          onClick={save}
          className="px-4 py-2 bg-sentinel-accent rounded text-sm hover:bg-blue-600"
        >
          Save
        </button>
      </div>
      <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
        <div className="flex items-center justify-between mb-4">
          <h3 className="font-medium">DNS Providers</h3>
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={failover}
              onChange={(e) => setFailover(e.target.checked)}
            />
            <span>Failover</span>
          </label>
        </div>
        {providers.length === 0 ? (
          <p className="text-sentinel-muted text-sm">No DNS providers configured</p>
        ) : (
          <table className="w-full text-sm">
            <thead className="text-sentinel-muted">
              <tr>
                <th className="text-left p-2">Name</th>
                <th className="text-left p-2">Transport</th>
                <th className="text-left p-2">Latency</th>
                <th className="text-left p-2">Health</th>
                <th className="text-left p-2">Priority</th>
                <th className="text-left p-2">Enabled</th>
              </tr>
            </thead>
            <tbody>
              {providers.map((p) => {
                const health = providerHealth(p);
                return (
                  <tr key={p.id} className="border-t border-slate-700">
                    <td className="p-2">
                      <p className="font-medium">{p.name}</p>
                      <p className="text-xs text-sentinel-muted truncate max-w-xs">{p.endpoint}</p>
                    </td>
                    <td className="p-2 uppercase text-xs">{p.transport}</td>
                    <td className="p-2 font-mono text-xs">
                      {p.latency_ms != null ? `${p.latency_ms} ms` : "—"}
                    </td>
                    <td className={`p-2 text-xs ${health.className}`}>{health.label}</td>
                    <td className="p-2">{p.priority}</td>
                    <td className="p-2">
                      <button
                        onClick={() => toggleProvider(p.id)}
                        className={`px-2 py-1 text-xs rounded ${
                          p.enabled ? "bg-green-800/50" : "bg-slate-700"
                        }`}
                      >
                        {p.enabled ? "On" : "Off"}
                      </button>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
        {providers.length > 0 && (
          <button
            onClick={saveProviders}
            className="mt-4 px-4 py-2 bg-sentinel-accent rounded text-sm hover:bg-blue-600"
          >
            Save providers
          </button>
        )}
      </div>
      <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
        <h3 className="font-medium mb-2">Filter Lists</h3>
        <div className="flex gap-2 mb-4">
          <input
            placeholder="Name"
            value={newListName}
            onChange={(e) => setNewListName(e.target.value)}
            className="flex-1 px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
          />
          <input
            placeholder="URL"
            value={newListUrl}
            onChange={(e) => setNewListUrl(e.target.value)}
            className="flex-[2] px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
          />
          <button onClick={addList} className="px-4 py-2 bg-sentinel-accent rounded text-sm">
            Add
          </button>
        </div>
        {filterLists.length === 0 ? (
          <p className="text-sentinel-muted text-sm">No filter lists configured</p>
        ) : (
          <ul className="space-y-2 text-sm">
            {filterLists.map((list) => (
              <li
                key={list.id}
                className="flex items-center justify-between p-2 bg-slate-800/50 rounded"
              >
                <div>
                  <p className="font-medium">{list.name}</p>
                  <p className="text-xs text-sentinel-muted truncate max-w-md">{list.url}</p>
                </div>
                <div className="space-x-2">
                  <button
                    onClick={() => toggleList(list)}
                    className="px-2 py-1 text-xs bg-slate-700 rounded"
                  >
                    {list.enabled ? "Disable" : "Enable"}
                  </button>
                  <button
                    onClick={() => updateList(list.id)}
                    className="px-2 py-1 text-xs bg-blue-800/50 rounded"
                  >
                    Update
                  </button>
                  <button
                    onClick={() => removeList(list.id)}
                    className="px-2 py-1 text-xs bg-red-800/50 rounded"
                  >
                    Delete
                  </button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>
      <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
        <h3 className="font-medium mb-2">DNS Logs</h3>
        {dnsLogs.length === 0 ? (
          <p className="text-sentinel-muted text-sm">No DNS logs yet</p>
        ) : (
          <table className="w-full text-sm">
            <thead className="text-sentinel-muted">
              <tr>
                <th className="text-left p-2">Query</th>
                <th className="text-left p-2">Type</th>
                <th className="text-left p-2">Blocked</th>
              </tr>
            </thead>
            <tbody>
              {dnsLogs.slice(0, 20).map((log) => (
                <tr key={log.id} className="border-t border-slate-700">
                  <td className="p-2">{log.qname}</td>
                  <td className="p-2">{log.qtype}</td>
                  <td className="p-2">{log.blocked ? "Yes" : "No"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}
