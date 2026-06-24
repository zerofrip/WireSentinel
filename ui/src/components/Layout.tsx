import { useState } from "react";
import { NavLink, Outlet, useLocation } from "react-router-dom";
import { navPillars, pillarForPath } from "../config/nav";
import { useService } from "../contexts/ServiceContext";

export function Layout() {
  const { connected, error, status } = useService();
  const location = useLocation();
  const activePillar = pillarForPath(location.pathname);
  const [expanded, setExpanded] = useState<Record<string, boolean>>({});

  const isExpanded = (pillarId: string) =>
    expanded[pillarId] ?? pillarId === activePillar;

  const toggle = (pillarId: string) => {
    setExpanded((prev) => ({ ...prev, [pillarId]: !isExpanded(pillarId) }));
  };

  return (
    <div className="flex h-screen">
      <aside className="w-56 bg-sentinel-panel border-r border-slate-700 flex flex-col">
        <div className="p-4 border-b border-slate-700">
          <h1 className="text-lg font-bold text-sentinel-accent">WireSentinel</h1>
          <p className="text-xs text-sentinel-muted mt-1">
            {connected ? "Service connected" : "Service offline"}
          </p>
        </div>
        <nav className="flex-1 p-2 space-y-1 overflow-y-auto">
          {navPillars.map((pillar) => (
            <div key={pillar.id}>
              <button
                type="button"
                onClick={() => toggle(pillar.id)}
                className={`w-full text-left px-3 py-2 rounded text-xs font-semibold uppercase tracking-wide ${
                  activePillar === pillar.id
                    ? "text-sentinel-accent"
                    : "text-sentinel-muted hover:text-white"
                }`}
              >
                {pillar.label}
              </button>
              {isExpanded(pillar.id) && (
                <div className="ml-2 space-y-0.5 mb-2">
                  {pillar.items.map((item) => (
                    <NavLink
                      key={item.to}
                      to={item.to}
                      end={item.end}
                      className={({ isActive }) =>
                        `block px-3 py-1.5 rounded text-sm ${
                          isActive
                            ? "bg-sentinel-accent text-white"
                            : "text-sentinel-muted hover:bg-slate-800"
                        }`
                      }
                    >
                      {item.label}
                    </NavLink>
                  ))}
                </div>
              )}
            </div>
          ))}
        </nav>
        {status && (
          <div className="p-3 text-xs text-sentinel-muted border-t border-slate-700">
            {status.connection_count} connections
            {status.kill_switch_active && (
              <span className="block text-sentinel-danger mt-1">Kill switch ON</span>
            )}
          </div>
        )}
      </aside>
      <main className="flex-1 overflow-auto p-6">
        {error && (
          <div className="mb-4 p-3 bg-red-900/30 border border-red-700 rounded text-sm">
            {error} — start <code className="text-sentinel-accent">wire-sentinel-service --console</code>
          </div>
        )}
        <Outlet />
      </main>
    </div>
  );
}
