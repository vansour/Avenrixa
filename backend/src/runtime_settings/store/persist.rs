use sqlx::PgPool;

use crate::error::AppError;

use super::super::model::{
    RuntimeSettings, SETTING_LOCAL_STORAGE_PATH, SETTING_S3_ACCESS_KEY, SETTING_S3_BUCKET,
    SETTING_S3_ENDPOINT, SETTING_S3_FORCE_PATH_STYLE, SETTING_S3_PREFIX, SETTING_S3_REGION,
    SETTING_S3_SECRET_KEY, SETTING_SITE_NAME, SETTING_STORAGE_BACKEND,
};

pub(crate) async fn persist_settings(
    pool: &PgPool,
    validated: &RuntimeSettings,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    upsert_setting(&mut tx, SETTING_SITE_NAME, &validated.site_name).await?;
    upsert_setting(
        &mut tx,
        SETTING_STORAGE_BACKEND,
        validated.storage_backend.as_str(),
    )
    .await?;
    upsert_setting(
        &mut tx,
        SETTING_LOCAL_STORAGE_PATH,
        &validated.local_storage_path,
    )
    .await?;
    upsert_setting_opt(
        &mut tx,
        SETTING_S3_ENDPOINT,
        validated.s3_endpoint.as_deref(),
    )
    .await?;
    upsert_setting_opt(&mut tx, SETTING_S3_REGION, validated.s3_region.as_deref()).await?;
    upsert_setting_opt(&mut tx, SETTING_S3_BUCKET, validated.s3_bucket.as_deref()).await?;
    upsert_setting_opt(&mut tx, SETTING_S3_PREFIX, validated.s3_prefix.as_deref()).await?;
    upsert_setting_opt(
        &mut tx,
        SETTING_S3_ACCESS_KEY,
        validated.s3_access_key.as_deref(),
    )
    .await?;
    upsert_setting_opt(
        &mut tx,
        SETTING_S3_SECRET_KEY,
        validated.s3_secret_key.as_deref(),
    )
    .await?;
    upsert_setting(
        &mut tx,
        SETTING_S3_FORCE_PATH_STYLE,
        if validated.s3_force_path_style {
            "true"
        } else {
            "false"
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

async fn upsert_setting(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    key: &str,
    value: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO settings (key, value, updated_at)
         VALUES ($1, $2, NOW())
         ON CONFLICT (key)
         DO UPDATE SET value = EXCLUDED.value, updated_at = NOW()",
    )
    .bind(key)
    .bind(value)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn upsert_setting_opt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    key: &str,
    value: Option<&str>,
) -> Result<(), AppError> {
    match value {
        Some(value) if !value.trim().is_empty() => upsert_setting(tx, key, value).await?,
        _ => {
            sqlx::query("DELETE FROM settings WHERE key = $1")
                .bind(key)
                .execute(&mut **tx)
                .await?;
        }
    }
    Ok(())
}
