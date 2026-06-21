//! Anonymous route orchestration for Tor, mixnet, and chain transports.

use mixnet_core::MixnetManager;
use mixnet_routing::{AnonymousChainHop, AnonymousRouteEngine};
use shared_types::{
    AnonymousChainHopKind, AnonymousRoute, Result, ServiceEvent, ServiceEventInner, TrafficRoute,
    WireSentinelError,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::anonymity::AnonymityService;
use crate::mixnet::MixnetService;
use crate::mixnet_security::MixnetSecurityPolicy;
use crate::proxy::ProxyService;
use crate::tor::TorService;

pub struct AnonymousRoutingService {
    engine: AnonymousRouteEngine,
    storage: Arc<storage::Storage>,
    tor: Arc<TorService>,
    mixnet: Arc<MixnetService>,
    anonymity: Arc<AnonymityService>,
    proxy: Arc<ProxyService>,
    events: event_bus::EventBus,
}

impl AnonymousRoutingService {
    pub fn new(
        security: Arc<MixnetSecurityPolicy>,
        storage: Arc<storage::Storage>,
        tor: Arc<TorService>,
        mixnet: Arc<MixnetService>,
        anonymity: Arc<AnonymityService>,
        proxy: Arc<ProxyService>,
        events: event_bus::EventBus,
    ) -> Self {
        let engine = AnonymousRouteEngine::new(MixnetManager::new(security.to_core_policy()))
            .with_chain_resolver(Arc::new(|route| resolve_anonymous_route(route)));

        Self {
            engine,
            storage,
            tor,
            mixnet,
            anonymity,
            proxy,
            events,
        }
    }

    pub fn engine(&self) -> &AnonymousRouteEngine {
        &self.engine
    }

    pub async fn ensure_ready(&self, route: &TrafficRoute) -> Result<u16> {
        match route {
            TrafficRoute::Tor(id) => {
                let profile = self.tor.start(*id).await?;
                Ok(profile.socks_port)
            }
            TrafficRoute::Anonymous(anon) => self.ensure_anonymous_ready(anon).await,
            TrafficRoute::Chain(chain_id) => self.ensure_chain_ready(*chain_id).await,
            _ => Err(WireSentinelError::Policy(
                "route does not require anonymous transport".into(),
            )),
        }
    }

    async fn ensure_anonymous_ready(&self, route: &AnonymousRoute) -> Result<u16> {
        let hops = resolve_anonymous_route(route)?;
        self.engine
            .validate_chain(&hops)
            .map_err(|e| WireSentinelError::Policy(format!("invalid anonymous chain: {e}")))?;

        match route {
            AnonymousRoute::Tor(id) => {
                let profile = self.tor.start(*id).await?;
                Ok(profile.socks_port)
            }
            AnonymousRoute::TorBridge(id) => {
                let profile = self.tor.start(*id).await?;
                Ok(profile.socks_port)
            }
            AnonymousRoute::FutureMixnet(id) => self.mixnet.ensure_profile_ready(*id).await,
            AnonymousRoute::Katzenpost(id) => {
                self.anonymity
                    .ensure_route_ready(&TrafficRoute::Katzenpost(*id))
                    .await
            }
            AnonymousRoute::Loopix(id) => {
                self.anonymity
                    .ensure_route_ready(&TrafficRoute::Loopix(*id))
                    .await
            }
            AnonymousRoute::FederatedMixnet { profile_id, .. } => {
                self.anonymity
                    .ensure_route_ready(&TrafficRoute::FederatedMixnet(*profile_id))
                    .await
            }
            AnonymousRoute::MultiHop(ids) => {
                if ids.len() >= 2 {
                    let _ = self.tor.start(ids[0]).await?;
                    self.mixnet.ensure_profile_ready(ids[1]).await
                } else if let Some(id) = ids.first() {
                    self.mixnet.ensure_profile_ready(*id).await
                } else {
                    Err(WireSentinelError::Other(
                        "multi-hop anonymous route has no profile ids".into(),
                    ))
                }
            }
        }
    }

    async fn ensure_chain_ready(&self, chain_id: Uuid) -> Result<u16> {
        let chain = self
            .storage
            .anonymous_chains
            .get(chain_id)
            .await?
            .ok_or_else(|| {
                WireSentinelError::Other(format!("anonymous chain {chain_id} not found"))
            })?;

        if !chain.enabled {
            return Err(WireSentinelError::Policy(format!(
                "anonymous chain {chain_id} is disabled"
            )));
        }

        let mut last_port = None;
        for hop in chain.hops.iter() {
            last_port = Some(self.ensure_hop_ready(hop).await?);
        }

        self.events.publish(ServiceEvent::now(
            ServiceEventInner::AnonymousChainStarted {
                chain_id,
                name: chain.name.clone(),
            },
        ));

        last_port.ok_or_else(|| WireSentinelError::Other("anonymous chain has no hops".into()))
    }

    async fn ensure_hop_ready(&self, hop: &shared_types::AnonymousChainHop) -> Result<u16> {
        match hop.kind {
            AnonymousChainHopKind::Tor => {
                let profile = self.tor.start(hop.profile_id).await?;
                Ok(profile.socks_port)
            }
            AnonymousChainHopKind::Mixnet => self.mixnet.ensure_profile_ready(hop.profile_id).await,
            AnonymousChainHopKind::Katzenpost => {
                self.anonymity
                    .ensure_route_ready(&TrafficRoute::Katzenpost(hop.profile_id))
                    .await
            }
            AnonymousChainHopKind::Loopix => {
                self.anonymity
                    .ensure_route_ready(&TrafficRoute::Loopix(hop.profile_id))
                    .await
            }
            AnonymousChainHopKind::FederatedMixnet => {
                self.anonymity
                    .ensure_route_ready(&TrafficRoute::FederatedMixnet(hop.profile_id))
                    .await
            }
            AnonymousChainHopKind::Proxy => {
                let proxy_route = TrafficRoute::Proxy(hop.profile_id);
                self.proxy.ensure_route_ready(&proxy_route).await
            }
            AnonymousChainHopKind::Vpn => Err(WireSentinelError::Other(
                "vpn hop in anonymous chain requires active vpn connection".into(),
            )),
            AnonymousChainHopKind::TlsTunnel | AnonymousChainHopKind::WebSocket => {
                Err(WireSentinelError::Other(format!(
                    "anonymous chain hop {:?} not yet supported",
                    hop.kind
                )))
            }
        }
    }
}

fn resolve_anonymous_route(route: &AnonymousRoute) -> Result<Vec<AnonymousChainHop>> {
    match route {
        AnonymousRoute::Tor(_) => Ok(vec![AnonymousChainHop::Tor, AnonymousChainHop::Mixnet]),
        AnonymousRoute::TorBridge(_) => Ok(vec![
            AnonymousChainHop::TorBridge,
            AnonymousChainHop::Mixnet,
        ]),
        AnonymousRoute::FutureMixnet(_) => Ok(vec![AnonymousChainHop::Mixnet]),
        AnonymousRoute::Katzenpost(_) => Ok(vec![AnonymousChainHop::Katzenpost]),
        AnonymousRoute::Loopix(_) => Ok(vec![AnonymousChainHop::Loopix]),
        AnonymousRoute::FederatedMixnet { .. } => Ok(vec![AnonymousChainHop::FederatedMixnet]),
        AnonymousRoute::MultiHop(_) => Ok(vec![AnonymousChainHop::Tor, AnonymousChainHop::Mixnet]),
    }
}
