use shared_types::WireSentinelError;
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous,
};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;
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
    let path = db_path
        .map(PathBuf::from)
        .unwrap_or_else(crate::pool::db_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(WireSentinelError::Io)?;
    }

    let url = format!("sqlite:{}?mode=rwc", path.display());
    let options = SqliteConnectOptions::from_str(&url)
        .map_err(|e| WireSentinelError::Config(format!("sqlite options: {e}")))?
        .create_if_missing(true)
        .foreign_keys(true)
        // WAL lets readers run concurrently with a single writer, avoiding the
        // whole-database lock that exhausted the connection pool under load.
        .journal_mode(SqliteJournalMode::Wal)
        // Wait for the write lock instead of failing immediately with SQLITE_BUSY.
        .busy_timeout(Duration::from_secs(5))
        // NORMAL is safe under WAL and avoids an fsync on every transaction.
        .synchronous(SqliteSynchronous::Normal)
        // Keep the WAL file from growing unbounded between checkpoints.
        .pragma("wal_autocheckpoint", "1000");

    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(10))
        .connect_with(options)
        .await
        .map_err(|e| WireSentinelError::Config(format!("sqlite connect: {e}")))?;

    crate::migrations::run_migrations(&pool).await?;

    info!(path = %path.display(), "SQLite database initialized");
    // #region agent log
    {
        let journal: String = sqlx::query_scalar("PRAGMA journal_mode")
            .fetch_one(&pool)
            .await
            .unwrap_or_else(|_| "unknown".to_string());
        let busy: i64 = sqlx::query_scalar("PRAGMA busy_timeout")
            .fetch_one(&pool)
            .await
            .unwrap_or(-1);
        shared_types::debug_log::emit_kv(
            "storage/src/pool.rs:init_pool",
            "sqlite pool initialized",
            &[
                ("hypothesisId", "DEPLOY_A".to_string()),
                ("max_connections", "10".to_string()),
                ("journal_mode", journal),
                ("busy_timeout_ms", busy.to_string()),
            ],
        );
    }
    // #endregion
    Ok(pool)
}

pub async fn init_pool_in_memory() -> Result<SqlitePool, WireSentinelError> {
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .map_err(|e| WireSentinelError::Config(format!("sqlite memory: {e}")))?;

    crate::migrations::run_migrations(&pool).await?;

    Ok(pool)
}
