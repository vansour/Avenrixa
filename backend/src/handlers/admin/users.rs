use super::common::admin_service;
use crate::db::AppState;
use crate::error::AppError;
use crate::middleware::AdminUser;
use crate::models::{AdminUserSummary, UserUpdateRequest};
use axum::{
    Json,
    extract::{Path, State},
};
use uuid::Uuid;

pub async fn get_users(
    State(state): State<AppState>,
    _admin_user: AdminUser,
) -> Result<Json<Vec<AdminUserSummary>>, AppError> {
    let service = admin_service(&state)?;
    let users = service.get_users().await?;
    Ok(Json(users))
}

pub async fn update_user_role(
    State(state): State<AppState>,
    _admin_user: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UserUpdateRequest>,
) -> Result<(), AppError> {
    let service = admin_service(&state)?;
    if let Some(ref role) = req.role {
        service.update_user_role(id, role).await?;
    }
    Ok(())
}
