import { useEffect, useState } from "react";
import { apiClient, type CoverTrafficSettings } from "../api/client";

export function CoverTraffic() {
  const [settings, setSettings] = useState<CoverTrafficSettings[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    apiClient
      .coverTraffic()
      .then(setSettings)
      .catch((e) => setError(String(e)));
  }, []);

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-semibold">Cover Traffic</h2>
      {error && <p className="text-sentinel-danger text-sm">{error}</p>}

      <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
        <h3 className="font-medium mb-3">{settings.length} profile(s)</h3>
        {settings.length === 0 ? (
          <p className="text-sentinel-muted text-sm">No cover traffic settings configured</p>
        ) : (
          <ul className="space-y-2 text-sm">
            {settings.map((s) => (
              <li
                key={s.id}
                className="p-3 bg-slate-800/50 rounded flex flex-wrap items-center justify-between gap-2"
              >
                <div>
                  <p className="font-medium">{s.profile}</p>
                  <p className="text-xs text-sentinel-muted">
                    {s.rate_bps != null ? `${s.rate_bps} bps` : "rate not set"}
                  </p>
                </div>
                <span className={s.enabled ? "text-sentinel-success" : "text-sentinel-muted"}>
                  {s.enabled ? "enabled" : "disabled"}
                </span>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
