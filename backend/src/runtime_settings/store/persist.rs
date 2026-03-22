use crate::error::AppError;

use super::super::model::{
    RuntimeSettings, SETTING_LOCAL_STORAGE_PATH, SETTING_MAIL_ENABLED, SETTING_MAIL_FROM_EMAIL,
    SETTING_MAIL_FROM_NAME, SETTING_MAIL_LINK_BASE_URL, SETTING_MAIL_SMTP_HOST,
    SETTING_MAIL_SMTP_PASSWORD, SETTING_MAIL_SMTP_PORT, SETTING_MAIL_SMTP_USER, SETTING_SITE_NAME,
    SETTING_STORAGE_BACKEND,
};

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
