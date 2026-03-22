use sqlx::PgPool;
use tracing::info;

use super::{DatabasePool, migrations::postgres_migrator};

pub async fn run_migrations(pool: &DatabasePool) -> Result<(), sqlx::Error> {
    match pool {
        DatabasePool::Postgres(pool) => run_postgres_migrations(pool).await?,
    }
    Ok(())
}

async fn run_postgres_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    postgres_migrator().run(pool).await?;
    info!("PostgreSQL migrations completed successfully");
    Ok(())
}
