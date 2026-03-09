use super::common::admin_service;
use crate::db::AppState;
use crate::error::AppError;
use crate::middleware::AdminUser;
use crate::models::{
    AdminSettingsConfig, Setting, UpdateAdminSettingsConfigRequest, UpdateSettingRequest,
};
use crate::runtime_settings::StorageBackend;
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
    let config = state.runtime_settings.get_admin_settings_config().await?;
    Ok(Json(config))
}

pub async fn update_admin_settings_config(
    State(state): State<AppState>,
    _admin_user: AdminUser,
    Json(req): Json<UpdateAdminSettingsConfigRequest>,
) -> Result<Json<AdminSettingsConfig>, AppError> {
    let updated = state
        .runtime_settings
        .update_admin_settings_config(req)
        .await?;
    if updated.storage_backend.eq_ignore_ascii_case("local") {
        state.storage_manager.ensure_local_storage_dir().await?;
    }
    Ok(Json(updated))
}

pub async fn update_setting(
    State(state): State<AppState>,
    _admin_user: AdminUser,
    Path(key): Path<String>,
    Json(req): Json<UpdateSettingRequest>,
) -> Result<(), AppError> {
    let updated = state
        .runtime_settings
        .update_raw_setting(&key, &req.value)
        .await?;
    if matches!(updated.storage_backend, StorageBackend::Local) {
        state.storage_manager.ensure_local_storage_dir().await?;
    }
    Ok(())
}
