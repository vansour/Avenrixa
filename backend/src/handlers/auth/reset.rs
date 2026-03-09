use super::common::{append_cleared_session_cookies, auth_domain_service};
use crate::audit::log_audit;
use crate::db::AppState;
use crate::domain::auth::user_token_version_key;
use crate::error::AppError;
use crate::models::{PasswordResetConfirmRequest, PasswordResetRequest};
use axum::{Json, extract::State, http::HeaderMap};
use lettre::message::Mailbox;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use redis::AsyncCommands;

fn reset_link(base_url: &str, token: &str) -> String {
    let separator = if base_url.contains('?') { '&' } else { '?' };
    format!("{}{}token={}", base_url, separator, token)
}

async fn send_password_reset_mail(
    state: &AppState,
    recipient_email: &str,
    recipient_name: &str,
    token: &str,
) -> Result<(), AppError> {
    if !state.config.mail.enabled {
        return Err(AppError::MailServiceNotEnabled);
    }

    let from = Mailbox::new(
        Some(state.config.mail.from_name.clone()),
        state.config.mail.from_email.parse().map_err(|error| {
            AppError::Internal(anyhow::anyhow!("invalid from email: {}", error))
        })?,
    );
    let to = Mailbox::new(
        Some(recipient_name.to_string()),
        recipient_email.parse().map_err(|error| {
            AppError::Internal(anyhow::anyhow!("invalid recipient email: {}", error))
        })?,
    );
    let reset_url = reset_link(&state.config.mail.reset_link_base_url, token);
    let body = format!(
        "你好，{}\n\n请访问以下链接重置密码：\n{}\n\n如果不是你发起的请求，请忽略这封邮件。",
        recipient_name, reset_url
    );

    let email = Message::builder()
        .from(from)
        .to(to)
        .subject("Vansour Image 密码重置")
        .body(body)
        .map_err(|error| {
            AppError::Internal(anyhow::anyhow!("build reset email failed: {}", error))
        })?;

    let mut builder =
        AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&state.config.mail.smtp_host)
            .port(state.config.mail.smtp_port);
    if let (Some(user), Some(password)) = (
        state.config.mail.smtp_user.clone(),
        state.config.mail.smtp_password.clone(),
    ) {
        builder = builder.credentials(Credentials::new(user, password));
    }

    builder.build().send(email).await.map_err(|error| {
        AppError::Internal(anyhow::anyhow!("send reset email failed: {}", error))
    })?;

    Ok(())
}

pub async fn request_password_reset(
    State(state): State<AppState>,
    Json(req): Json<PasswordResetRequest>,
) -> Result<(), AppError> {
    let auth_domain_service = auth_domain_service(&state)?;
    let Some(dispatch) = auth_domain_service
        .request_password_reset(&req.identity)
        .await?
    else {
        return Ok(());
    };

    send_password_reset_mail(&state, &dispatch.email, &dispatch.username, &dispatch.token).await?;

    log_audit(
        &state.pool,
        Some(dispatch.user_id),
        "user.password_reset_requested",
        "user",
        Some(dispatch.user_id),
        None,
        None,
    )
    .await;

    Ok(())
}

pub async fn confirm_password_reset(
    State(state): State<AppState>,
    Json(req): Json<PasswordResetConfirmRequest>,
) -> Result<(HeaderMap, ()), AppError> {
    let auth_domain_service = auth_domain_service(&state)?;
    let user = auth_domain_service
        .reset_password_by_token(&req.token, &req.new_password)
        .await?;

    let mut redis = state.redis.clone();
    let _: u64 = redis.incr(user_token_version_key(user.id), 1_u64).await?;

    log_audit(
        &state.pool,
        Some(user.id),
        "user.password_reset_completed",
        "user",
        Some(user.id),
        None,
        None,
    )
    .await;

    let mut headers = HeaderMap::new();
    append_cleared_session_cookies(&mut headers, &state.config.cookie)?;

    Ok((headers, ()))
}
