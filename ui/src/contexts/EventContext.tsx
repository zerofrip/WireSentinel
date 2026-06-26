import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useReducer,
  useRef,
  useState,
} from "react";
import { apiClient, initAuth, connectEvents } from "../api/client";
import {
  initialEventState,
  reduceEvent,
  type EventState,
  type ServiceEvent,
} from "../events/types";

type HydrateAction = {
  type: "hydrate";
  status: EventState["status"];
  bandwidth: EventState["bandwidth"];
  apps: EventState["apps"];
  vpnProfiles: EventState["vpnProfiles"];
  rules: EventState["rules"];
  dnsLogs: EventState["dnsLogs"];
  topDomains: EventState["topDomains"];
  filterLists: EventState["filterLists"];
  routeStats: EventState["routeStats"];
  blockedStats: EventState["blockedStats"];
  auditLog: EventState["auditLog"];
  privacySnapshot: EventState["privacySnapshot"];
  transports: EventState["transports"];
  transportStatus: EventState["transportStatus"];
  chains: EventState["chains"];
  dnsProviders: EventState["dnsProviders"];
  leakIncidents: EventState["leakIncidents"];
  performanceSnapshots: EventState["performanceSnapshots"];
  securityAudit: EventState["securityAudit"];
};

type EventAction = ServiceEvent | HydrateAction;

function eventReducer(state: EventState, action: EventAction): EventState {
  if ("type" in action && action.type === "hydrate") {
    return {
      ...state,
      status: action.status,
      bandwidth: action.bandwidth,
      apps: action.apps,
      vpnProfiles: action.vpnProfiles,
      rules: action.rules,
      dnsLogs: action.dnsLogs,
      topDomains: action.topDomains,
      filterLists: action.filterLists,
      routeStats: action.routeStats,
      blockedStats: action.blockedStats,
      auditLog: action.auditLog,
      privacySnapshot: action.privacySnapshot,
      transports: action.transports,
      transportStatus: action.transportStatus,
      chains: action.chains,
      dnsProviders: action.dnsProviders,
      leakIncidents: action.leakIncidents,
      performanceSnapshots: action.performanceSnapshots,
      securityAudit: action.securityAudit,
    };
  }
  return reduceEvent(state, action as ServiceEvent);
}

interface EventContextValue extends EventState {
  connected: boolean;
  error: string | null;
  bootstrapping: boolean;
  refresh: () => Promise<void>;
}

const EventContext = createContext<EventContextValue>({
  ...initialEventState,
  connected: false,
  error: null,
  bootstrapping: false,
  refresh: async () => {},
});

export function EventProvider({ children }: { children: React.ReactNode }) {
  const [state, dispatch] = useReducer(eventReducer, initialEventState);
  const [connected, setConnected] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [bootstrapping, setBootstrapping] = useState(true);
  const wsRef = useRef<WebSocket | null>(null);

  const refresh = useCallback(async () => {
    const retryDelaysMs = [0, 1000, 2000, 4000];
    let lastError: unknown = null;

    setBootstrapping(true);
    for (const delay of retryDelaysMs) {
      if (delay > 0) {
        await new Promise((resolve) => setTimeout(resolve, delay));
      }
      try {
        await initAuth();
        const [
          status,
          bandwidth,
          apps,
          vpnProfiles,
          rules,
          dnsLogs,
          topDomains,
          filterLists,
          routeStats,
          blockedStats,
          auditLog,
          privacySnapshot,
          transports,
          transportStatus,
          chains,
          dnsProviders,
          leakIncidents,
          performance,
        ] = await Promise.all([
          apiClient.status(),
          apiClient.traffic(100),
          apiClient.apps(),
          apiClient.vpnList(),
          apiClient.rules(),
          apiClient.dnsLogs({ limit: 50 }),
          apiClient.topDomains(10).catch(() => []),
          apiClient.filterLists().catch(() => []),
          apiClient.routeStatistics({ limit: 50 }).catch(() => []),
          apiClient.blockedStatistics(20).catch(() => []),
          apiClient.auditLog({ limit: 50 }).catch(() => []),
          apiClient.privacy().catch(() => null),
          apiClient.transports().catch(() => []),
          apiClient.transportStatus().catch(() => []),
          apiClient.chains().catch(() => []),
          apiClient.dnsProviders().catch(() => []),
          apiClient.leaks(50).catch(() => []),
          apiClient.performance(20).catch(() => ({ latest: null, snapshots: [] })),
        ]);
        dispatch({
          type: "hydrate",
          status,
          bandwidth,
          apps,
          vpnProfiles,
          rules,
          dnsLogs,
          topDomains,
          filterLists,
          routeStats,
          blockedStats,
          auditLog,
          privacySnapshot,
          transports,
          transportStatus,
          chains,
          dnsProviders,
          leakIncidents,
          performanceSnapshots: performance.snapshots,
          securityAudit: (await apiClient.securityAudit().catch(() => [])).map((f) => ({
            action: f.title,
            detail: `${f.severity}: ${f.category}`,
            timestamp: f.created_at,
          })),
        });
        setConnected(true);
        setError(null);
        setBootstrapping(false);
        return;
      } catch (e) {
        lastError = e;
        setConnected(false);
      }
    }

    setBootstrapping(false);
    setError(lastError instanceof Error ? lastError.message : "Service unavailable");
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  useEffect(() => {
    if (!connected) return;
    let closed = false;
    void (async () => {
      try {
        const ws = await connectEvents((raw) => {
          dispatch(raw as ServiceEvent);
        });
        if (closed) {
          ws.close();
          return;
        }
        wsRef.current = ws;
        ws.onclose = () => {
          if (!closed) setConnected(false);
        };
      } catch (e) {
        setError(e instanceof Error ? e.message : "WebSocket failed");
      }
    })();
    return () => {
      closed = true;
      wsRef.current?.close();
      wsRef.current = null;
    };
  }, [connected]);

  return (
    <EventContext.Provider value={{ ...state, connected, error, bootstrapping, refresh }}>
      {children}
    </EventContext.Provider>
  );
}

export function useEvents() {
  return useContext(EventContext);
}
