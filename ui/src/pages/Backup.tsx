import { useRef, useState } from "react";
import { apiClient } from "../api/client";

function downloadBlob(blob: Blob, filename: string) {
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

export function Backup() {
  const fileRef = useRef<HTMLInputElement>(null);
  const [importText, setImportText] = useState("");
  const [format, setFormat] = useState<"json" | "encrypted">("json");
  const [busy, setBusy] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const exportJson = async () => {
    setBusy("export-json");
    setError(null);
    setMessage(null);
    try {
      const bundle = await apiClient.backupExport("json");
      const json = JSON.stringify(bundle, null, 2);
      downloadBlob(
        new Blob([json], { type: "application/json" }),
        `wiresentinel-backup-${new Date().toISOString().slice(0, 10)}.json`
      );
      setMessage("Configuration exported as JSON");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Export failed");
    } finally {
      setBusy(null);
    }
  };

  const exportEncrypted = async () => {
    setBusy("export-encrypted");
    setError(null);
    setMessage(null);
    try {
      const blob = (await apiClient.backupExport("encrypted")) as Blob;
      downloadBlob(
        blob,
        `wiresentinel-backup-${new Date().toISOString().slice(0, 10)}.wsbackup`
      );
      setMessage("Configuration exported (encrypted)");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Encrypted export failed");
    } finally {
      setBusy(null);
    }
  };

  const importConfig = async () => {
    if (!importText.trim()) return;
    setBusy("import");
    setError(null);
    setMessage(null);
    try {
      await apiClient.backupImport(importText.trim(), format);
      setMessage("Configuration imported successfully");
      setImportText("");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Import failed");
    } finally {
      setBusy(null);
    }
  };

  const onFileSelected = async (file: File | undefined) => {
    if (!file) return;
    if (file.name.endsWith(".wsbackup")) {
      const buf = await file.arrayBuffer();
      const b64 = btoa(String.fromCharCode(...new Uint8Array(buf)));
      setImportText(b64);
      setFormat("encrypted");
    } else {
      setImportText(await file.text());
      setFormat("json");
    }
  };

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-semibold">Backup & Restore</h2>

      {error && (
        <div className="p-3 bg-red-900/30 border border-red-700 rounded text-sm">{error}</div>
      )}
      {message && (
        <div className="p-3 bg-green-900/30 border border-green-700 rounded text-sm">{message}</div>
      )}

      <div className="grid grid-cols-2 gap-4">
        <section className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
          <h3 className="font-medium mb-3">Export configuration</h3>
          <p className="text-sm text-sentinel-muted mb-4">
            Export VPN profiles, rules, DNS settings, and runtime preferences.
          </p>
          <div className="flex flex-wrap gap-2">
            <button
              onClick={exportJson}
              disabled={busy !== null}
              className="px-4 py-2 bg-sentinel-accent rounded text-sm hover:bg-blue-600 disabled:opacity-50"
            >
              {busy === "export-json" ? "Exporting..." : "Export JSON"}
            </button>
            <button
              onClick={exportEncrypted}
              disabled={busy !== null}
              className="px-4 py-2 bg-slate-700 rounded text-sm hover:bg-slate-600 disabled:opacity-50"
            >
              {busy === "export-encrypted" ? "Exporting..." : "Export encrypted"}
            </button>
          </div>
        </section>

        <section className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
          <h3 className="font-medium mb-3">Import configuration</h3>
          <p className="text-sm text-sentinel-muted mb-4">
            Restore from a JSON backup or encrypted .wsbackup file.
          </p>
          <div className="space-y-3">
            <div className="flex gap-2">
              <label className="px-4 py-2 bg-slate-700 rounded text-sm hover:bg-slate-600 cursor-pointer">
                Choose file
                <input
                  ref={fileRef}
                  type="file"
                  accept=".json,.wsbackup"
                  className="hidden"
                  onChange={(e) => onFileSelected(e.target.files?.[0])}
                />
              </label>
              <select
                value={format}
                onChange={(e) => setFormat(e.target.value as "json" | "encrypted")}
                className="px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
              >
                <option value="json">JSON</option>
                <option value="encrypted">Encrypted</option>
              </select>
            </div>
            <textarea
              value={importText}
              onChange={(e) => setImportText(e.target.value)}
              placeholder="Paste backup JSON or load a file..."
              rows={8}
              className="w-full px-3 py-2 bg-slate-800 border border-slate-600 rounded text-xs font-mono"
            />
            <button
              onClick={importConfig}
              disabled={busy !== null || !importText.trim()}
              className="px-4 py-2 bg-sentinel-accent rounded text-sm hover:bg-blue-600 disabled:opacity-50"
            >
              {busy === "import" ? "Importing..." : "Import configuration"}
            </button>
          </div>
        </section>
      </div>
    </div>
  );
}
