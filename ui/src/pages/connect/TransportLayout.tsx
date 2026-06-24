import { Outlet } from "react-router-dom";
import { Navigate, useLocation } from "react-router-dom";
import { TabBar } from "../../components/ui/TabBar";

const transportTabs = [
  { to: "/connect/transport/chains", label: "Chains" },
  { to: "/connect/transport/tor", label: "Tor" },
  { to: "/connect/transport/proxies", label: "Proxies" },
  { to: "/connect/transport/bridges", label: "Bridges" },
];

export function TransportLayout() {
  const location = useLocation();
  if (location.pathname === "/connect/transport") {
    return <Navigate to="/connect/transport/chains" replace />;
  }

  return (
    <div className="space-y-4">
      <TabBar tabs={transportTabs} />
      <Outlet />
    </div>
  );
}
