import { useEffect, useState } from "react";
import npmLicenses from "../generated/npm-licenses.json";

type NpmLicenseEntry = {
  licenses?: string;
  repository?: string;
  publisher?: string;
  path?: string;
};

function parsePackageKey(key: string): { name: string; version: string } {
  const at = key.lastIndexOf("@");
  if (at <= 0) return { name: key, version: "" };
  return { name: key.slice(0, at), version: key.slice(at + 1) };
}

export function Legal() {
  const [notices, setNotices] = useState<string | null>(null);
  const [noticesError, setNoticesError] = useState<string | null>(null);

  useEffect(() => {
    fetch("/legal/THIRD_PARTY_NOTICES.txt")
      .then((res) => {
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        return res.text();
      })
      .then(setNotices)
      .catch((e) =>
        setNoticesError(e instanceof Error ? e.message : "Failed to load notices"),
      );
  }, []);

  const entries = Object.entries(npmLicenses as Record<string, NpmLicenseEntry>)
    .map(([key, entry]) => ({ key, ...parsePackageKey(key), ...entry }))
    .filter((e) => e.name !== "wire-sentinel-ui");

  return (
    <div className="space-y-6 max-w-3xl">
      <h2 className="text-2xl font-semibold">Legal &amp; licenses</h2>
      <p className="text-sm text-sentinel-muted">
        WireSentinel is licensed under Apache-2.0. Bundled and invoked third-party
        components have separate license obligations.
      </p>

      <section className="bg-sentinel-panel rounded-lg border border-slate-700 p-4 space-y-3">
        <h3 className="font-medium">Third-party notices</h3>
        {noticesError && (
          <p className="text-sm text-red-400">{noticesError}</p>
        )}
        {notices ? (
          <pre className="text-xs whitespace-pre-wrap font-mono bg-slate-900/50 p-3 rounded border border-slate-700 max-h-96 overflow-y-auto">
            {notices}
          </pre>
        ) : (
          !noticesError && (
            <p className="text-sm text-sentinel-muted">Loading notices...</p>
          )
        )}
        <p className="text-xs text-sentinel-muted">
          Full GPL and LGPL texts ship with the Windows installer under{" "}
          <code className="text-sentinel-accent">licenses/</code>.
        </p>
      </section>

      <section className="bg-sentinel-panel rounded-lg border border-slate-700 p-4 space-y-3">
        <h3 className="font-medium">UI dependencies (npm)</h3>
        {entries.length === 0 ? (
          <p className="text-sm text-sentinel-muted">No production dependency licenses listed.</p>
        ) : (
          <ul className="text-sm space-y-2">
            {entries.map((entry) => (
              <li
                key={entry.key}
                className="flex flex-wrap justify-between gap-2 border-b border-slate-700/50 pb-2"
              >
                <span>
                  {entry.name}
                  {entry.version ? `@${entry.version}` : ""}
                </span>
                <span className="text-sentinel-muted font-mono text-xs">
                  {entry.licenses ?? "unknown"}
                </span>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
