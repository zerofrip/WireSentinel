import { NavLink, Outlet } from "react-router-dom";
import { useService } from "../contexts/ServiceContext";

const nav = [
  { to: "/", label: "Dashboard" },
  { to: "/apps", label: "Applications" },
  { to: "/traffic", label: "Traffic" },
  { to: "/vpn", label: "VPN Profiles" },
  { to: "/split-templates", label: "Split Templates" },
  { to: "/chains", label: "Chains" },
  { to: "/plugins", label: "Plugins" },
  { to: "/tailscale", label: "Tailscale" },
  { to: "/tor", label: "Tor" },
  { to: "/bridges", label: "Bridges" },
  { to: "/proxies", label: "Proxies" },
  { to: "/rules", label: "Rules" },
  { to: "/dns", label: "DNS" },
  { to: "/privacy", label: "Privacy" },
  { to: "/mixnet", label: "Mixnet" },
  { to: "/kernel", label: "Kernel" },
  { to: "/kernel/packets", label: "Kernel Packets" },
  { to: "/kernel/routes", label: "Kernel Routes" },
  { to: "/kernel/ndis", label: "NDIS" },
  { to: "/anonymous-routes", label: "Anonymous Routes" },
  { to: "/cover-traffic", label: "Cover Traffic" },
  { to: "/privacy-analytics", label: "Privacy Analytics" },
  { to: "/settings", label: "Settings" },
  { to: "/legal", label: "Legal" },
  { to: "/diagnostics", label: "Diagnostics" },
  { to: "/backup", label: "Backup" },
  { to: "/performance", label: "Performance" },
  { to: "/update", label: "Updates" },
];

export function Layout() {
  const { connected, error, status } = useService();

  return (
    <div className="flex h-screen">
      <aside className="w-52 bg-sentinel-panel border-r border-slate-700 flex flex-col">
        <div className="p-4 border-b border-slate-700">
          <h1 className="text-lg font-bold text-sentinel-accent">WireSentinel</h1>
          <p className="text-xs text-sentinel-muted mt-1">
            {connected ? "Service connected" : "Service offline"}
          </p>
        </div>
        <nav className="flex-1 p-2 space-y-1 overflow-y-auto">
          {nav.map(({ to, label }) => (
            <NavLink
              key={to}
              to={to}
              end={to === "/"}
              className={({ isActive }) =>
                `block px-3 py-2 rounded text-sm ${isActive ? "bg-sentinel-accent text-white" : "text-sentinel-muted hover:bg-slate-800"}`
              }
            >
              {label}
            </NavLink>
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
