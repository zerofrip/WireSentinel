import { useEffect, useState } from "react";
import { apiClient, type KernelPacketEntry } from "../api/client";

export function KernelPackets() {
  const [packets, setPackets] = useState<KernelPacketEntry[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    apiClient
      .kernelPackets()
      .then(setPackets)
      .catch((e) => setError(String(e)));
  }, []);

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-semibold">Kernel Packets</h2>
      {error && <p className="text-sentinel-danger text-sm">{error}</p>}
      {packets.length === 0 ? (
        <p className="text-sentinel-muted text-sm">No kernel packet flows recorded</p>
      ) : (
        <ul className="space-y-2 text-sm">
          {packets.map((p) => (
            <li
              key={p.flow_id}
              className="p-3 bg-slate-800/50 rounded flex justify-between gap-4"
            >
              <span>{p.flow_id}</span>
              <span className="text-sentinel-muted">
                pid {p.process_id} · proto {p.protocol} · {p.bytes} B · {p.direction}
              </span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
