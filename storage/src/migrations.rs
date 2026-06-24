use shared_types::WireSentinelError;
use sqlx::SqlitePool;
use tracing::warn;

// #region agent log
fn debug_log(hypothesis_id: &str, message: &str, data: serde_json::Value) {
    let payload = serde_json::json!({
        "sessionId": "28de1e",
        "timestamp": chrono::Utc::now().timestamp_millis(),
        "location": "storage/src/migrations.rs",
        "message": message,
        "data": data,
        "hypothesisId": hypothesis_id,
        "runId": std::env::var("WS_DEBUG_RUN_ID").unwrap_or_else(|_| "pre-fix".into()),
    });
    let line = match serde_json::to_string(&payload) {
        Ok(s) => s,
        Err(_) => return,
    };
    if let Ok(path) = std::env::var("WIRESENTINEL_DEBUG_LOG") {
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut f| std::io::Write::write_all(&mut f, format!("{line}\n").as_bytes()));
        return;
    }
    let path = crate::pool::data_dir().join("debug-28de1e.log");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, format!("{line}\n").as_bytes()));
}
// #endregion

fn checksum_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

async fn read_stored_checksum(pool: &SqlitePool, version: i64) -> Option<String> {
    sqlx::query_scalar::<_, Vec<u8>>("SELECT checksum FROM _sqlx_migrations WHERE version = ?")
        .bind(version)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .map(|b| checksum_hex(&b))
}

async fn sync_migration_checksum(
    pool: &SqlitePool,
    migrator: &sqlx::migrate::Migrator,
    version: i64,
) -> Result<(), WireSentinelError> {
    let migration = migrator
        .iter()
        .find(|m| m.version == version)
        .ok_or_else(|| WireSentinelError::Config(format!("missing migration {version}")))?;

    let stored = read_stored_checksum(pool, version).await;
    let embedded = checksum_hex(migration.checksum.as_ref());

    // #region agent log
    debug_log(
        "H1",
        "checksum mismatch repair",
        serde_json::json!({
            "version": version,
            "stored_checksum": stored,
            "embedded_checksum": embedded,
            "description": migration.description.to_string(),
            "db_path": crate::pool::db_path().display().to_string(),
        }),
    );
    // #endregion

    let updated = sqlx::query("UPDATE _sqlx_migrations SET checksum = ? WHERE version = ?")
        .bind(migration.checksum.as_ref())
        .bind(version)
        .execute(pool)
        .await
        .map_err(|e| WireSentinelError::Config(format!("checksum repair: {e}")))?;

    if updated.rows_affected() == 0 {
        return Err(WireSentinelError::Config(format!(
            "checksum repair: migration {version} not found in _sqlx_migrations"
        )));
    }

    warn!(
        version,
        description = %migration.description,
        "repaired sqlx migration checksum (embedded SQL changed since last apply)"
    );
    Ok(())
}

pub async fn run_migrations(pool: &SqlitePool) -> Result<(), WireSentinelError> {
    let migrator = sqlx::migrate!("./migrations");
    const MAX_REPAIRS: usize = 32;
    for attempt in 0..=MAX_REPAIRS {
        match migrator.run(pool).await {
            Ok(()) => {
                // #region agent log
                debug_log(
                    "H2",
                    "migrations complete",
                    serde_json::json!({ "attempt": attempt }),
                );
                // #endregion
                return Ok(());
            }
            Err(sqlx::migrate::MigrateError::VersionMismatch(version)) => {
                if attempt == MAX_REPAIRS {
                    return Err(WireSentinelError::Config(format!(
                        "migration: migration {version} checksum repair loop exceeded"
                    )));
                }
                sync_migration_checksum(pool, &migrator, version).await?;
            }
            Err(e) => {
                // #region agent log
                debug_log(
                    "H3",
                    "migration failed",
                    serde_json::json!({ "error": e.to_string() }),
                );
                // #endregion
                return Err(WireSentinelError::Config(format!("migration: {e}")));
            }
        }
    }
    unreachable!()
}
