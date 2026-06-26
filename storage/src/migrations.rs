use shared_types::WireSentinelError;
use sqlx::SqlitePool;
use tracing::warn;

async fn sync_migration_checksum(
    pool: &SqlitePool,
    migrator: &sqlx::migrate::Migrator,
    version: i64,
) -> Result<(), WireSentinelError> {
    let migration = migrator
        .iter()
        .find(|m| m.version == version)
        .ok_or_else(|| WireSentinelError::Config(format!("missing migration {version}")))?;

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
            Ok(()) => return Ok(()),
            Err(sqlx::migrate::MigrateError::VersionMismatch(version)) => {
                if attempt == MAX_REPAIRS {
                    return Err(WireSentinelError::Config(format!(
                        "migration: migration {version} checksum repair loop exceeded"
                    )));
                }
                sync_migration_checksum(pool, &migrator, version).await?;
            }
            Err(e) => return Err(WireSentinelError::Config(format!("migration: {e}"))),
        }
    }
    unreachable!()
}
