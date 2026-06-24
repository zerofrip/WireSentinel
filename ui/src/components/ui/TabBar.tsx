import { NavLink } from "react-router-dom";

export interface TabItem {
  to: string;
  label: string;
  end?: boolean;
}

export function TabBar({ tabs }: { tabs: TabItem[] }) {
  return (
    <div className="flex flex-wrap gap-2 border-b border-slate-700 pb-2">
      {tabs.map((tab) => (
        <NavLink
          key={tab.to}
          to={tab.to}
          end={tab.end}
          className={({ isActive }) =>
            `px-3 py-1.5 rounded text-sm ${
              isActive
                ? "bg-sentinel-accent text-white"
                : "text-sentinel-muted hover:bg-slate-800"
            }`
          }
        >
          {tab.label}
        </NavLink>
      ))}
    </div>
  );
}
