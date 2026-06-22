use crate::backend::{TransportBackend, TransportContext};
use crate::factory::TransportBackendFactory;
use parking_lot::RwLock;
use shared_types::{ChainProfile, Result, TransportKind, TransportState, WireSentinelError};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

const BASE_PROXY_PORT: u16 = 10800;

fn is_proxy_hop(kind: TransportKind) -> bool {
    matches!(
        kind,
        TransportKind::SingBox
            | TransportKind::Xray
            | TransportKind::TlsTunnel
            | TransportKind::WebSocketTunnel
            | TransportKind::Tor
            | TransportKind::Mixnet
            | TransportKind::Proxy
    )
}

/// Validates hop profile references before chain start.
pub fn validate_chain(chain: &ChainProfile) -> Result<()> {
    if chain.hops.is_empty() {
        return Err(WireSentinelError::Other(format!(
            "chain '{}' has no hops",
            chain.name
        )));
    }

    for (idx, hop) in chain.hops.iter().enumerate() {
        validate_hop(hop, idx)?;
    }
    Ok(())
}

fn validate_hop(hop: &shared_types::ChainHop, idx: usize) -> Result<()> {
    match hop.kind {
        TransportKind::Direct => Ok(()),
        TransportKind::WireGuard | TransportKind::AmneziaWg => {
            if hop.profile_id.is_none() {
                Err(WireSentinelError::Other(format!(
                    "hop {idx}: {:?} requires profile_id",
                    hop.kind
                )))
            } else {
                Ok(())
            }
        }
        TransportKind::SingBox
        | TransportKind::Xray
        | TransportKind::TlsTunnel
        | TransportKind::WebSocketTunnel => {
            if hop.transport_profile_id.is_none() {
                Err(WireSentinelError::Other(format!(
                    "hop {idx}: {:?} requires transport_profile_id",
                    hop.kind
                )))
            } else {
                Ok(())
            }
        }
        TransportKind::Tor => {
            if hop.transport_profile_id.is_none() && hop.profile_id.is_none() {
                Err(WireSentinelError::Other(format!(
                    "hop {idx}: tor requires transport_profile_id or profile_id"
                )))
            } else {
                Ok(())
            }
        }
        TransportKind::Mixnet => {
            if hop.profile_id.is_none() {
                Err(WireSentinelError::Other(format!(
                    "hop {idx}: mixnet requires profile_id"
                )))
            } else {
                Ok(())
            }
        }
        TransportKind::Proxy => {
            if hop.profile_id.is_none() && hop.transport_profile_id.is_none() {
                Err(WireSentinelError::Other(format!(
                    "hop {idx}: proxy requires profile_id or transport_profile_id"
                )))
            } else {
                Ok(())
            }
        }
    }
}

struct ActiveChain {
    hops: Vec<Arc<dyn TransportBackend>>,
    #[allow(dead_code)]
    contexts: Vec<TransportContext>,
}

/// Orchestrates multi-hop transport chains using the backend factory.
pub struct ChainOrchestrator {
    factory: Arc<TransportBackendFactory>,
    active: RwLock<HashMap<Uuid, ActiveChain>>,
}

impl ChainOrchestrator {
    pub fn new(factory: Arc<TransportBackendFactory>) -> Self {
        Self {
            factory,
            active: RwLock::new(HashMap::new()),
        }
    }

    /// Start all hops in order. Each hop after the first receives the previous hop's local SOCKS address.
    pub async fn start_chain(
        &self,
        chain: &ChainProfile,
        hop_contexts: &[TransportContext],
    ) -> Result<()> {
        validate_chain(chain)?;

        if !chain.enabled {
            return Err(WireSentinelError::Other(format!(
                "chain '{}' is disabled",
                chain.name
            )));
        }

        if hop_contexts.len() != chain.hops.len() {
            return Err(WireSentinelError::Other(format!(
                "expected {} hop contexts, got {}",
                chain.hops.len(),
                hop_contexts.len()
            )));
        }

        if self.active.read().contains_key(&chain.id) {
            return Err(WireSentinelError::Other(format!(
                "chain {} already active",
                chain.id
            )));
        }

        let mut backends: Vec<Arc<dyn TransportBackend>> = Vec::with_capacity(chain.hops.len());
        let mut contexts = Vec::with_capacity(chain.hops.len());
        let mut upstream: Option<String> = None;

        for (idx, (hop, mut ctx)) in chain
            .hops
            .iter()
            .zip(hop_contexts.iter().cloned())
            .enumerate()
        {
            let backend = self.factory.create(hop.kind);

            if is_proxy_hop(hop.kind) {
                let port = BASE_PROXY_PORT + idx as u16;
                ctx.listen_port = Some(port);
                if let Some(up) = &upstream {
                    ctx.upstream_socks = Some(up.clone());
                }
            }

            if let Err(e) = backend.start(&ctx).await {
                warn!(chain = %chain.name, hop = idx, error = %e, "chain hop start failed, rolling back");
                for started in backends.iter().rev() {
                    let _ = started.stop().await;
                }
                return Err(e);
            }

            if is_proxy_hop(hop.kind) {
                let port = ctx.listen_port.unwrap_or(BASE_PROXY_PORT + idx as u16);
                upstream = Some(format!("127.0.0.1:{port}"));
            }

            backends.push(backend);
            contexts.push(ctx);
        }

        self.active.write().insert(
            chain.id,
            ActiveChain {
                hops: backends,
                contexts,
            },
        );

        info!(chain = %chain.name, hops = chain.hops.len(), "chain started");
        Ok(())
    }

    pub async fn stop_chain(&self, chain_id: Uuid) -> Result<()> {
        let chain = self.active.write().remove(&chain_id);
        let Some(chain) = chain else {
            return Ok(());
        };

        for backend in chain.hops.iter().rev() {
            if let Err(e) = backend.stop().await {
                warn!(%chain_id, error = %e, "chain hop stop failed");
            }
        }

        info!(%chain_id, "chain stopped");
        Ok(())
    }

    pub fn chain_state(&self, chain_id: Uuid) -> Option<Vec<TransportState>> {
        self.active
            .read()
            .get(&chain_id)
            .map(|c| c.hops.iter().map(|b| b.status()).collect())
    }

    pub fn is_active(&self, chain_id: Uuid) -> bool {
        self.active.read().contains_key(&chain_id)
    }
}
