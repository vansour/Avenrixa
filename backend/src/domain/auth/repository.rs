//! 认证数据访问 trait
//!
//! 定义用户和认证相关的数据访问接口

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::User;

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
}
