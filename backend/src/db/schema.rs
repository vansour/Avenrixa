use sqlx::{MySqlPool, PgPool, SqlitePool, migrate::Migrator};
use tracing::info;

use super::DatabasePool;

static MYSQL_MIGRATOR: Migrator = sqlx::migrate!("./migrations/mysql");
static POSTGRES_MIGRATOR: Migrator = sqlx::migrate!("./migrations/postgresql");
static SQLITE_MIGRATOR: Migrator = sqlx::migrate!("./migrations/sqlite");

pub async fn run_migrations(pool: &DatabasePool) -> Result<(), sqlx::Error> {
    match pool {
        DatabasePool::Postgres(pool) => run_postgres_migrations(pool).await?,
        DatabasePool::MySql(pool) => run_mysql_migrations(pool).await?,
        DatabasePool::Sqlite(pool) => run_sqlite_migrations(pool).await?,
    }
    Ok(())
}

async fn run_mysql_migrations(pool: &MySqlPool) -> Result<(), sqlx::Error> {
    MYSQL_MIGRATOR.run(pool).await?;
    info!("MySQL migrations completed successfully");
    Ok(())
}

async fn run_postgres_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    POSTGRES_MIGRATOR.run(pool).await?;
    info!("PostgreSQL migrations completed successfully");
    Ok(())
}

async fn run_sqlite_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    SQLITE_MIGRATOR.run(pool).await?;
    info!("SQLite migrations completed successfully");
    Ok(())
}
