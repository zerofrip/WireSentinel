use crate::{AppExitConfig, AppRecord, ExitOnExhaustion, TrafficRoute};
use std::path::PathBuf;
use uuid::Uuid;

#[test]
fn effective_exit_config_migrates_legacy_default_route() {
    let profile = Uuid::new_v4();
    let mut record = AppRecord::new(PathBuf::from("test.exe"));
    record.default_route = Some(TrafficRoute::WireGuard(profile));
    let config = record.effective_exit_config().expect("config");
    assert_eq!(config.routes.len(), 1);
    assert!(matches!(config.routes[0], TrafficRoute::WireGuard(id) if id == profile));
}

#[test]
fn exit_config_syncs_default_route() {
    let mut record = AppRecord::new(PathBuf::from("test.exe"));
    record.exit_config = Some(AppExitConfig {
        routes: vec![TrafficRoute::Direct, TrafficRoute::Blocked],
        on_exhaustion: ExitOnExhaustion::KillSwitch,
    });
    record.sync_legacy_default_route();
    assert_eq!(record.default_route, Some(TrafficRoute::Direct));
}
