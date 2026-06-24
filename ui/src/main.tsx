import React from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter, Routes, Route } from "react-router-dom";
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
import "./index.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ServiceProvider>
      <BrowserRouter>
        <Routes>
          <Route element={<Layout />}>
            <Route index element={<Dashboard />} />
            <Route path="apps" element={<Applications />} />
            <Route path="traffic" element={<TrafficExplorer />} />
            <Route path="vpn" element={<VpnProfiles />} />
            <Route path="split-templates" element={<SplitTunnelTemplates />} />
            <Route path="chains" element={<TransportChains />} />
            <Route path="plugins" element={<Plugins />} />
            <Route path="tailscale" element={<Tailscale />} />
            <Route path="tor" element={<Tor />} />
            <Route path="bridges" element={<Bridges />} />
            <Route path="proxies" element={<Proxies />} />
            <Route path="rules" element={<RulesEditor />} />
            <Route path="dns" element={<DnsSettingsPage />} />
            <Route path="privacy" element={<Privacy />} />
            <Route path="mixnet" element={<Mixnet />} />
            <Route path="kernel" element={<Kernel />} />
            <Route path="kernel/packets" element={<KernelPackets />} />
            <Route path="kernel/routes" element={<KernelRoutes />} />
            <Route path="kernel/ndis" element={<NdisStatusPage />} />
            <Route path="anonymous-routes" element={<AnonymousRoutes />} />
            <Route path="cover-traffic" element={<CoverTraffic />} />
            <Route path="privacy-analytics" element={<PrivacyAnalytics />} />
            <Route path="settings" element={<Settings />} />
            <Route path="legal" element={<Legal />} />
            <Route path="diagnostics" element={<Diagnostics />} />
            <Route path="backup" element={<Backup />} />
            <Route path="performance" element={<Performance />} />
            <Route path="update" element={<Update />} />
          </Route>
        </Routes>
      </BrowserRouter>
    </ServiceProvider>
  </React.StrictMode>
);
