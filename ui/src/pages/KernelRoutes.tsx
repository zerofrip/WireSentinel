import { useEffect, useState } from "react";
import { apiClient, type KernelRouteEntry } from "../api/client";

export function KernelRoutes() {
  const [routes, setRoutes] = useState<KernelRouteEntry[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    apiClient
      .kernelRoutes()
      .then(setRoutes)
      .catch((e) => setError(String(e)));
  }, []);

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-semibold">Kernel Routes</h2>
      {error && <p className="text-sentinel-danger text-sm">{error}</p>}
      {routes.length === 0 ? (
        <p className="text-sentinel-muted text-sm">No kernel route assignments</p>
      ) : (
        <ul className="space-y-2 text-sm">
          {routes.map((r) => (
            <li
              key={r.route_id}
              className="p-3 bg-slate-800/50 rounded flex justify-between gap-4"
            >
              <div>
                <p className="font-medium">{r.label}</p>
                <p className="text-xs text-sentinel-muted">{r.route_kind}</p>
              </div>
              <span className={r.active ? "text-sentinel-success" : "text-sentinel-muted"}>
                {r.active ? "active" : "inactive"}
              </span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
