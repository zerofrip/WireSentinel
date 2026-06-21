import { useState } from "react";
import { useEvents } from "../contexts/ServiceContext";
import { apiClient, type TransportKind } from "../api/client";

const HOP_KINDS: { id: TransportKind; label: string }[] = [
  { id: "direct", label: "Direct" },
  { id: "wire_guard", label: "WireGuard" },
  { id: "sing_box", label: "Sing-box" },
  { id: "xray", label: "Xray" },
  { id: "tls_tunnel", label: "TLS Tunnel" },
  { id: "websocket_tunnel", label: "WebSocket Tunnel" },
  { id: "tor", label: "Tor" },
];

export function TransportChains() {
  const { chains, refresh } = useEvents();
  const [name, setName] = useState("");
  const [kind, setKind] = useState<TransportKind>("direct");

  const create = async () => {
    if (!name) return;
    await apiClient.createChain({ name, hops: [{ kind }], enabled: true });
    setName("");
    await refresh();
  };

  const start = async (id: string) => {
    await apiClient.startChain(id);
    await refresh();
  };

  const stop = async (id: string) => {
    await apiClient.stopChain(id);
    await refresh();
  };

  return (
    <div className="space-y-4">
      <h2 className="text-2xl font-semibold">Transport Chains</h2>
      <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4 grid grid-cols-3 gap-4">
        <input
          placeholder="Chain name"
          value={name}
          onChange={(e) => setName(e.target.value)}
          className="px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
        />
        <select
          value={kind}
          onChange={(e) => setKind(e.target.value as TransportKind)}
          className="px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
        >
          {HOP_KINDS.map((k) => (
            <option key={k.id} value={k.id}>
              {k.label}
            </option>
          ))}
        </select>
        <button
          onClick={create}
          className="px-4 py-2 bg-sentinel-accent rounded text-sm hover:bg-blue-600"
        >
          Create chain
        </button>
      </div>
      <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
        {chains.length === 0 ? (
          <p className="text-sentinel-muted text-sm">No transport chains</p>
        ) : (
          <ul className="space-y-2">
            {chains.map((c) => (
              <li
                key={c.id}
                className="flex items-center justify-between p-3 bg-slate-800/50 rounded text-sm"
              >
                <div>
                  <p className="font-medium">{c.name}</p>
                  <p className="text-xs text-sentinel-muted">
                    {c.hops.length} hop(s) · {c.enabled ? "enabled" : "disabled"}
                  </p>
                </div>
                <div className="space-x-2">
                  <button
                    onClick={() => start(c.id)}
                    className="px-3 py-1 text-xs bg-green-800/50 rounded hover:bg-green-700/50"
                  >
                    Start
                  </button>
                  <button
                    onClick={() => stop(c.id)}
                    className="px-3 py-1 text-xs bg-slate-700 rounded hover:bg-slate-600"
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
