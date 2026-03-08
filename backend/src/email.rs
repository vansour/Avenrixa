#![allow(dead_code)]
use crate::config::Config;
use anyhow::Result;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use tracing::{error, info};

/// 邮件服务
pub struct MailService {
    config: Config,
}

impl MailService {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// 发送密码重置邮件
    pub async fn send_password_reset_email(
        &self,
        username: &str,
        token: &str,
    ) -> Result<()> {
        // 如果邮件未启用，只记录日志
        if !self.config.mail.enabled {
            info!("邮件服务未启用，密码重置令牌记录在日志中: {} (用户: {})", token, username);
            info!("DEMO MODE: 重置链接: {}{}", self.config.mail.reset_link_base_url, token);
            return Ok(());
        }

        // 构建重置链接
        let reset_link = format!("{}{}", self.config.mail.reset_link_base_url, token);

        // 构建邮件内容
        let email_content = self.build_reset_email(username, &reset_link);

        // 发送邮件
        self.send_email(
            &self.config.mail.from_email,
            username,
            "重置您的密码",
            &email_content,
        ).await
    }

    /// 构建密码重置邮件内容
    fn build_reset_email(&self, username: &str, reset_link: &str) -> String {
        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>重置密码</title>
    <style>
        body {{ font-family: Arial, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px; }}
        .container {{ border: 1px solid #ddd; border-radius: 8px; padding: 30px; background: #f9f9f9; }}
        .header {{ text-align: center; margin-bottom: 30px; }}
        .logo {{ font-size: 24px; font-weight: bold; color: #333; }}
        .content {{ background: white; padding: 30px; border-radius: 6px; }}
        .greeting {{ font-size: 18px; margin-bottom: 20px; }}
        .button {{ display: inline-block; background: #007bff; color: white; padding: 12px 30px; text-decoration: none; border-radius: 4px; font-weight: bold; margin: 20px 0; }}
        .button:hover {{ background: #0056b3; }}
        .info {{ color: #666; font-size: 14px; margin-top: 20px; }}
        .footer {{ text-align: center; margin-top: 30px; color: #999; font-size: 12px; }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <div class="logo">{}</div>
        </div>
        <div class="content">
            <p class="greeting">你好，{}</p>
            <p>我们收到了您重置密码的请求。如果这不是您的操作，请忽略此邮件。</p>
            <p>点击下面的按钮重置您的密码：</p>
            <a href="{}" class="button">重置密码</a>
            <p class="info">此链接将在 1 小时后过期</p>
        </div>
        <div class="footer">
            <p>如果您没有请求重置密码，请联系我们的支持团队。</p>
        </div>
    </div>
</body>
</html>"#,
            self.config.mail.from_name, username, reset_link
        )
    }

    /// 发送邮件
    pub async fn send_email(
        &self,
        from_email: &str,
        to_email: &str,
        subject: &str,
        html_body: &str,
    ) -> Result<()> {
        let from = format!("{} <{}>", self.config.mail.from_name, from_email);

        info!("发送邮件: {} -> {}", from, to_email);

        // 检查是否配置了 SMTP 凭证
        let smtp_user = self.config.mail.smtp_user.as_ref();
        let smtp_password = self.config.mail.smtp_password.as_ref();

        match (smtp_user, smtp_password) {
            (Some(user), Some(pass)) => {
                // 使用 SMTP 发送
                self.send_via_smtp(user, pass, &from, to_email, subject, html_body).await?;
            }
            _ => {
                // 未配置 SMTP，记录日志并返回错误
                info!("SMTP 未配置，邮件未发送。请配置 SMTP_USER 和 SMTP_PASSWORD 环境变量");
                anyhow::bail!("SMTP 未配置，无法发送邮件");
            }
        }

        Ok(())
    }

    /// 通过 SMTP 发送邮件
    async fn send_via_smtp(
        &self,
        user: &str,
        pass: &str,
        from: &str,
        to: &str,
        subject: &str,
        html_body: &str,
    ) -> Result<()> {
        // 构建邮件
        let email = Message::builder()
            .from(from.parse()?)
            .to(to.parse()?)
            .subject(subject)
            .body(html_body.to_string())?;

        // 配置 SMTP 传输
        let creds = Credentials::new(user.to_string(), pass.to_string());

        let mailer = SmtpTransport::relay(&self.config.mail.smtp_host)?
            .credentials(creds)
            .port(self.config.mail.smtp_port)
            .build();

        // 发送邮件
        match mailer.send(&email) {
            Ok(_) => {
                info!("邮件已通过 SMTP 发送成功: {} -> {}", from, to);
                Ok(())
            }
            Err(e) => {
                error!("SMTP 发送失败: {}", e);
                anyhow::bail!("SMTP 发送失败: {}", e)
            }
        }
    }

    /// 通过 HTTP API 发送邮件（预留接口）
    #[allow(unused_variables, dead_code)]
    async fn send_via_http_api(
        &self,
        from: String,
        to: &str,
        subject: &str,
        html_body: &str,
    ) -> Result<()> {
        // TODO: 实现第三方邮件服务集成（SendGrid、Mailgun、AWS SES 等）
        // 目前仅记录日志
        info!("HTTP API 邮件发送（未实现）: {} -> {}", from, to);
        info!("主题: {}", subject);
        info!("内容预览: {}...", &html_body[..html_body.len().min(100)]);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_reset_email() {
        let config = Config::default();
        let mail_service = MailService::new(config);

        let email = mail_service.build_reset_email("testuser", "http://example.com/reset?token=abc123");

        assert!(email.contains("testuser"));
        assert!(email.contains("http://example.com/reset?token=abc123"));
        assert!(email.contains("重置密码"));
    }

    #[test]
    fn test_build_reset_email_with_special_chars() {
        let config = Config::default();
        let mail_service = MailService::new(config);

        let email = mail_service.build_reset_email("Test User <test@example.com>", "token&with=special&chars");
        assert!(email.contains("Test User"));
        assert!(email.contains("token"));
    }
}
