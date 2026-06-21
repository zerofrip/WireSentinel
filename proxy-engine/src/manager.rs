use crate::backend::{ProxyBackend, ProxyHealth, ProxyState, ProxyStatus};
use crate::context::ProxyListenPort;
use crate::socks5::create_backend;
use parking_lot::RwLock;
use shared_types::{ProxyChain, ProxyChainHopKind, ProxyProfile, Result, WireSentinelError};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::warn;
use uuid::Uuid;

pub struct ProxyManager {
    listen_ports: Arc<ProxyListenPort>,
    active: RwLock<HashMap<Uuid, Arc<dyn ProxyBackend>>>,
    active_chains: RwLock<HashMap<Uuid, Uuid>>,
}

impl ProxyManager {
    pub fn new(listen_ports: Arc<ProxyListenPort>) -> Self {
        Self {
            listen_ports,
            active: RwLock::new(HashMap::new()),
            active_chains: RwLock::new(HashMap::new()),
        }
    }

    pub fn listen_ports(&self) -> Arc<ProxyListenPort> {
        Arc::clone(&self.listen_ports)
    }

    pub async fn connect(&self, profile: &ProxyProfile) -> Result<u16> {
        if let Some(existing) = self.active.read().get(&profile.id) {
            if existing.status().state == ProxyState::Connected {
                if let Some(port) = existing.status().listen_port {
                    return Ok(port);
                }
            }
        }

        let backend = create_backend(profile.clone());
        let port = backend.connect().await?;
        self.listen_ports.set(profile.id, port);
        self.active.write().insert(profile.id, backend);
        Ok(port)
    }

    pub async fn disconnect(&self, profile_id: Uuid) -> Result<()> {
        let backend = self.active.write().remove(&profile_id);
        if let Some(backend) = backend {
            backend.disconnect().await?;
        }
        self.listen_ports.remove(profile_id);
        Ok(())
    }

    pub async fn health_check(&self, profile_id: Uuid) -> Result<ProxyHealth> {
        let backend = self
            .active
            .read()
            .get(&profile_id)
            .cloned()
            .ok_or_else(|| WireSentinelError::Proxy(format!("proxy {profile_id} not active")))?;
        Ok(backend.health_check().await)
    }

    pub async fn measure_latency(&self, profile: &ProxyProfile) -> Result<u64> {
        let backend = create_backend(profile.clone());
        backend.measure_latency().await
    }

    pub fn status(&self, profile_id: Uuid) -> Option<ProxyStatus> {
        self.active.read().get(&profile_id).map(|b| b.status())
    }

    pub fn socks_port_for(&self, id: Uuid) -> Option<u16> {
        self.listen_ports.get(id)
    }

    pub async fn connect_with_failover(&self, profiles: &[ProxyProfile]) -> Result<(Uuid, u16)> {
        let mut last_err = None;
        for profile in profiles {
            if !profile.enabled {
                continue;
            }
            match self.connect(profile).await {
                Ok(port) => return Ok((profile.id, port)),
                Err(e) => {
                    warn!(profile = %profile.id, err = %e, "proxy failover hop failed");
                    last_err = Some(e);
                }
            }
        }
        Err(last_err.unwrap_or_else(|| {
            WireSentinelError::Proxy("no enabled proxy profiles for failover".into())
        }))
    }

    pub async fn start_chain_with_profiles(
        &self,
        chain_id: Uuid,
        profiles: &[ProxyProfile],
    ) -> Result<u16> {
        let (exit_id, port) = self.connect_with_failover(profiles).await?;
        self.active_chains.write().insert(chain_id, exit_id);
        self.listen_ports.set(chain_id, port);
        Ok(port)
    }

    pub async fn start_chain(
        &self,
        chain: &ProxyChain,
        resolve_profile: impl Fn(Uuid) -> Option<ProxyProfile>,
    ) -> Result<u16> {
        let mut ordered = chain.hops.clone();
        ordered.sort_by_key(|h| h.order);
        let profiles: Vec<ProxyProfile> = ordered
            .iter()
            .filter_map(|hop| match hop.kind {
                ProxyChainHopKind::Socks5
                | ProxyChainHopKind::Http
                | ProxyChainHopKind::Https => resolve_profile(hop.profile_id),
                ProxyChainHopKind::Tor | ProxyChainHopKind::TlsTunnel => None,
            })
            .collect();

        let (exit_id, port) = self.connect_with_failover(&profiles).await?;
        self.active_chains.write().insert(chain.id, exit_id);
        self.listen_ports.set(chain.id, port);
        Ok(port)
    }

    pub async fn stop_chain(&self, chain_id: Uuid) -> Result<()> {
        let exit_id = self.active_chains.write().remove(&chain_id);
        self.listen_ports.remove(chain_id);
        if let Some(profile_id) = exit_id {
            self.disconnect(profile_id).await?;
        }
        Ok(())
    }

    pub fn is_chain_active(&self, chain_id: Uuid) -> bool {
        self.active_chains.read().contains_key(&chain_id)
    }
}

impl Default for ProxyManager {
    fn default() -> Self {
        Self::new(Arc::new(ProxyListenPort::new()))
    }
}
