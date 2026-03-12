use super::common::{
    REFRESH_TOKEN_COOKIE_NAME, append_cleared_session_cookies, auth_domain_service, read_cookie,
    revoke_token,
};
use crate::audit::log_audit_db;
use crate::db::AppState;
use crate::error::AppError;
use crate::middleware::AuthUser;
use crate::models::UserResponse;
use axum::{extract::State, http::HeaderMap};
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
    info!("User logged out: {}", auth_user.email);

    revoke_token(&state, &auth_user.token).await?;
    if let Some(refresh_token) = read_cookie(&headers, REFRESH_TOKEN_COOKIE_NAME) {
        revoke_token(&state, &refresh_token).await?;
    }

    log_audit_db(
        &state.database,
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
