use super::common::{append_session_cookies, auth_domain_service, ensure_app_installed};
use crate::audit::log_audit_db;
use crate::db::AppState;
use crate::domain::auth::user_token_version_key;
use crate::error::AppError;
use crate::models::{LoginRequest, UserResponse};
use axum::{Json, extract::State, http::HeaderMap};
use redis::AsyncCommands;

#[tracing::instrument(skip(state, req), fields(email = %req.email))]
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<(HeaderMap, Json<UserResponse>), AppError> {
    ensure_app_installed(&state).await?;
    let auth_domain_service = auth_domain_service(&state)?;
    let user = auth_domain_service.login(req).await?;

    let mut redis = state.redis.clone();
    let token_version = redis
        .get::<_, Option<u64>>(user_token_version_key(user.id))
        .await?
        .unwrap_or(0);
    let access_token =
        state
            .auth
            .generate_access_token(user.id, &user.email, &user.role, token_version)?;
    let refresh_token = state.auth.generate_refresh_token(user.id, token_version)?;

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
        Some(user.id),
        "user.login",
        "user",
        Some(user.id),
        None,
        Some(serde_json::json!({
            "email": user.email,
            "role": user.role,
            "result": "completed",
            "risk_level": "info",
        })),
    )
    .await;

    Ok((headers, Json(user)))
}
