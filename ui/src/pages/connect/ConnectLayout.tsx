import { Outlet } from "react-router-dom";
import { PageHeader } from "../../components/ui/PageHeader";
import { TabBar } from "../../components/ui/TabBar";

const providerTabs = [
  { to: "/connect", label: "Overview", end: true },
  { to: "/connect/vpn", label: "VPN" },
  { to: "/connect/tailscale", label: "Tailscale" },
  { to: "/connect/transport", label: "Transport" },
];

export function ConnectLayout() {
  return (
    <div className="space-y-4">
      <PageHeader
        title="Connect"
        description="Manage exit routes and transport providers"
      />
      <TabBar tabs={providerTabs} />
      <Outlet />
    </div>
  );
}
