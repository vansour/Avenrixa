use super::common::{
    REFRESH_TOKEN_COOKIE_NAME, append_session_cookies, auth_domain_service, read_cookie,
};
use crate::db::AppState;
use crate::domain::auth::user_token_version_key;
use crate::error::AppError;
use axum::{extract::State, http::HeaderMap};
use redis::AsyncCommands;

pub async fn refresh_session(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<(HeaderMap, ()), AppError> {
    let refresh_token =
        read_cookie(&headers, REFRESH_TOKEN_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let claims = state
        .auth
        .verify_refresh_token_claims(&refresh_token)
        .map_err(|_| AppError::Unauthorized)?;

    let mut redis = state.redis.clone();
    let revoked_key = format!("token_revoked:{}", refresh_token);
    let is_revoked: bool = redis.exists(&revoked_key).await?;
    if is_revoked {
        return Err(AppError::Unauthorized);
    }

    let current_token_version = redis
        .get::<_, Option<u64>>(user_token_version_key(claims.sub))
        .await?
        .unwrap_or(0);
    if claims.token_version < current_token_version {
        return Err(AppError::Unauthorized);
    }

    let auth_domain_service = auth_domain_service(&state)?;
    let user = auth_domain_service.get_current_user(claims.sub).await?;

    let refresh_ttl = state
        .auth
        .token_ttl_seconds(&refresh_token)
        .unwrap_or(1)
        .max(1);
    let _: () = redis.set_ex(revoked_key, "1", refresh_ttl).await?;

    let access_token = state.auth.generate_access_token(
        user.id,
        &user.username,
        &user.role,
        current_token_version,
    )?;
    let rotated_refresh_token = state
        .auth
        .generate_refresh_token(user.id, current_token_version)?;

    let mut response_headers = HeaderMap::new();
    append_session_cookies(
        &mut response_headers,
        &state.config.cookie,
        &access_token,
        state.auth.access_token_ttl_seconds(),
        &rotated_refresh_token,
        state.auth.session_ttl_seconds(),
    )?;

    Ok((response_headers, ()))
}
