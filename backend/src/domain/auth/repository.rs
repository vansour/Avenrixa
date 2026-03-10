//! 认证数据访问 trait
//!
//! 定义用户和认证相关的数据访问接口

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, SqlitePool};
use uuid::Uuid;

use crate::models::User;

#[derive(Debug, Clone)]
pub enum PasswordResetStatus {
    Applied(User),
    Expired,
    Invalid,
}

#[derive(Debug, Clone)]
pub enum EmailVerificationStatus {
    Applied(User),
    Expired,
    Invalid,
}

/// 认证数据访问 trait
#[async_trait]
pub trait AuthRepository: Send + Sync {
    /// 根据 ID 查找用户
    async fn find_user_by_id(&self, id: Uuid) -> Result<Option<User>, sqlx::Error>;

    /// 根据邮箱查找用户
    async fn find_user_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error>;

    /// 创建用户
    async fn create_user(&self, user: &User) -> Result<(), sqlx::Error>;

    /// 更新用户密码
    async fn update_user_password(
        &self,
        user_id: Uuid,
        password_hash: &str,
    ) -> Result<(), sqlx::Error>;

    async fn store_password_reset_token(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error>;

    async fn reset_password_by_token(
        &self,
        token_hash: &str,
        password_hash: &str,
    ) -> Result<PasswordResetStatus, sqlx::Error>;

    async fn store_email_verification_token(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error>;

    async fn verify_email_by_token(
        &self,
        token_hash: &str,
    ) -> Result<EmailVerificationStatus, sqlx::Error>;
}

/// PostgreSQL 认证仓库实现
pub struct PostgresAuthRepository {
    pool: PgPool,
}

impl PostgresAuthRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// SQLite 认证仓库实现
pub struct SqliteAuthRepository {
    pool: SqlitePool,
}

impl SqliteAuthRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

pub enum DatabaseAuthRepository {
    Postgres(PostgresAuthRepository),
    Sqlite(SqliteAuthRepository),
}

type TokenUserRecord = (
    Uuid,
    String,
    Option<DateTime<Utc>>,
    String,
    String,
    DateTime<Utc>,
    DateTime<Utc>,
    Option<DateTime<Utc>>,
);

#[async_trait]
impl AuthRepository for PostgresAuthRepository {
    async fn find_user_by_id(&self, id: Uuid) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "SELECT id, email, email_verified_at, password_hash, role, created_at
             FROM users
             WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn find_user_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "SELECT id, email, email_verified_at, password_hash, role, created_at
             FROM users
             WHERE email = $1
             LIMIT 1",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
    }

    async fn create_user(&self, user: &User) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO users (id, email, email_verified_at, password_hash, role, created_at)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(user.id)
        .bind(&user.email)
        .bind(user.email_verified_at)
        .bind(&user.password_hash)
        .bind(&user.role)
        .bind(user.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn update_user_password(
        &self,
        user_id: Uuid,
        password_hash: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
            .bind(password_hash)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn store_password_reset_token(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM password_reset_tokens WHERE user_id = $1 AND used_at IS NULL")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO password_reset_tokens (id, user_id, token_hash, expires_at, created_at)
             VALUES ($1, $2, $3, $4, NOW())",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn reset_password_by_token(
        &self,
        token_hash: &str,
        password_hash: &str,
    ) -> Result<PasswordResetStatus, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let record = sqlx::query_as::<_, (Uuid, String, Option<DateTime<Utc>>, String, String, DateTime<Utc>, DateTime<Utc>, Option<DateTime<Utc>>)>(
            "SELECT u.id, u.email, u.email_verified_at, u.password_hash, u.role, u.created_at, t.expires_at, t.used_at
             FROM password_reset_tokens t
             INNER JOIN users u ON u.id = t.user_id
             WHERE t.token_hash = $1
             ORDER BY t.created_at DESC
             LIMIT 1
             FOR UPDATE",
        )
        .bind(token_hash)
        .fetch_optional(&mut *tx)
        .await?;

        let Some((
            id,
            email,
            email_verified_at,
            _existing_password_hash,
            role,
            created_at,
            expires_at,
            used_at,
        )) = record
        else {
            return Ok(PasswordResetStatus::Invalid);
        };

        if used_at.is_some() {
            return Ok(PasswordResetStatus::Invalid);
        }
        if expires_at < Utc::now() {
            return Ok(PasswordResetStatus::Expired);
        }

        sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
            .bind(password_hash)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE password_reset_tokens SET used_at = NOW() WHERE token_hash = $1")
            .bind(token_hash)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;

        Ok(PasswordResetStatus::Applied(User {
            id,
            email,
            email_verified_at,
            password_hash: password_hash.to_string(),
            role,
            created_at,
        }))
    }

    async fn store_email_verification_token(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM email_verification_tokens WHERE user_id = $1 AND used_at IS NULL")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO email_verification_tokens (id, user_id, token_hash, expires_at, created_at)
             VALUES ($1, $2, $3, $4, NOW())",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn verify_email_by_token(
        &self,
        token_hash: &str,
    ) -> Result<EmailVerificationStatus, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let record = sqlx::query_as::<_, (Uuid, String, Option<DateTime<Utc>>, String, String, DateTime<Utc>, DateTime<Utc>, Option<DateTime<Utc>>)>(
            "SELECT u.id, u.email, u.email_verified_at, u.password_hash, u.role, u.created_at, t.expires_at, t.used_at
             FROM email_verification_tokens t
             INNER JOIN users u ON u.id = t.user_id
             WHERE t.token_hash = $1
             ORDER BY t.created_at DESC
             LIMIT 1
             FOR UPDATE",
        )
        .bind(token_hash)
        .fetch_optional(&mut *tx)
        .await?;

        let Some((
            id,
            email,
            email_verified_at,
            password_hash,
            role,
            created_at,
            expires_at,
            used_at,
        )) = record
        else {
            return Ok(EmailVerificationStatus::Invalid);
        };

        if used_at.is_some() {
            return Ok(EmailVerificationStatus::Invalid);
        }
        if expires_at < Utc::now() {
            return Ok(EmailVerificationStatus::Expired);
        }

        let verified_at = email_verified_at.unwrap_or_else(Utc::now);
        sqlx::query("UPDATE users SET email_verified_at = $1 WHERE id = $2")
            .bind(verified_at)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE email_verification_tokens SET used_at = NOW() WHERE token_hash = $1")
            .bind(token_hash)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;

        Ok(EmailVerificationStatus::Applied(User {
            id,
            email,
            email_verified_at: Some(verified_at),
            password_hash,
            role,
            created_at,
        }))
    }
}

#[async_trait]
impl AuthRepository for SqliteAuthRepository {
    async fn find_user_by_id(&self, id: Uuid) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "SELECT id, email, email_verified_at, password_hash, role, created_at
             FROM users
             WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn find_user_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "SELECT id, email, email_verified_at, password_hash, role, created_at
             FROM users
             WHERE email = ?1
             LIMIT 1",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
    }

    async fn create_user(&self, user: &User) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO users (id, email, email_verified_at, password_hash, role, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(user.id)
        .bind(&user.email)
        .bind(user.email_verified_at)
        .bind(&user.password_hash)
        .bind(&user.role)
        .bind(user.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn update_user_password(
        &self,
        user_id: Uuid,
        password_hash: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE users SET password_hash = ?1 WHERE id = ?2")
            .bind(password_hash)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn store_password_reset_token(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM password_reset_tokens WHERE user_id = ?1 AND used_at IS NULL")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO password_reset_tokens (id, user_id, token_hash, expires_at, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .bind(Utc::now())
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn reset_password_by_token(
        &self,
        token_hash: &str,
        password_hash: &str,
    ) -> Result<PasswordResetStatus, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let record = sqlx::query_as::<_, TokenUserRecord>(
            "SELECT u.id, u.email, u.email_verified_at, u.password_hash, u.role, u.created_at, t.expires_at, t.used_at
             FROM password_reset_tokens t
             INNER JOIN users u ON u.id = t.user_id
             WHERE t.token_hash = ?1
             ORDER BY t.created_at DESC
             LIMIT 1",
        )
        .bind(token_hash)
        .fetch_optional(&mut *tx)
        .await?;

        let Some((
            id,
            email,
            email_verified_at,
            _existing_password_hash,
            role,
            created_at,
            expires_at,
            used_at,
        )) = record
        else {
            return Ok(PasswordResetStatus::Invalid);
        };

        if used_at.is_some() {
            return Ok(PasswordResetStatus::Invalid);
        }
        if expires_at < Utc::now() {
            return Ok(PasswordResetStatus::Expired);
        }

        let mark_used = sqlx::query(
            "UPDATE password_reset_tokens
             SET used_at = ?1
             WHERE token_hash = ?2 AND used_at IS NULL",
        )
        .bind(Utc::now())
        .bind(token_hash)
        .execute(&mut *tx)
        .await?;
        if mark_used.rows_affected() != 1 {
            return Ok(PasswordResetStatus::Invalid);
        }

        sqlx::query("UPDATE users SET password_hash = ?1 WHERE id = ?2")
            .bind(password_hash)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;

        Ok(PasswordResetStatus::Applied(User {
            id,
            email,
            email_verified_at,
            password_hash: password_hash.to_string(),
            role,
            created_at,
        }))
    }

    async fn store_email_verification_token(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM email_verification_tokens WHERE user_id = ?1 AND used_at IS NULL")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO email_verification_tokens (id, user_id, token_hash, expires_at, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .bind(Utc::now())
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn verify_email_by_token(
        &self,
        token_hash: &str,
    ) -> Result<EmailVerificationStatus, sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        let record = sqlx::query_as::<_, TokenUserRecord>(
            "SELECT u.id, u.email, u.email_verified_at, u.password_hash, u.role, u.created_at, t.expires_at, t.used_at
             FROM email_verification_tokens t
             INNER JOIN users u ON u.id = t.user_id
             WHERE t.token_hash = ?1
             ORDER BY t.created_at DESC
             LIMIT 1",
        )
        .bind(token_hash)
        .fetch_optional(&mut *tx)
        .await?;

        let Some((
            id,
            email,
            email_verified_at,
            password_hash,
            role,
            created_at,
            expires_at,
            used_at,
        )) = record
        else {
            return Ok(EmailVerificationStatus::Invalid);
        };

        if used_at.is_some() {
            return Ok(EmailVerificationStatus::Invalid);
        }
        if expires_at < Utc::now() {
            return Ok(EmailVerificationStatus::Expired);
        }

        let verified_at = email_verified_at.unwrap_or_else(Utc::now);
        let mark_used = sqlx::query(
            "UPDATE email_verification_tokens
             SET used_at = ?1
             WHERE token_hash = ?2 AND used_at IS NULL",
        )
        .bind(Utc::now())
        .bind(token_hash)
        .execute(&mut *tx)
        .await?;
        if mark_used.rows_affected() != 1 {
            return Ok(EmailVerificationStatus::Invalid);
        }

        sqlx::query("UPDATE users SET email_verified_at = ?1 WHERE id = ?2")
            .bind(verified_at)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;

        Ok(EmailVerificationStatus::Applied(User {
            id,
            email,
            email_verified_at: Some(verified_at),
            password_hash,
            role,
            created_at,
        }))
    }
}

#[async_trait]
impl AuthRepository for DatabaseAuthRepository {
    async fn find_user_by_id(&self, id: Uuid) -> Result<Option<User>, sqlx::Error> {
        match self {
            Self::Postgres(repo) => repo.find_user_by_id(id).await,
            Self::Sqlite(repo) => repo.find_user_by_id(id).await,
        }
    }

    async fn find_user_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error> {
        match self {
            Self::Postgres(repo) => repo.find_user_by_email(email).await,
            Self::Sqlite(repo) => repo.find_user_by_email(email).await,
        }
    }

    async fn create_user(&self, user: &User) -> Result<(), sqlx::Error> {
        match self {
            Self::Postgres(repo) => repo.create_user(user).await,
            Self::Sqlite(repo) => repo.create_user(user).await,
        }
    }

    async fn update_user_password(
        &self,
        user_id: Uuid,
        password_hash: &str,
    ) -> Result<(), sqlx::Error> {
        match self {
            Self::Postgres(repo) => repo.update_user_password(user_id, password_hash).await,
            Self::Sqlite(repo) => repo.update_user_password(user_id, password_hash).await,
        }
    }

    async fn store_password_reset_token(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        match self {
            Self::Postgres(repo) => {
                repo.store_password_reset_token(user_id, token_hash, expires_at)
                    .await
            }
            Self::Sqlite(repo) => {
                repo.store_password_reset_token(user_id, token_hash, expires_at)
                    .await
            }
        }
    }

    async fn reset_password_by_token(
        &self,
        token_hash: &str,
        password_hash: &str,
    ) -> Result<PasswordResetStatus, sqlx::Error> {
        match self {
            Self::Postgres(repo) => {
                repo.reset_password_by_token(token_hash, password_hash)
                    .await
            }
            Self::Sqlite(repo) => {
                repo.reset_password_by_token(token_hash, password_hash)
                    .await
            }
        }
    }

    async fn store_email_verification_token(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        match self {
            Self::Postgres(repo) => {
                repo.store_email_verification_token(user_id, token_hash, expires_at)
                    .await
            }
            Self::Sqlite(repo) => {
                repo.store_email_verification_token(user_id, token_hash, expires_at)
                    .await
            }
        }
    }

    async fn verify_email_by_token(
        &self,
        token_hash: &str,
    ) -> Result<EmailVerificationStatus, sqlx::Error> {
        match self {
            Self::Postgres(repo) => repo.verify_email_by_token(token_hash).await,
            Self::Sqlite(repo) => repo.verify_email_by_token(token_hash).await,
        }
    }
}
