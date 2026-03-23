use super::common::admin_service;
use crate::audit::{AuditEvent, record_audit_sync};
use crate::db::AppState;
use crate::db::get_setting_value;
use crate::error::AppError;
use crate::handlers::storage_browser::{BrowseStorageDirectoriesQuery, browse_storage_directories};
use crate::middleware::AdminUser;
use crate::models::{
    AdminSettingsConfig, Setting, UpdateAdminSettingsConfigRequest, UpdateSettingRequest,
    storage_backend_kind_from_runtime,
};
use crate::runtime_settings::{
    RuntimeSettings, SETTING_LOCAL_STORAGE_PATH, SETTING_MAIL_ENABLED, SETTING_MAIL_FROM_EMAIL,
    SETTING_MAIL_FROM_NAME, SETTING_MAIL_LINK_BASE_URL, SETTING_MAIL_SMTP_HOST,
    SETTING_MAIL_SMTP_PASSWORD, SETTING_MAIL_SMTP_PORT, SETTING_MAIL_SMTP_USER, SETTING_SITE_NAME,
    SETTING_STORAGE_BACKEND, admin_setting_policy, mask_admin_setting_value,
};
use axum::{
    Json,
    extract::{Path, Query, State},
};

fn runtime_settings_to_admin_config(
    settings: &RuntimeSettings,
    restart_required: bool,
) -> AdminSettingsConfig {
    AdminSettingsConfig {
        site_name: settings.site_name.clone(),
        storage_backend: storage_backend_kind_from_runtime(settings.storage_backend),
        local_storage_path: settings.local_storage_path.clone(),
        mail_enabled: settings.mail_enabled,
        mail_smtp_host: settings.mail_smtp_host.clone(),
        mail_smtp_port: settings.mail_smtp_port,
        mail_smtp_user: settings.mail_smtp_user.clone(),
        mail_smtp_password_set: settings
            .mail_smtp_password
            .as_ref()
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false),
        mail_from_email: settings.mail_from_email.clone(),
        mail_from_name: settings.mail_from_name.clone(),
        mail_link_base_url: settings.mail_link_base_url.clone(),
        restart_required,
        settings_version: settings.settings_version(),
    }
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
    Ok(Json(runtime_settings_to_admin_config(
        &settings,
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
    let updated = state
        .runtime_settings
        .update_admin_settings_config(req, &state.storage_manager)
        .await?;
    let restart_required = state.storage_manager.restart_required(&updated);
    let changed_keys = changed_setting_keys(&current, &updated);
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
