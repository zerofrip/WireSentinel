import { useEffect, useState } from "react";
import {
  apiClient,
  type AnonymityPrivacySnapshot,
  type PrivacyAnalyticsSnapshot,
} from "../api/client";

export function PrivacyAnalytics() {
  const [snapshot, setSnapshot] = useState<PrivacyAnalyticsSnapshot | null>(null);
  const [anonymity, setAnonymity] = useState<AnonymityPrivacySnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    Promise.all([apiClient.privacyAnalytics(), apiClient.privacyAnonymity()])
      .then(([analytics, privacyAnonymity]) => {
        setSnapshot(analytics);
        setAnonymity(privacyAnonymity);
      })
      .catch((e) => setError(String(e)));
  }, []);

  const score = snapshot?.anonymity_score ?? anonymity?.anonymity_score;
  const federationPeers =
    anonymity?.federation_peer_count ?? snapshot?.federation_peer_count ?? null;
  const entropyBits = anonymity?.entropy_bits ?? snapshot?.entropy_bits ?? null;
  const activeRoutes = anonymity?.active_route_count ?? snapshot?.active_route_count ?? null;

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-semibold">Privacy Analytics</h2>
      {error && <p className="text-sentinel-danger text-sm">{error}</p>}

      {!snapshot && !anonymity ? (
        <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
          <p className="text-sentinel-muted text-sm">No privacy analytics snapshot available</p>
        </div>
      ) : (
        <>
          <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-6 text-center">
            <p className="text-sentinel-muted text-sm">Anonymity score</p>
            <p
              className={`text-5xl font-bold mt-2 ${
                (score ?? 0) >= 80
                  ? "text-sentinel-success"
                  : (score ?? 0) >= 50
                    ? "text-yellow-400"
                    : "text-sentinel-danger"
              }`}
            >
              {score ?? "—"}
            </p>
            <p className="text-xs text-sentinel-muted mt-2">
              Updated{" "}
              {new Date(snapshot?.timestamp ?? anonymity?.timestamp ?? Date.now()).toLocaleString()}
            </p>
          </div>
          <div className="grid grid-cols-3 gap-4">
            <Metric
              label="Route entropy"
              value={(snapshot?.route_entropy ?? anonymity?.route_entropy ?? 0).toFixed(2)}
            />
            <Metric
              label="Path diversity"
              value={(snapshot?.path_diversity ?? anonymity?.path_diversity ?? 0).toFixed(2)}
            />
            <Metric
              label="Cover traffic"
              value={(
                snapshot?.cover_traffic_effectiveness ??
                anonymity?.cover_traffic_effectiveness ??
                0
              ).toFixed(2)}
            />
          </div>
          {(federationPeers != null || entropyBits != null || activeRoutes != null) && (
            <div className="grid grid-cols-3 gap-4">
              {federationPeers != null && (
                <Metric label="Federation peers" value={String(federationPeers)} />
              )}
              {entropyBits != null && (
                <Metric label="Entropy bits" value={entropyBits.toFixed(1)} />
              )}
              {activeRoutes != null && (
                <Metric label="Active routes" value={String(activeRoutes)} />
              )}
            </div>
          )}
        </>
      )}
    </div>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
      <p className="text-sentinel-muted text-sm">{label}</p>
      <p className="text-2xl font-semibold mt-1">{value}</p>
    </div>
  );
}
