#[cfg(test)]
mod phase2_tests {
    use chrono::Utc;
    use shared_types::{
        AppIdentity, AppRecord, DNSQueryLog, Direction, FilterListRecord, FilterListType, Protocol,
        TrafficEvent, TrafficRoute, Verdict,
    };
    use std::path::PathBuf;
    use storage::{
        init_pool_in_memory, CorrelationQuery, DnsLogQuery, DnsSortField, SortOrder, Storage,
        TrafficLogQuery, TrafficSortField,
    };
    use uuid::Uuid;

    async fn storage() -> Storage {
        let pool = init_pool_in_memory().await.unwrap();
        Storage::new(pool)
    }

    fn sample_app() -> AppIdentity {
        AppIdentity::new(100, AppRecord::new(PathBuf::from("C:\\app.exe")))
    }

    fn sample_traffic(app: AppIdentity) -> TrafficEvent {
        let mut event = TrafficEvent::new(
            app,
            Direction::Outbound,
            Protocol::Tcp,
            "127.0.0.1:1234".parse().unwrap(),
            "8.8.8.8:443".parse().unwrap(),
            TrafficRoute::Direct,
            Verdict::allow("test"),
        );
        event.process_id = Some(100);
        event.source_ip = Some("127.0.0.1".into());
        event.destination_ip = Some("8.8.8.8".into());
        event.bytes_in = 512;
        event.bytes_out = 256;
        event
    }

    #[tokio::test]
    async fn traffic_logs_list_filter_sort() {
        let s = storage().await;
        let app = sample_app();
        s.traffic_logs
            .insert(&sample_traffic(app.clone()))
            .await
            .unwrap();
        let query = TrafficLogQuery {
            limit: 10,
            offset: 0,
            app_id: Some(app.id()),
            sort: TrafficSortField::Timestamp,
            order: SortOrder::Desc,
        };
        let rows = s.traffic_logs.list(query).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].bytes_in, 512);
    }

    #[tokio::test]
    async fn dns_logs_pagination_and_top_domains() {
        let s = storage().await;
        let log = DNSQueryLog {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            app_id: None,
            pid: None,
            qname: "example.com".into(),
            qtype: "A".into(),
            upstream: "cloudflare".into(),
            blocked: false,
            latency_ms: 10,
            answers: vec!["1.2.3.4".into()],
            response: None,
            correlation_id: None,
        };
        s.dns_logs.insert(&log).await.unwrap();
        let query = DnsLogQuery {
            limit: 5,
            offset: 0,
            qname: Some("example".into()),
            blocked: None,
            sort: DnsSortField::Timestamp,
            order: SortOrder::Desc,
        };
        let rows = s.dns_logs.list(query).await.unwrap();
        assert_eq!(rows.len(), 1);
        let top = s.dns_logs.top_domains(5).await.unwrap();
        assert_eq!(top[0].domain, "example.com");
    }

    #[tokio::test]
    async fn filter_lists_crud() {
        let s = storage().await;
        let id = Uuid::new_v4();
        let record = FilterListRecord {
            id,
            name: "test".into(),
            url: Some("https://example.com/list.txt".into()),
            list_type: FilterListType::Hosts,
            enabled: true,
            update_interval_secs: Some(3600),
            last_updated: None,
            cache_path: None,
        };
        s.filter_lists.insert(&record).await.unwrap();
        let listed = s.filter_lists.list().await.unwrap();
        assert_eq!(listed.len(), 1);
        assert!(s.filter_lists.delete(id).await.unwrap());
    }

    #[tokio::test]
    async fn correlations_record_and_list() {
        let s = storage().await;
        let app_id = Uuid::new_v4();
        s.correlations
            .record_dns(Some(app_id), "example.com", "1.2.3.4")
            .await
            .unwrap();
        let domain = s
            .correlations
            .record_traffic(Some(app_id), "1.2.3.4")
            .await
            .unwrap();
        assert_eq!(domain.as_deref(), Some("example.com"));
        let rows = s
            .correlations
            .list(CorrelationQuery {
                limit: 10,
                app_id: Some(app_id),
                domain: None,
            })
            .await
            .unwrap();
        assert!(!rows.is_empty());
    }
}

#[cfg(test)]
mod phase3_tests {
    use chrono::Utc;
    use shared_types::{
        AuditLogEntry, AuditLogQuery, ChainHop, ChainProfile, DnsProviderRecord, DomainCacheEntry,
        FirewallDecisionRecord, PrivacyScoreComponents, PrivacyScoreSnapshot, RouteStatisticsQuery,
        RouteStatisticsRecord, TrafficRoute, TransportKind, TransportProfile, TransportProfileKind,
        Verdict,
    };
    use storage::{init_pool_in_memory, Storage};
    use uuid::Uuid;

    async fn storage() -> Storage {
        let pool = init_pool_in_memory().await.unwrap();
        Storage::new(pool)
    }

    #[tokio::test]
    async fn route_statistics_upsert_and_list() {
        let s = storage().await;
        let now = Utc::now();
        let record = RouteStatisticsRecord {
            id: Uuid::new_v4(),
            app_id: Some(Uuid::new_v4()),
            profile_id: None,
            domain: Some("example.com".into()),
            route_type: "blocked".into(),
            bytes_in: 100,
            bytes_out: 50,
            connection_count: 1,
            window_start: now,
            window_end: now,
            updated_at: now,
        };
        s.route_statistics.upsert(&record).await.unwrap();
        let rows = s
            .route_statistics
            .list(RouteStatisticsQuery {
                route_type: Some("blocked".into()),
                limit: 10,
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[tokio::test]
    async fn audit_log_insert_and_list() {
        let s = storage().await;
        let entry = AuditLogEntry {
            id: Uuid::new_v4(),
            event_type: "policy_changed".into(),
            actor: Some("api".into()),
            target_type: Some("rule".into()),
            target_id: Some(Uuid::new_v4().to_string()),
            detail_json: None,
            timestamp: Utc::now(),
        };
        s.audit_log.insert(&entry).await.unwrap();
        let rows = s
            .audit_log
            .list(AuditLogQuery {
                event_type: Some("policy_changed".into()),
                limit: 10,
                offset: 0,
            })
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[tokio::test]
    async fn domain_cache_lookup() {
        let s = storage().await;
        let app_id = Uuid::new_v4();
        let now = Utc::now();
        let entry = DomainCacheEntry {
            id: Uuid::new_v4(),
            app_id: Some(app_id),
            domain: "example.com".into(),
            ip_address: "93.184.216.34".into(),
            wildcard: false,
            expires_at: now + chrono::Duration::minutes(5),
            first_seen: now,
            last_seen: now,
            hit_count: 1,
        };
        s.domain_cache.upsert(&entry).await.unwrap();
        let found = s
            .domain_cache
            .lookup_by_ip(Some(app_id), "93.184.216.34")
            .await
            .unwrap();
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn firewall_decisions_insert() {
        let s = storage().await;
        let record = FirewallDecisionRecord {
            id: Uuid::new_v4(),
            app_id: Some(Uuid::new_v4()),
            domain: Some("blocked.test".into()),
            dest_ip: Some("1.2.3.4".into()),
            route: TrafficRoute::Blocked,
            verdict: Verdict::block("test"),
            timestamp: Utc::now(),
        };
        s.firewall_decisions.insert(&record).await.unwrap();
        let rows = s.firewall_decisions.list_recent(10).await.unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[tokio::test]
    async fn transport_profile_crud() {
        let s = storage().await;
        let now = Utc::now();
        let profile = TransportProfile {
            id: Uuid::new_v4(),
            name: "test-singbox".into(),
            transport_kind: TransportProfileKind::SingBox,
            config_json: Some(r#"{"outbounds":[]}"#.into()),
            config_path: None,
            binary_path: None,
            enabled: true,
            created_at: now,
            updated_at: now,
        };
        s.transport_profiles.insert(&profile).await.unwrap();
        let rows = s.transport_profiles.list().await.unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[tokio::test]
    async fn chain_profile_insert() {
        let s = storage().await;
        let now = Utc::now();
        let chain = ChainProfile {
            id: Uuid::new_v4(),
            name: "wg-singbox".into(),
            hops: vec![ChainHop {
                kind: TransportKind::WireGuard,
                profile_id: Some(Uuid::new_v4()),
                transport_profile_id: None,
            }],
            obfuscation_profile_id: None,
            enabled: true,
            created_at: now,
            updated_at: now,
        };
        s.chain_profiles.insert(&chain).await.unwrap();
        let rows = s.chain_profiles.list().await.unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[tokio::test]
    async fn dns_provider_upsert() {
        let s = storage().await;
        let now = Utc::now();
        let provider = DnsProviderRecord {
            id: Uuid::new_v4(),
            name: "test-doh".into(),
            transport: shared_types::DnsTransport::Doh,
            endpoint: "https://dns.example/dns-query".into(),
            priority: 5,
            enabled: true,
            latency_ms: None,
            last_check: None,
            failure_count: 0,
            created_at: now,
            updated_at: now,
        };
        s.dns_providers.upsert(&provider).await.unwrap();
        let rows = s.dns_providers.list().await.unwrap();
        assert!(!rows.is_empty());
    }

    #[tokio::test]
    async fn privacy_snapshot_latest() {
        let s = storage().await;
        let snapshot = PrivacyScoreSnapshot {
            id: Uuid::new_v4(),
            score: 85,
            components: PrivacyScoreComponents::default(),
            timestamp: Utc::now(),
        };
        s.privacy_snapshots.insert(&snapshot).await.unwrap();
        let latest = s.privacy_snapshots.latest().await.unwrap();
        assert!(latest.is_some());
    }
}

#[cfg(test)]
mod phase5_tests {
    use chrono::Utc;
    use shared_types::{
        BackupManifestEntry, EnterprisePolicy, PerformanceSnapshot, RuntimeStateRecord,
    };
    use storage::{init_pool_in_memory, Storage};
    use uuid::Uuid;

    async fn storage() -> Storage {
        let pool = init_pool_in_memory().await.unwrap();
        Storage::new(pool)
    }

    #[tokio::test]
    async fn runtime_state_upsert_and_list_by_scope() {
        let s = storage().await;
        let entity_id = Uuid::new_v4().to_string();
        let record = RuntimeStateRecord {
            id: Uuid::new_v4(),
            scope: "vpn".into(),
            entity_id: entity_id.clone(),
            state_json: r#"{"connected":true}"#.into(),
            updated_at: Utc::now(),
        };
        s.runtime_state.upsert(&record).await.unwrap();

        let updated = RuntimeStateRecord {
            id: Uuid::new_v4(),
            scope: "vpn".into(),
            entity_id: entity_id.clone(),
            state_json: r#"{"connected":false}"#.into(),
            updated_at: Utc::now(),
        };
        s.runtime_state.upsert(&updated).await.unwrap();

        let rows = s.runtime_state.list_by_scope("vpn").await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].entity_id, entity_id);
        assert!(rows[0].state_json.contains("false"));
    }

    #[tokio::test]
    async fn performance_snapshots_insert_latest_and_list() {
        let s = storage().await;
        let now = Utc::now();
        let snapshot = PerformanceSnapshot {
            id: Uuid::new_v4(),
            cpu_percent: 12.5,
            memory_bytes: 64 * 1024 * 1024,
            api_latency_ms: 3.2,
            wfp_latency_ms: 1.1,
            event_throughput: 42.0,
            timestamp: now,
        };
        s.performance.insert(&snapshot).await.unwrap();

        let latest = s
            .performance
            .latest()
            .await
            .unwrap()
            .expect("latest snapshot");
        assert_eq!(latest.id, snapshot.id);
        let recent = s.performance.list_recent(5).await.unwrap();
        assert_eq!(recent.len(), 1);
    }

    #[tokio::test]
    async fn enterprise_policy_upsert_and_get_active() {
        let s = storage().await;
        let policy = EnterprisePolicy {
            id: Uuid::new_v4(),
            version: 2,
            policy_json: serde_json::json!({ "log_level": "warn" }),
            locked_keys: vec!["log_level".into()],
            updated_at: Utc::now(),
        };
        s.enterprise_policy.upsert(&policy).await.unwrap();
        let active = s
            .enterprise_policy
            .get_active()
            .await
            .unwrap()
            .expect("active policy");
        assert_eq!(active.id, policy.id);
        assert_eq!(active.version, 2);
        assert!(active.locked_keys.contains(&"log_level".to_string()));
    }

    #[tokio::test]
    async fn backup_manifest_insert_and_list_recent() {
        let s = storage().await;
        let entry = BackupManifestEntry {
            id: Uuid::new_v4(),
            operation: "export".into(),
            format: "json".into(),
            checksum: "abc123".into(),
            created_at: Utc::now(),
            detail_json: serde_json::json!({ "version": 1 }),
        };
        s.backup_manifest.insert(&entry).await.unwrap();
        let rows = s.backup_manifest.list_recent(10).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].operation, "export");
    }

    #[tokio::test]
    async fn phase6_validation_benchmark_security_repos() {
        use shared_types::{
            BenchmarkSnapshot, SecurityFinding, SecuritySeverity, ValidationCheck, ValidationStatus,
        };

        let s = storage().await;
        let now = Utc::now();

        let check = ValidationCheck {
            id: Uuid::new_v4(),
            check_name: "wfp_availability".into(),
            status: ValidationStatus::Pass,
            message: Some("ok".into()),
            checked_at: now,
        };
        s.validation_results.upsert(&check).await.unwrap();
        let recent = s.validation_results.list_recent(5).await.unwrap();
        assert_eq!(recent.len(), 1);

        let bench = BenchmarkSnapshot {
            id: Uuid::new_v4(),
            wfp_latency_ms: 1.0,
            route_latency_ms: 2.0,
            dns_latency_ms: 3.0,
            transport_startup_ms: 4.0,
            ui_event_throughput: 10.0,
            timestamp: now,
        };
        s.benchmarks.insert(&bench).await.unwrap();
        assert!(s.benchmarks.latest().await.unwrap().is_some());

        let finding = SecurityFinding {
            id: Uuid::new_v4(),
            severity: SecuritySeverity::Info,
            category: "token".into(),
            title: "present".into(),
            detail_json: serde_json::json!({}),
            resolved: false,
            created_at: now,
            resolved_at: None,
        };
        s.security_findings.insert(&finding).await.unwrap();
        let findings = s.security_findings.list(false, 10).await.unwrap();
        assert_eq!(findings.len(), 1);
    }

    #[tokio::test]
    async fn phase7_plugin_tailnet_tor_bridge_repos() {
        use shared_types::{
            BridgeProfile, BridgeType, PluginFormat, PluginManifest, PluginPermission,
            PluginRecord, PluginState, TailnetProfile, TorProfile,
        };

        let s = storage().await;
        let now = Utc::now();

        let manifest = PluginManifest {
            id: Uuid::new_v4(),
            name: "test-filter".into(),
            version: "1.0.0".into(),
            format: PluginFormat::Wasm,
            capabilities: vec![],
            permissions: vec![PluginPermission::FilterDomains],
            min_core_version: "0.1.0".into(),
            path: "/tmp/test.wasm".into(),
            sha256: None,
        };
        let plugin = PluginRecord {
            id: manifest.id,
            manifest: manifest.clone(),
            state: PluginState::Installed,
            error_message: None,
            installed_at: now,
            loaded_at: None,
        };
        s.plugins.upsert(&plugin).await.unwrap();
        assert_eq!(s.plugins.list().await.unwrap().len(), 1);

        let tailnet = TailnetProfile {
            id: Uuid::new_v4(),
            name: "home".into(),
            auth_key: None,
            exit_node: None,
            subnet_router: false,
            magic_dns: true,
            hostname: None,
            tailnet_ip: None,
            connected: false,
            created_at: now,
            updated_at: now,
        };
        s.tailnet_profiles.insert(&tailnet).await.unwrap();
        assert!(s.tailnet_profiles.get(tailnet.id).await.unwrap().is_some());

        let tor = TorProfile {
            id: Uuid::new_v4(),
            name: "default".into(),
            control_port: 9051,
            socks_port: 9050,
            data_dir: "/tmp/tor".into(),
            bridge_ids: vec![],
            enabled: true,
            bootstrap_progress: 0,
            circuit_count: 0,
            created_at: now,
            updated_at: now,
        };
        s.tor_profiles.insert(&tor).await.unwrap();

        let bridge = BridgeProfile {
            id: Uuid::new_v4(),
            name: "obfs4".into(),
            bridge_type: BridgeType::Obfs4,
            config_json: serde_json::json!({}),
            enabled: true,
            created_at: now,
            updated_at: now,
        };
        s.bridge_profiles.insert(&bridge).await.unwrap();
        assert_eq!(s.bridge_profiles.list().await.unwrap().len(), 1);
    }
}
