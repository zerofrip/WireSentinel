use sha2::{Digest, Sha384};
use sqlx::SqlitePool;
use storage::init_pool;

fn fake_checksum() -> Vec<u8> {
    Sha384::digest(b"stale-migration-sql").to_vec()
}

#[tokio::test]
async fn repairs_stale_migration_checksum_on_disk() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("wiresentinel.db");

    {
        let pool = init_pool(Some(&db_path)).await.expect("first init");
        drop(pool);
    }

    let pool = SqlitePool::connect(&format!("sqlite:{}?mode=rwc", db_path.display()))
        .await
        .expect("connect");
    sqlx::query("UPDATE _sqlx_migrations SET checksum = ? WHERE version = 1")
        .bind(fake_checksum())
        .execute(&pool)
        .await
        .expect("corrupt checksum");
    drop(pool);

    let pool = init_pool(Some(&db_path)).await.expect("repair init");
    let checksum: Vec<u8> =
        sqlx::query_scalar("SELECT checksum FROM _sqlx_migrations WHERE version = 1")
            .fetch_one(&pool)
            .await
            .expect("read checksum");
    assert_ne!(checksum, fake_checksum());
}
