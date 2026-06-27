use crate::discovery::exe_path_for_pid;
use crate::identity::{file_sha256, icon_path_for_exe, publisher_for_exe};
use chrono::Utc;
use event_bus::EventBus;
use parking_lot::RwLock;
use shared_types::{AppIdentity, AppRecord, ServiceEventInner, WireSentinelError};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use storage::{AppFilter, AppRepository};
use tracing::debug;
use uuid::Uuid;

/// How long a successful PID -> identity resolution stays valid before we
/// re-resolve. Bounds the window in which a recycled PID could be misattributed.
const PID_CACHE_TTL: Duration = Duration::from_secs(60);
/// How long a failed resolution (e.g. protected/system process) is remembered
/// so we stop hammering OpenProcess and flooding the log for the same PID.
const PID_NEG_CACHE_TTL: Duration = Duration::from_secs(30);
/// Only rewrite `last_seen` to the DB when it is at least this stale, so steady
/// traffic from the same app does not produce a DB write per connection.
const LAST_SEEN_REFRESH_SECS: i64 = 300;
/// Soft caps to keep the in-memory caches from growing with dead PIDs/paths.
const PID_CACHE_MAX: usize = 8192;
const SHA_CACHE_MAX: usize = 4096;

struct PidCacheEntry {
    identity: AppIdentity,
    cached_at: Instant,
}

struct ShaCacheEntry {
    mtime: Option<SystemTime>,
    size: u64,
    hash: String,
}

pub struct AppRegistryService {
    apps: Arc<dyn AppRepository>,
    events: EventBus,
    pid_cache: RwLock<HashMap<u32, PidCacheEntry>>,
    pid_neg_cache: RwLock<HashMap<u32, Instant>>,
    sha_cache: RwLock<HashMap<PathBuf, ShaCacheEntry>>,
}

impl AppRegistryService {
    pub fn new(apps: Arc<dyn AppRepository>, events: EventBus) -> Self {
        Self {
            apps,
            events,
            pid_cache: RwLock::new(HashMap::new()),
            pid_neg_cache: RwLock::new(HashMap::new()),
            sha_cache: RwLock::new(HashMap::new()),
        }
    }

    fn pid_cache_get(&self, pid: u32) -> Option<AppIdentity> {
        let cache = self.pid_cache.read();
        let entry = cache.get(&pid)?;
        (entry.cached_at.elapsed() < PID_CACHE_TTL).then(|| entry.identity.clone())
    }

    fn pid_cache_insert(&self, pid: u32, identity: AppIdentity) {
        let mut cache = self.pid_cache.write();
        if cache.len() >= PID_CACHE_MAX {
            cache.retain(|_, e| e.cached_at.elapsed() < PID_CACHE_TTL);
        }
        cache.insert(
            pid,
            PidCacheEntry {
                identity,
                cached_at: Instant::now(),
            },
        );
    }

    fn pid_neg_cached(&self, pid: u32) -> bool {
        self.pid_neg_cache
            .read()
            .get(&pid)
            .is_some_and(|t| t.elapsed() < PID_NEG_CACHE_TTL)
    }

    fn pid_neg_cache_insert(&self, pid: u32) {
        let mut cache = self.pid_neg_cache.write();
        if cache.len() >= PID_CACHE_MAX {
            cache.retain(|_, t| t.elapsed() < PID_NEG_CACHE_TTL);
        }
        cache.insert(pid, Instant::now());
    }

    /// SHA256 of an executable, cached by (path, mtime, size) so repeated lookups
    /// for the same binary avoid re-reading and re-hashing the whole file.
    fn cached_sha256(&self, path: &Path) -> Option<String> {
        let meta = std::fs::metadata(path).ok()?;
        let size = meta.len();
        let mtime = meta.modified().ok();
        if let Some(entry) = self.sha_cache.read().get(path) {
            if entry.size == size && entry.mtime == mtime {
                return Some(entry.hash.clone());
            }
        }
        let hash = file_sha256(path).ok()?;
        let mut cache = self.sha_cache.write();
        if cache.len() >= SHA_CACHE_MAX {
            cache.clear();
        }
        cache.insert(
            path.to_path_buf(),
            ShaCacheEntry {
                mtime,
                size,
                hash: hash.clone(),
            },
        );
        Some(hash)
    }

    pub async fn resolve_or_register(
        &self,
        pid: u32,
    ) -> Result<(AppIdentity, bool), WireSentinelError> {
        if let Some(identity) = self.pid_cache_get(pid) {
            return Ok((identity, false));
        }
        if self.pid_neg_cached(pid) {
            return Err(WireSentinelError::Other(format!(
                "pid {pid} unresolvable (cached)"
            )));
        }

        let exe_path = match exe_path_for_pid(pid) {
            Ok(path) => path,
            Err(e) => {
                self.pid_neg_cache_insert(pid);
                return Err(e);
            }
        };
        let sha256 = self.cached_sha256(&exe_path);

        let existing = if let Some(ref hash) = sha256 {
            self.apps.find_by_sha256(hash).await?
        } else {
            None
        }
        .or(self.apps.find_by_exe_path(&exe_path).await?);

        if let Some(mut record) = existing {
            let updated = record.sha256.is_none() && sha256.is_some();
            if updated {
                record.sha256 = sha256.clone();
                record.publisher = publisher_for_exe(&exe_path);
                record.icon_path = icon_path_for_exe(&exe_path);
            }
            let last_seen_stale =
                (Utc::now() - record.last_seen).num_seconds() >= LAST_SEEN_REFRESH_SECS;
            if updated || last_seen_stale {
                record.touch();
                self.apps.upsert(&record).await?;
            }
            if updated {
                self.events.publish(
                    ServiceEventInner::AppUpdated {
                        app: record.clone(),
                    }
                    .with_timestamp(Utc::now()),
                );
            }
            let identity = AppIdentity::new(pid, record);
            self.pid_cache_insert(pid, identity.clone());
            return Ok((identity, false));
        }

        let mut record = AppRecord::new(exe_path.clone());
        record.sha256 = sha256;
        record.publisher = publisher_for_exe(&exe_path);
        record.icon_path = icon_path_for_exe(&exe_path);
        self.apps.upsert(&record).await?;
        debug!(app = %record.display_name, pid, "app discovered");
        self.events.publish(
            ServiceEventInner::AppDiscovered {
                app: record.clone(),
            }
            .with_timestamp(Utc::now()),
        );
        let identity = AppIdentity::new(pid, record);
        self.pid_cache_insert(pid, identity.clone());
        Ok((identity, true))
    }

    pub async fn list(
        &self,
        search: Option<String>,
        limit: Option<u32>,
    ) -> Result<Vec<AppRecord>, WireSentinelError> {
        self.apps.list(AppFilter { search, limit }).await
    }

    pub async fn get(&self, id: Uuid) -> Result<Option<AppRecord>, WireSentinelError> {
        self.apps.find_by_id(id).await
    }

    pub async fn set_default_route(
        &self,
        app_id: Uuid,
        route: Option<shared_types::TrafficRoute>,
    ) -> Result<Option<AppRecord>, WireSentinelError> {
        let exit_config = route.map(shared_types::AppExitConfig::from_single);
        self.set_exit_config(app_id, exit_config).await
    }

    pub async fn set_exit_config(
        &self,
        app_id: Uuid,
        exit_config: Option<shared_types::AppExitConfig>,
    ) -> Result<Option<AppRecord>, WireSentinelError> {
        let mut record = self
            .apps
            .find_by_id(app_id)
            .await?
            .ok_or_else(|| WireSentinelError::Config("app not found".into()))?;
        record.exit_config = exit_config;
        record.sync_legacy_default_route();
        record.touch();
        self.apps.upsert(&record).await?;
        self.invalidate_app(app_id);
        self.events.publish(
            ServiceEventInner::AppUpdated {
                app: record.clone(),
            }
            .with_timestamp(Utc::now()),
        );
        Ok(Some(record))
    }

    /// Drop cached PID resolutions for an app so routing/config changes take
    /// effect immediately instead of after the PID cache TTL.
    fn invalidate_app(&self, app_id: Uuid) {
        self.pid_cache
            .write()
            .retain(|_, e| e.identity.id() != app_id);
    }
}
