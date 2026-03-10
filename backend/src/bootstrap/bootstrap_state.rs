use std::sync::Arc;
use std::time::Instant;

use crate::bootstrap::store::BootstrapConfigStore;
use crate::config::Config;

#[derive(Clone)]
pub struct BootstrapAppState {
    pub config: Config,
    pub store: Arc<BootstrapConfigStore>,
    pub runtime_error: Option<String>,
    pub started_at: Instant,
}
