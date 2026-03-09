use super::common::{
    REFRESH_TOKEN_COOKIE_NAME, append_cleared_session_cookies, auth_domain_service, read_cookie,
};
use crate::audit::log_audit;
use crate::db::AppState;
use crate::error::AppError;
use crate::middleware::AuthUser;
use crate::models::UserResponse;
use axum::{extract::State, http::HeaderMap};
use redis::AsyncCommands;
use tracing::info;

pub async fn get_current_user(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<UserResponse, AppError> {
    let auth_domain_service = auth_domain_service(&state)?;
    auth_domain_service.get_current_user(auth_user.id).await
}

pub async fn logout(
    headers: HeaderMap,
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<(HeaderMap, ()), AppError> {
    info!("User logged out: {}", auth_user.username);

    let ttl = state
        .auth
        .token_ttl_seconds(&auth_user.token)
        .unwrap_or(7 * 24 * 3600)
        .max(1);
    let mut redis = state.redis.clone();
    let revoked_key = format!("token_revoked:{}", auth_user.token);
    let _: () = redis.set_ex(revoked_key, "1", ttl).await?;
    if let Some(refresh_token) = read_cookie(&headers, REFRESH_TOKEN_COOKIE_NAME) {
        let refresh_ttl = state
            .auth
            .token_ttl_seconds(&refresh_token)
            .unwrap_or(1)
            .max(1);
        let refresh_revoked_key = format!("token_revoked:{}", refresh_token);
        let _: () = redis.set_ex(refresh_revoked_key, "1", refresh_ttl).await?;
    }

    log_audit(
        &state.pool,
        Some(auth_user.id),
        "user.logout",
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
