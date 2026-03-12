use sqlx::{MySqlPool, PgPool, SqlitePool};
use tracing::info;

use super::{
    DatabasePool,
    migrations::{mysql_migrator, postgres_migrator, sqlite_migrator},
};

pub async fn run_migrations(pool: &DatabasePool) -> Result<(), sqlx::Error> {
    match pool {
        DatabasePool::Postgres(pool) => run_postgres_migrations(pool).await?,
        DatabasePool::MySql(pool) => run_mysql_migrations(pool).await?,
        DatabasePool::Sqlite(pool) => run_sqlite_migrations(pool).await?,
    }
    Ok(())
}

async fn run_mysql_migrations(pool: &MySqlPool) -> Result<(), sqlx::Error> {
    mysql_migrator().run(pool).await?;
    info!("MySQL migrations completed successfully");
    Ok(())
}

async fn run_postgres_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    postgres_migrator().run(pool).await?;
    info!("PostgreSQL migrations completed successfully");
    Ok(())
}

async fn run_sqlite_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlite_migrator().run(pool).await?;
    info!("SQLite migrations completed successfully");
    Ok(())
}
