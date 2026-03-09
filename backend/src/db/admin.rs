use crate::auth::AuthService;
use sqlx::{PgPool, Row};
use tracing::{error, info};
use uuid::Uuid;

pub const ADMIN_USER_ID: Uuid = uuid::uuid!("00000000-0000-0000-0000-000000000001");
pub const DEFAULT_ADMIN_USERNAME: &str = "admin";
const MIN_ADMIN_PASSWORD_LENGTH: usize = 12;

pub struct AdminAccountInit {
    pub username: String,
    pub created: bool,
}

struct AdminBootstrapConfig {
    username: String,
    email: Option<String>,
    password: String,
}

fn validate_admin_bootstrap_config(
    username: Option<String>,
    email: Option<String>,
    password: Option<String>,
) -> Result<AdminBootstrapConfig, anyhow::Error> {
    let username = username
        .unwrap_or_else(|| DEFAULT_ADMIN_USERNAME.to_string())
        .trim()
        .to_string();
    if username.is_empty() {
        return Err(anyhow::anyhow!(
            "ADMIN_USERNAME cannot be empty when bootstrapping the admin account"
        ));
    }

    let email = email
        .map(|email| email.trim().to_string())
        .filter(|email| !email.is_empty());

    let password = password
        .ok_or_else(|| {
            anyhow::anyhow!(
                "ADMIN_PASSWORD must be set before the first startup to bootstrap the admin account"
            )
        })?
        .trim()
        .to_string();
    if password.is_empty() {
        return Err(anyhow::anyhow!(
            "ADMIN_PASSWORD cannot be empty when bootstrapping the admin account"
        ));
    }
    if password.len() < MIN_ADMIN_PASSWORD_LENGTH {
        return Err(anyhow::anyhow!(
            "ADMIN_PASSWORD must be at least {} characters long",
            MIN_ADMIN_PASSWORD_LENGTH
        ));
    }
    if password.eq_ignore_ascii_case("password") {
        return Err(anyhow::anyhow!(
            "Refusing to bootstrap admin account with a default or weak password"
        ));
    }

    Ok(AdminBootstrapConfig {
        username,
        email,
        password,
    })
}

fn resolve_admin_bootstrap_config() -> Result<AdminBootstrapConfig, anyhow::Error> {
    validate_admin_bootstrap_config(
        std::env::var("ADMIN_USERNAME").ok(),
        std::env::var("ADMIN_EMAIL").ok(),
        std::env::var("ADMIN_PASSWORD").ok(),
    )
}

pub async fn create_admin_account(pool: &PgPool) -> Result<AdminAccountInit, anyhow::Error> {
    let existing = sqlx::query("SELECT username FROM users WHERE id = $1")
        .bind(ADMIN_USER_ID)
        .fetch_optional(pool)
        .await?;

    if let Some(row) = existing {
        let username: String = row.get("username");
        if let Ok(email) = std::env::var("ADMIN_EMAIL") {
            let trimmed = email.trim();
            if !trimmed.is_empty() {
                sqlx::query(
                    "UPDATE users SET email = $1 WHERE id = $2 AND (email IS NULL OR email = '')",
                )
                .bind(trimmed)
                .bind(ADMIN_USER_ID)
                .execute(pool)
                .await?;
            }
        }
        info!("Using existing admin account: {}", username);
        return Ok(AdminAccountInit {
            username,
            created: false,
        });
    }

    let bootstrap = resolve_admin_bootstrap_config()?;
    let password_hash = AuthService::hash_password(&bootstrap.password)?;

    let result = sqlx::query(
        "INSERT INTO users (id, username, email, password_hash, role, created_at)
         VALUES ($1, $2, $3, $4, 'admin', NOW())
         ON CONFLICT (id) DO NOTHING
         RETURNING username",
    )
    .bind(ADMIN_USER_ID)
    .bind(&bootstrap.username)
    .bind(&bootstrap.email)
    .bind(&password_hash)
    .fetch_optional(pool)
    .await;

    match result {
        Ok(Some(row)) => {
            let username: String = row.get("username");
            info!("Admin account created: {}", username);

            Ok(AdminAccountInit {
                username,
                created: true,
            })
        }
        Ok(None) => {
            let existing = sqlx::query("SELECT username FROM users WHERE id = $1")
                .bind(ADMIN_USER_ID)
                .fetch_optional(pool)
                .await?;

            match existing {
                Some(row) => {
                    let username: String = row.get("username");
                    info!("Using existing admin account: {}", username);
                    Ok(AdminAccountInit {
                        username,
                        created: false,
                    })
                }
                None => {
                    error!("Admin account not found in database");
                    Err(anyhow::anyhow!("Admin account not found"))
                }
            }
        }
        Err(error) => {
            error!("Failed to create admin account: {}", error);
            Err(anyhow::anyhow!("Failed to create admin account: {}", error))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_admin_bootstrap_requires_password() {
        let result = validate_admin_bootstrap_config(Some("admin".to_string()), None, None);
        assert!(result.is_err());
    }

    #[test]
    fn validate_admin_bootstrap_rejects_weak_password() {
        let result = validate_admin_bootstrap_config(
            Some("admin".to_string()),
            None,
            Some("password".to_string()),
        );
        assert!(result.is_err());
    }

    #[test]
    fn validate_admin_bootstrap_accepts_explicit_strong_password() {
        let result = validate_admin_bootstrap_config(
            Some("admin".to_string()),
            Some("admin@example.com".to_string()),
            Some("change-this-admin-password".to_string()),
        )
        .expect("bootstrap config should be valid");

        assert_eq!(result.username, "admin");
        assert_eq!(result.email.as_deref(), Some("admin@example.com"));
        assert_eq!(result.password, "change-this-admin-password");
    }
}
