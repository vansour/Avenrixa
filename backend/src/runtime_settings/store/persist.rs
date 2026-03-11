use crate::db::DatabasePool;
use crate::error::AppError;

use super::super::model::{
    RuntimeSettings, SETTING_LOCAL_STORAGE_PATH, SETTING_MAIL_ENABLED, SETTING_MAIL_FROM_EMAIL,
    SETTING_MAIL_FROM_NAME, SETTING_MAIL_LINK_BASE_URL, SETTING_MAIL_SMTP_HOST,
    SETTING_MAIL_SMTP_PASSWORD, SETTING_MAIL_SMTP_PORT, SETTING_MAIL_SMTP_USER,
    SETTING_S3_ACCESS_KEY, SETTING_S3_BUCKET, SETTING_S3_ENDPOINT, SETTING_S3_FORCE_PATH_STYLE,
    SETTING_S3_PREFIX, SETTING_S3_REGION, SETTING_S3_SECRET_KEY, SETTING_SITE_NAME,
    SETTING_STORAGE_BACKEND,
};

pub(crate) async fn persist_settings(
    pool: &DatabasePool,
    validated: &RuntimeSettings,
) -> Result<(), AppError> {
    match pool {
        DatabasePool::Postgres(pool) => {
            let mut tx = pool.begin().await?;
            persist_settings_tx(&mut tx, validated).await?;
            tx.commit().await?;
        }
        DatabasePool::MySql(pool) => {
            let mut tx = pool.begin().await?;
            persist_settings_mysql_tx(&mut tx, validated).await?;
            tx.commit().await?;
        }
        DatabasePool::Sqlite(pool) => {
            let mut tx = pool.begin().await?;
            persist_settings_sqlite_tx(&mut tx, validated).await?;
            tx.commit().await?;
        }
    }
    Ok(())
}

pub(crate) async fn persist_settings_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    validated: &RuntimeSettings,
) -> Result<(), AppError> {
    upsert_setting(tx, SETTING_SITE_NAME, &validated.site_name).await?;
    upsert_setting(
        tx,
        SETTING_STORAGE_BACKEND,
        validated.storage_backend.as_str(),
    )
    .await?;
    upsert_setting(
        tx,
        SETTING_LOCAL_STORAGE_PATH,
        &validated.local_storage_path,
    )
    .await?;
    upsert_setting(
        tx,
        SETTING_MAIL_ENABLED,
        if validated.mail_enabled {
            "true"
        } else {
            "false"
        },
    )
    .await?;
    upsert_setting(tx, SETTING_MAIL_SMTP_HOST, &validated.mail_smtp_host).await?;
    upsert_setting(
        tx,
        SETTING_MAIL_SMTP_PORT,
        &validated.mail_smtp_port.to_string(),
    )
    .await?;
    upsert_setting_opt(
        tx,
        SETTING_MAIL_SMTP_USER,
        validated.mail_smtp_user.as_deref(),
    )
    .await?;
    upsert_setting_opt(
        tx,
        SETTING_MAIL_SMTP_PASSWORD,
        validated.mail_smtp_password.as_deref(),
    )
    .await?;
    upsert_setting(tx, SETTING_MAIL_FROM_EMAIL, &validated.mail_from_email).await?;
    upsert_setting(tx, SETTING_MAIL_FROM_NAME, &validated.mail_from_name).await?;
    upsert_setting(
        tx,
        SETTING_MAIL_LINK_BASE_URL,
        &validated.mail_link_base_url,
    )
    .await?;
    upsert_setting_opt(tx, SETTING_S3_ENDPOINT, validated.s3_endpoint.as_deref()).await?;
    upsert_setting_opt(tx, SETTING_S3_REGION, validated.s3_region.as_deref()).await?;
    upsert_setting_opt(tx, SETTING_S3_BUCKET, validated.s3_bucket.as_deref()).await?;
    upsert_setting_opt(tx, SETTING_S3_PREFIX, validated.s3_prefix.as_deref()).await?;
    upsert_setting_opt(
        tx,
        SETTING_S3_ACCESS_KEY,
        validated.s3_access_key.as_deref(),
    )
    .await?;
    upsert_setting_opt(
        tx,
        SETTING_S3_SECRET_KEY,
        validated.s3_secret_key.as_deref(),
    )
    .await?;
    upsert_setting(
        tx,
        SETTING_S3_FORCE_PATH_STYLE,
        if validated.s3_force_path_style {
            "true"
        } else {
            "false"
        },
    )
    .await?;
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

pub(crate) async fn persist_settings_mysql_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::MySql>,
    validated: &RuntimeSettings,
) -> Result<(), AppError> {
    upsert_setting_mysql(tx, SETTING_SITE_NAME, &validated.site_name).await?;
    upsert_setting_mysql(
        tx,
        SETTING_STORAGE_BACKEND,
        validated.storage_backend.as_str(),
    )
    .await?;
    upsert_setting_mysql(
        tx,
        SETTING_LOCAL_STORAGE_PATH,
        &validated.local_storage_path,
    )
    .await?;
    upsert_setting_mysql(
        tx,
        SETTING_MAIL_ENABLED,
        if validated.mail_enabled {
            "true"
        } else {
            "false"
        },
    )
    .await?;
    upsert_setting_mysql(tx, SETTING_MAIL_SMTP_HOST, &validated.mail_smtp_host).await?;
    upsert_setting_mysql(
        tx,
        SETTING_MAIL_SMTP_PORT,
        &validated.mail_smtp_port.to_string(),
    )
    .await?;
    upsert_setting_mysql_opt(
        tx,
        SETTING_MAIL_SMTP_USER,
        validated.mail_smtp_user.as_deref(),
    )
    .await?;
    upsert_setting_mysql_opt(
        tx,
        SETTING_MAIL_SMTP_PASSWORD,
        validated.mail_smtp_password.as_deref(),
    )
    .await?;
    upsert_setting_mysql(tx, SETTING_MAIL_FROM_EMAIL, &validated.mail_from_email).await?;
    upsert_setting_mysql(tx, SETTING_MAIL_FROM_NAME, &validated.mail_from_name).await?;
    upsert_setting_mysql(
        tx,
        SETTING_MAIL_LINK_BASE_URL,
        &validated.mail_link_base_url,
    )
    .await?;
    upsert_setting_mysql_opt(tx, SETTING_S3_ENDPOINT, validated.s3_endpoint.as_deref()).await?;
    upsert_setting_mysql_opt(tx, SETTING_S3_REGION, validated.s3_region.as_deref()).await?;
    upsert_setting_mysql_opt(tx, SETTING_S3_BUCKET, validated.s3_bucket.as_deref()).await?;
    upsert_setting_mysql_opt(tx, SETTING_S3_PREFIX, validated.s3_prefix.as_deref()).await?;
    upsert_setting_mysql_opt(
        tx,
        SETTING_S3_ACCESS_KEY,
        validated.s3_access_key.as_deref(),
    )
    .await?;
    upsert_setting_mysql_opt(
        tx,
        SETTING_S3_SECRET_KEY,
        validated.s3_secret_key.as_deref(),
    )
    .await?;
    upsert_setting_mysql(
        tx,
        SETTING_S3_FORCE_PATH_STYLE,
        if validated.s3_force_path_style {
            "true"
        } else {
            "false"
        },
    )
    .await?;
    Ok(())
}

async fn upsert_setting_mysql(
    tx: &mut sqlx::Transaction<'_, sqlx::MySql>,
    key: &str,
    value: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO settings (`key`, `value`, updated_at)
         VALUES (?, ?, CURRENT_TIMESTAMP(6))
         ON DUPLICATE KEY UPDATE
             `value` = VALUES(`value`),
             updated_at = CURRENT_TIMESTAMP(6)",
    )
    .bind(key)
    .bind(value)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn upsert_setting_mysql_opt(
    tx: &mut sqlx::Transaction<'_, sqlx::MySql>,
    key: &str,
    value: Option<&str>,
) -> Result<(), AppError> {
    match value {
        Some(value) if !value.trim().is_empty() => upsert_setting_mysql(tx, key, value).await?,
        _ => {
            sqlx::query("DELETE FROM settings WHERE `key` = ?")
                .bind(key)
                .execute(&mut **tx)
                .await?;
        }
    }
    Ok(())
}

pub(crate) async fn persist_settings_sqlite_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    validated: &RuntimeSettings,
) -> Result<(), AppError> {
    upsert_setting_sqlite(tx, SETTING_SITE_NAME, &validated.site_name).await?;
    upsert_setting_sqlite(
        tx,
        SETTING_STORAGE_BACKEND,
        validated.storage_backend.as_str(),
    )
    .await?;
    upsert_setting_sqlite(
        tx,
        SETTING_LOCAL_STORAGE_PATH,
        &validated.local_storage_path,
    )
    .await?;
    upsert_setting_sqlite(
        tx,
        SETTING_MAIL_ENABLED,
        if validated.mail_enabled {
            "true"
        } else {
            "false"
        },
    )
    .await?;
    upsert_setting_sqlite(tx, SETTING_MAIL_SMTP_HOST, &validated.mail_smtp_host).await?;
    upsert_setting_sqlite(
        tx,
        SETTING_MAIL_SMTP_PORT,
        &validated.mail_smtp_port.to_string(),
    )
    .await?;
    upsert_setting_sqlite_opt(
        tx,
        SETTING_MAIL_SMTP_USER,
        validated.mail_smtp_user.as_deref(),
    )
    .await?;
    upsert_setting_sqlite_opt(
        tx,
        SETTING_MAIL_SMTP_PASSWORD,
        validated.mail_smtp_password.as_deref(),
    )
    .await?;
    upsert_setting_sqlite(tx, SETTING_MAIL_FROM_EMAIL, &validated.mail_from_email).await?;
    upsert_setting_sqlite(tx, SETTING_MAIL_FROM_NAME, &validated.mail_from_name).await?;
    upsert_setting_sqlite(
        tx,
        SETTING_MAIL_LINK_BASE_URL,
        &validated.mail_link_base_url,
    )
    .await?;
    upsert_setting_sqlite_opt(tx, SETTING_S3_ENDPOINT, validated.s3_endpoint.as_deref()).await?;
    upsert_setting_sqlite_opt(tx, SETTING_S3_REGION, validated.s3_region.as_deref()).await?;
    upsert_setting_sqlite_opt(tx, SETTING_S3_BUCKET, validated.s3_bucket.as_deref()).await?;
    upsert_setting_sqlite_opt(tx, SETTING_S3_PREFIX, validated.s3_prefix.as_deref()).await?;
    upsert_setting_sqlite_opt(
        tx,
        SETTING_S3_ACCESS_KEY,
        validated.s3_access_key.as_deref(),
    )
    .await?;
    upsert_setting_sqlite_opt(
        tx,
        SETTING_S3_SECRET_KEY,
        validated.s3_secret_key.as_deref(),
    )
    .await?;
    upsert_setting_sqlite(
        tx,
        SETTING_S3_FORCE_PATH_STYLE,
        if validated.s3_force_path_style {
            "true"
        } else {
            "false"
        },
    )
    .await?;
    Ok(())
}

async fn upsert_setting_sqlite(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    key: &str,
    value: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO settings (key, value, updated_at)
         VALUES (?1, ?2, STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now'))
         ON CONFLICT (key)
         DO UPDATE SET value = excluded.value,
                       updated_at = STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')",
    )
    .bind(key)
    .bind(value)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn upsert_setting_sqlite_opt(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    key: &str,
    value: Option<&str>,
) -> Result<(), AppError> {
    match value {
        Some(value) if !value.trim().is_empty() => upsert_setting_sqlite(tx, key, value).await?,
        _ => {
            sqlx::query("DELETE FROM settings WHERE key = ?1")
                .bind(key)
                .execute(&mut **tx)
                .await?;
        }
    }
    Ok(())
}
