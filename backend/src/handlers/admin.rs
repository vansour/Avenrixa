use crate::db::AppState;
use crate::error::AppError;
use crate::middleware::{AdminUser, AuthUser};
use crate::models::*;
use axum::{
    Json,
    extract::{Path, Query, State},
};
use uuid::Uuid;

#[tracing::instrument(skip(state))]
pub async fn health_check(State(state): State<AppState>) -> Result<Json<HealthStatus>, AppError> {
    let service = state
        .admin_domain_service
        .as_ref()
        .ok_or(AppError::Internal(anyhow::anyhow!(
            "Admin service not found"
        )))?;
    let status = service
        .health_check(state.started_at.elapsed().as_secs())
        .await?;
    Ok(Json(status))
}

pub async fn cleanup_deleted_files(
    State(state): State<AppState>,
    admin_user: AdminUser,
) -> Result<Json<Vec<String>>, AppError> {
    let service = state
        .admin_domain_service
        .as_ref()
        .ok_or(AppError::Internal(anyhow::anyhow!(
            "Admin service not found"
        )))?;
    let removed = service
        .cleanup_deleted_files(admin_user.id, &admin_user.username)
        .await?;
    Ok(Json(removed))
}

pub async fn cleanup_expired_images(
    State(state): State<AppState>,
    admin_user: AdminUser,
) -> Result<Json<i64>, AppError> {
    let service = state
        .admin_domain_service
        .as_ref()
        .ok_or(AppError::Internal(anyhow::anyhow!(
            "Admin service not found"
        )))?;
    let affected = service.cleanup_expired_images(admin_user.id).await?;
    Ok(Json(affected))
}

pub async fn backup_database(
    State(state): State<AppState>,
    admin_user: AdminUser,
) -> Result<Json<BackupResponse>, AppError> {
    let service = state
        .admin_domain_service
        .as_ref()
        .ok_or(AppError::Internal(anyhow::anyhow!(
            "Admin service not found"
        )))?;
    let response = service
        .backup_database(admin_user.id, &admin_user.username)
        .await?;
    Ok(Json(response))
}

pub async fn approve_images(
    State(state): State<AppState>,
    _admin_user: AdminUser,
    Json(req): Json<ApproveRequest>,
) -> Result<(), AppError> {
    let service = state
        .admin_domain_service
        .as_ref()
        .ok_or(AppError::Internal(anyhow::anyhow!(
            "Admin service not found"
        )))?;
    service.approve_images(&req.image_ids, req.approved).await?;
    Ok(())
}

pub async fn get_users(
    State(state): State<AppState>,
    _admin_user: AdminUser,
) -> Result<Json<Vec<AdminUserSummary>>, AppError> {
    let service = state
        .admin_domain_service
        .as_ref()
        .ok_or(AppError::Internal(anyhow::anyhow!(
            "Admin service not found"
        )))?;
    let users = service.get_users().await?;
    Ok(Json(users))
}

pub async fn update_user_role(
    State(state): State<AppState>,
    _admin_user: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UserUpdateRequest>,
) -> Result<(), AppError> {
    let service = state
        .admin_domain_service
        .as_ref()
        .ok_or(AppError::Internal(anyhow::anyhow!(
            "Admin service not found"
        )))?;
    if let Some(ref role) = req.role {
        service.update_user_role(id, role).await?;
    }
    Ok(())
}

pub async fn get_audit_logs(
    State(state): State<AppState>,
    _admin_user: AdminUser,
    Query(params): Query<PaginationParams>,
) -> Result<Json<AuditLogResponse>, AppError> {
    let service = state
        .admin_domain_service
        .as_ref()
        .ok_or(AppError::Internal(anyhow::anyhow!(
            "Admin service not found"
        )))?;
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(50).clamp(1, 100);

    let response = service
        .get_audit_logs(page as i64, page_size as i64)
        .await?;
    Ok(Json(response))
}

pub async fn get_system_stats(
    State(state): State<AppState>,
    _admin_user: AdminUser,
) -> Result<Json<SystemStats>, AppError> {
    let service = state
        .admin_domain_service
        .as_ref()
        .ok_or(AppError::Internal(anyhow::anyhow!(
            "Admin service not found"
        )))?;
    let stats = service.get_system_stats().await?;
    Ok(Json(stats))
}

/// 普通用户也可以访问的设置（只读）
pub async fn get_settings_public(
    State(state): State<AppState>,
    _auth_user: AuthUser,
) -> Result<Json<Vec<Setting>>, AppError> {
    let service = state
        .admin_domain_service
        .as_ref()
        .ok_or(AppError::Internal(anyhow::anyhow!(
            "Admin service not found"
        )))?;
    let settings = service.get_settings().await?;
    Ok(Json(settings))
}

/// 管理员专用设置端点
pub async fn get_settings_admin(
    State(state): State<AppState>,
    _admin_user: AdminUser,
) -> Result<Json<Vec<Setting>>, AppError> {
    let service = state
        .admin_domain_service
        .as_ref()
        .ok_or(AppError::Internal(anyhow::anyhow!(
            "Admin service not found"
        )))?;
    let settings = service.get_settings().await?;
    Ok(Json(settings))
}

/// 管理员设置（结构化）
pub async fn get_admin_settings_config(
    State(state): State<AppState>,
    _admin_user: AdminUser,
) -> Result<Json<AdminSettingsConfig>, AppError> {
    let config = state.runtime_settings.get_admin_settings_config().await?;
    Ok(Json(config))
}

/// 更新管理员设置（结构化）
pub async fn update_admin_settings_config(
    State(state): State<AppState>,
    _admin_user: AdminUser,
    Json(req): Json<UpdateAdminSettingsConfigRequest>,
) -> Result<Json<AdminSettingsConfig>, AppError> {
    let updated = state
        .runtime_settings
        .update_admin_settings_config(req)
        .await?;
    state.runtime_settings.invalidate_cache().await;
    Ok(Json(updated))
}

pub async fn update_setting(
    State(state): State<AppState>,
    _admin_user: AdminUser,
    Path(key): Path<String>,
    Json(req): Json<UpdateSettingRequest>,
) -> Result<(), AppError> {
    let service = state
        .admin_domain_service
        .as_ref()
        .ok_or(AppError::Internal(anyhow::anyhow!(
            "Admin service not found"
        )))?;
    service.update_setting(&key, &req.value).await?;
    Ok(())
}
