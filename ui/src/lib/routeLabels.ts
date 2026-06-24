import type { TrafficRoute } from "../api/client";

export function routeLabel(route: TrafficRoute | null | undefined): string {
  if (!route) return "Default policy";
  switch (route.type) {
    case "direct":
      return "Direct";
    case "blocked":
      return "Blocked";
    case "wire_guard":
      return `WireGuard (${route.value.slice(0, 8)}…)`;
    case "amnezia_wg":
      return `AmneziaWG (${route.value.slice(0, 8)}…)`;
    case "tailnet":
      return `Tailscale (${route.value.slice(0, 8)}…)`;
    case "tor":
      return `Tor (${route.value.slice(0, 8)}…)`;
    case "proxy":
      return `Proxy (${route.value.slice(0, 8)}…)`;
    case "proxy_chain":
      return `Proxy chain (${route.value.slice(0, 8)}…)`;
    case "chain":
      return `Transport chain (${route.value.slice(0, 8)}…)`;
    case "katzenpost":
      return `Katzenpost (${route.value.slice(0, 8)}…)`;
    case "loopix":
      return `Loopix (${route.value.slice(0, 8)}…)`;
    case "federated_mixnet":
      return `Federated mixnet (${route.value.slice(0, 8)}…)`;
    case "anonymous":
      return `Anonymous (${route.value.type})`;
    default:
      return "Unknown route";
  }
}

export function routeSummary(route: TrafficRoute): string {
  return routeLabel(route);
}
