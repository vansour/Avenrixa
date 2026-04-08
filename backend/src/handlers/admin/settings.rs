use super::common::admin_service;
use crate::audit::{AuditEvent, record_audit_sync};
use crate::db::{
    AppState, SITE_FAVICON_DATA_URL_SETTING_KEY, delete_setting_tx, get_setting_value,
    upsert_setting_tx,
};
use crate::error::AppError;
use crate::handlers::settings_config::{
    favicon_is_configured, runtime_settings_to_admin_config, validate_favicon_data_url,
};
use crate::handlers::storage_browser::{BrowseStorageDirectoriesQuery, browse_storage_directories};
use crate::middleware::AdminUser;
use crate::models::{
    AdminSettingsConfig, Setting, UpdateAdminSettingsConfigRequest, UpdateSettingRequest,
};
use crate::runtime_settings::{
    RuntimeSettings, SETTING_LOCAL_STORAGE_PATH, SETTING_MAIL_ENABLED, SETTING_MAIL_FROM_EMAIL,
    SETTING_MAIL_FROM_NAME, SETTING_MAIL_LINK_BASE_URL, SETTING_MAIL_SMTP_HOST,
    SETTING_MAIL_SMTP_PASSWORD, SETTING_MAIL_SMTP_PORT, SETTING_MAIL_SMTP_USER,
    SETTING_SITE_FAVICON_DATA_URL, SETTING_SITE_NAME, SETTING_STORAGE_BACKEND,
    admin_setting_policy, mask_admin_setting_value,
};
use axum::{
    Json,
    extract::{Path, Query, State},
};

enum FaviconMutation {
    Unchanged,
    Clear,
    Set(String),
}

fn parse_favicon_mutation(
    req: &UpdateAdminSettingsConfigRequest,
) -> Result<FaviconMutation, AppError> {
    if req.clear_favicon {
        if req
            .favicon_data_url
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty())
        {
            return Err(AppError::ValidationError(
                "不能同时上传并清空网站图标".to_string(),
            ));
        }
        return Ok(FaviconMutation::Clear);
    }

    match validate_favicon_data_url(req.favicon_data_url.clone())? {
        Some(data_url) => Ok(FaviconMutation::Set(data_url)),
        None => Ok(FaviconMutation::Unchanged),
    }
}

async fn apply_favicon_mutation(
    database: &crate::db::DatabasePool,
    mutation: &FaviconMutation,
) -> Result<(), AppError> {
    if matches!(mutation, FaviconMutation::Unchanged) {
        return Ok(());
    }

    match database {
        crate::db::DatabasePool::Postgres(pool) => {
            let mut tx = pool.begin().await?;
            match mutation {
                FaviconMutation::Unchanged => {}
                FaviconMutation::Clear => {
                    delete_setting_tx(&mut tx, SITE_FAVICON_DATA_URL_SETTING_KEY).await?;
                }
                FaviconMutation::Set(data_url) => {
                    upsert_setting_tx(&mut tx, SITE_FAVICON_DATA_URL_SETTING_KEY, data_url).await?;
                }
            }
            tx.commit().await?;
        }
    }

    Ok(())
}

async fn restore_previous_favicon(
    database: &crate::db::DatabasePool,
    previous_favicon: Option<&str>,
) -> Result<(), AppError> {
    let mutation = match previous_favicon
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(value) => FaviconMutation::Set(value.to_string()),
        None => FaviconMutation::Clear,
    };

    apply_favicon_mutation(database, &mutation).await
}

pub async fn get_settings_admin(
    State(state): State<AppState>,
    _admin_user: AdminUser,
) -> Result<Json<Vec<Setting>>, AppError> {
    let service = admin_service(&state)?;
    let settings = service.get_settings().await?;
    Ok(Json(settings))
}

pub async fn get_admin_settings_config(
    State(state): State<AppState>,
    _admin_user: AdminUser,
) -> Result<Json<AdminSettingsConfig>, AppError> {
    let settings = state.runtime_settings.get_runtime_settings().await?;
    let favicon_configured = favicon_is_configured(&state.database).await?;
    Ok(Json(runtime_settings_to_admin_config(
        &settings,
        favicon_configured,
        state.storage_manager.restart_required(&settings),
    )))
}

pub async fn browse_admin_storage_directories(
    Query(query): Query<BrowseStorageDirectoriesQuery>,
    _admin_user: AdminUser,
) -> Result<Json<crate::models::StorageDirectoryBrowseResponse>, AppError> {
    Ok(Json(
        browse_storage_directories(query.path.as_deref()).await?,
    ))
}

pub async fn update_admin_settings_config(
    State(state): State<AppState>,
    admin_user: AdminUser,
    Json(req): Json<UpdateAdminSettingsConfigRequest>,
) -> Result<Json<AdminSettingsConfig>, AppError> {
    let current = state.runtime_settings.get_runtime_settings().await?;
    let favicon_mutation = parse_favicon_mutation(&req)?;
    let previous_favicon = if matches!(&favicon_mutation, FaviconMutation::Unchanged) {
        None
    } else {
        Some(get_setting_value(&state.database, SITE_FAVICON_DATA_URL_SETTING_KEY).await?)
    };

    if !matches!(&favicon_mutation, FaviconMutation::Unchanged) {
        apply_favicon_mutation(&state.database, &favicon_mutation).await?;
    }

    let updated = match state
        .runtime_settings
        .update_admin_settings_config(req, &state.storage_manager)
        .await
    {
        Ok(updated) => updated,
        Err(error) => {
            if let Some(previous_favicon) = previous_favicon.as_ref() {
                let _ =
                    restore_previous_favicon(&state.database, previous_favicon.as_deref()).await;
            }
            return Err(error);
        }
    };

    let favicon_configured = match &favicon_mutation {
        FaviconMutation::Unchanged => favicon_is_configured(&state.database).await?,
        FaviconMutation::Clear => false,
        FaviconMutation::Set(_) => true,
    };
    let restart_required = state.storage_manager.restart_required(&updated);
    let mut changed_keys = changed_setting_keys(&current, &updated);
    if !matches!(&favicon_mutation, FaviconMutation::Unchanged) {
        changed_keys.push(SETTING_SITE_FAVICON_DATA_URL);
    }
    let has_high_risk_change = changed_keys.iter().any(|key| raw_setting_is_high_risk(key));

    if !changed_keys.is_empty() {
        record_audit_sync(
            &state.database,
            state.observability.as_ref(),
            AuditEvent::new("admin.settings.config_updated", "settings")
                .with_user_id(admin_user.id)
                .with_details(serde_json::json!({
                    "admin_email": admin_user.email,
                    "changed_keys": changed_keys,
                    "restart_required": restart_required,
                    "risk_level": if has_high_risk_change { "danger" } else { "warning" },
                })),
        )
        .await;
    }

    Ok(Json(runtime_settings_to_admin_config(
        &updated,
        favicon_configured,
        restart_required,
    )))
}

pub async fn update_setting(
    State(state): State<AppState>,
    admin_user: AdminUser,
    Path(key): Path<String>,
    Json(req): Json<UpdateSettingRequest>,
) -> Result<(), AppError> {
    let previous_value = get_setting_value(&state.database, &key).await?;
    state
        .runtime_settings
        .update_raw_setting(&key, &req.value, &state.storage_manager)
        .await?;

    let policy = admin_setting_policy(&key);
    let previous_value_masked = previous_value.map(|value| mask_admin_setting_value(&key, &value));
    let next_value_masked = mask_admin_setting_value(&key, &req.value);
    let risk_level = if policy.requires_confirmation && raw_setting_is_high_risk(&key) {
        "danger"
    } else if policy.requires_confirmation {
        "warning"
    } else {
        "info"
    };

    record_audit_sync(
        &state.database,
        state.observability.as_ref(),
        AuditEvent::new("admin.settings.raw_setting_updated", "setting")
            .with_user_id(admin_user.id)
            .with_details(serde_json::json!({
                "admin_email": admin_user.email,
                "setting_key": key,
                "previous_value": previous_value_masked,
                "new_value": next_value_masked,
                "requires_confirmation": policy.requires_confirmation,
                "risk_level": risk_level,
                "restart_required": false,
            })),
    )
    .await;

    Ok(())
}

fn changed_setting_keys(current: &RuntimeSettings, updated: &RuntimeSettings) -> Vec<&'static str> {
    let mut changed = Vec::new();

    if current.site_name != updated.site_name {
        changed.push(SETTING_SITE_NAME);
    }
    if current.storage_backend != updated.storage_backend {
        changed.push(SETTING_STORAGE_BACKEND);
    }
    if current.local_storage_path != updated.local_storage_path {
        changed.push(SETTING_LOCAL_STORAGE_PATH);
    }
    if current.mail_enabled != updated.mail_enabled {
        changed.push(SETTING_MAIL_ENABLED);
    }
    if current.mail_smtp_host != updated.mail_smtp_host {
        changed.push(SETTING_MAIL_SMTP_HOST);
    }
    if current.mail_smtp_port != updated.mail_smtp_port {
        changed.push(SETTING_MAIL_SMTP_PORT);
    }
    if current.mail_smtp_user != updated.mail_smtp_user {
        changed.push(SETTING_MAIL_SMTP_USER);
    }
    if current.mail_smtp_password != updated.mail_smtp_password {
        changed.push(SETTING_MAIL_SMTP_PASSWORD);
    }
    if current.mail_from_email != updated.mail_from_email {
        changed.push(SETTING_MAIL_FROM_EMAIL);
    }
    if current.mail_from_name != updated.mail_from_name {
        changed.push(SETTING_MAIL_FROM_NAME);
    }
    if current.mail_link_base_url != updated.mail_link_base_url {
        changed.push(SETTING_MAIL_LINK_BASE_URL);
    }

    changed
}

fn raw_setting_is_high_risk(key: &str) -> bool {
    matches!(key, SETTING_STORAGE_BACKEND | SETTING_LOCAL_STORAGE_PATH)
}
