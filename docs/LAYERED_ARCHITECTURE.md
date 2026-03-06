# 后端分层架构重构详细方案

## 目录结构目标

```
src/
├── main.rs
├── config/
│   └── app.rs
│   ├── database.rs
│   ├── redis.rs
│   └── storage.rs
└── mod.rs
├── domain/
│   ├── mod.rs
│   ├── auth/
│   │   └── models.rs, service.rs, repository.rs
│   ├── image/
│   │   └── models.rs, service.rs, repository.rs, processor.rs
│   └── admin/
│       └── models.rs, service.rs, repository.rs
├── infrastructure/
│   └── mod.rs
│       ├── database/
│       │   ├── pool.rs, schema.rs
│       ├── cache/
│       │   └── redis.rs
│       ├── storage/
│       │   ├── local.rs, file_queue.rs, image_processor.rs
│       └── email/
│           └── smtp.rs
└── api/
│   └── mod.rs
│       ├── routes/
│       │   ├── auth.rs, images.rs, admin.rs, health.rs
│   ├── handlers/
       │   ├── auth.rs, images.rs, images_cursor.rs, admin.rs
│   └── middleware/
│       └── auth.rs
└── shared/
│   └── mod.rs
│       ├── error.rs
│       ├── utils.rs
│       ├── audit.rs
│       └── tasks.rs
```

## 重构的三个核心原则

### 1. 单一职责原则 (SRP)
- **Domain Layer**: 只包含业务逻辑，不处理 HTTP
- **Infrastructure Layer**: 只处理数据访问和外部集成，不处理业务逻辑
- **API Layer**: 只处理 HTTP 请求/响应，不处理业务逻辑

### 2. 依赖倒置原则 (DIP)
- Domain Layer 不依赖 Infrastructure Layer
- API Layer 不依赖 Domain Layer，通过 Service 接口交互
- Infrastructure Layer 不依赖 Domain Layer，使用 Repository 接口

### 3. 接口隔离原则
- 使用 trait 定义接口
- 使用泛型抽象，具体实现依赖接口
- 允许不同实现通过依赖注入替换

## 重构步骤详解

### 第一步：创建目录结构

```bash
# 创建新的目录结构
mkdir -p src/config src/domain/{auth,image,admin} \
         src/infrastructure/{database,cache,storage,email}
```

### 第二步：创建配置模块

```bash
# 移动并拆分 config.rs
mv src/config.rs src/config/app.rs

# 创建子配置文件
touch src/config/database.rs
touch src/config/redis.rs
touch src/config/storage.rs
touch src/config/mod.rs
```

**config/app.rs 示例**:
```rust
// config/app.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub storage: StorageConfig,
    pub cache: CacheConfig,
    pub auth: AuthConfig,
    pub cleanup: CleanupConfig,
    pub mail: MailConfig,
    pub image: ImageConfig,
    pub queue: QueueConfig,
}
```

### 第三步：创建 Domain 层 - Auth 模块

**3.1 创建 models.rs**
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

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
    user: UserResponse,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}
```

**3.2 创建 repository.rs**
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
    async fn find_user_by_email(&self, email: &str) -> Result<Option<User>, AppError>;
    async fn create_user(&self, user: &User) -> Result<(), AppError>;
    async fn update_user_password(&self, user_id: Uuid, password_hash: &str) -> Result<(), AppError>;
    async fn save_refresh_token(&self, token: &RefreshToken) -> Result<(), AppError>;
    async fn find_refresh_token(&self, token: &str) -> Result<Option<RefreshToken>, AppError>;
    async fn mark_token_used(&self, token: &str) -> Result<(), AppError>;
    async fn delete_token(&self, token: &str) -> Result<(), AppError>;
}
```

**3.3 创建 service.rs**
```rust
// domain/auth/service.rs
use crate::domain::auth::repository::AuthRepository;
use crate::domain::auth::models::*;
use crate::shared::error::AppError;
use chrono::{DateTime, Utc, Duration};

pub struct AuthService<R: AuthRepository> {
    repository: R,
}

impl<R: AuthRepository> AuthService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }
    
    pub async fn register(&self, req: RegisterRequest) -> Result<(User, AuthTokens), AppError> {
        // 验证用户名长度
        if req.username.len() < 3 || req.username.len() > 50 {
            return Err(AppError::InvalidUsernameLength);
        }
        
        // 验证密码长度
        if req.password.len() < 6 {
            return Err(AppError::InvalidPasswordLength);
        }
        
        // 生成密码哈希
        let password_hash = bcrypt::hash(req.password.as_bytes(), bcrypt::DEFAULT_COST)
            .map_err(|_| AppError::Internal(anyhow::anyhow!("密码哈希失败"))?;
        
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
            expires_in: 900,
            user: user.into(),
        }))
    }
    
    pub async fn login(&self, req: LoginRequest) -> Result<(User, AuthTokens), AppError> {
        let user = self.repository
            .find_user_by_username(&req.username)
            .await?
            .ok_or(AppError::UserNotFound)?;
        
        // 验证密码
        let is_valid = bcrypt::verify(req.password.as_bytes(), &user.password_hash)
            .map_err(|_| AppError::Internal(anyhow::anyhow!("密码验证失败"))?;
        
        if !is_valid {
            return Err(AppError::InvalidPassword);
        }
        
        let access_token = self.generate_access_token(&user)?;
        let refresh_token = self.generate_refresh_token(&user)?;
        
        Ok((user, AuthTokens {
            access_token,
            refresh_token,
            expires_in: 900,
            user: user.into(),
        }))
    }
    
    pub async fn change_password(
        &self,
        user_id: Uuid,
        current_password: String,
        new_password: Option<String>
    ) -> Result<(), AppError> {
        let user = self.repository.find_user_by_id(user_id).await?;
        
        // 验证当前密码
        let is_valid = bcrypt::verify(
            current_password.as_bytes(), 
            &user.password_hash
        ).map_err(|_| AppError::Internal(anyhow::anyhow!("密码验证失败"))?;
        
        if !is_valid {
            return Err(AppError::InvalidPassword);
        }
        
        // 更新密码
        if let Some(new_pass) = new_password {
            if new_pass.len() < 6 || new_pass.len() > 100 {
                return Err(AppError::InvalidPasswordLength);
            }
            
            let new_hash = bcrypt::hash(new_pass.as_bytes(), bcrypt::DEFAULT_COST)
                .map_err(|_| AppError::Internal(anyhow::anyhow!("密码哈希失败"))?;
                
            self.repository.update_user_password(user_id, &new_hash).await?;
        } else {
            return Ok(());
        }
    }
    
    fn generate_access_token(&self, user: &User) -> Result<String, AppError> {
        let now = Utc::now();
        let exp = now + Duration::minutes(15);
        let jti = Uuid::new_v4().to_string();
        
        let claims = Claims {
            sub: user.id,
            username: user.username.clone(),
            role: user.role.clone(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            jti: jti.clone(),
            version: 1,
        };
        
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_ref()),
        )
        .map_err(Into::into)
    }
    
    fn generate_refresh_token(&self, user: &User) -> Result<String, AppError> {
        let now = Utc::now();
        let exp = now + Duration::hours(7 * 24); // 7天
        let jti = Uuid::new_v4().to_string();
        
        let claims = Claims {
            sub: user.id,
            role: "refresh".to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            jti,
            version: 1,
        };
        
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_ref()),
        )
        .map_err(Into::into)
    }
}
```

### 第四步：创建 Infrastructure 层 - Database 模块

**4.1 创建 schema.rs**
```rust
// infrastructure/database/schema.rs
pub const SCHEMA_SQL: &str = r#"
    -- 用户表
    CREATE TABLE IF NOT EXISTS users (
        id UUID PRIMARY KEY,
        username VARCHAR(50) UNIQUE NOT NULL,
        password_hash VARCHAR(255) NOT NULL,
        role VARCHAR(20) DEFAULT 'user',
        created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
    );
    
    -- 密码重置令牌表
    CREATE TABLE IF NOT EXISTS password_reset_tokens (
        id UUID PRIMARY KEY,
        user_id UUID REFERENCES users(id) ON DELETE CASCADE,
        token VARCHAR(255) UNIQUE NOT NULL,
        expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
        used_at TIMESTAMP WITH TIME ZONE,
        created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
        CONSTRAINT unique_user_active_token UNIQUE (user_id, used_at)
    );
    
    -- 创建索引
    CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
    CREATE INDEX IF NOT EXISTS idx_users_created_at ON users(created_at DESC);
    CREATE INDEX IF NOT EXISTS idx_users_role ON users(role);
";
```

**4.2 创建 pool.rs**
```rust
// infrastructure/database/pool.rs
use sqlx::PgPool;

pub fn create_pool(database_url: &str, max_connections: u32) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(database_url)
        .await
}

pub async fn init_schema(pool: &PgPool) -> Result<(), sqlx::Error> {
    pool.execute(SCHEMA_SQL).await?;
    info!("Database schema initialized successfully");
    Ok(())
}
```

### 第五步：创建 API 层 - Auth Handler

**5.1 重构 handler**
```rust
// api/handlers/auth.rs
use crate::domain::auth::service::AuthService;
use crate::domain::auth::models::*;
use crate::shared::error::AppError;
use axum::{Json, extract::State};
use axum::http::{HeaderMap};
use crate::api::middleware::AuthUser;

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<AuthTokens>, AppError> {
    let (user, tokens) = state.auth_service.register(req).await?;
    
    Ok(Json(AuthTokens {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        expires_in: tokens.expires_in,
        user: user.into(),
    }))
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<(axum::http::HeaderMap, Json<AuthTokens>), AppError> {
    let (user, tokens) = state.auth_service.login(req).await?;
    
    let cookie_value = format!(
        "auth_token={}; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age={}",
        tokens.refresh_token,
        7 * 24 * 3600 // 7天
    );
    
    let mut headers = HeaderMap::new();
    headers.insert(
        "Set-Cookie",
        axum::http::HeaderValue::from_str(&cookie_value).unwrap(),
    );
    
    Ok((
        headers,
        Json(AuthTokens {
            access_token: tokens.access_token,
            refresh_token: tokens.refresh_token,
            expires_in: tokens.expires_in,
            user: user.into(),
        }),
    ))
}

pub async fn change_password(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<(), AppError> {
    state.auth_service
        .change_password(auth_user.id, req.current_password, req.new_password)
        .await
}
```

### 第六步：更新 main.rs

**6.1 创建数据库连接池**
```rust
// main.rs - 添加到现有的初始化代码
let pool = create_pool(&config.database.url, config.database.max_connections).await?;

// 创建 repository
let auth_repository = PostgresAuthRepository::new(pool.clone());

// 创建 service
let auth_service = AuthService::new(auth_repository, &config.auth);
```

## 重构前后对比

### 当前架构 (单体)
```
handlers/auth.rs (744 行)
  ├─ 上传业务逻辑
  ├─ 处理 HTTP 请求
  └─ 调用数据库
```

### 重构后 (分层)
```
api/handlers/auth.rs (150 行)
  ├─ 仅处理 HTTP 请求/响应
  ├─ 认证用户
  └─ 返回 JSON

domain/auth/
  ├─ models.rs (100 行)
  ├─ service.rs (300 行)  ── 包含所有业务逻辑
  └─ repository.rs (200 行)    ── 数据库访问

api/handlers/auth.rs
  │  └─── 委托给 service 层
        auth_service.register(req).await
```

## 重构收益

### 1. 代码组织
- 按业务领域组织代码
- 单个文件从 744 行减少到 150 行
- 按职责分离关注点组织代码

### 2. 可测试性
- Repository 层可以轻松 mock
- Service 层可以轻松测试
- Handler 层可以测试 HTTP 层

### 3. 可维护性
- 业务逻辑集中管理
- 修改业务逻辑不影响 HTTP 层
- 添加新功能更容易

### 4. 可扩展性
- 可以轻松添加新的存储实现 (如 MongoDB)
- 可以轻松切换到不同的认证方式 (如 OAuth)
- 可以轻松添加新的缓存实现 (如 Memcached)

### 实施路线图

```
当前架构
    第一步：创建配置模块
    │
    └──> 第二步：创建 Domain 层 - Auth
          ├── 1. 创建 domain/auth/models.rs
          │   └──> 第三步：创建 Domain 层 - Image
          └──> 第四步：创建 Infrastructure 层
                  ├── Database
                  └──> 第五步：重构 API 层
                  ├── Handlers (委托给 Service)
                  └──> 第六步：更新 main.rs
```

## 重构的最佳实践

1. **不要一次性重构所有代码**
   - 一次重构容易出错
   - 难以调试
   - 每次重构一个模块

2. **保持向后兼容**
   - 不要同时删除旧文件
   - 新旧 handler 仍可使用
   - 逐步迁移

3. **确保测试覆盖**
   - 重构前添加测试
   - 重构后运行测试
   - 确保功能正常

4. **使用 IDE 重构
   - 大型重构使用 IDE 重构功能
   - 逐步进行
   - 不要一次性重命名

5. **关注编译错误**
   - 及时修复编译错误
   - 保持代码可编译

## 总结

分层架构虽然需要更多的文件，但带来长期收益：
- 更好的代码组织
- 更容易测试
- 更容易维护
- 更容易扩展
- 更容易理解
- 更容易替换实现
