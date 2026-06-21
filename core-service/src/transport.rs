//! Transport profile manager backed by transport-engine.

use dpi_transforms::TransformPipeline;
use shared_types::{
    ChainProfile, ObfuscationPreset, Result, TransportProfile, TransportState,
    TransportStatusRecord,
};
use std::sync::Arc;
use storage::Storage;
use transport_engine::{
    ChainOrchestrator, ProcessManager, TransportBackendFactory, TransportConfigStore,
    TransportContext,
};
use uuid::Uuid;

pub struct TransportManager {
    storage: Arc<Storage>,
    #[allow(dead_code)]
    factory: Arc<TransportBackendFactory>,
    chains: Arc<ChainOrchestrator>,
    process_manager: Arc<ProcessManager>,
}

impl TransportManager {
    pub fn new(storage: Arc<Storage>, vpn_factory: &vpn_engine::VpnBackendFactory) -> Self {
        let process_manager = Arc::new(ProcessManager::new());
        let config_store = Arc::new(TransportConfigStore::new());
        let factory = Arc::new(TransportBackendFactory::new(
            vpn_factory.wireguard_backend(),
            vpn_factory.amnezia_backend(),
            Arc::clone(&process_manager),
            config_store,
        ));
        let chains = Arc::new(ChainOrchestrator::new(Arc::clone(&factory)));
        Self {
            storage,
            factory,
            chains,
            process_manager,
        }
    }

    pub fn chain_orchestrator(&self) -> Arc<ChainOrchestrator> {
        Arc::clone(&self.chains)
    }

    pub async fn list_profiles(&self) -> Result<Vec<TransportProfile>> {
        self.storage.transport_profiles.list().await
    }

    pub async fn status(&self) -> Result<Vec<TransportStatusRecord>> {
        let profiles = self.storage.transport_profiles.list().await?;
        Ok(profiles
            .into_iter()
            .map(|p| {
                let running = self.process_manager.is_running(p.id);
                TransportStatusRecord {
                    id: p.id,
                    name: p.name,
                    kind: p.transport_kind,
                    state: if running {
                        TransportState::Running
                    } else {
                        TransportState::Stopped
                    },
                    message: if running {
                        Some("process active".into())
                    } else {
                        None
                    },
                }
            })
            .collect())
    }

    async fn resolve_obfuscation_preset(&self, chain: &ChainProfile) -> Option<ObfuscationPreset> {
        let profile_id = chain.obfuscation_profile_id?;
        let profile = self
            .storage
            .obfuscation_profiles
            .get(profile_id)
            .await
            .ok()??;
        Some(profile.preset)
    }

    pub async fn start_chain(&self, chain: &ChainProfile) -> Result<()> {
        let start = std::time::Instant::now();
        let obfuscation_preset = self.resolve_obfuscation_preset(chain).await;
        let pipeline = obfuscation_preset.map(TransformPipeline::from_preset);

        let mut contexts = Vec::with_capacity(chain.hops.len());
        for (idx, hop) in chain.hops.iter().enumerate() {
            let instance_id = hop
                .transport_profile_id
                .or(hop.profile_id)
                .unwrap_or(chain.id);
            let mut ctx = TransportContext::new(instance_id, format!("{}-hop-{idx}", chain.name));
            ctx.obfuscation_preset = obfuscation_preset;

            if let Some(profile_id) = hop.profile_id {
                ctx.vpn_profile = self.storage.vpn_profiles.get(profile_id).await?;
            }
            if let Some(tp_id) = hop.transport_profile_id {
                ctx.transport_profile = self.storage.transport_profiles.get(tp_id).await?;
            }

            if let (Some(tp), Some(ref pipe)) = (&ctx.transport_profile, &pipeline) {
                if let Some(json) = &tp.config_json {
                    if let Ok(mut config) = serde_json::from_str::<serde_json::Value>(json) {
                        pipe.apply_to_config(&mut config);
                    }
                }
            }

            contexts.push(ctx);
        }

        self.chains.start_chain(chain, &contexts).await?;
        crate::benchmark::record_transport_startup_ms(start.elapsed().as_secs_f64() * 1000.0);
        Ok(())
    }

    pub async fn stop_chain(&self, chain_id: Uuid) -> Result<()> {
        self.chains.stop_chain(chain_id).await
    }

    pub fn is_chain_active(&self, chain_id: Uuid) -> bool {
        self.chains.is_active(chain_id)
    }
}
