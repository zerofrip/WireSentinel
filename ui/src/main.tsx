import React from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter, Navigate, Routes, Route } from "react-router-dom";
import { ServiceProvider } from "./contexts/ServiceContext";
import { Layout } from "./components/Layout";
import { Dashboard } from "./pages/Dashboard";
import { Applications } from "./pages/Applications";
import { TrafficExplorer } from "./pages/TrafficExplorer";
import { VpnProfiles } from "./pages/VpnProfiles";
import { RulesEditor } from "./pages/RulesEditor";
import { DnsSettingsPage } from "./pages/DnsSettings";
import { Privacy } from "./pages/Privacy";
import { Settings } from "./pages/Settings";
import { Legal } from "./pages/Legal";
import { Diagnostics } from "./pages/Diagnostics";
import { Backup } from "./pages/Backup";
import { Performance } from "./pages/Performance";
import { Update } from "./pages/Update";
import { Plugins } from "./pages/Plugins";
import { Tailscale } from "./pages/Tailscale";
import { Tor } from "./pages/Tor";
import { Bridges } from "./pages/Bridges";
import { Proxies } from "./pages/Proxies";
import { Mixnet } from "./pages/Mixnet";
import { Kernel } from "./pages/Kernel";
import { KernelPackets } from "./pages/KernelPackets";
import { KernelRoutes } from "./pages/KernelRoutes";
import { NdisStatusPage } from "./pages/NdisStatus";
import { AnonymousRoutes } from "./pages/AnonymousRoutes";
import { CoverTraffic } from "./pages/CoverTraffic";
import { PrivacyAnalytics } from "./pages/PrivacyAnalytics";
import { TransportChains } from "./pages/TransportChains";
import { SplitTunnelTemplates } from "./pages/SplitTunnelTemplates";
import { ConnectLayout } from "./pages/connect/ConnectLayout";
import { ConnectOverview } from "./pages/connect/ConnectOverview";
import { TransportLayout } from "./pages/connect/TransportLayout";
import "./index.css";

function LegacyRedirect({ to }: { to: string }) {
  return <Navigate to={to} replace />;
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ServiceProvider>
      <BrowserRouter>
        <Routes>
          <Route element={<Layout />}>
            <Route index element={<Dashboard />} />

            <Route path="connect" element={<ConnectLayout />}>
              <Route index element={<ConnectOverview />} />
              <Route path="vpn" element={<VpnProfiles />} />
              <Route path="tailscale" element={<Tailscale />} />
              <Route path="transport" element={<TransportLayout />}>
                <Route path="chains" element={<TransportChains />} />
                <Route path="tor" element={<Tor />} />
                <Route path="proxies" element={<Proxies />} />
                <Route path="bridges" element={<Bridges />} />
              </Route>
            </Route>

            <Route path="routing">
              <Route path="apps" element={<Applications />} />
              <Route path="split-templates" element={<SplitTunnelTemplates />} />
              <Route path="rules" element={<RulesEditor />} />
              <Route path="traffic" element={<TrafficExplorer />} />
              <Route path="anonymous" element={<AnonymousRoutes />} />
            </Route>

            <Route path="privacy">
              <Route index element={<Privacy />} />
              <Route path="mixnet" element={<Mixnet />} />
              <Route path="cover-traffic" element={<CoverTraffic />} />
              <Route path="analytics" element={<PrivacyAnalytics />} />
            </Route>

            <Route path="network">
              <Route path="dns" element={<DnsSettingsPage />} />
              <Route path="kernel" element={<Kernel />} />
              <Route path="kernel/packets" element={<KernelPackets />} />
              <Route path="kernel/routes" element={<KernelRoutes />} />
              <Route path="kernel/ndis" element={<NdisStatusPage />} />
            </Route>

            <Route path="advanced">
              <Route path="plugins" element={<Plugins />} />
              <Route path="diagnostics" element={<Diagnostics />} />
              <Route path="performance" element={<Performance />} />
            </Route>

            <Route path="system">
              <Route path="settings" element={<Settings />} />
              <Route path="backup" element={<Backup />} />
              <Route path="update" element={<Update />} />
              <Route path="legal" element={<Legal />} />
            </Route>

            {/* Legacy redirects */}
            <Route path="apps" element={<LegacyRedirect to="/routing/apps" />} />
            <Route path="traffic" element={<LegacyRedirect to="/routing/traffic" />} />
            <Route path="vpn" element={<LegacyRedirect to="/connect/vpn" />} />
            <Route path="split-templates" element={<LegacyRedirect to="/routing/split-templates" />} />
            <Route path="chains" element={<LegacyRedirect to="/connect/transport/chains" />} />
            <Route path="plugins" element={<LegacyRedirect to="/advanced/plugins" />} />
            <Route path="tailscale" element={<LegacyRedirect to="/connect/tailscale" />} />
            <Route path="tor" element={<LegacyRedirect to="/connect/transport/tor" />} />
            <Route path="bridges" element={<LegacyRedirect to="/connect/transport/bridges" />} />
            <Route path="proxies" element={<LegacyRedirect to="/connect/transport/proxies" />} />
            <Route path="rules" element={<LegacyRedirect to="/routing/rules" />} />
            <Route path="dns" element={<LegacyRedirect to="/network/dns" />} />
            <Route path="privacy" element={<LegacyRedirect to="/privacy" />} />
            <Route path="mixnet" element={<LegacyRedirect to="/privacy/mixnet" />} />
            <Route path="kernel" element={<LegacyRedirect to="/network/kernel" />} />
            <Route path="kernel/packets" element={<LegacyRedirect to="/network/kernel/packets" />} />
            <Route path="kernel/routes" element={<LegacyRedirect to="/network/kernel/routes" />} />
            <Route path="kernel/ndis" element={<LegacyRedirect to="/network/kernel/ndis" />} />
            <Route path="anonymous-routes" element={<LegacyRedirect to="/routing/anonymous" />} />
            <Route path="cover-traffic" element={<LegacyRedirect to="/privacy/cover-traffic" />} />
            <Route path="privacy-analytics" element={<LegacyRedirect to="/privacy/analytics" />} />
            <Route path="settings" element={<LegacyRedirect to="/system/settings" />} />
            <Route path="legal" element={<LegacyRedirect to="/system/legal" />} />
            <Route path="diagnostics" element={<LegacyRedirect to="/advanced/diagnostics" />} />
            <Route path="backup" element={<LegacyRedirect to="/system/backup" />} />
            <Route path="performance" element={<LegacyRedirect to="/advanced/performance" />} />
            <Route path="update" element={<LegacyRedirect to="/system/update" />} />
          </Route>
        </Routes>
      </BrowserRouter>
    </ServiceProvider>
  </React.StrictMode>
);
