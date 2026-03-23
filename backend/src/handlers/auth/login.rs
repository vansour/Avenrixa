use super::common::{
    append_session_cookies, auth_domain_service, ensure_app_installed, issue_session_tokens,
};
use crate::audit::{AuditEvent, record_audit_sync};
use crate::db::AppState;
use crate::error::AppError;
use crate::models::{LoginRequest, UserResponse};
use axum::{Json, extract::State, http::HeaderMap};

#[tracing::instrument(skip(state, req), fields(email = %req.email))]
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<(HeaderMap, Json<UserResponse>), AppError> {
    ensure_app_installed(&state).await?;
    let auth_domain_service = auth_domain_service(&state)?;
    let user = auth_domain_service.login(req).await?;

    let (access_token, refresh_token) =
        issue_session_tokens(&state, user.id, &user.email, &user.role).await?;

    let mut headers = HeaderMap::new();
    append_session_cookies(
        &mut headers,
        &state.config.cookie,
        &access_token,
        state.auth.access_token_ttl_seconds(),
        &refresh_token,
        state.auth.session_ttl_seconds(),
    )?;

    record_audit_sync(
        &state.database,
        state.observability.as_ref(),
        AuditEvent::new("user.login", "user")
            .with_user_id(user.id)
            .with_target_id(user.id)
            .with_details(serde_json::json!({
                "email": user.email,
                "role": user.role,
                "result": "completed",
                "risk_level": "info",
            })),
    )
    .await;

    Ok((headers, Json(user)))
}
