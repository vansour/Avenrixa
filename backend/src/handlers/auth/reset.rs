use super::common::{
    append_cleared_session_cookies, append_query_params, auth_domain_service, ensure_app_installed,
    load_mail_runtime_config, send_text_mail_with_config,
};
use crate::audit::{AuditEvent, record_audit_sync};
use crate::db::AppState;
use crate::domain::auth::state_repository::AuthStateRepository;
use crate::error::AppError;
use crate::models::{PasswordResetConfirmRequest, PasswordResetRequest};
use axum::{Json, extract::State, http::HeaderMap};

fn reset_link(base_url: &str, token: &str) -> String {
    append_query_params(base_url, &[("token", token)])
}

async fn send_password_reset_mail(
    state: &AppState,
    recipient_email: &str,
    recipient_name: &str,
    token: &str,
) -> Result<(), AppError> {
    let mail = load_mail_runtime_config(state).await?;
    let reset_url = reset_link(&mail.link_base_url, token);
    let body = format!(
        "你好，{}\n\n请访问以下链接重置密码：\n{}\n\n如果不是你发起的请求，请忽略这封邮件。",
        recipient_name, reset_url
    );
    send_text_mail_with_config(
        &mail,
        recipient_email,
        recipient_name,
        "Avenrixa 密码重置",
        body,
    )
    .await
}

pub async fn request_password_reset(
    State(state): State<AppState>,
    Json(req): Json<PasswordResetRequest>,
) -> Result<(), AppError> {
    ensure_app_installed(&state).await?;
    let auth_domain_service = auth_domain_service(&state)?;
    let Some(dispatch) = auth_domain_service
        .request_password_reset(&req.email)
        .await?
    else {
        return Ok(());
    };

    send_password_reset_mail(&state, &dispatch.email, &dispatch.email, &dispatch.token).await?;

    record_audit_sync(
        &state.database,
        state.observability.as_ref(),
        AuditEvent::new("user.password_reset_requested", "user")
            .with_user_id(dispatch.user_id)
            .with_target_id(dispatch.user_id),
    )
    .await;

    Ok(())
}

pub async fn confirm_password_reset(
    State(state): State<AppState>,
    Json(req): Json<PasswordResetConfirmRequest>,
) -> Result<(HeaderMap, ()), AppError> {
    ensure_app_installed(&state).await?;
    let auth_domain_service = auth_domain_service(&state)?;
    let user = auth_domain_service
        .reset_password_by_token(&req.token, &req.new_password)
        .await?;

    state
        .auth_state_repository
        .bump_user_token_version(user.id)
        .await?;

    record_audit_sync(
        &state.database,
        state.observability.as_ref(),
        AuditEvent::new("user.password_reset_completed", "user")
            .with_user_id(user.id)
            .with_target_id(user.id),
    )
    .await;

    let mut headers = HeaderMap::new();
    append_cleared_session_cookies(&mut headers, &state.config.cookie)?;

    Ok((headers, ()))
}
