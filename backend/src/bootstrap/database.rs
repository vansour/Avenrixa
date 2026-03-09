use crate::config::Config;
use crate::db;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tracing::info;

pub async fn initialize_database(config: &Config) -> anyhow::Result<PgPool> {
    info!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .connect(&config.database.url)
        .await?;

    info!("Initializing database schema...");
    db::init_schema(&pool).await?;
    initialize_admin_account(&pool).await?;

    Ok(pool)
}

async fn initialize_admin_account(pool: &PgPool) -> anyhow::Result<()> {
    let init = db::create_admin_account(pool).await?;
    if init.created {
        info!("Admin account initialized successfully: {}", init.username);
    } else {
        info!("Admin account loaded: {}", init.username);
    }

    Ok(())
}
