use crate::auth::AuthService;
use crate::models::{User, UserRole};
use chrono::Utc;
use lettre::Address;
use sqlx::{MySql, Postgres, Sqlite, Transaction};
use uuid::Uuid;

use super::DatabasePool;

pub const ADMIN_USER_ID: Uuid = uuid::uuid!("00000000-0000-0000-0000-000000000001");
pub const INSTALL_STATE_SETTING_KEY: &str = "system_installed";
pub const SITE_FAVICON_DATA_URL_SETTING_KEY: &str = "site_favicon_data_url";
pub const INSTALLATION_LOCK_KEY: i64 = 2_024_031_001;
const MIN_ADMIN_PASSWORD_LENGTH: usize = 12;

pub struct AdminBootstrapConfig {
    pub email: String,
    pub password: String,
}

pub fn validate_admin_bootstrap_config(
    email: String,
    password: String,
) -> Result<AdminBootstrapConfig, anyhow::Error> {
    let email = email.trim().to_string();
    if email.is_empty() {
        return Err(anyhow::anyhow!("管理员邮箱不能为空"));
    }
    email
        .parse::<Address>()
        .map_err(|_| anyhow::anyhow!("管理员邮箱格式无效"))?;

    let password = password.trim().to_string();
    if password.is_empty() {
        return Err(anyhow::anyhow!("管理员密码不能为空"));
    }
    if password.len() < MIN_ADMIN_PASSWORD_LENGTH {
        return Err(anyhow::anyhow!(
            "管理员密码至少需要 {} 个字符",
            MIN_ADMIN_PASSWORD_LENGTH
        ));
    }
    if password.eq_ignore_ascii_case("password") {
        return Err(anyhow::anyhow!("管理员密码过弱，请使用更强的密码"));
    }

    Ok(AdminBootstrapConfig { email, password })
}

pub async fn acquire_installation_lock(
    tx: &mut Transaction<'_, Postgres>,
) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(INSTALLATION_LOCK_KEY)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub async fn is_app_installed(pool: &DatabasePool) -> Result<bool, sqlx::Error> {
    let value = get_setting_value(pool, INSTALL_STATE_SETTING_KEY).await?;
    Ok(matches!(
        value.as_deref().map(str::trim),
        Some("true" | "TRUE" | "True" | "1")
    ))
}

pub async fn is_app_installed_tx(tx: &mut Transaction<'_, Postgres>) -> Result<bool, sqlx::Error> {
    let value = sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = $1")
        .bind(INSTALL_STATE_SETTING_KEY)
        .fetch_optional(&mut **tx)
        .await?;
    Ok(matches!(
        value.as_deref().map(str::trim),
        Some("true" | "TRUE" | "True" | "1")
    ))
}

pub async fn is_app_installed_mysql_tx(
    tx: &mut Transaction<'_, MySql>,
) -> Result<bool, sqlx::Error> {
    let value = sqlx::query_scalar::<_, String>("SELECT `value` FROM settings WHERE `key` = ?")
        .bind(INSTALL_STATE_SETTING_KEY)
        .fetch_optional(&mut **tx)
        .await?;
    Ok(matches!(
        value.as_deref().map(str::trim),
        Some("true" | "TRUE" | "True" | "1")
    ))
}

pub async fn has_admin_account(pool: &DatabasePool) -> Result<bool, sqlx::Error> {
    let exists = match pool {
        DatabasePool::Postgres(pool) => {
            sqlx::query_scalar::<_, i32>(
                "SELECT 1 FROM users WHERE id = $1 AND role = 'admin' LIMIT 1",
            )
            .bind(ADMIN_USER_ID)
            .fetch_optional(pool)
            .await?
        }
        DatabasePool::MySql(pool) => {
            sqlx::query_scalar::<_, i32>(
                "SELECT 1 FROM users WHERE id = ? AND role = 'admin' LIMIT 1",
            )
            .bind(ADMIN_USER_ID)
            .fetch_optional(pool)
            .await?
        }
        DatabasePool::Sqlite(pool) => {
            sqlx::query_scalar::<_, i32>(
                "SELECT 1 FROM users WHERE id = ?1 AND role = 'admin' LIMIT 1",
            )
            .bind(ADMIN_USER_ID)
            .fetch_optional(pool)
            .await?
        }
    };
    Ok(exists.is_some())
}

pub async fn has_admin_account_tx(tx: &mut Transaction<'_, Postgres>) -> Result<bool, sqlx::Error> {
    let exists = sqlx::query_scalar::<_, i32>(
        "SELECT 1 FROM users WHERE id = $1 AND role = 'admin' LIMIT 1",
    )
    .bind(ADMIN_USER_ID)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(exists.is_some())
}

pub async fn has_admin_account_mysql_tx(
    tx: &mut Transaction<'_, MySql>,
) -> Result<bool, sqlx::Error> {
    let exists =
        sqlx::query_scalar::<_, i32>("SELECT 1 FROM users WHERE id = ? AND role = 'admin' LIMIT 1")
            .bind(ADMIN_USER_ID)
            .fetch_optional(&mut **tx)
            .await?;
    Ok(exists.is_some())
}

pub async fn has_admin_account_sqlite_tx(
    tx: &mut Transaction<'_, Sqlite>,
) -> Result<bool, sqlx::Error> {
    let exists = sqlx::query_scalar::<_, i32>(
        "SELECT 1 FROM users WHERE id = ?1 AND role = 'admin' LIMIT 1",
    )
    .bind(ADMIN_USER_ID)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(exists.is_some())
}

pub async fn create_admin_account_tx(
    tx: &mut Transaction<'_, Postgres>,
    email: &str,
    password: &str,
) -> Result<User, anyhow::Error> {
    let password_hash = AuthService::hash_password(password)?;
    let created_at = Utc::now();

    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (id, email, email_verified_at, password_hash, role, created_at)
         VALUES ($1, $2, NOW(), $3, 'admin', $4)
         RETURNING id, email, email_verified_at, password_hash, role, created_at",
    )
    .bind(ADMIN_USER_ID)
    .bind(email)
    .bind(&password_hash)
    .bind(created_at)
    .fetch_one(&mut **tx)
    .await?;

    Ok(user)
}

pub async fn create_admin_account_mysql_tx(
    tx: &mut Transaction<'_, MySql>,
    email: &str,
    password: &str,
) -> Result<User, anyhow::Error> {
    let password_hash = AuthService::hash_password(password)?;
    let created_at = Utc::now();
    let verified_at = Utc::now();

    sqlx::query(
        "INSERT INTO users (id, email, email_verified_at, password_hash, role, created_at)
         VALUES (?, ?, ?, ?, 'admin', ?)",
    )
    .bind(ADMIN_USER_ID)
    .bind(email)
    .bind(verified_at)
    .bind(&password_hash)
    .bind(created_at)
    .execute(&mut **tx)
    .await?;

    Ok(User {
        id: ADMIN_USER_ID,
        email: email.to_string(),
        email_verified_at: Some(verified_at),
        password_hash,
        role: UserRole::Admin,
        created_at,
    })
}

pub async fn create_admin_account_sqlite_tx(
    tx: &mut Transaction<'_, Sqlite>,
    email: &str,
    password: &str,
) -> Result<User, anyhow::Error> {
    let password_hash = AuthService::hash_password(password)?;
    let created_at = Utc::now();
    let verified_at = Utc::now();

    sqlx::query(
        "INSERT INTO users (id, email, email_verified_at, password_hash, role, created_at)
         VALUES (?1, ?2, ?3, ?4, 'admin', ?5)",
    )
    .bind(ADMIN_USER_ID)
    .bind(email)
    .bind(verified_at)
    .bind(&password_hash)
    .bind(created_at)
    .execute(&mut **tx)
    .await?;

    Ok(User {
        id: ADMIN_USER_ID,
        email: email.to_string(),
        email_verified_at: Some(verified_at),
        password_hash,
        role: UserRole::Admin,
        created_at,
    })
}

pub async fn delete_admin_account_tx(
    tx: &mut Transaction<'_, Postgres>,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(ADMIN_USER_ID)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub async fn delete_admin_account_mysql_tx(
    tx: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(ADMIN_USER_ID)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub async fn delete_admin_account_sqlite_tx(
    tx: &mut Transaction<'_, Sqlite>,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM users WHERE id = ?1")
        .bind(ADMIN_USER_ID)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub async fn mark_app_installed_tx(tx: &mut Transaction<'_, Postgres>) -> Result<(), sqlx::Error> {
    upsert_setting_tx(tx, INSTALL_STATE_SETTING_KEY, "true").await
}

pub async fn mark_app_installed_mysql_tx(
    tx: &mut Transaction<'_, MySql>,
) -> Result<(), sqlx::Error> {
    upsert_setting_mysql_tx(tx, INSTALL_STATE_SETTING_KEY, "true").await
}

pub async fn mark_app_installed_sqlite_tx(
    tx: &mut Transaction<'_, Sqlite>,
) -> Result<(), sqlx::Error> {
    upsert_setting_sqlite_tx(tx, INSTALL_STATE_SETTING_KEY, "true").await
}

pub async fn get_setting_value(
    pool: &DatabasePool,
    key: &str,
) -> Result<Option<String>, sqlx::Error> {
    match pool {
        DatabasePool::Postgres(pool) => {
            sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = $1")
                .bind(key)
                .fetch_optional(pool)
                .await
        }
        DatabasePool::MySql(pool) => {
            sqlx::query_scalar::<_, String>("SELECT `value` FROM settings WHERE `key` = ?")
                .bind(key)
                .fetch_optional(pool)
                .await
        }
        DatabasePool::Sqlite(pool) => {
            sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?1")
                .bind(key)
                .fetch_optional(pool)
                .await
        }
    }
}

pub async fn upsert_setting_tx(
    tx: &mut Transaction<'_, Postgres>,
    key: &str,
    value: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO settings (key, value, updated_at)
         VALUES ($1, $2, NOW())
         ON CONFLICT (key)
         DO UPDATE SET value = EXCLUDED.value, updated_at = NOW()",
    )
    .bind(key)
    .bind(value)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn upsert_setting_mysql_tx(
    tx: &mut Transaction<'_, MySql>,
    key: &str,
    value: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO settings (`key`, `value`, updated_at)
         VALUES (?, ?, CURRENT_TIMESTAMP(6))
         ON DUPLICATE KEY UPDATE
             `value` = VALUES(`value`),
             updated_at = CURRENT_TIMESTAMP(6)",
    )
    .bind(key)
    .bind(value)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn upsert_setting_sqlite_tx(
    tx: &mut Transaction<'_, Sqlite>,
    key: &str,
    value: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO settings (key, value, updated_at)
         VALUES (?1, ?2, STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now'))
         ON CONFLICT (key)
         DO UPDATE SET value = excluded.value,
                       updated_at = STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')",
    )
    .bind(key)
    .bind(value)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn delete_setting_tx(
    tx: &mut Transaction<'_, Postgres>,
    key: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM settings WHERE key = $1")
        .bind(key)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub async fn delete_setting_mysql_tx(
    tx: &mut Transaction<'_, MySql>,
    key: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM settings WHERE `key` = ?")
        .bind(key)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub async fn delete_setting_sqlite_tx(
    tx: &mut Transaction<'_, Sqlite>,
    key: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM settings WHERE key = ?1")
        .bind(key)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub async fn is_app_installed_sqlite_tx(
    tx: &mut Transaction<'_, Sqlite>,
) -> Result<bool, sqlx::Error> {
    let value = sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?1")
        .bind(INSTALL_STATE_SETTING_KEY)
        .fetch_optional(&mut **tx)
        .await?;
    Ok(matches!(
        value.as_deref().map(str::trim),
        Some("true" | "TRUE" | "True" | "1")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_admin_bootstrap_requires_password() {
        let result =
            validate_admin_bootstrap_config("admin@example.com".to_string(), String::new());
        assert!(result.is_err());
    }

    #[test]
    fn validate_admin_bootstrap_rejects_weak_password() {
        let result = validate_admin_bootstrap_config(
            "admin@example.com".to_string(),
            "password".to_string(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn validate_admin_bootstrap_accepts_explicit_strong_password() {
        let result = validate_admin_bootstrap_config(
            "admin@example.com".to_string(),
            "change-this-admin-password".to_string(),
        )
        .expect("bootstrap config should be valid");

        assert_eq!(result.email, "admin@example.com");
        assert_eq!(result.password, "change-this-admin-password");
    }
}
