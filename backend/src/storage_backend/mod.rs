mod manager;
mod ops;
mod path;
mod s3;

use aws_sdk_s3::Client as S3Client;
use std::sync::Arc;

use crate::runtime_settings::RuntimeSettingsService;

#[derive(Clone)]
pub struct StorageManager {
    pub(super) runtime_settings: Arc<RuntimeSettingsService>,
    pub(super) s3_client_cache: Arc<tokio::sync::RwLock<Option<CachedS3Client>>>,
}

#[derive(Clone)]
pub(super) struct CachedS3Client {
    pub(super) signature: String,
    pub(super) bucket: String,
    pub(super) prefix: Option<String>,
    pub(super) client: S3Client,
}
