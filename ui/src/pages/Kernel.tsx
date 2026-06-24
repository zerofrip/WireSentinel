import { useEffect, useState } from "react";
import { apiClient, type EnforcementSettings, type KernelStatus, type KernelTelemetry } from "../api/client";

export function Kernel() {
  const [status, setStatus] = useState<KernelStatus | null>(null);
  const [telemetry, setTelemetry] = useState<KernelTelemetry | null>(null);
  const [enforcement, setEnforcement] = useState<EnforcementSettings | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    Promise.all([apiClient.kernelStatus(), apiClient.kernelTelemetry(), apiClient.enforcementSettings()])
      .then(([s, t, e]) => {
        setStatus(s);
        setTelemetry(t);
        setEnforcement(e);
        setError(null);
      })
      .catch((e) => setError(String(e)));
  }, []);

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-semibold">
        {enforcement?.enforcement_backend === "custom_kernel" ? "Kernel Guardian" : "Signed enforcement"}
      </h2>
      {error && <p className="text-sentinel-danger text-sm">{error}</p>}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <Stat label="Backend" value={enforcement?.enforcement_backend ?? "—"} />
        <Stat label="Mode" value={status?.guardian_mode ?? "—"} />
        <Stat label="Lifecycle" value={status?.lifecycle_state ?? "—"} />
        <Stat label="Filters" value={String(status?.filter_count ?? 0)} />
        <Stat
          label="Health"
          value={status?.healthy ? "healthy" : "degraded"}
          tone={status?.healthy ? "ok" : "warn"}
        />
      </div>
      {telemetry && (
        <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4 text-sm">
          <p>
            classify {telemetry.classify_count} · route {telemetry.route_count} · packets/s{" "}
            {telemetry.packets_per_sec}
          </p>
          <p className="text-sentinel-muted mt-2">
            latency avg {telemetry.avg_classify_latency_ns} ns · max{" "}
            {telemetry.max_classify_latency_ns} ns
          </p>
        </div>
      )}
    </div>
  );
}

function Stat({
  label,
  value,
  tone = "default",
}: {
  label: string;
  value: string;
  tone?: "default" | "ok" | "warn";
}) {
  const toneClass =
    tone === "ok"
      ? "text-sentinel-success"
      : tone === "warn"
        ? "text-yellow-400"
        : "text-white";
  return (
    <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
      <div className="text-xs text-sentinel-muted">{label}</div>
      <div className={`text-lg font-semibold mt-1 ${toneClass}`}>{value}</div>
    </div>
  );
}
