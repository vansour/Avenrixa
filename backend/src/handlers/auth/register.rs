use super::common::{
    append_query_params, auth_domain_service, ensure_app_installed, load_mail_runtime_config,
    send_text_mail_with_config,
};
use crate::audit::{AuditEvent, record_audit_sync};
use crate::db::AppState;
use crate::error::AppError;
use crate::models::{EmailVerificationConfirmRequest, RegisterRequest};
use axum::{Json, extract::State};

fn verification_link(base_url: &str, token: &str) -> String {
    append_query_params(base_url, &[("mode", "verify-email"), ("token", token)])
}

async fn send_verification_mail(
    state: &AppState,
    recipient_email: &str,
    recipient_name: &str,
    token: &str,
) -> Result<(), AppError> {
    let mail = load_mail_runtime_config(state).await?;
    let verify_url = verification_link(&mail.link_base_url, token);
    let body = format!(
        "你好，{}\n\n请访问以下链接完成邮箱验证：\n{}\n\n如果不是你本人注册，请忽略这封邮件。",
        recipient_name, verify_url
    );
    send_text_mail_with_config(
        &mail,
        recipient_email,
        recipient_name,
        "Avenrixa 邮箱验证",
        body,
    )
    .await
}

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<(), AppError> {
    ensure_app_installed(&state).await?;
    let auth_domain_service = auth_domain_service(&state)?;
    let dispatch = auth_domain_service.register(req).await?;
    send_verification_mail(&state, &dispatch.email, &dispatch.email, &dispatch.token).await?;

    record_audit_sync(
        &state.database,
        state.observability.as_ref(),
        AuditEvent::new("user.register_requested", "user")
            .with_user_id(dispatch.user_id)
            .with_target_id(dispatch.user_id),
    )
    .await;

    Ok(())
}

pub async fn verify_registration_email(
    State(state): State<AppState>,
    Json(req): Json<EmailVerificationConfirmRequest>,
) -> Result<(), AppError> {
    ensure_app_installed(&state).await?;
    let auth_domain_service = auth_domain_service(&state)?;
    let user = auth_domain_service.verify_email(&req.token).await?;

    record_audit_sync(
        &state.database,
        state.observability.as_ref(),
        AuditEvent::new("user.email_verified", "user")
            .with_user_id(user.id)
            .with_target_id(user.id),
    )
    .await;

    Ok(())
}
