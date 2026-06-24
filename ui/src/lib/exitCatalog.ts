import type { TrafficRoute } from "../api/client";
import type { VpnListEntry } from "../api/client";

export interface ExitOption {
  id: string;
  label: string;
  route: TrafficRoute;
  group: string;
}

export function buildExitCatalog(input: {
  vpnProfiles?: VpnListEntry[];
  tailnetProfiles?: { id: string; name: string }[];
  torProfiles?: { id: string; name: string }[];
  proxies?: { id: string; name: string }[];
  proxyChains?: { id: string; name: string }[];
  chains?: { id: string; name: string }[];
}): ExitOption[] {
  const options: ExitOption[] = [
    { id: "direct", label: "Direct", route: { type: "direct" }, group: "Built-in" },
    { id: "blocked", label: "Blocked", route: { type: "blocked" }, group: "Built-in" },
  ];

  for (const entry of input.vpnProfiles ?? []) {
    const backend = entry.profile.backend;
    const route: TrafficRoute =
      backend === "amnezia_wg"
        ? { type: "amnezia_wg", value: entry.profile.id }
        : { type: "wire_guard", value: entry.profile.id };
    options.push({
      id: `vpn-${entry.profile.id}`,
      label: entry.profile.name,
      route,
      group: "VPN",
    });
  }

  for (const p of input.tailnetProfiles ?? []) {
    options.push({
      id: `tailnet-${p.id}`,
      label: p.name,
      route: { type: "tailnet", value: p.id },
      group: "Tailscale",
    });
  }

  for (const p of input.chains ?? []) {
    options.push({
      id: `chain-${p.id}`,
      label: p.name,
      route: { type: "chain", value: p.id },
      group: "Transport",
    });
  }

  for (const p of input.torProfiles ?? []) {
    options.push({
      id: `tor-${p.id}`,
      label: p.name,
      route: { type: "tor", value: p.id },
      group: "Tor",
    });
  }

  for (const p of input.proxies ?? []) {
    options.push({
      id: `proxy-${p.id}`,
      label: p.name,
      route: { type: "proxy", value: p.id },
      group: "Proxy",
    });
  }

  for (const p of input.proxyChains ?? []) {
    options.push({
      id: `proxy-chain-${p.id}`,
      label: p.name,
      route: { type: "proxy_chain", value: p.id },
      group: "Proxy chain",
    });
  }

  return options;
}

export function routeKey(route: TrafficRoute): string {
  if (route.type === "direct" || route.type === "blocked") return route.type;
  return `${route.type}:${route.value}`;
}
