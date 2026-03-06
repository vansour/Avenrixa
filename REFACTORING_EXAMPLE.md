# Auth 模块重构示例

这个示例展示了如何将 `handlers/auth.rs` 中的业务逻辑提取到 `domain/auth/service.rs`，创建 `domain/auth/repository.rs` trait，并重构 handler。

## 1. 创建 domain/auth/models.rs

```rust
// domain/auth/models.rs
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub new_password: String,
}
```

## 2. 创建 domain/auth/repository.rs

```rust
// domain/auth/repository.rs
use crate::domain::auth::models::*;
use crate::shared::error::AppError;
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[async_trait]
pub trait AuthRepository: Send + Sync {
    async fn find_user_by_id(&self, id: Uuid) -> Result<Option<User>, AppError>;
    async fn find_user_by_username(&self, username: &str) -> Result<Option<User>, AppError>;
    async fn create_user(&self, user: &User) -> Result<(), AppError>;
    async fn update_user_password(&self, user_id: Uuid, password_hash: &str) -> Result<(), AppError>;
    async fn save_refresh_token(&self, token: &RefreshToken) -> Result<(), AppError>;
    async fn find_reset_token(&self, token: &str) -> Result<Option<RefreshToken>, AppError>;
}

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
    async fn find_user_by_id(&self, id: Uuid) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AppError::UserNotFound)
    }
    
    async fn find_user_by_username(&self, username: &str) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE username = $1"
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AppError::UserNotFound)
    }
    
    async fn create_user(&self, user: &User) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO users (id, username, password_hash, role, created_at) VALUES ($1, $2, $3, 'user', NOW())"
        )
        .bind(user.id)
        .bind(&user.username)
        .bind(&user.password_hash)
        .execute(&self.pool)
        .await
        .map_err(AppError::DatabaseError)?;
        
        Ok(())
    }
    
    async fn update_user_password(&self, user_id: Uuid, password_hash: &str) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE users SET password_hash = $1 WHERE id = $2"
        )
        .bind(password_hash)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(AppError::DatabaseError)?;
        
        Ok(())
    }
    
    async fn save_refresh_token(&self, token: RefreshToken) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO password_reset_tokens (id, user_id, token, expires_at, used_at, created_at) 
             VALUES ($1, $2, $3, $4, NULL, NOW())"
        )
        .bind(token.id)
        .bind(token.user_id)
        .bind(token.expires_at)
        .execute(&self.pool)
        .await
        .map_err(AppError::DatabaseError)?;
        
        Ok(())
    }
    
    async fn find_reset_token(&self, token: &str) -> Result<Option<RefreshToken>, AppError> {
        let token = sqlx::query_as::<_, RefreshToken>(
            "SELECT * FROM password_reset_tokens
             WHERE token = $1 AND used_at IS NULL AND expires_at > $2",
        )
        .bind(token)
        .bind(Utc::now())
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AppError::ResetTokenInvalid)
    }
}
```

## 3. 创建 domain/auth/service.rs

```rust
// domain/auth/service.rs
use crate::domain::auth::repository::AuthRepository;
use crate::domain::auth::models::*;
use crate::shared::error::AppError;
use crate::auth::AuthService as LegacyAuthService;
use chrono::{DateTime, Utc, Duration};
use uuid::Uuid;

pub struct AuthService<R: AuthRepository> {
    repository: R,
    jwt_secret: String,
    access_token_ttl: i64,
    refresh_token_ttl: i64,
}

impl<R: AuthRepository> AuthService<R> {
    pub fn new(repository: R, jwt_secret: String) -> Self {
        Self { 
            repository,
            jwt_secret,
            access_token_ttl: 900, // 15分钟
            refresh_token_ttl: 7 * 24 * 3600, // 7天
        }
    }
    
    pub async fn register(&self, req: RegisterRequest) -> Result<(User, AuthTokens), AppError> {
        // 验证用户名
        if req.username.len() < 3 || req.username.len() > 50 {
            return Err(AppError::InvalidUsernameLength);
        }
        
        // 验证密码长度
        if req.password.len() < 6 {
            return Err(AppError::InvalidPasswordLength);
        }
        
        // 生成密码哈希
        let password_hash = LegacyAuthService::hash_password(&req.password)?;
        
        // 创建用户
        let user = User {
            id: Uuid::new_v4(),
            username: req.username.clone(),
            password_hash,
            role: "user".to_string(),
            created_at: Utc::now(),
        };
        
        self.repository.create_user(&user).await?;
        
        // 生成令牌
        let access_token = self.generate_access_token(&user)?;
        let refresh_token = self.generate_refresh_token(&user)?;
        
        Ok((user, AuthTokens {
            access_token,
            refresh_token,
            expires_in: self.access_token_ttl,
        }))
    }
    
    pub async fn login(&self, req: LoginRequest) -> Result<(User, AuthTokens), AppError> {
        // 查找用户
        let user = self.repository.find_user_by_username(&req.username).await?;
        
        // 验证密码
        let is_valid = LegacyAuthService::verify_password(&req.password, &user.password_hash)?;
        
        if !is_valid {
            return Err(AppError::InvalidPassword);
        }
        
        // 生成令牌
        let access_token = self.generate_access_token(&user)?;
        let refresh_token = self.generate_refresh_token(&user)?;
        
        Ok((user, AuthTokens {
            access_token,
            refresh_token,
            expires_in: self.access_token_ttl,
        }))
    }
    
    pub async fn change_password(
        &self, 
        user_id: Uuid, 
        current_password: String, 
        new_password: Option<String>
    ) -> Result<(), AppError> {
        // 获取用户
        let user = self.repository.find_user_by_id(user_id).await?;
        
        // 验证当前密码
        let is_valid = LegacyAuthService::verify_password(&current_password, &user.password_hash)?;
        
        if !is_valid {
            return Err(AppError::InvalidPassword);
        }
        
        // 验证新密码
        if let Some(ref new) = new_password {
            if new.len() < 6 {
                return Err(AppError::InvalidPasswordLength);
            }
            
            // 生成新密码哈希
            let new_hash = LegacyAuthService::hash_password(&new)?;
            
            // 更新密码
            self.repository.update_user_password(user_id, &new_hash).await?;
        }
        
        Ok(())
    }
    
    pub async fn forgot_password(&self, email: String) -> Result<(), AppError> {
        // 查找用户
        let user = self.repository.find_user_by_username(&email).await?;
        
        // 生成重置令牌
        let reset_token = LegacyAuthService::generate_reset_token();
        let expires_at = Utc::now() + Duration::hours(1);
        
        let token = RefreshToken {
            id: Uuid::new_v4(),
            user_id: user.id,
            token: reset_token,
            expires_at,
            used_at: None,
            created_at: Utc::now(),
        };
        
        self.repository.save_refresh_token(&token).await?;
        
        Ok(())
    }
    
    pub async fn reset_password(&self, token: String, new_password: String) -> Result<(), AppError> {
        // 验证令牌
        let token_data = self.repository.find_reset_token(&token).await?;
        
        // 标记为已使用
        let _ = sqlx::query(
            "UPDATE password_reset_tokens SET used_at = $1 WHERE id = $2",
        )
        .bind(Utc::now())
        .bind(token_data.id)
        .execute(&self.repository.pool)
        .await
        .map_err(AppError::DatabaseError)?;
        
        Ok(())
    }
}
```

## 4. 重构 handlers/auth.rs

```rust
// api/handlers/auth.rs
use crate::domain::auth::service::AuthService;
use crate::domain::auth::models::*;
use crate::shared::error::AppError;
use crate::api::middleware::AuthUser;
use axum::{Json, extract::State};
use axum::http::{HeaderMap};

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let (user, tokens) = state.auth_service.register(req).await?;
    
    Ok(Json(AuthResponse {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        expires_in: tokens.expires_in,
        user: user.into(),
    }))
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<(HeaderMap, Json<AuthResponse>), AppError> {
    let (user, tokens) = state.auth_service.login(req).await?;
    
    let cookie_value = format!(
        "auth_token={}; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age={}",
        tokens.refresh_token,
        7 * 24 * 3600
    );
    
    let mut headers = HeaderMap::new();
    headers.insert(
        "Set-Cookie",
        HeaderValue::from_str(&cookie_value).unwrap(),
    );
    
    Ok((
        headers,
        Json(AuthResponse {
            access_token: tokens.access_token,
            refresh_token: tokens.refresh_token,
            expires_in: tokens.expires_in,
            user: user.into(),
        }),
    ))
}
```

## 5. 更新 main.rs

```rust
use crate::domain::auth::service::AuthService;
use crate::domain::auth::repository::{AuthRepository, PostgresAuthRepository};
use crate::infrastructure::cache::redis::RedisCache;
use crate::api::routes::create_routes;
use crate::api::middleware::AuthUser;
use axum::Router;
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ... 初始化数据库连接
    let pool = PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .connect(&config.database.url)
        .await?;
    
    // 创建 repository
    let auth_repository = PostgresAuthRepository::new(pool.clone());
    
    // 创建 service
    let auth_service = AuthService::new(
        auth_repository,
        config.jwt_secret,
    );
    
    // 创建应用状态
    let state = AppState {
        pool,
        redis: redis_conn,
        config: config.clone(),
        auth: auth: auth_service,
        image_processor,
        file_save_queue,
        started_at: std::time::Instant::now(),
    };
    
    // 创建路由
    let app = create_routes(state).layer(TraceLayer::new_for_http());
    
    // 启动服务器
    let listener = TcpListener::bind(&config.addr()).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}
```
```

## 重构优势

1. **职责分离**:
   - Service: 业务逻辑
   - Repository: 数据访问
   - Handler: HTTP 处理

2. **易于测试**:
   - 可以轻松 mock repository 测试 service
   - 可以轻松 mock service 测试 handler

3. **易于扩展**:
   - 添加新功能只需添加对应的 service 和 repository

4. **易于维护**:
   - 修改业务逻辑不影响其他层
   - 修改数据访问不影响业务逻辑

## 下一步

按照此示例，重构其他模块：
1. domain/image
2. domain/user
3. domain/admin
4. infrastructure/database/pool.rs
5. infrastructure/cache/redis.rs
6. infrastructure/storage/
7. api/handlers/images.rs
8. api/handlers/admin.rs
