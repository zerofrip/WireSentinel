//! Production logging with rolling files and runtime level control.

use parking_lot::RwLock;
use shared_types::{LogEntry, LogLevel, WireSentinelError};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use storage::data_dir;
use tracing_subscriber::{
    fmt, layer::SubscriberExt, reload, util::SubscriberInitExt, EnvFilter, Registry,
};

const RING_CAPACITY: usize = 500;

static GLOBAL_LOGGING: OnceLock<Arc<LoggingService>> = OnceLock::new();

pub struct LoggingService {
    ring: Arc<RwLock<VecDeque<LogEntry>>>,
    log_dir: PathBuf,
    reload_handle: reload::Handle<EnvFilter, Registry>,
}

pub fn global() -> Option<Arc<LoggingService>> {
    GLOBAL_LOGGING.get().cloned()
}

impl LoggingService {
    pub fn init(default_level: LogLevel, json_enabled: bool, max_files: u32) -> Arc<Self> {
        let ring = Arc::new(RwLock::new(VecDeque::with_capacity(RING_CAPACITY)));
        let log_dir = data_dir().join("logs");
        let _ = std::fs::create_dir_all(&log_dir);

        let filter = EnvFilter::new(default_level.as_filter());
        let (reload_layer, reload_handle) = reload::Layer::new(filter);

        let file_appender = tracing_appender::rolling::daily(&log_dir, "wiresentinel.log");
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
        std::mem::forget(_guard);

        if json_enabled {
            Registry::default()
                .with(reload_layer)
                .with(fmt::layer().json().with_writer(non_blocking))
                .init();
        } else {
            Registry::default()
                .with(reload_layer)
                .with(fmt::layer().with_writer(non_blocking))
                .init();
        }

        prune_old_log_files(&log_dir, max_files);

        let service = Arc::new(Self {
            ring,
            log_dir,
            reload_handle,
        });
        let _ = GLOBAL_LOGGING.set(Arc::clone(&service));
        service
    }

    pub fn set_level(&self, level: LogLevel) -> shared_types::Result<()> {
        self.reload_handle
            .modify(|filter| {
                *filter = EnvFilter::new(level.as_filter());
            })
            .map_err(|e| WireSentinelError::Config(format!("log level reload: {e}")))
    }

    pub fn recent(&self, limit: usize, level_filter: Option<&str>) -> Vec<LogEntry> {
        let ring = self.ring.read();
        ring.iter()
            .rev()
            .filter(|e| {
                level_filter
                    .map(|l| e.level.eq_ignore_ascii_case(l))
                    .unwrap_or(true)
            })
            .take(limit)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    pub fn log_dir(&self) -> &PathBuf {
        &self.log_dir
    }
}

fn prune_old_log_files(log_dir: &Path, max_files: u32) {
    if max_files == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(log_dir) else {
        return;
    };
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = entries
        .filter_map(|e| e.ok())
        .map(|e| {
            let path = e.path();
            let modified = e
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            (modified, path)
        })
        .filter(|(_, p)| p.is_file())
        .collect();
    if files.len() <= max_files as usize {
        return;
    }
    files.sort_by_key(|(t, _)| *t);
    let remove_count = files.len() - max_files as usize;
    for (_, path) in files.into_iter().take(remove_count) {
        let _ = std::fs::remove_file(path);
    }
}

pub fn zip_log_files(log_dir: &PathBuf) -> shared_types::Result<Vec<u8>> {
    use std::io::{Cursor, Write};

    let mut buffer = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buffer);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        if log_dir.exists() {
            for entry in std::fs::read_dir(log_dir).map_err(WireSentinelError::Io)? {
                let entry = entry.map_err(WireSentinelError::Io)?;
                let path = entry.path();
                if path.is_file() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        let data = std::fs::read(&path).map_err(WireSentinelError::Io)?;
                        zip.start_file(format!("logs/{name}"), options)
                            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
                        zip.write_all(&data)
                            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
                    }
                }
            }
        }
        zip.finish()
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;
    }
    Ok(buffer.into_inner())
}
