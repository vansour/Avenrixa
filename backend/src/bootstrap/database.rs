use crate::config::{Config, DatabaseKind};
use crate::db;
use crate::db::DatabasePool;
use sqlx::postgres::PgPoolOptions;
use tracing::info;

pub async fn initialize_database(config: &Config) -> anyhow::Result<DatabasePool> {
    match config.database.kind {
        DatabaseKind::Postgres => {
            info!("Connecting to PostgreSQL database...");
            let pool = PgPoolOptions::new()
                .max_connections(config.database.max_connections)
                .connect(&config.database.url)
                .await?;

            info!("Running PostgreSQL database migrations...");
            let database = DatabasePool::Postgres(pool);
            db::run_migrations(&database).await?;

            Ok(database)
        }
    }
}
