use super::common::{append_cleared_session_cookies, auth_domain_service, revoke_token};
use crate::audit::log_audit_db;
use crate::db::AppState;
use crate::domain::auth::state_repository::AuthStateRepository;
use crate::error::AppError;
use crate::middleware::AuthUser;
use crate::models::UpdateProfileRequest;
use axum::{Json, extract::State, http::HeaderMap};

pub async fn change_password(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<(HeaderMap, ()), AppError> {
    let auth_domain_service = auth_domain_service(&state)?;

    auth_domain_service
        .change_password(auth_user.id, req)
        .await?;

    revoke_token(&state, &auth_user.token).await?;
    state
        .auth_state_repository
        .bump_user_token_version(auth_user.id)
        .await?;

    log_audit_db(
        &state.database,
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
