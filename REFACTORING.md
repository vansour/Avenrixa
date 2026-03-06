# 后端项目分层架构重构方案

## 概述

本文档描述了如何将当前的单体架构重构为分层架构，提高代码的可维护性和可测试性。

## 重构后的目录结构

```
src/
├── main.rs                    # 应用入口
├── lib.rs                     # 库入口
├── config/                    # 配置模块
│   ├── mod.rs
│   ├── app.rs                  # 主配置类
│   ├── database.rs             # 数据库配置
│   ├── redis.rs                # Redis 配置
│   └── storage.rs              # 存储配置
├── domain/                    # 领域层（业务逻辑）
│   ├── mod.rs
│   ├── user/
│   │   ├── mod.rs
│   │   ├── service.rs          # 用户业务逻辑
│   │   ├── repository.rs       # 用户数据访问
│   │   └── models.rs           # 用户相关模型
│   ├── auth/
│   │   ├── mod.rs
│   │   ├── service.rs          # 认证业务逻辑
│   │   ├── repository.rs       # 认证数据访问
│   │   └── models.rs           # 认证相关模型
│   ├── image/
│   │   ├── mod.rs
│   │   ├── service.rs          # 图片业务逻辑
│   │   ├── repository.rs       # 图片数据访问
│   │   ├── processor.rs        # 图片处理
│   │   └── models.rs           # 图片相关模型
│   └── admin/
│       ├── mod.rs
│       ├── service.rs          # 管理员业务逻辑
│       ├── repository.rs       # 管理员数据访问
│       └── models.rs           # 管理员相关模型
├── infrastructure/             # 基础设施层
│   ├── mod.rs
│   ├── database/
│   │   ├── mod.rs
│   ├── pool.rs             # 数据库连接池
│   │   └── migrations/         # 数据库迁移
│   ├── cache/
│   │   ├── mod.rs
│   │   └── redis.rs            # Redis 实现
│   ├── storage/
│   │   ├── mod.rs
│   │   ├── local.rs            # 本地存储
│   │   ├── file_queue.rs        # 文件队列
│   │   └── image_processor.rs  # 图片处理器
│   └── email/
│       ├── mod.rs
│       └── smtp.rs             # SMTP 实现
├── api/                       # API 层（HTTP 处理）
│   ├── mod.rs
│   ├── routes/                 # 路由定义
│   │   ├── mod.rs
│   │   ├── auth.rs
│   │   ├── images.rs
│   │   ├── admin.rs
│   │   └── health.rs
│   └── handlers/               # 请求处理器
│       ├── mod.rs
│       ├── auth.rs              # 仅处理 HTTP 请求/响应
│       ├── images.rs           # 仅处理 HTTP 请求/响应
│       ├── images_cursor.rs    # Cursor 分页
│       └── admin.rs             # 仅处理 HTTP 请求/响应
├── shared/                    # 共享模块
│   ├── mod.rs
│   ├── error.rs               # 统一错误处理
│   ├── utils.rs               # 工具函数
│   ├── audit.rs               # 审计日志
│   └── tasks.rs               # 后台任务
└── db.rs                      # AppState 定义（临时保留，后续整合到 infrastructure）
```

## 分层职责

### 1. Domain Layer (领域层)

**职责**: 包含业务逻辑和业务规则，不依赖外部框架

**示例**:
```rust
// domain/auth/service.rs
pub struct AuthService {
    repository: Arc<dyn AuthRepository>,
    config: AuthConfig,
}

impl AuthService {
    pub async fn register(&self, req: RegisterRequest) -> Result<User> {
        // 1. 验证用户名长度
        if req.username.len() < 3 || req.username.len() > 50 {
            return Err(AuthError::InvalidUsernameLength);
        }
        
        // 2. 验证密码强度
        if req.password.len() < 6 {
            return Err(AuthError::InvalidPasswordLength);
        }
        
        // 3. 调用 repository 创建用户
        let user = self.repository.create_user(&req).await?;
        
        // 4. 生成令牌
        let access_token = self.generate_access_token(&user)?;
        let refresh_token = self.generate_refresh_token(&user)?;
        
        Ok(user)
    }
}

// domain/auth/repository.rs (trait)
#[async_trait]
pub trait AuthRepository: Send + Sync {
    async fn find_user_by_username(&self, username: &str) -> Result<Option<User>>;
    async fn create_user(&self, user: &User) -> Result<()>;
    async fn save_refresh_token(&self, token: &RefreshToken) -> Result<()>;
}
```

### 2. Infrastructure Layer (基础设施层)

**职责**: 实现数据访问、外部服务集成

**示例**:
```rust
// infrastructure/database/postgres/auth_repository.rs
pub struct PostgresAuthRepository {
    pool: PgPool,
}

#[async_trait]
impl AuthRepository for PostgresAuthRepository {
    async fn find_user_by_username(&self, username: &str) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE username = $1"
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(user)
    }
}

// infrastructure/cache/redis.rs
pub struct RedisCache {
    client: ConnectionManager,
}

impl RedisCache {
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let value: Option<String> = self.client.get(key).await?;
        match value {
            Some(v) => Ok(serde_json::from_str(&v)?),
            None => Ok(None),
        }
    }
    
    pub async fn set<T: Serialize>(&self, key: &str, value: &T, ttl: u64) -> Result<()> {
        let json = serde_json::to_string(value)?;
        self.client.set_ex(key, json, ttl).await?;
        Ok(())
    }
}
```

### 3. API Layer (API 层)

**职责**: 仅处理 HTTP 请求/响应，委托给 service 层

**示例**:
```rust
// api/handlers/auth.rs
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    // 调用 service 层
    let result = state.auth_service.register(req).await?;
    
    // 返回 HTTP 响应
    Ok(Json(AuthResponse {
        access_token: result.access_token,
        refresh_token: result.refresh_token,
        user: result.user.into(),
    }))
}
```

## 重构步骤

### 第一阶段：创建目录结构和基础模块

1. 创建新目录：
   ```bash
   mkdir -p src/config src/domain/{user,auth,image,admin}
   mkdir -p src/infrastructure/{database,cache,storage,email}
   mkdir -p src/api/{routes,handlers,middleware}
   mkdir -p src/shared
   ```

2. 创建各模块的 mod.rs 文件

### 第二阶段：提取 Service 层

1. 从 `handlers/auth.rs` 提取业务逻辑到 `domain/auth/service.rs`
2. 从 `handlers/images.rs` 提取业务逻辑到 `domain/image/service.rs`
3. 从 `handlers/admin.rs` 提取业务逻辑到 `domain/admin/service.rs`

### 第三阶段：创建 Repository 层

1. 定义 Repository traits
2. 实现 PostgreSQL repositories
3. 更新 Service 层使用 repositories

### 第四阶段：重构 Handlers

1. 将 handler 简化为纯 HTTP 层
2. 委托给 service 层处理业务逻辑
3. 移除业务逻辑

### 第五阶段：迁移基础设施

1. 移动数据库相关代码到 `infrastructure/database/`
2. 移动缓存代码到 `infrastructure/cache/`
3. 移动存储代码到 `infrastructure/storage/`

### 第六阶段：清理和优化

1. 删除旧文件
2. 更新 imports
3. 运行测试确保功能正常

## 优势

### 1. 职责清晰
- **Domain Layer**: 业务逻辑
- **Infrastructure Layer**: 数据访问和外部集成
- **API Layer**: HTTP 处理
- **Shared**: 共享功能

### 2. 易于测试
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_register_success() {
        // 可以轻松 mock repository
        let mock_repo = MockAuthRepository::new();
        let service = AuthService::new(mock_repo, config);
        
        let req = RegisterRequest {
            username: "test".to_string(),
            password: "test123".to_string(),
        };
        
        let result = service.register(req).await.unwrap();
        assert_eq!(result.username, "test");
    }
}
```

### 3. 易于扩展
```rust
// 添加新功能只需添加对应的 domain
// 例如：添加分类功能

// 1. 创建 domain/category/mod.rs, service.rs, repository.rs, models.rs
// 2. 在 domain/mod.rs 导出
// 3. 在 handlers 添加路由
```

### 4. 易于维护
- 代码按业务领域组织
- 减少文件大小（当前 handlers/images.rs 744 行）
- 降低模块间耦合

### 5. 易于替换
```rust
// 轻松替换缓存实现
// domain/auth/service.rs 使用 trait，不依赖具体实现

pub struct AuthService<R: AuthRepository> {
    repository: R,
    config: AuthConfig,
}

// 可以轻松切换到不同的实现
let pg_repo = PostgresAuthRepository::new(pool);
let auth_service = AuthService::new(pg_repo, config);
```

## 实施建议

### 渐进式重构

不要一次性重构所有代码，而是：
1. 每次重构一个模块
2. 确保重构后代码能编译通过
3. 运行测试确保功能正常
4. 提交代码到版本控制

### 重构顺序建议

1. **config**: 优先级低，可以最后做
2. **domain/image**: 优先级高，业务逻辑复杂
3. **domain/auth**: 优先级高，认证是核心功能
4. **infrastructure**: 优先级中
5. **api/handlers**: 优先级高，依赖其他层

### 测试策略

重构过程中：
1. 先为 service 层编写单元测试
2. 确保 repository 层可测试
3. 集成测试验证整体功能

## 示例：重构 auth 模块

### 1. 定义 Repository trait

```rust
// domain/auth/repository.rs
use crate::domain::auth::models::{User, RefreshToken};
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
}

// PostgreSQL 实现
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
            "INSERT INTO users (id, username, password_hash, role, created_at) 
             VALUES ($1, $2, $3, 'user', NOW())"
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
    
    async fn save_refresh_token(&self, token: &RefreshToken) -> Result<(), AppError> {
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
}
```

### 2. 创建 Service 层

```rust
// domain/auth/service.rs
use crate::domain::auth::repository::AuthRepository;
use crate::domain::auth::models::*;
use crate::infrastructure::storage::image_processor::ImageProcessor;
use crate::shared::error::AppError;
use chrono::{DateTime, Utc, Duration};
use uuid::Uuid;

pub struct AuthService<R: AuthRepository> {
    repository: R,
    config: AuthConfig,
    image_processor: Option<ImageProcessor>,
}

impl<R: AuthRepository> AuthService<R> {
    pub fn new(repository: R, config: AuthConfig) -> Self {
        Self { 
            repository,
            config,
            image_processor: None,
        }
    }
    
    pub fn with_image_processor(mut self, processor: ImageProcessor) -> Self {
        self.image_processor = Some(processor);
        self
    }
    
    pub async fn register(&self, req: RegisterRequest) -> Result<(User, AuthTokens), AppError> {
        // 1. 验证用户名
        if req.username.len() < 3 || req.username.len() > 50 {
            return Err(AppError::InvalidUsernameLength);
        }
        
        // 2. 验证密码长度
        if req.password.len() < 6 {
            return Err(AppError::InvalidPasswordLength);
        }
        
        // 3. 生成密码哈希
        let password_hash = crate::auth::AuthService::hash_password(&req.password)?;
        
        // 4. 创建用户
        let user = User {
            id: Uuid::new_v4(),
            username: req.username.clone(),
            password_hash,
            role: "user".to_string(),
            created_at: Utc::now(),
        };
        
        self.repository.create_user(&user).await?;
        
        // 5. 生成令牌
        let access_token = self.generate_access_token(&user)?;
        let refresh_token = self.generate_refresh_token(&user)?;
        
        Ok((user, AuthTokens {
            access_token,
            refresh_token,
            expires_in: 900,
        }))
    }
    
    pub async fn login(&self, req: LoginRequest) -> Result<(User, AuthTokens), AppError> {
        // 查找用户
        let user = self.repository.find_user_by_username(&req.username).await?;
        
        // 验证密码
        let is_valid = crate::auth::AuthService::verify_password(&req.password, &user.password_hash)?;
        
        if !is_valid {
            return Err(AppError::InvalidPassword);
        }
        
        // 生成令牌
        let access_token = self.generate_access_token(&user)?;
        let refresh_token = self.generate_refresh_token(&user)?;
        
        Ok((user, AuthTokens {
            access_token,
            refresh_token,
            expires_in: 900,
        }))
    }
    
    pub async fn change_password(
        &self, 
        user_id: Uuid, 
        current_password: String, 
        new_password: String
    ) -> Result<(), AppError> {
        // 获取用户
        let user = self.repository.find_user_by_id(user_id).await?;
        
        // 验证当前密码
        let is_valid = crate::auth::AuthService::verify_password(&current_password, &user.password_hash)?;
        
        if !is_valid {
            return Err(AppError::InvalidPassword);
        }
        
        // 验证新密码
        if new_password.len() < 6 {
            return Err(AppError::InvalidPasswordLength);
        }
        
        // 生成新密码哈希
        let new_hash = crate::auth::AuthService::hash_password(&new_password)?;
        
        // 更新密码
        self.repository.update_user_password(user_id, &new_hash).await?;
        
        Ok(())
    }
    
    pub async fn forgot_password(&self, email: String) -> Result<(), AppError> {
        // 查找用户
        let user = self.repository.find_user_by_username(&email).await?;
        
        // 生成重置令牌
        let reset_token = crate::auth::AuthService::generate_reset_token();
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
        // 验证新密码
        if new_password.len() < 6 {
            return Err(AppError::InvalidPasswordLength);
        }
        
        // TODO: 验证令牌并重置密码
        
        Ok(())
    }
}
```

### 3. 更新 Handler

```rust
// api/handlers/auth.rs
use crate::domain::auth::service::AuthService;
use crate::domain::auth::models::*;
use crate::shared::error::AppError;
use crate::api::middleware::AuthUser;
use axum::{Json, extract::State};

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    // 调用 service 层
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
) -> Result<(axum::http::HeaderMap, Json<AuthResponse>), AppError> {
    let (user, tokens) = state.auth_service.login(req).await?;
    
    // 设置 Cookie
    let cookie_value = format!(
        "auth_token={}; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age={}",
        tokens.refresh_token,
        7 * 24 * 3600
    );
    
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        "Set-Cookie",
        axum::http::HeaderValue::from_str(&cookie_value).unwrap(),
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

## 迁移到分层架构的建议

### 1. 遵循 SOLID 原则
- **S**ingle Responsibility: 每个类只有一个改变的理由
- **O**pen/Closed Principle: 对扩展开放，对修改关闭
- **L**iskov Substitution Principle: 使用接口而非实现
- **I**nterface Segregation Principle: 接口隔离原则
- **D**ependency Inversion Principle: 依赖倒置

### 2. 不要过度设计
- 从简单开始，逐步演进
- 避免过早抽象
- 按需创建接口

### 3. 保持实用主义
- 代码能工作即可
- 不追求完美设计
- 确保团队可以理解

### 4. 文档化重构决策
- 记录为什么这样拆分
- 记录权衡决策
- 提供架构图

## 总结

这个分层架构将带来以下好处：

1. **更好的代码组织**: 按业务领域组织代码
2. **更高的可测试性**: Service 层可以轻松测试
3. **更好的可维护性**: 修改业务逻辑不影响其他层
4. **更高的灵活性**: 可以轻松替换基础设施实现
5. **更好的可扩展性**: 添加新功能更容易

建议按照本文档的步骤逐步进行重构，每一步都要确保代码能编译通过并运行测试。
