//! 认证数据访问 trait
//!
//! 定义用户和认证相关的数据访问接口

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::User;

#[derive(Debug, Clone)]
pub enum PasswordResetStatus {
    Applied(User),
    Expired,
    Invalid,
}

/// 认证数据访问 trait
#[async_trait]
pub trait AuthRepository: Send + Sync {
    /// 根据 ID 查找用户
    async fn find_user_by_id(&self, id: Uuid) -> Result<Option<User>, sqlx::Error>;

    /// 根据用户名查找用户
    async fn find_user_by_username(&self, username: &str) -> Result<Option<User>, sqlx::Error>;

    /// 根据用户名或邮箱查找用户
    async fn find_user_by_identity(&self, identity: &str) -> Result<Option<User>, sqlx::Error>;

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

#[async_trait]
impl AuthRepository for PostgresAuthRepository {
    async fn find_user_by_id(&self, id: Uuid) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "SELECT id, username, email, password_hash, role, created_at FROM users WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn find_user_by_username(&self, username: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "SELECT id, username, email, password_hash, role, created_at FROM users WHERE username = $1",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
    }

    async fn find_user_by_identity(&self, identity: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "SELECT id, username, email, password_hash, role, created_at
             FROM users
             WHERE username = $1 OR email = $1
             LIMIT 1",
        )
        .bind(identity)
        .fetch_optional(&self.pool)
        .await
    }

    async fn create_user(&self, user: &User) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO users (id, username, email, password_hash, role, created_at)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(user.id)
        .bind(&user.username)
        .bind(&user.email)
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
        let record = sqlx::query_as::<_, (Uuid, String, Option<String>, String, String, DateTime<Utc>, DateTime<Utc>, Option<DateTime<Utc>>)>(
            "SELECT u.id, u.username, u.email, u.password_hash, u.role, u.created_at, t.expires_at, t.used_at
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
            username,
            email,
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
            username,
            email,
            password_hash: password_hash.to_string(),
            role,
            created_at,
        }))
    }
}
