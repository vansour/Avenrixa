# Vansour Image Frontend Design

**日期**: 2026-03-08
**框架**: Dioxus (Rust Web Framework)
**用户**: 普通用户 + 管理员

---

## 1. 整体架构

```
┌─────────────────────────────────────────────────┐
│              前端架构 (Dioxus + Rust)          │
└─────────────────────────────────────────────────┘

┌────────────┐    ┌────────────┐    ┌────────────┐
│   App     │────▶│   Router   │────▶│   Layout   │
│  (根组件)  │    │  (路由枚举)  │    │  (布局容器)  │
└────────────┘    └────────────┘    └────────────┘
                                       │
                                       ▼
         ┌─────────────────────────────────┐
         │       Pages (页面组件)       │
         │  Login | Dashboard | Images │
         │  ImageDetail | Admin*       │
         └─────────────────────────────────┘
                    │
                    ▼
         ┌─────────────────────────────────┐
         │      Components (UI组件)      │
         │  ImageGrid | ImageCard        │
         │  UploadArea | ImageViewer      │
         │  Loading | Toast | Modal        │
         └─────────────────────────────────┘
                    │
                    ▼
         ┌─────────────────────────────────┐
         │       Store (状态管理)        │
         │  AuthStore | ImageStore        │
         │  UIStore (加载/选择)         │
         └─────────────────────────────────┘
                    │
                    ▼
         ┌─────────────────────────────────┐
         │     Services (API层)          │
         │  ApiClient | AuthService       │
         │  ImageService | AdminService     │
         └─────────────────────────────────┘
                    │
                    ▼
         ┌─────────────────────────────────┐
         │      Types (类型定义)          │
         │  API | Models | Errors          │
         └─────────────────────────────────┘
```

---

## 2. 数据流设计

### 认证流程

```
用户登录
    │
    ▼
LoginRequest → AuthService.login()
    │
    ▼
返回 (UserResponse, refresh_token)
    │
    ├─────────────┐
    │             │
    ▼             ▼
设置Cookie      AuthStore.user = user
    │
    ▼
导航到 Dashboard
```

### 图片上传流程

```
选择文件 → UploadArea组件
    │
    ▼
ImageService.upload(file, token)
    │
    ├─────────────┬─────────────┐
    │             │             │
    ▼             ▼             ▼
UIStore       返回Image    ImageStore
显示加载    加入列表
    │
    └─────────────┘
    ▼
清除加载状态
```

### 无限滚动加载

```
滚动到底部
    │
    ▼
IntersectionObserver 检测
    │
    ▼
ImageStore.current_page + 1
    │
    ▼
ImageService.get_images(page, filters)
    │
    ▼
追加到 ImageStore.images
```

---

## 3. 组件结构设计

### 组件层次树

```
App
├── LoginPage
│   └── LoginForm
│       ├── Input (username)
│       ├── Input (password)
│       └── Button
│
├── Layout
│   ├── Sidebar (导航菜单)
│   │   ├── NavItem (Dashboard)
│   │   ├── NavItem (Images)
│   │   ├── NavItem (Trash)
│   │   ├── NavItem (Admin*)
│   │   └── UserMenu (用户信息/登出)
│   │
│   └── MainContent
│       └── (路由页面)
│
├── DashboardPage
│   └── StatsCards (统计概览)
│
├── ImageListPage
│   ├── HeaderBar (标题 + 搜索 + 上传按钮)
│   ├── ImageGrid
│   │   └── ImageCard * N
│   │       ├── Thumbnail
│   │       ├── Info (文件名/大小/日期)
│   │       └── Actions (查看/删除/下载)
│   └── LoadingSkeleton (骨架屏)
│
├── ImageDetailPage
│   └── ImageViewer
│       ├── LargeImage
│       └── InfoPanel
│
├── TrashPage
│   └── DeletedImageList
│
└── Admin*Pages
    ├── AdminDashboard (系统统计)
    ├── AdminSettings (配置管理)
    ├── AdminAuditLogs (审计日志)
    └── AdminBackup (数据库备份)
```

### 核心组件职责

| 组件 | 职责 |
|------|------|
| `ImageCard` | 单张图片展示，缩略图+元数据+操作按钮 |
| `ImageGrid` | 响应式网格布局，支持选择状态 |
| `UploadArea` | 拖拽上传，文件选择，进度显示 |
| `ImageViewer` | 全屏查看器，支持缩放和下载 |
| `Toast` | 全局消息提示 |
| `Loading` | 加载状态和骨架屏 |
| `Pagination` | 无限滚动触发器（IntersectionObserver） |

---

## 4. 状态管理设计

### Store 架构

```rust
// 全局状态通过 use_signal 共享

// AuthStore - 认证状态
pub struct AuthStore {
    pub user: Signal<Option<UserResponse>>,
    pub token: Signal<Option<String>>,      // Authorization Header token
    pub is_loading: Signal<bool>,
}

// ImageStore - 图片状态
pub struct ImageStore {
    pub images: Signal<Vec<ImageItem>>,
    pub current_page: Signal<u32>,
    pub total_items: Signal<u64>,
    pub filters: Signal<ImageFilters>,
    pub selected_ids: Signal<HashSet<String>>,
    pub is_loading: Signal<bool>,
    pub has_more: Signal<bool>,  // 无限滚动标志
}

// UIStore - UI 状态
pub struct UIStore {
    pub sidebar_open: Signal<bool>,
    pub theme: Signal<Theme>,
    pub toast_message: Signal<Option<ToastMsg>>,
}
```

### 状态更新模式

```rust
// 示例：加载更多图片
pub async fn load_more_images() {
    // 1. 检查是否正在加载
    if *image_store.is_loading.read() {
        return;
    }

    // 2. 检查是否还有更多
    if !*image_store.has_more.read() {
        return;
    }

    // 3. 设置加载状态
    *image_store.is_loading.write() = true;

    // 4. 请求 API
    let page = *image_store.current_page.read() + 1;
    let result = image_service.get_images(
        page,
        *image_store.filters.read()
    ).await;

    // 5. 更新状态
    match result {
        Ok(data) => {
            image_store.images.modify(|imgs| imgs.extend(data));
            *image_store.current_page.write() = page;
            *image_store.has_more.write() = data.len() >= PAGE_SIZE;
        }
        Err(e) => {
            ui_store.show_error(e.to_string());
        }
    }

    // 6. 清除加载状态
    *image_store.is_loading.write() = false;
}
```

---

## 5. API 客户端设计

### 统一错误处理

```rust
// types/errors.rs
#[derive(Debug, Clone)]
pub enum AppError {
    Network(String),
    Unauthorized,
    NotFound,
    Forbidden,
    Server(String),
    Validation(String),
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            AppError::Network("请求超时".to_string())
        } else if e.is_connect() {
            AppError::Network("连接失败".to_string())
        } else {
            AppError::Server(format!("{}", e))
        }
    }
}

// 自动处理 401/403 跳转登录
impl AppError {
    pub fn should_redirect_login(&self) -> bool {
        matches!(self, AppError::Unauthorized | AppError::Forbidden)
    }
}
```

### API 客户端

```rust
// services/api_client.rs
pub struct ApiClient {
    base_url: String,
    token: Signal<Option<String>>,  // 从 AuthStore 获取
}

impl ApiClient {
    // 自动添加 Authorization Header
    async fn request(&self, method: Method, path: &str) -> Result<Response, AppError> {
        let token = self.token.read();
        let mut headers = HeaderMap::new();

        if let Some(t) = token.as_ref() {
            headers.insert("Authorization", format!("Bearer {}", t));
        }

        // 发送请求...
    }
}
```

### 服务层封装

```rust
// services/image_service.rs
pub async fn get_images(
    page: u32,
    filters: ImageFilters
) -> Result<Vec<ImageItem>, AppError> {
    api_client.get("/api/v1/images", params).await?
        .json()
        .await?
}
```

---

## 6. 样式与响应式设计

### 设计风格

- **极简风格**: 参考 GitHub/Unsplash，内容优先
- **配色方案**:
  - 主色: `#2563eb` (蓝色)
  - 背景: `#f6f8fa` (浅灰)
  - 卡片: `#ffffff` (白色)
  - 文字: `#1a1a1a` (深灰)
- **间距系统**: 4px 基准
- **字体**: 系统默认，保持原生体验

### 响应式断点

```css
--mobile: 480px
--tablet: 768px
--desktop: 1024px
--wide: 1280px

/* 图片网格 */
.image-grid {
    display: grid;
    gap: 16px;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
}

/* 移动端单列 */
@media (max-width: 480px) {
    .image-grid {
        grid-template-columns: 1fr;
    }
}
```

---

## 7. 关键特性实现

### 无限滚动

- 使用 `IntersectionObserver` 检测底部元素
- 触发时调用 `load_more_images()`
- 防抖处理，避免重复触发
- 加载中禁用触发

### Token 管理

- 存储: 使用 `Signal<Option<String>>`
- 请求: 每次自动添加 `Authorization: Bearer {token}`
- 失效: 401/403 自动跳转登录页

### 拖拽上传

- 使用 HTML5 Drag & Drop API
- 支持多文件选择
- 实时进度显示
- 文件类型验证

---

## 8. 测试策略

### 单元测试

- Store 状态逻辑测试
- 工具函数测试
- 类型序列化测试

### 集成测试

- API 客户端 Mock 测试
- 组件渲染测试 (使用 dioxus-testing)

### E2E 测试

- 关键用户流程测试
- 使用 Playwright/WebDriver

---

## 9. 开发优先级

### Phase 1: 基础架构
1. 项目配置 (dx.toml, index.html)
2. 路由系统实现
3. Layout 组件
4. AuthStore 实现
5. API 客户端基础

### Phase 2: 认证功能
1. 登录页面
2. LoginForm 组件
3. 登录 API 调用
4. Token 存储与验证
5. 登出功能

### Phase 3: 图片列表
1. ImageList 页面
2. ImageGrid 组件
3. ImageCard 组件
4. 无限滚动实现
5. 加载状态与骨架屏

### Phase 4: 图片上传
1. UploadArea 组件
2. 拖拽上传
3. 文件选择
4. 上传进度
5. 上传成功处理

### Phase 5: 图片详情
1. ImageDetail 页面
2. ImageViewer 组件
3. 下载功能
4. 返回列表

### Phase 6: 回收站
1. Trash 页面
2. 已删除图片列表
3. 恢复功能
4. 永久删除

### Phase 7: 管理面板
1. AdminDashboard 页面
2. AdminSettings 页面
3. AdminAuditLogs 页面
4. AdminBackup 页面

### Phase 8: 优化与部署
1. 性能优化
2. 响应式完善
3. 错误处理完善
4. 构建 & 部署

---

## 附录: 技术选型

| 类别 | 选择 | 原因 |
|------|------|------|
| 前端框架 | Dioxus | Rust 原生，类型安全 |
| HTTP 客户端 | reqwest | Rust 生态成熟，支持 Cookie |
| 状态管理 | Signal (Dioxus 内置) | 响应式，无额外依赖 |
| 样式方案 | CSS 模块 + Tailwind | 简洁高效 |
| 构建 | Trunk | Dioxus 官方推荐 |
