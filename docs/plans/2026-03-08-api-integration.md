# 前后端 API 集成计划

## 概述

本文档描述 Vansour Image 前端与后端 API 的完整集成步骤。

---

## 后端 API 概览

### 公共路由（无需认证）

| 路由 | 方法 | 功能 |
|--------|------|------|
| `/health` | GET | 健康检查 |
| `/thumbnails/{id}` | GET | 获取缩略图 |
| `/auth/login` | POST | 用户登录 |

### 认证路由（需要 AuthUser 中间件）

| 路由 | 方法 | 功能 |
|--------|------|------|
| `/auth/me` | GET | 获取当前用户 |
| `/auth/logout` | POST | 用户登出 |
| `/auth/change-password` | POST | 修改密码 |

### 图片路由（需要认证）

| 路由 | 方法 | 功能 |
|--------|------|------|
| `/upload` | POST | 上传图片 |
| `/images` | GET | 获取图片列表（分页） |
| `/images/cursor` | GET | 获取图片列表（cursor 分页） |
| `/images` | DELETE | 批量删除图片 |
| `/images/{id}` | GET | 获取单个图片 |
| `/images/{id}` | PUT | 更新图片 |
| `/images/{id}/edit` | POST | 编辑图片（裁剪、滤镜等） |
| `/images/{id}/rename` | PUT | 重命名图片 |
| `/images/{id}/expiry` | PUT | 设置过期时间 |
| `/images/{id}/duplicate` | POST | 复制图片 |
| `/images/deleted` | GET | 获取已删除图片 |
| `/images/restore` | POST | 恢复已删除图片 |

### 管理员路由（需要 AdminUser 中间件）

| 路由 | 方法 | 功能 |
|--------|------|------|
| `/cleanup` | POST | 清理已删除文件 |
| `/cleanup/expired` | POST | 清理过期图片 |
| `/backup` | POST | 备份数据库 |
| `/users` | GET | 获取用户列表 |
| `/users/{id}` | PUT | 更新用户角色 |
| `/audit-logs` | GET | 获取审计日志 |
| `/stats` | GET | 获取系统统计 |
| `/settings` | GET | 获取设置 |
| `/settings/{key}` | PUT | 更新设置 |

---

## 后端数据类型

### 认证相关

```rust
// 登录请求
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

// 用户响应
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

// 修改密码
pub struct UpdateProfileRequest {
    pub username: Option<String>,
    pub current_password: String,
    pub new_password: Option<String>,
}
```

### 图片相关

```rust
// 图片
pub struct Image {
    pub id: Uuid,
    pub user_id: Uuid,
    pub category_id: Option<Uuid>,
    pub filename: String,
    pub thumbnail: Option<String>,
    pub original_filename: Option<String>,
    pub size: i64,
    pub hash: String,
    pub format: String,
    pub views: i32,
    pub status: String,  // "active", "deleted"
    pub expires_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

// 分页参数
pub struct PaginationParams {
    pub page: Option<i32>,
    pub page_size: Option<i32>,
    pub sort_by: Option<String>,  // created_at, size, views, filename, hash
    pub sort_order: Option<String>,  // ASC, DESC
    pub search: Option<String>,
    pub category_id: Option<Uuid>,
    pub tag: Option<String>,
    pub cursor: Option<(DateTime<Utc>, String)>,  // Cursor 分页
}

// 分页响应
pub struct Paginated<T> {
    pub data: Vec<T>,
    pub page: i32,
    pub page_size: i32,
    pub total: i64,
    pub has_next: bool,
}

// Cursor 分页响应
pub struct CursorPaginated<T> {
    pub data: Vec<T>,
    pub next_cursor: Option<(DateTime<Utc>, String)>,
}
```

### 操作相关

```rust
// 批量删除
pub struct DeleteRequest {
    pub image_ids: Vec<Uuid>,
    pub permanent: bool,  // true=永久删除，false=移至回收站
}

// 恢复
pub struct RestoreRequest {
    pub image_ids: Vec<Uuid>,
}

// 重命名
pub struct RenameRequest {
    pub filename: String,
}

// 设置过期
pub struct SetExpiryRequest {
    pub expires_at: Option<DateTime<Utc>>,
}

// 复制
pub struct DuplicateRequest {
    pub image_id: Uuid,
}

// 更新分类
pub struct UpdateCategoryRequest {
    pub category_id: Option<Uuid>,
    pub tags: Option<Vec<String>>,
}
```

---

## 集成步骤

### 阶段 1：类型同步

**目标：** 前端类型与后端类型完全匹配

**任务：**

1. 更新 `frontend/src/types/api.rs`
   - 添加 `PaginationParams`
   - 添加 `Paginated<T>` 和 `CursorPaginated<T>`
   - 添加图片操作请求类型

2. 更新 `frontend/src/types/models.rs`
   - 同步 `ImageItem` 与后端 `Image` 结构
   - 添加 `status`、`expires_at`、`deleted_at` 字段
   - 添加辅助方法：`is_deleted()`, `is_expired()`, `size_formatted()`

### 阶段 2：Cookie 认证处理

**目标：** 实现 HttpOnly Cookie 认证

**任务：**

1. 创建 `frontend/src/utils/cookie.rs`
   ```rust
   // 解析和构建 HttpOnly Cookie
   pub fn parse_auth_token(cookie_header: &str) -> Option<String>
   pub fn build_auth_cookie(token: &str, max_age_days: u32) -> String
   ```

2. 更新 `ApiClient`
   - 添加 Cookie 管理逻辑
   - 从响应中提取 `Set-Cookie`
   - 在请求中添加 `Cookie: auth_token=...`

### 阶段 3：API 客户端完整实现

**目标：** 实现完整的 API 调用功能

**任务：**

1. 更新 `frontend/src/services/api_client.rs`
   ```rust
   impl ApiClient {
       // 基础方法
       pub fn url(&self, path: &str) -> String
       pub fn add_auth_header(&self, token: Option<&str>) -> HeaderMap

       // HTTP 方法
       pub async fn get<T>(&self, path: &str) -> Result<T>
       pub async fn post<T>(&self, path: &str, body: &impl Serialize) -> Result<T>
       pub async fn put<T>(&self, path: &str, body: &impl Serialize) -> Result<T>
       pub async fn delete(&self, path: &str) -> Result<()>
       pub async fn multipart<T>(&self, path: &str, file: &str) -> Result<T>

       // 高级方法
       pub async fn post_with_auth<T>(&self, path: &str, body: &impl Serialize) -> Result<T>
       pub async fn get_images(&self, params: &PaginationParams) -> Result<Paginated<ImageItem>>
       pub async fn get_images_cursor(&self, params: &PaginationParams) -> Result<CursorPaginated<ImageItem>>
   }
   ```

### 阶段 4：服务层实现

**目标：** 实现业务逻辑服务

**任务：**

1. 完善 `frontend/src/services/auth_service.rs`
   ```rust
   impl AuthService {
       pub async fn login(&self, req: LoginRequest) -> Result<UserResponse>
       pub async fn logout(&self) -> Result<()>
       pub async fn get_current_user(&self) -> Result<UserResponse>
       pub async fn change_password(&self, req: UpdateProfileRequest) -> Result<()>
   }
   ```

2. 完善 `frontend/src/services/image_service.rs`
   ```rust
   impl ImageService {
       pub async fn upload_image(&self, file_path: &str) -> Result<ImageItem>
       pub async fn get_images(&self, params: &PaginationParams) -> Result<Paginated<ImageItem>>
       pub async fn get_images_cursor(&self, params: &PaginationParams) -> Result<CursorPaginated<ImageItem>>
       pub async fn get_image(&self, id: Uuid) -> Result<ImageItem>
       pub async fn delete_images(&self, ids: Vec<Uuid>, permanent: bool) -> Result<()>
       pub async fn restore_images(&self, ids: Vec<Uuid>) -> Result<()>
       pub async fn rename_image(&self, id: Uuid, filename: String) -> Result<()>
       pub async fn set_expiry(&self, id: Uuid, expires_at: Option<DateTime<Utc>>) -> Result<()>
       pub async fn duplicate_image(&self, id: Uuid) -> Result<ImageItem>
       pub async fn get_deleted_images(&self) -> Result<Vec<ImageItem>>
       pub async fn update_image_category(&self, id: Uuid, category_id: Option<Uuid>, tags: Option<Vec<String>>) -> Result<()>
   }
   ```

### 阶段 5：Store 状态集成

**目标：** 连接 Store 与 API 服务

**任务：**

1. 更新 `frontend/src/store/images.rs`
   - 添加分页状态（page、page_size、total、has_more）
   - 添加 Cursor 状态
   - 添加过滤条件状态
   - 实现 `load_more()`, `refresh()`, `set_filters()`

2. 更新 `frontend/src/pages/image_list_page.rs`
   - 集成 ImageService
   - 实现无限滚动加载
   - 实现过滤和排序
   - 实现批量选择和操作

### 阶段 6：登录页面集成

**目标：** 实现真实的登录流程

**任务：**

1. 更新 `frontend/src/pages/login_page.rs`
   - 调用 AuthService::login()
   - 处理 Cookie 存储
   - 登录成功后跳转到首页
   - 显示真实的错误信息

### 阶段 7：路由和导航

**目标：** 实现页面路由

**任务：**

1. 创建路由枚举 `frontend/src/routes/mod.rs`
   ```rust
   #[derive(Debug, Clone, PartialEq, Routable)]
   pub enum Route {
       #[route("/login")]
       Login,
       #[route("/")]
       #[route("/images")]
       Images,
       #[route("/images/:id")]
       ImageDetail { id: String },
       #[route("/trash")]
       Trash,
   }
   ```

2. 更新 `frontend/src/app.rs`
   - 设置 Router 和路由配置
   - 实现 RouteGuard 进行认证检查

3. 更新 `frontend/src/routes/layout.rs`
   - 集成路由导航
   - 根据当前路由高亮导航项

---

## 技术考虑

### 1. 错误处理

- 网络错误：重试机制或显示友好提示
- 认证错误：401/403 自动跳转登录页
- 服务器错误：显示错误提示和重试选项

### 2. 加载状态

- 页面级别加载：首次加载
- 操作级别加载：上传、删除等
- 骨架刷新加载：无数据时显示骨架屏

### 3. 分页策略

- Cursor 分页：用于无限滚动（性能更好）
- 传统分页：用于统计页面或管理页面
- 切换：根据场景选择合适的分页方式

### 4. 缓存策略

- 图片列表缓存：避免重复请求
- 用户信息缓存：本地存储
- 失败重试：指数退避策略

### 5. 安全考虑

- Cookie 安全：HttpOnly、Secure、SameSite=Strict
- CSRF 保护：添加 CSRF token（如需要）
- 请求验证：输入验证和数据清理

---

## 开发顺序建议

1. **阶段 1（类型同步）** → 阶段 2（Cookie）
2. **阶段 2** → 阶段 3（API 客户端）
3. **阶段 3** → 阶段 4（服务层）
4. **阶段 4** → 阶段 5（Store 集成）
5. **阶段 5** → 阶段 6（登录页面）
6. **阶段 6** → 阶段 7（路由导航）

每完成一个阶段后运行 `cargo test` 确保测试通过。

---

## 测试策略

### 单元测试

- API 客户端方法测试（mock HTTP 响应）
- 服务层逻辑测试
- Cookie 解析测试

### 集成测试

- 启动后端服务器
- 使用真实 API 进行端到端测试
- 测试登录/登出流程
- 测试图片 CRUD 操作

---

## 预期成果

完成集成后，前端将能够：

1. ✅ 用户登录/登出
2. ✅ 查看图片列表（分页）
3. ✅ 上传图片
4. ✅ 删除/恢复图片
5. ✅ 重命名图片
6. ✅ 设置过期时间
7. ✅ 查看图片详情
8. ✅ 查看已删除图片（回收站）
9. ✅ 修改用户密码
10. ✅ 响应式布局
