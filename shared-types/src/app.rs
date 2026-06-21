use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::TrafficRoute;

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
    /// Per-app default route override (optional).
    pub default_route: Option<TrafficRoute>,
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
        }
    }

    pub fn touch(&mut self) {
        self.last_seen = Utc::now();
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
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub connection_count: u32,
}

impl From<AppRecord> for AppSummary {
    fn from(r: AppRecord) -> Self {
        Self {
            id: r.app_id,
            pid: None,
            display_name: r.display_name,
            exe_path: r.exe_path,
            publisher: r.publisher,
            sha256: r.sha256,
            default_route: r.default_route,
            bytes_in: 0,
            bytes_out: 0,
            connection_count: 0,
        }
    }
}