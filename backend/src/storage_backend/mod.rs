mod cleanup;
mod manager;
mod ops;
mod path;
mod s3;

use aws_sdk_s3::Client as S3Client;
#[cfg(test)]
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use crate::runtime_settings::RuntimeSettings;

pub use cleanup::{enqueue_storage_cleanup_jobs, process_pending_storage_cleanup_jobs};

#[derive(Clone)]
pub struct StorageManager {
    pub(super) active_settings: Arc<RwLock<RuntimeSettings>>,
    pub(super) s3_client_cache: Arc<tokio::sync::RwLock<Option<CachedS3Client>>>,
    pub(super) s3_health_cache: Arc<tokio::sync::RwLock<Option<CachedS3HealthProbe>>>,
    #[cfg(test)]
    pub(super) fail_next_apply: Arc<AtomicBool>,
    #[cfg(test)]
    pub(super) s3_health_probe_override: Arc<tokio::sync::RwLock<Option<S3HealthProbeOutcome>>>,
}

#[derive(Clone)]
pub(super) struct CachedS3Client {
    pub(super) signature: String,
    pub(super) bucket: String,
    pub(super) prefix: Option<String>,
    pub(super) client: S3Client,
}

#[derive(Clone)]
pub(super) struct CachedS3HealthProbe {
    pub(super) signature: String,
    pub(super) checked_at: Instant,
    pub(super) outcome: S3HealthProbeOutcome,
}

#[derive(Clone, Debug)]
pub(super) enum S3HealthProbeOutcome {
    Healthy,
    Timeout(String),
    Failure(String),
}
