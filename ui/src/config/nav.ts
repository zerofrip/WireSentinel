export interface NavItem {
  to: string;
  label: string;
  end?: boolean;
}

export interface NavPillar {
  id: string;
  label: string;
  prefix: string;
  items: NavItem[];
}

export const navPillars: NavPillar[] = [
  {
    id: "dashboard",
    label: "Dashboard",
    prefix: "/",
    items: [{ to: "/", label: "Overview", end: true }],
  },
  {
    id: "connect",
    label: "Connect",
    prefix: "/connect",
    items: [
      { to: "/connect", label: "Exit routes", end: true },
      { to: "/connect/vpn", label: "VPN" },
      { to: "/connect/tailscale", label: "Tailscale" },
      { to: "/connect/transport", label: "Transport", end: true },
      { to: "/connect/transport/chains", label: "Chains" },
      { to: "/connect/transport/tor", label: "Tor" },
      { to: "/connect/transport/proxies", label: "Proxies" },
      { to: "/connect/transport/bridges", label: "Bridges" },
    ],
  },
  {
    id: "routing",
    label: "Routing",
    prefix: "/routing",
    items: [
      { to: "/routing/apps", label: "Applications" },
      { to: "/routing/split-templates", label: "Split templates" },
      { to: "/routing/rules", label: "Rules" },
      { to: "/routing/traffic", label: "Traffic" },
      { to: "/routing/anonymous", label: "Anonymous routes" },
    ],
  },
  {
    id: "privacy",
    label: "Privacy",
    prefix: "/privacy",
    items: [
      { to: "/privacy", label: "Privacy score", end: true },
      { to: "/privacy/mixnet", label: "Mixnet" },
      { to: "/privacy/cover-traffic", label: "Cover traffic" },
      { to: "/privacy/analytics", label: "Analytics" },
    ],
  },
  {
    id: "network",
    label: "Network",
    prefix: "/network",
    items: [
      { to: "/network/dns", label: "DNS" },
      { to: "/network/kernel", label: "Kernel", end: true },
      { to: "/network/kernel/packets", label: "Packets" },
      { to: "/network/kernel/routes", label: "Routes" },
      { to: "/network/kernel/ndis", label: "NDIS" },
    ],
  },
  {
    id: "advanced",
    label: "Advanced",
    prefix: "/advanced",
    items: [
      { to: "/advanced/plugins", label: "Plugins" },
      { to: "/advanced/diagnostics", label: "Diagnostics" },
      { to: "/advanced/performance", label: "Performance" },
    ],
  },
  {
    id: "system",
    label: "System",
    prefix: "/system",
    items: [
      { to: "/system/settings", label: "Settings" },
      { to: "/system/backup", label: "Backup" },
      { to: "/system/update", label: "Updates" },
      { to: "/system/legal", label: "Legal" },
    ],
  },
];

export function pillarForPath(pathname: string): string {
  if (pathname === "/") return "dashboard";
  for (const pillar of navPillars) {
    if (pillar.id === "dashboard") continue;
    if (pathname === pillar.prefix || pathname.startsWith(`${pillar.prefix}/`)) {
      return pillar.id;
    }
  }
  return "dashboard";
}
