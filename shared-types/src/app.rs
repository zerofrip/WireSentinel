use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::TrafficRoute;

/// Action when all ordered exit routes for an app are exhausted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExitOnExhaustion {
    KillSwitch,
    #[default]
    Blocked,
    Direct,
}

/// Ordered per-app exit routes with failover exhaustion policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AppExitConfig {
    pub routes: Vec<TrafficRoute>,
    #[serde(default)]
    pub on_exhaustion: ExitOnExhaustion,
}

impl AppExitConfig {
    pub fn from_single(route: TrafficRoute) -> Self {
        Self {
            routes: vec![route],
            on_exhaustion: ExitOnExhaustion::Blocked,
        }
    }
}

/// Persistent application record in the registry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppRecord {
    pub app_id: Uuid,
    pub display_name: String,
    pub exe_path: PathBuf,
    pub publisher: Option<String>,
    pub sha256: Option<String>,
    pub icon_path: Option<PathBuf>,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    /// Per-app default route override (legacy; kept in sync with `exit_config.routes[0]`).
    pub default_route: Option<TrafficRoute>,
    /// Ordered exit routes with failover policy.
    #[serde(default)]
    pub exit_config: Option<AppExitConfig>,
}

impl AppRecord {
    pub fn new(exe_path: PathBuf) -> Self {
        let display_name = exe_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        let now = Utc::now();
        Self {
            app_id: Uuid::new_v4(),
            display_name,
            exe_path,
            publisher: None,
            sha256: None,
            icon_path: None,
            first_seen: now,
            last_seen: now,
            default_route: None,
            exit_config: None,
        }
    }

    pub fn touch(&mut self) {
        self.last_seen = Utc::now();
    }

    /// Effective exit configuration, migrating legacy `default_route` when needed.
    pub fn effective_exit_config(&self) -> Option<AppExitConfig> {
        if let Some(config) = &self.exit_config {
            if !config.routes.is_empty() {
                return Some(config.clone());
            }
        }
        self.default_route
            .as_ref()
            .map(|r| AppExitConfig::from_single(r.clone()))
    }

    pub fn sync_legacy_default_route(&mut self) {
        self.default_route = self
            .exit_config
            .as_ref()
            .and_then(|c| c.routes.first().cloned());
    }
}

/// Runtime process identity (registry record + live PID).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppIdentity {
    pub record: AppRecord,
    pub pid: u32,
}

impl AppIdentity {
    pub fn new(pid: u32, record: AppRecord) -> Self {
        Self { record, pid }
    }

    pub fn id(&self) -> Uuid {
        self.record.app_id
    }

    pub fn exe_path(&self) -> &PathBuf {
        &self.record.exe_path
    }

    pub fn display_name(&self) -> &str {
        &self.record.display_name
    }
}

/// Summary DTO for API responses.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AppSummary {
    pub id: Uuid,
    pub pid: Option<u32>,
    pub display_name: String,
    #[schema(value_type = String)]
    pub exe_path: PathBuf,
    pub publisher: Option<String>,
    pub sha256: Option<String>,
    pub default_route: Option<TrafficRoute>,
    pub exit_config: Option<AppExitConfig>,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub connection_count: u32,
}

impl From<AppRecord> for AppSummary {
    fn from(r: AppRecord) -> Self {
        let exit_config = r.effective_exit_config();
        Self {
            id: r.app_id,
            pid: None,
            display_name: r.display_name,
            exe_path: r.exe_path,
            publisher: r.publisher,
            sha256: r.sha256,
            default_route: exit_config
                .as_ref()
                .and_then(|c| c.routes.first().cloned())
                .or(r.default_route),
            exit_config,
            bytes_in: 0,
            bytes_out: 0,
            connection_count: 0,
        }
    }
}
