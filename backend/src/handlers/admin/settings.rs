use super::common::admin_service;
use crate::audit::log_audit_db;
use crate::db::AppState;
use crate::db::get_setting_value;
use crate::error::AppError;
use crate::middleware::AdminUser;
use crate::models::{
    AdminSettingsConfig, Setting, UpdateAdminSettingsConfigRequest, UpdateSettingRequest,
};
use crate::runtime_settings::{
    RuntimeSettings, SETTING_LOCAL_STORAGE_PATH, SETTING_MAIL_ENABLED, SETTING_MAIL_FROM_EMAIL,
    SETTING_MAIL_FROM_NAME, SETTING_MAIL_LINK_BASE_URL, SETTING_MAIL_SMTP_HOST,
    SETTING_MAIL_SMTP_PASSWORD, SETTING_MAIL_SMTP_PORT, SETTING_MAIL_SMTP_USER,
    SETTING_S3_ACCESS_KEY, SETTING_S3_BUCKET, SETTING_S3_ENDPOINT, SETTING_S3_FORCE_PATH_STYLE,
    SETTING_S3_PREFIX, SETTING_S3_REGION, SETTING_S3_SECRET_KEY, SETTING_SITE_NAME,
    SETTING_STORAGE_BACKEND, admin_setting_policy, mask_admin_setting_value,
};
use axum::{
    Json,
    extract::{Path, State},
};

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
    Ok(Json(settings.to_admin_config(
        state.storage_manager.restart_required(&settings),
    )))
}

pub async fn update_admin_settings_config(
    State(state): State<AppState>,
    admin_user: AdminUser,
    Json(req): Json<UpdateAdminSettingsConfigRequest>,
) -> Result<Json<AdminSettingsConfig>, AppError> {
    let current = state.runtime_settings.get_runtime_settings().await?;
    let updated = state
        .runtime_settings
        .update_admin_settings_config(req)
        .await?;
    let restart_required = state.storage_manager.restart_required(&updated);
    let changed_keys = changed_setting_keys(&current, &updated);

    if !changed_keys.is_empty() {
        log_audit_db(
            &state.database,
            Some(admin_user.id),
            "admin.settings.config_updated",
            "settings",
            None,
            None,
            Some(serde_json::json!({
                "admin_email": admin_user.email,
                "changed_keys": changed_keys,
                "restart_required": restart_required,
                "risk_level": if restart_required { "danger" } else { "warning" },
            })),
        )
        .await;
    }

    Ok(Json(updated.to_admin_config(restart_required)))
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
        .update_raw_setting(&key, &req.value)
        .await?;

    let policy = admin_setting_policy(&key);
    let previous_value_masked = previous_value.map(|value| mask_admin_setting_value(&key, &value));
    let next_value_masked = mask_admin_setting_value(&key, &req.value);
    let risk_level = if policy.requires_confirmation && raw_setting_requires_restart(&key) {
        "danger"
    } else if policy.requires_confirmation {
        "warning"
    } else {
        "info"
    };

    log_audit_db(
        &state.database,
        Some(admin_user.id),
        "admin.settings.raw_setting_updated",
        "setting",
        None,
        None,
        Some(serde_json::json!({
            "admin_email": admin_user.email,
            "setting_key": key,
            "previous_value": previous_value_masked,
            "new_value": next_value_masked,
            "requires_confirmation": policy.requires_confirmation,
            "risk_level": risk_level,
            "restart_required": raw_setting_requires_restart(&key),
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
    if current.s3_endpoint != updated.s3_endpoint {
        changed.push(SETTING_S3_ENDPOINT);
    }
    if current.s3_region != updated.s3_region {
        changed.push(SETTING_S3_REGION);
    }
    if current.s3_bucket != updated.s3_bucket {
        changed.push(SETTING_S3_BUCKET);
    }
    if current.s3_prefix != updated.s3_prefix {
        changed.push(SETTING_S3_PREFIX);
    }
    if current.s3_access_key != updated.s3_access_key {
        changed.push(SETTING_S3_ACCESS_KEY);
    }
    if current.s3_secret_key != updated.s3_secret_key {
        changed.push(SETTING_S3_SECRET_KEY);
    }
    if current.s3_force_path_style != updated.s3_force_path_style {
        changed.push(SETTING_S3_FORCE_PATH_STYLE);
    }

    changed
}

fn raw_setting_requires_restart(key: &str) -> bool {
    matches!(
        key,
        SETTING_STORAGE_BACKEND
            | SETTING_LOCAL_STORAGE_PATH
            | SETTING_S3_ENDPOINT
            | SETTING_S3_REGION
            | SETTING_S3_BUCKET
            | SETTING_S3_PREFIX
            | SETTING_S3_FORCE_PATH_STYLE
    )
}
