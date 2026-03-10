use super::common::admin_service;
use crate::audit::log_audit_db;
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
    admin_user: AdminUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UserUpdateRequest>,
) -> Result<(), AppError> {
    let service = admin_service(&state)?;
    if let Some(ref role) = req.role {
        let result = service.update_user_role(id, role).await?;
        if result.changed {
            let risk_level = if result.previous_role.eq_ignore_ascii_case("admin")
                && result.new_role.eq_ignore_ascii_case("user")
            {
                "danger"
            } else {
                "warning"
            };
            log_audit_db(
                &state.database,
                Some(admin_user.id),
                "admin.user.role_updated",
                "user",
                Some(id),
                None,
                Some(serde_json::json!({
                    "admin_email": admin_user.email,
                    "user_email": result.email,
                    "previous_role": result.previous_role,
                    "new_role": result.new_role,
                    "risk_level": risk_level,
                })),
            )
            .await;
        }
    }
    Ok(())
}
