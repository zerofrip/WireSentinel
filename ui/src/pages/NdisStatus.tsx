import { useEffect, useState } from "react";
import { apiClient, type NdisStatus } from "../api/client";

export function NdisStatusPage() {
  const [status, setStatus] = useState<NdisStatus | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    apiClient
      .ndisStatus()
      .then(setStatus)
      .catch((e) => setError(String(e)));
  }, []);

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-semibold">NDIS Filter</h2>
      {error && <p className="text-sentinel-danger text-sm">{error}</p>}
      {status ? (
        <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4 space-y-2 text-sm">
          <p>
            <span className={status.enabled ? "text-sentinel-success" : "text-sentinel-muted"}>
              {status.enabled ? "Enabled" : "Disabled"}
            </span>
            <span className="text-sentinel-muted ml-2">· {status.lifecycle_state}</span>
          </p>
          <p className="text-sentinel-muted">
            classify {status.classify_count} · redirect {status.redirect_count} · transform{" "}
            {status.transform_count} · cover {status.cover_traffic_count}
          </p>
          <p className="text-sentinel-muted">
            errors {status.error_count} · pending {status.pending_events}
          </p>
        </div>
      ) : (
        <p className="text-sentinel-muted text-sm">NDIS status unavailable</p>
      )}
    </div>
  );
}
