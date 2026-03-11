use axum::{Json, extract::State, http::HeaderMap};
use base64::Engine;
use redis::AsyncCommands;

use crate::audit::log_audit_db;
use crate::config::DatabaseKind;
use crate::db::{
    AppState, SITE_FAVICON_DATA_URL_SETTING_KEY, acquire_installation_lock,
    create_admin_account_mysql_tx, create_admin_account_sqlite_tx, create_admin_account_tx,
    delete_setting_mysql_tx, delete_setting_sqlite_tx, delete_setting_tx, get_setting_value,
    has_admin_account, has_admin_account_mysql_tx, has_admin_account_sqlite_tx,
    has_admin_account_tx, is_app_installed, is_app_installed_mysql_tx, is_app_installed_sqlite_tx,
    is_app_installed_tx, mark_app_installed_mysql_tx, mark_app_installed_sqlite_tx,
    mark_app_installed_tx, upsert_setting_mysql_tx, upsert_setting_sqlite_tx, upsert_setting_tx,
    validate_admin_bootstrap_config,
};
use crate::domain::auth::user_token_version_key;
use crate::error::AppError;
use crate::handlers::auth::common::append_session_cookies;
use crate::models::{InstallBootstrapRequest, InstallBootstrapResponse, InstallStatusResponse};
use crate::runtime_settings::validate_and_merge;
use crate::runtime_settings::{
    persist_settings_mysql_tx, persist_settings_sqlite_tx, persist_settings_tx,
};

const MAX_FAVICON_BYTES: usize = 256 * 1024;

pub async fn get_install_status(
    State(state): State<AppState>,
) -> Result<Json<InstallStatusResponse>, AppError> {
    let installed = is_app_installed(&state.database).await?;
    let has_admin = has_admin_account(&state.database).await?;
    let settings = state.runtime_settings.get_runtime_settings().await?;
    let favicon_configured = get_setting_value(&state.database, SITE_FAVICON_DATA_URL_SETTING_KEY)
        .await?
        .is_some_and(|value| !value.trim().is_empty());

    Ok(Json(InstallStatusResponse {
        installed,
        has_admin,
        favicon_configured,
        config: settings.to_admin_config(if installed {
            state.storage_manager.restart_required(&settings)
        } else {
            false
        }),
    }))
}

pub async fn bootstrap_installation(
    State(state): State<AppState>,
    Json(req): Json<InstallBootstrapRequest>,
) -> Result<(HeaderMap, Json<InstallBootstrapResponse>), AppError> {
    let InstallBootstrapRequest {
        admin_email,
        admin_password,
        favicon_data_url,
        config,
    } = req;

    let admin = validate_admin_bootstrap_config(admin_email, admin_password)
        .map_err(|error| AppError::ValidationError(error.to_string()))?;
    let favicon_data_url = validate_favicon_data_url(favicon_data_url)?;
    let current_settings = state.runtime_settings.get_runtime_settings().await?;
    let validated_settings = validate_and_merge(current_settings, config)?;

    let user = match state.database_kind() {
        DatabaseKind::Postgres => {
            let pool = state.postgres_pool()?;
            let mut tx = pool.begin().await?;
            acquire_installation_lock(&mut tx).await?;

            if is_app_installed_tx(&mut tx).await? || has_admin_account_tx(&mut tx).await? {
                return Err(AppError::AppAlreadyInstalled);
            }

            persist_settings_tx(&mut tx, &validated_settings).await?;
            if let Some(value) = favicon_data_url.as_deref() {
                upsert_setting_tx(&mut tx, SITE_FAVICON_DATA_URL_SETTING_KEY, value).await?;
            } else {
                delete_setting_tx(&mut tx, SITE_FAVICON_DATA_URL_SETTING_KEY).await?;
            }

            let user = create_admin_account_tx(&mut tx, &admin.email, &admin.password)
                .await
                .map_err(AppError::Internal)?;
            mark_app_installed_tx(&mut tx).await?;
            tx.commit().await?;
            user
        }
        DatabaseKind::MySql => {
            let pool = match &state.database {
                crate::db::DatabasePool::MySql(pool) => pool,
                crate::db::DatabasePool::Postgres(_) | crate::db::DatabasePool::Sqlite(_) => {
                    unreachable!()
                }
            };
            let mut tx = pool.begin().await?;

            if is_app_installed_mysql_tx(&mut tx).await?
                || has_admin_account_mysql_tx(&mut tx).await?
            {
                return Err(AppError::AppAlreadyInstalled);
            }

            persist_settings_mysql_tx(&mut tx, &validated_settings).await?;
            if let Some(value) = favicon_data_url.as_deref() {
                upsert_setting_mysql_tx(&mut tx, SITE_FAVICON_DATA_URL_SETTING_KEY, value).await?;
            } else {
                delete_setting_mysql_tx(&mut tx, SITE_FAVICON_DATA_URL_SETTING_KEY).await?;
            }

            let user: crate::models::User =
                create_admin_account_mysql_tx(&mut tx, &admin.email, &admin.password)
                    .await
                    .map_err(AppError::Internal)?;
            mark_app_installed_mysql_tx(&mut tx).await?;
            tx.commit().await?;
            user
        }
        DatabaseKind::Sqlite => {
            let pool = match &state.database {
                crate::db::DatabasePool::Sqlite(pool) => pool,
                crate::db::DatabasePool::Postgres(_) | crate::db::DatabasePool::MySql(_) => {
                    unreachable!()
                }
            };
            let mut tx = pool.begin().await?;

            if is_app_installed_sqlite_tx(&mut tx).await?
                || has_admin_account_sqlite_tx(&mut tx).await?
            {
                return Err(AppError::AppAlreadyInstalled);
            };

            persist_settings_sqlite_tx(&mut tx, &validated_settings).await?;
            if let Some(value) = favicon_data_url.as_deref() {
                upsert_setting_sqlite_tx(&mut tx, SITE_FAVICON_DATA_URL_SETTING_KEY, value).await?;
            } else {
                delete_setting_sqlite_tx(&mut tx, SITE_FAVICON_DATA_URL_SETTING_KEY).await?;
            }

            let user: crate::models::User =
                create_admin_account_sqlite_tx(&mut tx, &admin.email, &admin.password)
                    .await
                    .map_err(AppError::Internal)?;
            mark_app_installed_sqlite_tx(&mut tx).await?;
            tx.commit().await?;
            user
        }
    };

    state.runtime_settings.invalidate_cache().await;
    let settings = state.runtime_settings.get_runtime_settings().await?;
    let user_response = crate::models::UserResponse::from(user);

    let mut redis = state.redis.clone();
    let token_version = redis
        .get::<_, Option<u64>>(user_token_version_key(user_response.id))
        .await?
        .unwrap_or(0);
    let access_token = state.auth.generate_access_token(
        user_response.id,
        &user_response.email,
        &user_response.role,
        token_version,
    )?;
    let refresh_token = state
        .auth
        .generate_refresh_token(user_response.id, token_version)?;

    let mut headers = HeaderMap::new();
    append_session_cookies(
        &mut headers,
        &state.config.cookie,
        &access_token,
        state.auth.access_token_ttl_seconds(),
        &refresh_token,
        state.auth.session_ttl_seconds(),
    )?;

    log_audit_db(
        &state.database,
        Some(user_response.id),
        "system.install_completed",
        "system",
        Some(user_response.id),
        None,
        Some(serde_json::json!({
            "admin_email": user_response.email,
            "site_name": settings.site_name,
            "storage_backend": settings.storage_backend.as_str(),
            "mail_enabled": settings.mail_enabled,
            "favicon_configured": favicon_data_url.is_some(),
        })),
    )
    .await;

    Ok((
        headers,
        Json(InstallBootstrapResponse {
            user: user_response,
            favicon_configured: favicon_data_url.is_some(),
            config: settings.to_admin_config(state.storage_manager.restart_required(&settings)),
        }),
    ))
}

fn validate_favicon_data_url(value: Option<String>) -> Result<Option<String>, AppError> {
    let Some(value) = value.map(|value| value.trim().to_string()) else {
        return Ok(None);
    };
    if value.is_empty() {
        return Ok(None);
    }

    let Some((mime_prefix, encoded)) = value.split_once(";base64,") else {
        return Err(AppError::ValidationError(
            "网站图标必须使用 data URL(base64) 格式上传".to_string(),
        ));
    };
    let Some(mime) = mime_prefix.strip_prefix("data:") else {
        return Err(AppError::ValidationError("网站图标格式无效".to_string()));
    };

    if !matches!(
        mime,
        "image/x-icon"
            | "image/vnd.microsoft.icon"
            | "image/png"
            | "image/svg+xml"
            | "image/jpeg"
            | "image/webp"
    ) {
        return Err(AppError::ValidationError(
            "网站图标仅支持 ico/png/svg/jpeg/webp".to_string(),
        ));
    }

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| AppError::ValidationError("网站图标内容无法解析".to_string()))?;
    if bytes.is_empty() {
        return Err(AppError::ValidationError("网站图标不能为空".to_string()));
    }
    if bytes.len() > MAX_FAVICON_BYTES {
        return Err(AppError::ValidationError(format!(
            "网站图标不能超过 {} KB",
            MAX_FAVICON_BYTES / 1024
        )));
    }

    Ok(Some(format!(
        "data:{};base64,{}",
        mime,
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )))
}
