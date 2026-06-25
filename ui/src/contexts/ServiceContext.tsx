import React, { createContext, useContext } from "react";
import { EventProvider, useEvents } from "./EventContext";

interface ServiceContextValue {
  connected: boolean;
  status: ReturnType<typeof useEvents>["status"];
  bandwidth: ReturnType<typeof useEvents>["bandwidth"];
  refresh: () => Promise<void>;
  error: string | null;
  bootstrapping: boolean;
}

const ServiceContext = createContext<ServiceContextValue>({
  connected: false,
  status: null,
  bandwidth: [],
  refresh: async () => {},
  error: null,
  bootstrapping: false,
});

function ServiceBridge({ children }: { children: React.ReactNode }) {
  const events = useEvents();
  return (
    <ServiceContext.Provider
      value={{
        connected: events.connected,
        status: events.status,
        bandwidth: events.bandwidth,
        refresh: events.refresh,
        error: events.error,
        bootstrapping: events.bootstrapping,
      }}
    >
      {children}
    </ServiceContext.Provider>
  );
}

export function ServiceProvider({ children }: { children: React.ReactNode }) {
  return (
    <EventProvider>
      <ServiceBridge>{children}</ServiceBridge>
    </EventProvider>
  );
}

export function useService() {
  return useContext(ServiceContext);
}

export { useEvents } from "./EventContext";
