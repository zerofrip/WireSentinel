use chrono::Utc;
use dpi_transforms::{
    LegacyTransformModule, SingBoxModule, TransformConfig, TransformKind, TransformRegistry,
    XrayModule,
};
use shared_types::{TransportProfile, TransportProfileKind, TransportState};
use std::sync::Arc;
use transport_engine::{
    build_singbox_config, build_xray_config, ProcessManager, SingBoxOutboundSpec, SingBoxProtocol,
    SingBoxTransport, TransportBackend, TransportConfigStore, TransportContext, XrayOutboundSpec,
    XrayProtocol, XrayTransport,
};
use uuid::Uuid;

#[tokio::test]
async fn singbox_config_generation_and_start() {
    let spec = SingBoxOutboundSpec {
        protocol: SingBoxProtocol::Vless,
        server: "proxy.example.com".into(),
        server_port: 443,
        uuid: Some(Uuid::new_v4().to_string()),
        password: None,
        method: None,
        flow: Some("xtls-rprx-vision".into()),
        tls: true,
        sni: Some("proxy.example.com".into()),
        network: None,
        ws_path: None,
        ws_host: None,
    };

    let cfg = build_singbox_config(1080, &spec, None);
    assert_eq!(cfg["inbounds"][0]["type"], "mixed");

    let id = Uuid::new_v4();
    let store = Arc::new(TransportConfigStore::with_dir(
        std::env::temp_dir().join(format!("ws-singbox-{id}")),
    ));
    let path = store.write_json(id, &cfg).unwrap();
    assert!(path.exists());

    let pm = Arc::new(ProcessManager::new());
    let transport = SingBoxTransport::new(Arc::clone(&pm), Arc::clone(&store));

    let ctx = TransportContext {
        id,
        name: "test-singbox".into(),
        vpn_profile: None,
        transport_profile: Some(TransportProfile {
            id,
            name: "singbox-test".into(),
            transport_kind: TransportProfileKind::SingBox,
            config_json: Some(serde_json::to_string(&spec).unwrap()),
            config_path: None,
            binary_path: None,
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }),
        config_path: None,
        listen_port: Some(1080),
        upstream_socks: None,
        obfuscation_preset: None,
        tor_spec: None,
    };

    transport.start(&ctx).await.unwrap();
    assert_eq!(transport.status(), TransportState::Running);
    let health = transport.health_check().await;
    assert!(health.healthy);

    transport.stop().await.unwrap();
    assert_eq!(transport.status(), TransportState::Stopped);
}

#[tokio::test]
async fn xray_reality_config_and_transform_registry() {
    let spec = XrayOutboundSpec {
        protocol: XrayProtocol::Reality,
        server: "edge.example.com".into(),
        server_port: 443,
        uuid: Some(Uuid::new_v4().to_string()),
        password: None,
        flow: Some("xtls-rprx-vision".into()),
        sni: Some("www.microsoft.com".into()),
        public_key: Some("test-pk".into()),
        short_id: Some("a1b2".into()),
        network: None,
        ws_path: None,
        ws_host: None,
    };

    let cfg = build_xray_config(1081, &spec, None);
    assert_eq!(cfg["outbounds"][0]["protocol"], "vless");

    let mut registry = TransformRegistry::new();
    registry.register(TransformConfig {
        kind: TransformKind::ExternalModule {
            name: "sing-box".into(),
            config_path: "/tmp/sb.json".into(),
        },
        enabled: true,
    });
    registry.register_module(Box::new(SingBoxModule));
    registry.register_module(Box::new(XrayModule));

    assert_eq!(registry.active().len(), 1);
    let sb = SingBoxModule;
    let xr = XrayModule;
    assert_eq!(sb.name(), "sing-box");
    assert_eq!(xr.name(), "xray-core");
    assert!(!sb.apply("example.com").await);
    assert!(!xr.apply("example.com").await);

    let id = Uuid::new_v4();
    let store = Arc::new(TransportConfigStore::with_dir(
        std::env::temp_dir().join(format!("ws-xray-{id}")),
    ));
    let pm = Arc::new(ProcessManager::new());
    let transport = XrayTransport::new(Arc::clone(&pm), Arc::clone(&store));

    let ctx = TransportContext {
        id,
        name: "test-xray".into(),
        vpn_profile: None,
        transport_profile: Some(TransportProfile {
            id,
            name: "xray-test".into(),
            transport_kind: TransportProfileKind::Xray,
            config_json: Some(serde_json::to_string(&spec).unwrap()),
            config_path: None,
            binary_path: None,
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }),
        config_path: None,
        listen_port: Some(1081),
        upstream_socks: None,
        obfuscation_preset: None,
        tor_spec: None,
    };

    transport.start(&ctx).await.unwrap();
    assert_eq!(transport.status(), TransportState::Running);
    transport.stop().await.unwrap();
}
