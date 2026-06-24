import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { apiClient, type TailscaleStatus, type TorStatus } from "../../api/client";
import { Panel } from "../../components/ui/Panel";
import { useEvents } from "../../contexts/ServiceContext";

export function ConnectOverview() {
  const { vpnProfiles, chains, transportStatus } = useEvents();
  const [tailnet, setTailnet] = useState<TailscaleStatus | null>(null);
  const [tor, setTor] = useState<TorStatus | null>(null);

  useEffect(() => {
    apiClient.tailnetStatus().then(setTailnet).catch(() => setTailnet(null));
    apiClient.torStatus().then(setTor).catch(() => setTor(null));
  }, []);

  const activeVpn = vpnProfiles.filter(
    (e) => typeof e.status === "object" && e.status.status === "connected",
  ).length;

  return (
    <div className="grid grid-cols-2 gap-4 max-w-3xl">
      <Panel>
        <p className="text-sentinel-muted text-sm">VPN profiles</p>
        <p className="text-lg font-semibold">{activeVpn} connected</p>
        <Link to="/connect/vpn" className="text-xs text-sentinel-accent hover:underline">
          Manage VPN
        </Link>
      </Panel>
      <Panel>
        <p className="text-sentinel-muted text-sm">Tailscale</p>
        <p className="text-lg font-semibold">
          {tailnet?.connected ? "Connected" : "Disconnected"}
        </p>
        <Link to="/connect/tailscale" className="text-xs text-sentinel-accent hover:underline">
          Manage Tailscale
        </Link>
      </Panel>
      <Panel>
        <p className="text-sentinel-muted text-sm">Transport chains</p>
        <p className="text-lg font-semibold">{chains.length} configured</p>
        <Link to="/connect/transport/chains" className="text-xs text-sentinel-accent hover:underline">
          Manage chains
        </Link>
      </Panel>
      <Panel>
        <p className="text-sentinel-muted text-sm">Tor</p>
        <p className="text-lg font-semibold">{tor?.running ? "Running" : "Stopped"}</p>
        <Link to="/connect/transport/tor" className="text-xs text-sentinel-accent hover:underline">
          Manage Tor
        </Link>
      </Panel>
      {transportStatus && transportStatus.length > 0 && (
        <Panel className="col-span-2">
          <p className="text-xs text-sentinel-muted">
            Active transports: {transportStatus.filter((t) => t.state === "running").length}
          </p>
        </Panel>
      )}
    </div>
  );
}
