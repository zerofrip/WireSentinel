import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { apiClient, type UpdateInfo } from "../api/client";

export function Update() {
  const [info, setInfo] = useState<UpdateInfo | null>(null);
  const [checking, setChecking] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = async () => {
    try {
      setInfo(await apiClient.updateInfo());
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load update info");
    }
  };

  useEffect(() => {
    load();
  }, []);

  const check = async () => {
    setChecking(true);
    setError(null);
    try {
      setInfo(await apiClient.checkUpdate());
    } catch (e) {
      setError(e instanceof Error ? e.message : "Update check failed");
    } finally {
      setChecking(false);
    }
  };

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-semibold">Updates</h2>

      {error && (
        <div className="p-3 bg-red-900/30 border border-red-700 rounded text-sm">{error}</div>
      )}

      {!info ? (
        <p className="text-sentinel-muted">Loading update information...</p>
      ) : (
        <>
          <div className="grid grid-cols-3 gap-4">
            <div className="bg-sentinel-panel rounded-lg p-4 border border-slate-700">
              <p className="text-sentinel-muted text-sm">Current version</p>
              <p className="text-2xl font-bold mt-1">{info.current_version}</p>
            </div>
            <div className="bg-sentinel-panel rounded-lg p-4 border border-slate-700">
              <p className="text-sentinel-muted text-sm">Latest version</p>
              <p className="text-2xl font-bold mt-1">
                {info.latest_version ?? "—"}
              </p>
            </div>
            <div className="bg-sentinel-panel rounded-lg p-4 border border-slate-700">
              <p className="text-sentinel-muted text-sm">Channel</p>
              <p className="text-2xl font-bold mt-1 capitalize">{info.channel}</p>
              <Link to="/system/settings" className="text-xs text-sentinel-accent hover:underline mt-2 inline-block">
                Change channel in Settings →
              </Link>
            </div>
          </div>

          <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4 max-w-lg space-y-4">
            <div className="flex items-center justify-between">
              <div>
                <p className="font-medium">Update status</p>
                <p
                  className={
                    info.update_available
                      ? "text-sentinel-success text-sm mt-1"
                      : "text-sentinel-muted text-sm mt-1"
                  }
                >
                  {info.update_available ? "Update available" : "Up to date"}
                </p>
              </div>
              <button
                onClick={check}
                disabled={checking}
                className="px-4 py-2 bg-sentinel-accent rounded text-sm hover:bg-blue-600 disabled:opacity-50"
              >
                {checking ? "Checking..." : "Check for updates"}
              </button>
            </div>

            {info.update_available && (
              <>
                <div>
                  <p className="text-sm text-sentinel-muted mb-1">Staged rollout</p>
                  <div className="h-2 bg-slate-800 rounded overflow-hidden">
                    <div
                      className="h-full bg-sentinel-accent rounded"
                      style={{ width: `${info.staged_percent}%` }}
                    />
                  </div>
                  <p className="text-xs text-sentinel-muted mt-1">{info.staged_percent}% eligible</p>
                </div>
                {info.download_url && (
                  <a
                    href={info.download_url}
                    target="_blank"
                    rel="noreferrer"
                    className="inline-block text-sm text-sentinel-accent hover:underline"
                  >
                    Download update →
                  </a>
                )}
              </>
            )}
          </div>
        </>
      )}
    </div>
  );
}
