import { useEffect, useState } from "react";
import {
  apiClient,
  type SplitTemplateModeSettings,
  type SplitTunnelTemplate,
  type TemplateMode,
  type TrafficRoute,
} from "../api/client";
import { routeSummary } from "../lib/routeLabels";

const TEMPLATE_MODES: { id: TemplateMode; label: string; hint: string }[] = [
  { id: "disabled", label: "Disabled", hint: "Templates are not applied" },
  { id: "merge", label: "Merge", hint: "Template rules supplement policy rules" },
  { id: "override", label: "Override", hint: "Template replaces policy for matching traffic" },
];

function emptyTemplate(): SplitTunnelTemplate {
  const now = new Date().toISOString();
  return {
    id: crypto.randomUUID(),
    name: "",
    description: "",
    default_route: { type: "direct" },
    app_rules: [],
    domain_rules: [],
    enabled: true,
    created_at: now,
    updated_at: now,
  };
}

export function SplitTunnelTemplates() {
  const [templates, setTemplates] = useState<SplitTunnelTemplate[]>([]);
  const [modeSettings, setModeSettings] = useState<SplitTemplateModeSettings | null>(null);
  const [draft, setDraft] = useState<SplitTunnelTemplate | null>(null);
  const [cloneName, setCloneName] = useState("");
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  const refresh = async () => {
    setLoading(true);
    setError(null);
    try {
      const [list, mode] = await Promise.all([
        apiClient.splitTemplates(),
        apiClient.splitTemplateMode(),
      ]);
      setTemplates(list);
      setModeSettings(mode);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load templates");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    refresh();
  }, []);

  const saveMode = async (mode: TemplateMode, activeTemplateId?: string | null) => {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const updated = await apiClient.setSplitTemplateMode({
        mode,
        active_template_id: activeTemplateId ?? modeSettings?.active_template_id ?? null,
      });
      setModeSettings(updated);
      setMessage("Template mode updated");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to update mode");
    } finally {
      setBusy(false);
    }
  };

  const saveTemplate = async () => {
    if (!draft || !draft.name.trim()) return;
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const payload = { ...draft, name: draft.name.trim(), updated_at: new Date().toISOString() };
      const exists = templates.some((t) => t.id === draft.id);
      if (exists) {
        await apiClient.updateSplitTemplate(draft.id, payload);
      } else {
        await apiClient.createSplitTemplate(payload);
      }
      setDraft(null);
      setMessage("Template saved");
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Save failed");
    } finally {
      setBusy(false);
    }
  };

  const removeTemplate = async (id: string) => {
    setBusy(true);
    setError(null);
    try {
      await apiClient.deleteSplitTemplate(id);
      if (draft?.id === id) setDraft(null);
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Delete failed");
    } finally {
      setBusy(false);
    }
  };

  const cloneTemplate = async (id: string) => {
    setBusy(true);
    setError(null);
    try {
      await apiClient.cloneSplitTemplate(id, cloneName.trim() || undefined);
      setCloneName("");
      setMessage("Template cloned");
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Clone failed");
    } finally {
      setBusy(false);
    }
  };

  if (loading && !modeSettings) {
    return <p className="text-sentinel-muted">Loading split tunnel templates...</p>;
  }

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-semibold">Split Tunnel Templates</h2>
      <p className="text-sentinel-muted text-sm">
        Reusable global split-tunnel policies with merge or override application modes.
      </p>

      {error && (
        <div className="p-3 bg-red-900/30 border border-red-700 rounded text-sm">{error}</div>
      )}
      {message && (
        <div className="p-3 bg-green-900/30 border border-green-700 rounded text-sm">{message}</div>
      )}

      <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4 space-y-4 max-w-2xl">
        <h3 className="font-medium">Application mode</h3>
        <div className="grid grid-cols-3 gap-3">
          {TEMPLATE_MODES.map((m) => (
            <button
              key={m.id}
              disabled={busy}
              onClick={() => saveMode(m.id)}
              className={`p-3 rounded border text-left text-sm ${
                modeSettings?.mode === m.id
                  ? "border-sentinel-accent bg-sentinel-accent/10"
                  : "border-slate-600 bg-slate-800/50 hover:bg-slate-800"
              }`}
            >
              <p className="font-medium">{m.label}</p>
              <p className="text-xs text-sentinel-muted mt-1">{m.hint}</p>
            </button>
          ))}
        </div>
        {modeSettings && modeSettings.mode !== "disabled" && (
          <div>
            <label className="block text-sm text-sentinel-muted mb-1">Active template</label>
            <select
              value={modeSettings.active_template_id ?? ""}
              disabled={busy}
              onChange={(e) =>
                saveMode(modeSettings.mode, e.target.value || null)
              }
              className="w-full px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
            >
              <option value="">Select template…</option>
              {templates.filter((t) => t.enabled).map((t) => (
                <option key={t.id} value={t.id}>
                  {t.name}
                </option>
              ))}
            </select>
          </div>
        )}
      </div>

      <div className="grid grid-cols-2 gap-4">
        <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4">
          <div className="flex items-center justify-between mb-3">
            <h3 className="font-medium">{templates.length} template(s)</h3>
            <button
              onClick={() => setDraft(emptyTemplate())}
              className="px-3 py-1 text-xs bg-sentinel-accent rounded hover:bg-blue-600"
            >
              New
            </button>
          </div>
          {templates.length === 0 ? (
            <p className="text-sentinel-muted text-sm">No templates configured</p>
          ) : (
            <ul className="space-y-2 text-sm">
              {templates.map((t) => (
                <li
                  key={t.id}
                  className="p-3 bg-slate-800/50 rounded flex flex-wrap items-center justify-between gap-2"
                >
                  <div>
                    <p className="font-medium">{t.name}</p>
                    <p className="text-xs text-sentinel-muted">
                      Default: {routeSummary(t.default_route)} · {t.app_rules.length} app ·{" "}
                      {t.domain_rules.length} domain
                    </p>
                    {!t.enabled && (
                      <p className="text-xs text-yellow-400 mt-1">Disabled</p>
                    )}
                  </div>
                  <div className="flex gap-2">
                    <button
                      onClick={() => setDraft({ ...t })}
                      className="px-2 py-1 text-xs bg-slate-700 rounded"
                    >
                      Edit
                    </button>
                    <button
                      onClick={() => cloneTemplate(t.id)}
                      disabled={busy}
                      className="px-2 py-1 text-xs border border-slate-600 rounded"
                    >
                      Clone
                    </button>
                    <button
                      onClick={() => removeTemplate(t.id)}
                      disabled={busy}
                      className="px-2 py-1 text-xs bg-red-800/50 rounded"
                    >
                      Delete
                    </button>
                  </div>
                </li>
              ))}
            </ul>
          )}
          <div className="mt-4 pt-3 border-t border-slate-700">
            <label className="block text-xs text-sentinel-muted mb-1">Clone name (optional)</label>
            <input
              value={cloneName}
              onChange={(e) => setCloneName(e.target.value)}
              placeholder="Copy name for next clone"
              className="w-full px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
            />
          </div>
        </div>

        <div className="bg-sentinel-panel rounded-lg border border-slate-700 p-4 space-y-3">
          <h3 className="font-medium">{draft ? "Edit template" : "Select or create a template"}</h3>
          {!draft ? (
            <p className="text-sentinel-muted text-sm">
              Templates define a default route plus optional per-app and per-domain rules.
            </p>
          ) : (
            <>
              <input
                placeholder="Template name"
                value={draft.name}
                onChange={(e) => setDraft({ ...draft, name: e.target.value })}
                className="w-full px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
              />
              <textarea
                placeholder="Description"
                value={draft.description}
                onChange={(e) => setDraft({ ...draft, description: e.target.value })}
                rows={2}
                className="w-full px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
              />
              <label className="flex items-center gap-2 text-sm">
                <input
                  type="checkbox"
                  checked={draft.enabled}
                  onChange={(e) => setDraft({ ...draft, enabled: e.target.checked })}
                />
                Enabled
              </label>
              <div>
                <label className="block text-xs text-sentinel-muted mb-1">Default route</label>
                <select
                  value={draft.default_route.type}
                  onChange={(e) => {
                    const type = e.target.value as TrafficRoute["type"];
                    if (type === "direct" || type === "blocked") {
                      setDraft({ ...draft, default_route: { type } });
                    }
                  }}
                  className="w-full px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm"
                >
                  <option value="direct">Direct</option>
                  <option value="blocked">Blocked</option>
                </select>
              </div>
              <div className="flex gap-2">
                <button
                  onClick={saveTemplate}
                  disabled={busy || !draft.name.trim()}
                  className="px-4 py-2 bg-sentinel-accent rounded text-sm hover:bg-blue-600 disabled:opacity-50"
                >
                  Save
                </button>
                <button
                  onClick={() => setDraft(null)}
                  className="px-4 py-2 bg-slate-700 rounded text-sm"
                >
                  Cancel
                </button>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
