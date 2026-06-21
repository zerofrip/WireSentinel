use shared_types::WireSentinelError;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tracing::info;

pub fn data_dir() -> PathBuf {
    if cfg!(windows) {
        std::env::var("PROGRAMDATA")
            .map(|p| PathBuf::from(p).join("WireSentinel"))
            .unwrap_or_else(|_| PathBuf::from(r"C:\ProgramData\WireSentinel"))
    } else {
        PathBuf::from("/tmp/WireSentinel")
    }
}

pub fn db_path() -> PathBuf {
    data_dir().join("wiresentinel.db")
}

pub async fn init_pool(db_path: Option<&Path>) -> Result<SqlitePool, WireSentinelError> {
    let path = db_path.map(PathBuf::from).unwrap_or_else(crate::pool::db_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(WireSentinelError::Io)?;
    }

    let url = format!("sqlite:{}?mode=rwc", path.display());
    let options = SqliteConnectOptions::from_str(&url)
        .map_err(|e| WireSentinelError::Config(format!("sqlite options: {e}")))?
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .map_err(|e| WireSentinelError::Config(format!("sqlite connect: {e}")))?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| WireSentinelError::Config(format!("migration: {e}")))?;

    info!(path = %path.display(), "SQLite database initialized");
    Ok(pool)
}

pub async fn init_pool_in_memory() -> Result<SqlitePool, WireSentinelError> {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .map_err(|e| WireSentinelError::Config(format!("sqlite memory: {e}")))?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| WireSentinelError::Config(format!("migration: {e}")))?;

    Ok(pool)
}
