use super::common::{append_cleared_session_cookies, auth_domain_service};
use crate::audit::log_audit;
use crate::db::AppState;
use crate::domain::auth::user_token_version_key;
use crate::error::AppError;
use crate::middleware::AuthUser;
use crate::models::UpdateProfileRequest;
use axum::{Json, extract::State, http::HeaderMap};
use redis::AsyncCommands;

pub async fn change_password(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<(HeaderMap, ()), AppError> {
    let auth_domain_service = auth_domain_service(&state)?;

    auth_domain_service
        .change_password(auth_user.id, req)
        .await?;

    let ttl = state
        .auth
        .token_ttl_seconds(&auth_user.token)
        .unwrap_or(7 * 24 * 3600)
        .max(1);
    let mut redis = state.redis.clone();
    let revoked_key = format!("token_revoked:{}", auth_user.token);
    let _: () = redis.set_ex(revoked_key, "1", ttl).await?;
    let _: u64 = redis
        .incr(user_token_version_key(auth_user.id), 1_u64)
        .await?;

    log_audit(
        &state.pool,
        Some(auth_user.id),
        "user.password_changed",
        "user",
        Some(auth_user.id),
        None,
        None,
    )
    .await;

    let mut headers = HeaderMap::new();
    append_cleared_session_cookies(&mut headers, &state.config.cookie)?;

    Ok((headers, ()))
}
