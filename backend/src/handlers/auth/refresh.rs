use super::common::{
    REFRESH_TOKEN_COOKIE_NAME, append_session_cookies, auth_domain_service, ensure_app_installed,
    issue_session_tokens, load_auth_state_snapshot, read_cookie, revoke_token,
};
use crate::db::AppState;
use crate::domain::auth::state_repository::{AuthStateRepository, hash_token};
use crate::error::AppError;
use axum::{extract::State, http::HeaderMap};
use std::time::Instant;

pub async fn refresh_session(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<(HeaderMap, ()), AppError> {
    let started_at = Instant::now();
    let result: Result<(HeaderMap, ()), (&'static str, AppError)> = async {
        ensure_app_installed(&state)
            .await
            .map_err(|error| ("ensure_app_installed_failed", error))?;

        let refresh_token = read_cookie(&headers, REFRESH_TOKEN_COOKIE_NAME)
            .ok_or(("missing_refresh_cookie", AppError::Unauthorized))?;
        let claims = state
            .auth
            .verify_refresh_token_claims(&refresh_token)
            .map_err(|_| ("invalid_refresh_token", AppError::Unauthorized))?;

        let is_revoked = state
            .auth_state_repository
            .is_token_hash_revoked(&hash_token(&refresh_token))
            .await
            .map_err(|error| ("refresh_revocation_lookup_failed", error.into()))?;
        if is_revoked {
            return Err(("refresh_token_revoked", AppError::Unauthorized));
        }

        let snapshot = load_auth_state_snapshot(&state, claims.sub)
            .await
            .map_err(|error| ("auth_state_snapshot_failed", error))?;
        if claims.token_version != snapshot.user_token_version
            || claims.session_epoch != snapshot.session_epoch
        {
            return Err(("session_epoch_mismatch", AppError::Unauthorized));
        }

        let auth_domain_service =
            auth_domain_service(&state).map_err(|error| ("auth_service_unavailable", error))?;
        let user = auth_domain_service
            .get_current_user(claims.sub)
            .await
            .map_err(|error| ("load_current_user_failed", error))?;

        revoke_token(&state, &refresh_token)
            .await
            .map_err(|error| ("revoke_old_refresh_failed", error))?;
        let (access_token, rotated_refresh_token) =
            issue_session_tokens(&state, user.id, &user.email, &user.role)
                .await
                .map_err(|error| ("issue_rotated_tokens_failed", error))?;

        let mut response_headers = HeaderMap::new();
        append_session_cookies(
            &mut response_headers,
            &state.config.cookie,
            &access_token,
            state.auth.access_token_ttl_seconds(),
            &rotated_refresh_token,
            state.auth.session_ttl_seconds(),
        )
        .map_err(|error| ("append_refresh_cookies_failed", error))?;

        Ok((response_headers, ()))
    }
    .await;

    match result {
        Ok(response) => {
            state
                .observability
                .record_auth_refresh_success(started_at.elapsed());
            Ok(response)
        }
        Err((reason, error)) => {
            state
                .observability
                .record_auth_refresh_failure(started_at.elapsed(), reason);
            Err(error)
        }
    }
}
