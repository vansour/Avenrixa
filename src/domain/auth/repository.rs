//! 认证数据访问 trait
//!
//! 定义用户和认证相关的数据访问接口

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::User;

/// 密码重置令牌
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PasswordResetToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// 认证数据访问 trait
#[async_trait]
pub trait AuthRepository: Send + Sync {
    /// 根据 ID 查找用户
    async fn find_user_by_id(&self, id: Uuid) -> Result<Option<User>, sqlx::Error>;

    /// 根据用户名查找用户
    async fn find_user_by_username(&self, username: &str) -> Result<Option<User>, sqlx::Error>;

    /// 创建用户
    async fn create_user(&self, user: &User) -> Result<(), sqlx::Error>;

    /// 更新用户密码
    async fn update_user_password(&self, user_id: Uuid, password_hash: &str) -> Result<(), sqlx::Error>;

    /// 创建密码重置令牌
    async fn create_password_reset_token(&self, token: &PasswordResetToken) -> Result<(), sqlx::Error>;

    /// 查找密码重置令牌
    async fn find_password_reset_token(&self, token: &str) -> Result<Option<PasswordResetToken>, sqlx::Error>;

    /// 标记密码重置令牌已使用
    async fn mark_token_used(&self, token_id: Uuid) -> Result<(), sqlx::Error>;
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
            "SELECT id, username, password_hash, role, created_at FROM users WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    async fn find_user_by_username(&self, username: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "SELECT id, username, password_hash, role, created_at FROM users WHERE username = $1"
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
    }

    async fn create_user(&self, user: &User) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO users (id, username, password_hash, role, created_at)
             VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(user.id)
        .bind(&user.username)
        .bind(&user.password_hash)
        .bind(&user.role)
        .bind(user.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn update_user_password(&self, user_id: Uuid, password_hash: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE users SET password_hash = $1 WHERE id = $2"
        )
        .bind(password_hash)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn create_password_reset_token(&self, token: &PasswordResetToken) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO password_reset_tokens (id, user_id, token, expires_at, used_at, created_at)
             VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind(token.id)
        .bind(token.user_id)
        .bind(&token.token)
        .bind(token.expires_at)
        .bind(token.used_at)
        .bind(token.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn find_password_reset_token(&self, token: &str) -> Result<Option<PasswordResetToken>, sqlx::Error> {
        sqlx::query_as::<_, PasswordResetToken>(
            "SELECT id, user_id, token, expires_at, used_at, created_at
             FROM password_reset_tokens WHERE token = $1"
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await
    }

    async fn mark_token_used(&self, token_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE password_reset_tokens SET used_at = NOW() WHERE id = $1"
        )
        .bind(token_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
