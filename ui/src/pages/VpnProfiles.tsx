import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useEvents } from "../contexts/ServiceContext";
import {
  apiClient,
  HandshakeProxySettings,
  ObfuscationPreset,
  TransportKind,
  VpnState,
} from "../api/client";

const TRANSPORT_KINDS: { id: TransportKind; label: string }[] = [
  { id: "direct", label: "Direct" },
  { id: "wire_guard", label: "WireGuard" },
  { id: "amnezia_wg", label: "AmneziaWG" },
  { id: "sing_box", label: "Sing-box" },
  { id: "xray", label: "Xray" },
];

const OBFUSCATION_PRESETS: { id: ObfuscationPreset; label: string }[] = [
  { id: "disabled", label: "Disabled" },
  { id: "basic", label: "Basic" },
  { id: "balanced", label: "Balanced" },
  { id: "aggressive", label: "Aggressive" },
];

function defaultHandshakeProxy(): HandshakeProxySettings {
  return {
    enabled: false,
    proxy_type: "socks5",
    host: "",
    port: 1080,
    username: null,
    password: null,
  };
}

function statsFor(entry: { status: string | VpnState }) {
  if (typeof entry.status === "string") return null;
  return entry.status.stats;
}

export function VpnProfiles() {
  const { vpnProfiles, chains, refresh } = useEvents();
  const [name, setName] = useState("");
  const [config, setConfig] = useState("");
  const [transportKind, setTransportKind] = useState<TransportKind>("wire_guard");
  const [obfuscationPreset, setObfuscationPreset] = useState<ObfuscationPreset>("disabled");
  const [chainName, setChainName] = useState("");
  const [selectedProfileId, setSelectedProfileId] = useState<string | null>(null);
  const [handshakeProxy, setHandshakeProxy] = useState<HandshakeProxySettings>(
    defaultHandshakeProxy()
  );
  const [proxySaving, setProxySaving] = useState(false);
  const [proxyMessage, setProxyMessage] = useState<string | null>(null);
  const [proxyError, setProxyError] = useState<string | null>(null);

  const importProfile = async () => {
    if (!name || !config) return;
    await apiClient.addVpn(name, config);
    setName("");
    setConfig("");
    await refresh();
  };

  const createChainLink = async () => {
    if (!chainName) return;
    await apiClient.createChain({
      name: chainName,
      hops: [{ kind: transportKind }],
      enabled: true,
    });
    setChainName("");
    await refresh();
  };

  const statusFor = (id: string) => {
    const entry = vpnProfiles.find((e) => e.profile.id === id);
    if (!entry) return "disconnected";
    if (typeof entry.status === "string") return entry.status;
    return (entry.status as VpnState).status ?? "disconnected";
  };

  const formatBytes = (n: number) => {
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
    return `${(n / (1024 * 1024)).toFixed(1)} MB`;
  };

  const loadHandshakeProxy = async (profileId: string) => {
    setSelectedProfileId(profileId);
    setProxyMessage(null);
    setProxyError(null);
    const entry = vpnProfiles.find((e) => e.profile.id === profileId);
    if (entry?.profile.handshake_proxy) {
      setHandshakeProxy({ ...defaultHandshakeProxy(), ...entry.profile.handshake_proxy });
      return;
    }
    try {
      const settings = await apiClient.getVpnHandshakeProxy(profileId);
      setHandshakeProxy({ ...defaultHandshakeProxy(), ...settings });
    } catch {
      setHandshakeProxy(defaultHandshakeProxy());
    }
  };

  useEffect(() => {
    if (selectedProfileId) {
      loadHandshakeProxy(selectedProfileId);
    }
  }, [selectedProfileId, vpnProfiles]);

  const saveHandshakeProxy = async () => {
    if (!selectedProfileId) return;
    setProxySaving(true);
    setProxyMessage(null);
    setProxyError(null);
    try {
      await apiClient.setVpnHandshakeProxy(selectedProfileId, handshakeProxy);
      setProxyMessage("Handshake proxy saved");
      await refresh();
    } catch (e) {
      setProxyError(e instanceof Error ? e.message : "Save failed");
    } finally {
      setProxySaving(false);
    }
  };

  const selectedProfile = vpnProfiles.find((e) => e.profile.id === selectedProfileId)?.profile;

  return (
    <div className="space-y-4">
      <h2 className="text-2xl font-semibold">VPN Profiles</h2>
      <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4 grid grid-cols-3 gap-4">
        <div>
          <label className="block text-sm text-sentinel-muted mb-1">Transport kind</label>
          <select
            value={transportKind}
            onChange={(e) => setTransportKind(e.target.value as TransportKind)}
            className="w-full px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
          >
            {TRANSPORT_KINDS.map((k) => (
              <option key={k.id} value={k.id}>
                {k.label}
              </option>
            ))}
          </select>
        </div>
        <div>
          <label className="block text-sm text-sentinel-muted mb-1">Obfuscation preset</label>
          <select
            value={obfuscationPreset}
            onChange={(e) => setObfuscationPreset(e.target.value as ObfuscationPreset)}
            className="w-full px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
          >
            {OBFUSCATION_PRESETS.map((p) => (
              <option key={p.id} value={p.id}>
                {p.label}
              </option>
            ))}
          </select>
        </div>
        <div>
          <label className="block text-sm text-sentinel-muted mb-1">Chain link</label>
          <div className="flex gap-2">
            <input
              placeholder="Chain name"
              value={chainName}
              onChange={(e) => setChainName(e.target.value)}
              className="flex-1 px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
            />
            <button
              onClick={createChainLink}
              className="px-3 py-2 bg-sentinel-accent rounded text-sm hover:bg-blue-600 whitespace-nowrap"
            >
              Create
            </button>
          </div>
          <p className="text-xs text-sentinel-muted mt-1">
            {chains.length} chain(s) ·{" "}
            <Link to="/privacy" className="text-sentinel-accent hover:underline">
              Privacy dashboard
            </Link>
          </p>
        </div>
      </div>
      <div className="grid grid-cols-2 gap-4">
        <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4 space-y-3">
          <h3 className="font-medium">Import WireGuard .conf</h3>
          <input
            placeholder="Profile name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            className="w-full px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
          />
          <textarea
            placeholder="Paste WireGuard config..."
            value={config}
            onChange={(e) => setConfig(e.target.value)}
            rows={8}
            className="w-full px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm font-mono"
          />
          <button
            onClick={importProfile}
            className="px-4 py-2 bg-sentinel-accent rounded text-sm hover:bg-blue-600"
          >
            Import
          </button>
        </div>
        <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
          <h3 className="font-medium mb-3">Profiles</h3>
          {vpnProfiles.length === 0 && (
            <p className="text-sentinel-muted text-sm">No VPN profiles configured</p>
          )}
          <ul className="space-y-2">
            {vpnProfiles.map((entry) => {
              const p = entry.profile;
              const stats = statsFor(entry);
              const isSelected = selectedProfileId === p.id;
              return (
                <li
                  key={p.id}
                  className={`p-2 rounded ${isSelected ? "bg-slate-700/80 ring-1 ring-sentinel-accent" : "bg-slate-800/50"}`}
                >
                  <div className="flex items-center justify-between">
                    <button
                      type="button"
                      onClick={() => loadHandshakeProxy(p.id)}
                      className="text-left flex-1"
                    >
                      <p className="font-medium">{p.name}</p>
                      <p className="text-xs text-sentinel-muted">{statusFor(p.id)}</p>
                      {p.handshake_proxy?.enabled && (
                        <p className="text-xs text-sentinel-accent mt-0.5">
                          Handshake proxy: {p.handshake_proxy.host}:{p.handshake_proxy.port}
                        </p>
                      )}
                      {stats && (
                        <p className="text-xs text-sentinel-muted">
                          RX {formatBytes(stats.rx_bytes)} · TX {formatBytes(stats.tx_bytes)}
                        </p>
                      )}
                    </button>
                    <div className="space-x-2">
                      <button
                        onClick={() => apiClient.connectVpn(p.id).then(refresh)}
                        className="px-3 py-1 text-xs bg-green-800/50 rounded hover:bg-green-700/50"
                      >
                        Connect
                      </button>
                      <button
                        onClick={() => apiClient.disconnectVpn(p.id).then(refresh)}
                        className="px-3 py-1 text-xs bg-slate-700 rounded hover:bg-slate-600"
                      >
                        Disconnect
                      </button>
                    </div>
                  </div>
                </li>
              );
            })}
          </ul>
        </div>
      </div>

      <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4 max-w-lg space-y-4">
        <h3 className="font-medium">Handshake Proxy</h3>
        <p className="text-xs text-sentinel-muted">
          SOCKS5 / HTTP(S) proxy for WireGuard handshake obfuscation on the selected profile.
        </p>
        {!selectedProfile ? (
          <p className="text-sentinel-muted text-sm">Select a profile above to configure handshake proxy.</p>
        ) : (
          <>
            <p className="text-sm">
              Profile: <span className="font-medium">{selectedProfile.name}</span>
            </p>
            {proxyError && (
              <div className="p-2 bg-red-900/30 border border-red-700 rounded text-xs">{proxyError}</div>
            )}
            {proxyMessage && (
              <div className="p-2 bg-green-900/30 border border-green-700 rounded text-xs">{proxyMessage}</div>
            )}
            <label className="flex items-center gap-3 text-sm">
              <input
                type="checkbox"
                checked={handshakeProxy.enabled}
                onChange={(e) =>
                  setHandshakeProxy({ ...handshakeProxy, enabled: e.target.checked })
                }
              />
              Enable handshake proxy
            </label>
            <div className="grid grid-cols-2 gap-3">
              <div>
                <label className="block text-xs text-sentinel-muted mb-1">Host</label>
                <input
                  value={handshakeProxy.host}
                  onChange={(e) =>
                    setHandshakeProxy({ ...handshakeProxy, host: e.target.value })
                  }
                  placeholder="proxy.example.com"
                  className="w-full px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
                />
              </div>
              <div>
                <label className="block text-xs text-sentinel-muted mb-1">Port</label>
                <input
                  type="number"
                  min={1}
                  max={65535}
                  value={handshakeProxy.port}
                  onChange={(e) =>
                    setHandshakeProxy({ ...handshakeProxy, port: Number(e.target.value) })
                  }
                  className="w-full px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
                />
              </div>
              <div>
                <label className="block text-xs text-sentinel-muted mb-1">Username</label>
                <input
                  value={handshakeProxy.username ?? ""}
                  onChange={(e) =>
                    setHandshakeProxy({
                      ...handshakeProxy,
                      username: e.target.value || null,
                    })
                  }
                  className="w-full px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
                />
              </div>
              <div>
                <label className="block text-xs text-sentinel-muted mb-1">Password</label>
                <input
                  type="password"
                  value={handshakeProxy.password ?? ""}
                  onChange={(e) =>
                    setHandshakeProxy({
                      ...handshakeProxy,
                      password: e.target.value || null,
                    })
                  }
                  className="w-full px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
                />
              </div>
            </div>
            <button
              onClick={saveHandshakeProxy}
              disabled={proxySaving}
              className="px-4 py-2 bg-sentinel-accent rounded text-sm hover:bg-blue-600 disabled:opacity-50"
            >
              {proxySaving ? "Saving..." : "Save handshake proxy"}
            </button>
          </>
        )}
      </div>
    </div>
  );
}
