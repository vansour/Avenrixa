use tracing::Level;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

pub fn init_logging() -> Level {
    let log_level = std::env::var("RUST_LOG")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(Level::INFO);

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env().add_directive(log_level.into()))
        .init();

    log_level
}
