use std::collections::HashMap;

use crate::db::DatabasePool;
use crate::error::AppError;

use super::super::model::{
    RuntimeSettings, SETTING_LOCAL_STORAGE_PATH, SETTING_MAIL_ENABLED, SETTING_MAIL_FROM_EMAIL,
    SETTING_MAIL_FROM_NAME, SETTING_MAIL_LINK_BASE_URL, SETTING_MAIL_SMTP_HOST,
    SETTING_MAIL_SMTP_PASSWORD, SETTING_MAIL_SMTP_PORT, SETTING_MAIL_SMTP_USER,
    SETTING_S3_ACCESS_KEY, SETTING_S3_BUCKET, SETTING_S3_ENDPOINT, SETTING_S3_FORCE_PATH_STYLE,
    SETTING_S3_PREFIX, SETTING_S3_REGION, SETTING_S3_SECRET_KEY, SETTING_SITE_NAME,
    SETTING_STORAGE_BACKEND, StorageBackend,
};
use super::super::validation::normalize_s3_prefix;

pub(crate) async fn load_from_db(
    pool: &DatabasePool,
    defaults: &RuntimeSettings,
) -> Result<RuntimeSettings, AppError> {
    let rows = match pool {
        DatabasePool::Postgres(pool) => {
            sqlx::query_as::<_, (String, String)>("SELECT key, value FROM settings")
                .fetch_all(pool)
                .await?
        }
        DatabasePool::Sqlite(pool) => {
            sqlx::query_as::<_, (String, String)>("SELECT key, value FROM settings")
                .fetch_all(pool)
                .await?
        }
    };
    let mut kv = HashMap::new();
    for (key, value) in rows {
        kv.insert(key, value);
    }

    let mut settings = defaults.clone();

    if let Some(site_name) = kv
        .get(SETTING_SITE_NAME)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        settings.site_name = site_name;
    }

    if let Some(storage_backend) = kv
        .get(SETTING_STORAGE_BACKEND)
        .and_then(|value| StorageBackend::parse(value.trim()))
    {
        settings.storage_backend = storage_backend;
    }

    if let Some(local_path) = kv
        .get(SETTING_LOCAL_STORAGE_PATH)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        settings.local_storage_path = local_path;
    }

    settings.mail_enabled = kv
        .get(SETTING_MAIL_ENABLED)
        .and_then(|value| value.parse::<bool>().ok())
        .unwrap_or(settings.mail_enabled);
    if let Some(mail_smtp_host) = kv.get(SETTING_MAIL_SMTP_HOST) {
        settings.mail_smtp_host = mail_smtp_host.trim().to_string();
    }
    settings.mail_smtp_port = kv
        .get(SETTING_MAIL_SMTP_PORT)
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(settings.mail_smtp_port);
    settings.mail_smtp_user = kv
        .get(SETTING_MAIL_SMTP_USER)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    settings.mail_smtp_password = kv
        .get(SETTING_MAIL_SMTP_PASSWORD)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if let Some(mail_from_email) = kv.get(SETTING_MAIL_FROM_EMAIL) {
        settings.mail_from_email = mail_from_email.trim().to_string();
    }
    if let Some(mail_from_name) = kv.get(SETTING_MAIL_FROM_NAME) {
        settings.mail_from_name = mail_from_name.trim().to_string();
    }
    if let Some(mail_link_base_url) = kv.get(SETTING_MAIL_LINK_BASE_URL) {
        settings.mail_link_base_url = mail_link_base_url.trim().to_string();
    }

    settings.s3_endpoint = kv
        .get(SETTING_S3_ENDPOINT)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    settings.s3_region = kv
        .get(SETTING_S3_REGION)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    settings.s3_bucket = kv
        .get(SETTING_S3_BUCKET)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    settings.s3_prefix = kv
        .get(SETTING_S3_PREFIX)
        .map(|value| normalize_s3_prefix(value.trim()));
    settings.s3_access_key = kv
        .get(SETTING_S3_ACCESS_KEY)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    settings.s3_secret_key = kv
        .get(SETTING_S3_SECRET_KEY)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    settings.s3_force_path_style = kv
        .get(SETTING_S3_FORCE_PATH_STYLE)
        .and_then(|value| value.parse::<bool>().ok())
        .unwrap_or(true);

    Ok(settings)
}
