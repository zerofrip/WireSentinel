use chrono::Utc;
use shared_types::{ChainHop, ChainProfile, TransportKind, TransportState};
use std::sync::Arc;
use transport_engine::{
    ChainOrchestrator, TransportBackendFactory, TransportConfigStore, TransportContext,
    ProcessManager,
};
use uuid::Uuid;
use vpn_engine::ScmTunnelDllBackend;

fn test_factory() -> Arc<TransportBackendFactory> {
    let wg = Arc::new(ScmTunnelDllBackend::new()) as Arc<dyn vpn_engine::VpnBackend>;
    let awg = Arc::new(ScmTunnelDllBackend::new()) as Arc<dyn vpn_engine::VpnBackend>;
    let pm = Arc::new(ProcessManager::new());
    let store = Arc::new(TransportConfigStore::with_dir(
        std::env::temp_dir().join("wiresentinel-test-transports"),
    ));
    Arc::new(TransportBackendFactory::new(wg, awg, pm, store))
}

#[tokio::test]
async fn start_stop_direct_chain() {
    let factory = test_factory();
    let orchestrator = ChainOrchestrator::new(factory);

    let chain_id = Uuid::new_v4();
    let chain = ChainProfile {
        id: chain_id,
        name: "direct-only".into(),
        hops: vec![
            ChainHop {
                kind: TransportKind::Direct,
                profile_id: None,
                transport_profile_id: None,
            },
            ChainHop {
                kind: TransportKind::Direct,
                profile_id: None,
                transport_profile_id: None,
            },
        ],
        obfuscation_profile_id: None,
        enabled: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let contexts = vec![
        TransportContext::new(Uuid::new_v4(), "hop-0"),
        TransportContext::new(Uuid::new_v4(), "hop-1"),
    ];

    orchestrator.start_chain(&chain, &contexts).await.unwrap();
    assert!(orchestrator.is_active(chain_id));

    let states = orchestrator.chain_state(chain_id).unwrap();
    assert_eq!(states.len(), 2);
    assert!(states.iter().all(|s| *s == TransportState::Running));

    orchestrator.stop_chain(chain_id).await.unwrap();
    assert!(!orchestrator.is_active(chain_id));
}

#[tokio::test]
async fn disabled_chain_rejected() {
    let factory = test_factory();
    let orchestrator = ChainOrchestrator::new(factory);

    let chain = ChainProfile {
        id: Uuid::new_v4(),
        name: "disabled".into(),
        hops: vec![ChainHop {
            kind: TransportKind::Direct,
            profile_id: None,
            transport_profile_id: None,
        }],
        obfuscation_profile_id: None,
        enabled: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let contexts = vec![TransportContext::new(Uuid::new_v4(), "hop-0")];
    assert!(orchestrator.start_chain(&chain, &contexts).await.is_err());
}
