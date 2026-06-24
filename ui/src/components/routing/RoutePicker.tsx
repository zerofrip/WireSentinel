import type { ExitOption } from "../../lib/exitCatalog";
import { routeKey } from "../../lib/exitCatalog";
import { routeLabel } from "../../lib/routeLabels";
import type { TrafficRoute } from "../../api/client";

export function RoutePicker({
  routes,
  catalog,
  onChange,
  disabled,
}: {
  routes: TrafficRoute[];
  catalog: ExitOption[];
  onChange: (routes: TrafficRoute[]) => void;
  disabled?: boolean;
}) {
  const addRoute = (optionId: string) => {
    const option = catalog.find((o) => o.id === optionId);
    if (!option) return;
    if (routes.some((r) => routeKey(r) === routeKey(option.route))) return;
    onChange([...routes, option.route]);
  };

  const removeAt = (index: number) => {
    onChange(routes.filter((_, i) => i !== index));
  };

  const move = (index: number, dir: -1 | 1) => {
    const next = index + dir;
    if (next < 0 || next >= routes.length) return;
    const copy = [...routes];
    [copy[index], copy[next]] = [copy[next], copy[index]];
    onChange(copy);
  };

  return (
    <div className="space-y-2">
      <ul className="space-y-1">
        {routes.length === 0 ? (
          <li className="text-xs text-sentinel-muted">No exit routes — uses policy default</li>
        ) : (
          routes.map((route, index) => (
            <li
              key={`${routeKey(route)}-${index}`}
              className="flex items-center gap-2 text-sm bg-slate-800/50 rounded px-2 py-1"
            >
              <span className="text-xs text-sentinel-muted w-5">{index + 1}.</span>
              <span className="flex-1">{routeLabel(route)}</span>
              <button
                type="button"
                disabled={disabled || index === 0}
                onClick={() => move(index, -1)}
                className="text-xs text-sentinel-muted hover:text-white disabled:opacity-30"
              >
                ↑
              </button>
              <button
                type="button"
                disabled={disabled || index === routes.length - 1}
                onClick={() => move(index, 1)}
                className="text-xs text-sentinel-muted hover:text-white disabled:opacity-30"
              >
                ↓
              </button>
              <button
                type="button"
                disabled={disabled}
                onClick={() => removeAt(index)}
                className="text-xs text-sentinel-danger hover:underline disabled:opacity-30"
              >
                Remove
              </button>
            </li>
          ))
        )}
      </ul>
      <select
        disabled={disabled}
        defaultValue=""
        onChange={(e) => {
          if (e.target.value) addRoute(e.target.value);
          e.target.value = "";
        }}
        className="w-full px-3 py-2 bg-slate-800 border border-slate-600 rounded text-sm disabled:opacity-50"
      >
        <option value="">Add exit route…</option>
        {catalog.map((o) => (
          <option key={o.id} value={o.id}>
            [{o.group}] {o.label}
          </option>
        ))}
      </select>
    </div>
  );
}
