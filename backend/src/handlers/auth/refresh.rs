use super::common::{
    REFRESH_TOKEN_COOKIE_NAME, append_session_cookies, auth_domain_service, ensure_app_installed,
    issue_session_tokens, load_auth_state_snapshot, read_cookie, revoke_token,
};
use crate::db::AppState;
use crate::domain::auth::state_repository::{AuthStateRepository, hash_token};
use crate::error::AppError;
use axum::{extract::State, http::HeaderMap};

pub async fn refresh_session(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<(HeaderMap, ()), AppError> {
    ensure_app_installed(&state).await?;
    let refresh_token =
        read_cookie(&headers, REFRESH_TOKEN_COOKIE_NAME).ok_or(AppError::Unauthorized)?;
    let claims = state
        .auth
        .verify_refresh_token_claims(&refresh_token)
        .map_err(|_| AppError::Unauthorized)?;

    let is_revoked = state
        .auth_state_repository
        .is_token_hash_revoked(&hash_token(&refresh_token))
        .await?;
    if is_revoked {
        return Err(AppError::Unauthorized);
    }

    let snapshot = load_auth_state_snapshot(&state, claims.sub).await?;
    if claims.token_version != snapshot.user_token_version
        || claims.session_epoch != snapshot.session_epoch
    {
        return Err(AppError::Unauthorized);
    }

    let auth_domain_service = auth_domain_service(&state)?;
    let user = auth_domain_service.get_current_user(claims.sub).await?;

    revoke_token(&state, &refresh_token).await?;
    let (access_token, rotated_refresh_token) =
        issue_session_tokens(&state, user.id, &user.email, &user.role).await?;

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
