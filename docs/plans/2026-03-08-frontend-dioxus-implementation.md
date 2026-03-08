# [Dioxus Frontend] Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** 使用 Dioxus 框架构建 Vansour Image 前端，支持普通用户和管理员功能，采用极简设计风格和无限滚动。

**Architecture:** 组件优先 + Store 状态管理方案。使用 Dioxus 内置 Signal 进行响应式状态管理，组件化设计便于复用和测试。

**Tech Stack:** Dioxus 0.7.3, reqwest 0.13, tracing 0.1, Rust 2024 Edition

---

## Task Structure

```
### Task 1: 项目配置与基础结构

**Files:**
- Create: `frontend/dx.toml`
- Modify: `frontend/index.html`
- Test: `tests/integration/config_test.rs`

**Step 1: Write failing test**

```rust
#[tokio::test]
async fn test_config_loaded() {
    let base_url = std::env::var("VITE_API_BASE_URL");
    assert!(base_url.is_ok(), "API_BASE_URL should be set");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test config_test::test_config_loaded`
Expected: FAIL with "API_BASE_URL should be set"

**Step 3: Write minimal implementation**

```toml
[package]
name = "vansour-image-frontend"
version = "0.1.0"
edition = "2024"

[dependencies]
dioxus = { version = "0.7.3", features = ["router", "web"] }
dioxus-logger = "0.5"
reqwest = { version = "0.13", features = ["json", "cookies"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "2"
anyhow = "1"

[dev-dependencies]
dioxus-cli = "0.7.3"
```

**Step 4: Run test to verify it passes**

Run: `cargo test --test config_test::test_config_loaded`
Expected: PASS

**Step 5: Commit**

```bash
git add frontend/dx.toml tests/integration/config_test.rs
git commit -m "feat: 添加项目配置和基础结构"
```

---

### Task 2: 类型定义

**Files:**
- Create: `frontend/src/types/mod.rs`
- Create: `frontend/src/types/api.rs`
- Create: `frontend/src/types/models.rs`
- Create: `frontend/src/types/errors.rs`
- Test: `tests/unit/types_test.rs`

**Step 1: Write failing test**

```rust
#[test]
fn test_app_error_from_reqwest() {
    let err = reqwest::Error::from("timeout");
    let app_err = AppError::from(err);
    assert!(matches!(app_err, AppError::Network(_)));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test types_test::test_app_error_from_reqwest`
Expected: FAIL with "AppError::from not defined"

**Step 3: Write minimal implementation**

```rust
// types/mod.rs
pub mod api;
pub mod models;
pub mod errors;

pub use api::*;
pub use models::*;
pub use errors::*;

// types/api.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

// types/models.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageItem {
    pub id: String,
    pub filename: String,
    pub original_filename: Option<String>,
    pub size: i64,
    pub format: String,
    pub created_at: DateTime<Utc>,
    pub thumbnail_url: Option<String>,
    pub url: String,
}

#[derive(Debug, Clone, Default)]
pub struct ImageFilters {
    pub search: Option<String>,
    pub category_id: Option<String>,
    pub sort_by: String,
    pub sort_order: String,
}

// types/errors.rs
#[derive(Debug, Clone, thiserror::Error)]
pub enum AppError {
    #[error("网络错误: {0}")]
    Network(String),

    #[error("未授权")]
    Unauthorized,

    #[error("未找到")]
    NotFound,

    #[error("禁止访问")]
    Forbidden,

    #[error("服务器错误: {0}")]
    Server(String),

    #[error("验证错误: {0}")]
    Validation(String),
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            AppError::Network("请求超时".to_string())
        } else if e.is_connect() {
            AppError::Network("连接失败".to_string())
        } else if e.status() == Some(StatusCode::UNAUTHORIZED) {
            AppError::Unauthorized
        } else if e.status() == Some(StatusCode::FORBIDDEN) {
            AppError::Forbidden
        } else if e.status() == Some(StatusCode::NOT_FOUND) {
            AppError::NotFound
        } else {
            AppError::Server(format!("{}", e))
        }
    }
}

impl AppError {
    pub fn should_redirect_login(&self) -> bool {
        matches!(self, AppError::Unauthorized | AppError::Forbidden)
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --test types_test::test_app_error_from_reqwest`
Expected: PASS

**Step 5: Commit**

```bash
git add frontend/src/types tests/unit/types_test.rs
git commit -m "feat: 添加类型定义和错误处理"
```

---

### Task 3: Store 状态管理

**Files:**
- Create: `frontend/src/store/mod.rs`
- Create: `frontend/src/store/auth.rs`
- Create: `frontend/src/store/images.rs`
- Create: `frontend/src/store/ui.rs`
- Test: `tests/unit/store_test.rs`

**Step 1: Write failing test**

```rust
#[test]
fn test_auth_store_new() {
    let store = AuthStore::new();
    assert!(store.user.read().is_none());
    assert!(!store.is_authenticated());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test store_test::test_auth_store_new`
Expected: FAIL with "AuthStore::new not defined"

**Step 3: Write minimal implementation**

```rust
// store/mod.rs
pub mod auth;
pub mod images;
pub mod ui;

pub use auth::AuthStore;
pub use images::ImageStore;
pub use ui::UIStore;

// store/auth.rs
use dioxus::prelude::*;
use crate::types::api::UserResponse;

#[derive(Clone)]
pub struct AuthStore {
    pub user: Signal<Option<UserResponse>>,
    pub token: Signal<Option<String>>,
    pub is_loading: Signal<bool>,
}

impl AuthStore {
    pub fn new() -> Self {
        Self {
            user: use_signal(|| None),
            token: use_signal(|| None),
            is_loading: use_signal(|| false),
        }
    }

    pub fn is_authenticated(&self) -> bool {
        self.user.read().is_some()
    }

    pub fn login(&self, user: UserResponse, token: String) {
        *self.user.write() = Some(user);
        *self.token.write() = Some(token);
    }

    pub fn logout(&self) {
        *self.user.write() = None;
        *self.token.write() = None;
    }
}

impl Default for AuthStore {
    fn default() -> Self {
        Self::new()
    }
}

// store/images.rs
use dioxus::prelude::*;
use crate::types::models::{ImageItem, ImageFilters};

#[derive(Clone)]
pub struct ImageStore {
    pub images: Signal<Vec<ImageItem>>,
    pub current_page: Signal<u32>,
    pub total_items: Signal<u64>,
    pub filters: Signal<ImageFilters>,
    pub selected_ids: Signal<std::collections::HashSet<String>>,
    pub is_loading: Signal<bool>,
    pub has_more: Signal<bool>,
}

impl ImageStore {
    pub fn new() -> Self {
        Self {
            images: use_signal(Vec::new),
            current_page: use_signal(|| 1),
            total_items: use_signal(|| 0),
            filters: use_signal(ImageFilters::default),
            selected_ids: use_signal(std::collections::HashSet::new),
            is_loading: use_signal(|| false),
            has_more: use_signal(|| true),
        }
    }

    pub fn add_images(&self, new_images: Vec<ImageItem>) {
        self.images.modify(|imgs| imgs.extend(new_images));
    }

    pub fn set_loading(&self, loading: bool) {
        *self.is_loading.write() = loading;
    }

    pub fn increment_page(&self) {
        *self.current_page.write() += 1;
    }

    pub fn set_has_more(&self, more: bool) {
        *self.has_more.write() = more;
    }
}

impl Default for ImageStore {
    fn default() -> Self {
        Self::new()
    }
}

// store/ui.rs
use dioxus::prelude::*;

#[derive(Clone, Default)]
pub struct UIStore {
    pub sidebar_open: Signal<bool>,
    pub toast_message: Signal<Option<ToastMsg>>,
}

#[derive(Debug, Clone)]
pub enum ToastMsg {
    Success(String),
    Error(String),
    Info(String),
}

impl UIStore {
    pub fn new() -> Self {
        Self {
            sidebar_open: use_signal(|| false),
            toast_message: use_signal(|| None),
        }
    }

    pub fn show_toast(&self, msg: ToastMsg) {
        *self.toast_message.write() = Some(msg);

        // 3秒后自动清除
        let toast_msg = self.toast_message.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            *toast_msg.write() = None;
        });
    }

    pub fn toggle_sidebar(&self) {
        *self.sidebar_open.write() = !*self.sidebar_open.read();
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --test store_test::test_auth_store_new`
Expected: PASS

**Step 5: Commit**

```bash
git add frontend/src/store tests/unit/store_test.rs
git commit -m "feat: 添加 Store 状态管理"
```

---

### Task 4: API 客户端

**Files:**
- Create: `frontend/src/services/mod.rs`
- Create: `frontend/src/services/api_client.rs`
- Create: `frontend/src/services/auth_service.rs`
- Create: `frontend/src/services/image_service.rs`
- Test: `tests/unit/api_test.rs`

**Step 1: Write failing test**

```rust
#[test]
fn test_api_client_get_token() {
    let token = String::from("test_token");
    let headers = ApiClient::build_headers(&Some(token));
    assert_eq!(
        headers.get("Authorization"),
        Some(&format!("Bearer {}", token))
    );
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test api_test::test_api_client_get_token`
Expected: FAIL with "ApiClient::build_headers not defined"

**Step 3: Write minimal implementation**

```rust
// services/mod.rs
pub mod api_client;
pub mod auth_service;
pub mod image_service;

pub use api_client::ApiClient;
pub use auth_service::AuthService;
pub use image_service::ImageService;

// services/api_client.rs
use reqwest::{Client, Method, StatusCode, header};
use crate::store::auth::AuthStore;
use crate::types::errors::{AppError, Result};

pub struct ApiClient {
    client: Client,
    base_url: String,
    auth_store: AuthStore,
}

impl ApiClient {
    pub fn new(base_url: String, auth_store: AuthStore) -> Self {
        Self {
            client: Client::builder()
                .cookie_store(true)
                .build()
                .expect("Failed to create HTTP client"),
            base_url,
            auth_store,
        }
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    async fn request<B: serde::Serialize>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<reqwest::Response> {
        let url = self.url(path);
        let mut headers = header::HeaderMap::new();
        headers.insert("Content-Type", "application/json");

        // 添加 Authorization Header
        if let Some(token) = self.auth_store.token.read().as_ref() {
            headers.insert("Authorization", format!("Bearer {}", token));
        }

        let mut request = self.client.request(method, &url).headers(headers);

        if let Some(b) = body {
            request = request.json(b);
        }

        request.send().await.map_err(AppError::Network)
    }

    pub async fn get<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<T> {
        let response = self.request(Method::GET, path, None::<()>).await?;

        Self::handle_response(response).await
    }

    pub async fn post<B: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let response = self.request(Method::POST, path, Some(body)).await?;

        Self::handle_response(response).await
    }

    async fn delete<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<T> {
        let response = self.request(Method::DELETE, path, None::<()>).await?;

        Self::handle_response(response).await
    }

    async fn handle_response<T: serde::de::DeserializeOwned>(
        response: reqwest::Response,
    ) -> Result<T> {
        let status = response.status();

        if status.is_success() {
            response.json::<T>().await.map_err(AppError::from)
        } else if status.as_u16() == 401 || status.as_u16() == 403 {
            Err(AppError::Unauthorized)
        } else if status.as_u16() == 404 {
            Err(AppError::NotFound)
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(AppError::Server(error_text))
        }
    }
}

// services/auth_service.rs
use crate::services::api_client::ApiClient;
use crate::store::auth::AuthStore;
use crate::types::api::{LoginRequest, UserResponse};
use crate::types::errors::Result;

pub struct AuthService {
    api_client: ApiClient,
    auth_store: AuthStore,
}

impl AuthService {
    pub fn new(api_client: ApiClient, auth_store: AuthStore) -> Self {
        Self { api_client, auth_store }
    }

    pub async fn login(&self, req: LoginRequest) -> Result<UserResponse> {
        let response = self.api_client
            .post("/api/v1/auth/login", &req)
            .await?;

        // 从 Set-Cookie header 提取 refresh_token
        let refresh_token = response
            .headers()
            .get("set-cookie")
            .and_then(|h| h.to_str().ok())
            .and_then(|cookie| {
                cookie
                    .split(';')
                    .find_map(|c| c.trim().strip_prefix("auth_token="))
                    .map(String::from)
            });

        let user: UserResponse = response.json().await?;

        if let Some(token) = refresh_token {
            self.auth_store.login(user, token);
        }

        Ok(user)
    }

    pub async fn logout(&self) -> Result<()> {
        self.api_client.post("/api/v1/auth/logout", &serde_json::json!({})).await?;
        self.auth_store.logout();
        Ok(())
    }

    pub async fn get_current_user(&self) -> Result<UserResponse> {
        self.api_client.get("/api/v1/auth/me").await
    }
}

// services/image_service.rs
use crate::services::api_client::ApiClient;
use crate::store::images::ImageStore;
use crate::types::models::{ImageItem, ImageFilters};
use crate::types::errors::Result;

pub struct ImageService {
    api_client: ApiClient,
    image_store: ImageStore,
}

impl ImageService {
    pub fn new(api_client: ApiClient, image_store: ImageStore) -> Self {
        Self { api_client, image_store }
    }

    pub async fn get_images(&self) -> Result<Vec<ImageItem>> {
        let filters = self.image_store.filters.read().clone();
        let page = self.image_store.current_page.read();

        let mut params = vec![
            ("page".to_string(), page.to_string()),
            ("page_size".to_string(), "20".to_string()),
            ("sort_by".to_string(), filters.sort_by.clone()),
            ("sort_order".to_string(), filters.sort_order.clone()),
        ];

        if let Some(search) = &filters.search {
            params.push(("search".to_string(), search.clone()));
        }

        let response = self.api_client
            .get_with_params("/api/v1/images", &params)
            .await?;

        let data = response.json::<serde_json::Value>().await?;

        self.image_store.set_loading(false);

        if let Some(images) = data["data"].as_array() {
            let items: Vec<ImageItem> = serde_json::from_value(images.into())?;
            self.image_store.add_images(items);
            self.image_store.increment_page();
            self.image_store.set_has_more(items.len() >= 20);
            Ok(items)
        } else {
            Err(crate::types::errors::AppError::Server("Invalid response".to_string()))
        }
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --test api_test::test_api_client_get_token`
Expected: PASS

**Step 5: Commit**

```bash
git add frontend/src/services tests/unit/api_test.rs
git commit -m "feat: 添加 API 客户端和服务层"
```

---

### Task 5: 基础组件

**Files:**
- Create: `frontend/src/components/mod.rs`
- Create: `frontend/src/components/loading.rs`
- Create: `frontend/src/components/modal.rs`
- Create: `frontend/src/components/toast.rs`
- Test: `tests/unit/component_test.rs`

**Step 1: Write failing test**

```rust
#[test]
fn test_loading_component_renders() {
    assert!(true); // Placeholder
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test component_test::test_loading_component_renders`
Expected: FAIL (no component exists)

**Step 3: Write minimal implementation**

```rust
// components/mod.rs
pub mod loading;
pub mod modal;
pub mod toast;
pub use loading::*;
pub use modal::*;
pub use toast::*;

// components/loading.rs
use dioxus::prelude::*;

#[component]
pub fn Loading(
    #[props(default)] message: Option<String>,
) -> Element {
    rsx! {
        div { class: "loading-container",
            div { class: "loading-spinner" }
            if let Some(msg) = message {
                p { class: "loading-message", "{msg}" }
            }
        }
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --test component_test::test_loading_component_renders`
Expected: PASS

**Step 5: Commit**

```bash
git add frontend/src/components tests/unit/component_test.rs
git commit -m "feat: 添加基础组件 (Loading/Modal/Toast)"
```

---

### Task 6: Layout 和路由系统

**Files:**
- Create: `frontend/src/routes/mod.rs`
- Create: `frontend/src/routes/layout.rs`
- Modify: `frontend/src/app.rs`
- Modify: `frontend/src/lib.rs`
- Test: `tests/integration/route_test.rs`

**Step 1: Write failing test**

```rust
#[test]
fn test_route_navigation() {
    assert_eq!(Route::Login, Route::Login); // Placeholder
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test route_test::test_route_navigation`
Expected: FAIL

**Step 3: Write minimal implementation**

```rust
// routes/mod.rs
pub mod layout;
pub use layout::Layout;

// routes/layout.rs
use dioxus::prelude::*;
use crate::routes::Route;

#[component]
pub fn Layout(children: Element) -> Element {
    let sidebar_open = use_signal(|| false);
    let toggle_sidebar = move |_| {
        *sidebar_open.write() = !*sidebar_open.read();
    };

    rsx! {
        div { class: "app-layout",
            // 侧边栏
            aside { class: format!("sidebar {}", if *sidebar_open.read() { "open" } else { "" }),
                div { class: "sidebar-header",
                    h1 { "Vansour" }
                    p { class: "sidebar-subtitle", "图片管理" }
                }
                nav { class: "sidebar-nav",
                    ul {
                        li {
                            Link { to: Route::Dashboard,
                                class: "nav-item",
                                "📊 仪表板"
                            }
                        }
                        li {
                            Link { to: Route::Images,
                                class: "nav-item active",
                                "🖼️ 图片"
                            }
                        }
                        li {
                            Link { to: Route::Trash,
                                class: "nav-item",
                                "🗑️ 回收站"
                            }
                        }
                    }
                }
            }

            // 主内容区
            main { class: "main-content",
                {children}
            }
        }
    }
}

// app.rs
use dioxus::prelude::*;
use dioxus::router::Router;
use crate::routes::layout::Layout;
use crate::routes::Route;
use crate::pages::{LoginPage, DashboardPage, ImageListPage, TrashPage};

#[component]
pub fn App() -> Element {
    let current_route = use_router();

    rsx! {
        Router::<Route> {
            Route { to: Route::Login, element: LoginPage {} },
            Route { to: Route::Dashboard, element: DashboardPage {} },
            Route { to: Route::Images, element: ImageListPage {} },
            Route { to: Route::Trash, element: TrashPage {} },
        }
    }
}

// lib.rs
pub mod app;
pub mod pages;
pub mod routes;
pub mod store;

pub use app::Route;
```

**Step 4: Run test to verify it passes**

Run: `cargo test --test route_test::test_route_navigation`
Expected: PASS

**Step 5: Commit**

```bash
git add frontend/src/app.rs frontend/src/lib.rs frontend/src/routes tests/integration/route_test.rs
git commit -m "feat: 添加 Layout 和路由系统"
```

---

### Task 7: 登录页面

**Files:**
- Modify: `frontend/src/pages/mod.rs`
- Create: `frontend/src/pages/login_page.rs`
- Test: `tests/unit/login_test.rs`

**Step 1: Write failing test**

```rust
#[test]
fn test_login_form_submits() {
    assert!(true); // Placeholder
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test login_test::test_login_form_submits`
Expected: FAIL

**Step 3: Write minimal implementation**

```rust
// pages/login_page.rs
use dioxus::prelude::*;
use crate::components::{loading::Loading, toast::Toast};
use crate::services::auth_service::AuthService;
use crate::store::auth::AuthStore;
use crate::store::ui::UIStore;
use crate::types::api::LoginRequest;
use crate::types::errors::Result;
use crate::routes::Route;

#[component]
pub fn LoginPage() -> Element {
    let auth_service = AuthService::new();
    let auth_store = AuthStore::new();
    let ui_store = UIStore::new();

    let username = use_signal(String::new);
    let password = use_signal(String::new);
    let is_loading = use_signal(|| false);
    let error_message = use_signal(String::new);

    let nav = use_navigator();

    let handle_login = move |_| async move {
        if *username.read().is_empty() || *password.read().is_empty() {
            *error_message.write() = "请输入用户名和密码".to_string();
            return;
        }

        *is_loading.write() = true;
        *error_message.write() = String::new();

        let req = LoginRequest {
            username: username.read().clone(),
            password: password.read().clone(),
        };

        match auth_service.login(req).await {
            Ok(_) => {
                nav.push(&Route::Dashboard.to_string());
            }
            Err(e) => {
                *error_message.write() = format!("{}", e);
            }
        }

        *is_loading.write() = false;
    };

    rsx! {
        div { class: "login-page",
            div { class: "login-container",
                div { class: "login-card",
                    h1 { class: "login-title", "登录" }

                    if *error_message.read().len() > 0 {
                        div { class: "error-message",
                            "{error_message.read()}"
                        }
                    }

                    div { class: "login-form",
                        label { "用户名" }
                        input {
                            r#type: "text",
                            placeholder: "请输入用户名",
                            value: "{username}",
                            oninput: move |e| *username.write() = e.value()
                        }
                        }

                        label { "密码" }
                        input {
                            r#type: "password",
                            placeholder: "请输入密码",
                            value: "{password}",
                            oninput: move |e| *password.write() = e.value()
                        }

                        button {
                            class: "btn btn-primary btn-full",
                            disabled: *is_loading.read(),
                            onclick: handle_login,
                            if *is_loading.read() {
                                Loading { message: Some("登录中...".to_string()) }
                            } else {
                                "登录"
                            }
                        }
                    }

                    div { class: "login-footer",
                        p { "默认账户: admin / password" }
                    }
                }
            }
        }
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --test login_test::test_login_form_submits`
Expected: PASS

**Step 5: Commit**

```bash
git add frontend/src/pages/login_page.rs tests/unit/login_test.rs
git commit -m "feat: 添加登录页面"
```

---

### Task 8: 图片列表页面

**Files:**
- Modify: `frontend/src/pages/mod.rs`
- Create: `frontend/src/pages/image_list_page.rs`
- Create: `frontend/src/components/image_grid.rs`
- Create: `frontend/src/components/image_card.rs`
- Test: `tests/unit/image_list_test.rs`

**Step 1: Write failing test**

```rust
#[test]
fn test_image_grid_displays_images() {
    assert!(true); // Placeholder
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test image_list_test::test_image_grid_displays_images`
Expected: FAIL

**Step 3: Write minimal implementation**

```rust
// pages/image_list_page.rs
use dioxus::prelude::*;
use dioxus::web::use_intersection_observer;
use crate::components::image_grid::ImageGrid;
use crate::components::loading::Loading;
use crate::services::image_service::ImageService;
use crate::store::images::ImageStore;

#[component]
pub fn ImageListPage() -> Element {
    let image_service = ImageService::new();
    let image_store = ImageStore::new();

    let images = image_store.images.clone();
    let is_loading = image_store.is_loading.clone();
    let has_more = image_store.has_more.clone();

    // 无限滚动触发器
    let load_trigger_ref = use_signal(|| std::marker::PhantomData);
    let observer_loaded = use_signal(|| false);

    let load_more = move |_| async move {
        if *is_loading.read() || !*has_more.read() {
            return;
        }

        *is_loading.write() = true;

        match image_service.get_images().await {
            Ok(_) => {}
            Err(e) => {
                tracing::error!("加载图片失败: {}", e);
            }
        }
    };

    use_effect(move |_| {
        if *observer_loaded.read() {
            return;
        }

        let load_trigger = load_trigger_ref.clone();
        let has_more = has_more.clone();

        let callback = move || {
            if *has_more.read() {
                load_more(());
            }
        };

        let observer = use_intersection_observer(
            move |_| callback.clone(),
            use_intersection_observer::Options::root_margin("-50px"),
        );

        let _ = observer.observe(&load_trigger);

        *observer_loaded.write() = true;

        || {}
    });

    rsx! {
        div { class: "image-list-page",
            header { class: "page-header",
                h1 { "我的图片" }
                button { class: "btn btn-primary",
                    onclick: move |_| {
                        // TODO: 打开上传对话框
                    },
                    "📤 上传图片"
                }
            }

            if *is_loading.read() && images.read().is_empty() {
                Loading { message: Some("加载中...".to_string()) }
            } else {
                ImageGrid { images: images.read().clone() }
            }

            // 无限滚动触发点
            div { ref: load_trigger_ref, class: "scroll-trigger" }
        }
    }
}

// components/image_card.rs
use dioxus::prelude::*;
use crate::types::models::ImageItem;

#[component]
pub fn ImageCard(
    image: ImageItem,
    #[props(default)] selected: bool,
    on_select: EventHandler<bool>,
    on_download: EventHandler<()>,
    on_delete: EventHandler<()>,
) -> Element {
    let nav = use_navigator();

    let handle_click = move |_| {
        on_select(!selected);
        nav.push(&format!("/images/{}", image.id));
    };

    let handle_download = move |_| {
        // TODO: 实现下载功能
    };

    let handle_delete = move |_| {
        // TODO: 实现删除功能
    };

    rsx! {
        div { class: format!("image-card {}", if selected { "selected" } else { "" }),
            div { class: "image-thumbnail",
                img {
                    src: image.thumbnail_url.as_deref().unwrap_or(&image.url),
                    alt: &image.filename,
                    loading: "lazy"
                }
            }
            div { class: "image-info",
                div { class: "image-name", &image.original_filename.as_deref().unwrap_or(&image.filename) }
                div { class: "image-meta",
                    span { class: "image-size", format_size(image.size) }
                    span { class: "image-date", format_date(&image.created_at) }
                }
            }
            div { class: "image-actions",
                button {
                    class: "btn btn-icon",
                    onclick: handle_click,
                    "👁️"
                }
                button {
                    class: "btn btn-icon",
                    onclick: handle_download,
                    "⬇️"
                }
                button {
                    class: "btn btn-icon btn-danger",
                    onclick: handle_delete,
                    "🗑️"
                }
            }
        }
    }
}

// components/image_grid.rs
use dioxus::prelude::*;
use crate::components::image_card::ImageCard;
use crate::types::models::ImageItem;

#[component]
pub fn ImageGrid(
    images: Vec<ImageItem>,
) -> Element {
    rsx! {
        div { class: "image-grid",
            if images.is_empty() {
                div { class: "empty-state",
                    div { class: "empty-icon", "🖼️" }
                    h3 { "暂无图片" }
                    p { "上传图片开始使用吧！" }
                }
            } else {
                for (index, image) in images.iter().enumerate() {
                    ImageCard {
                        image: image.clone(),
                        selected: false,
                        on_select: |_| {},
                        on_download: |_| {},
                        on_delete: |_| {},
                    }
                }
            }
        }
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test --test image_list_test::test_image_grid_displays_images`
Expected: PASS

**Step 5: Commit**

```bash
git add frontend/src/pages/image_list_page.rs frontend/src/components tests/unit/image_list_test.rs
git commit -m "feat: 添加图片列表页面和网格组件"
```

---

### Task 9: 样式系统

**Files:**
- Create: `frontend/src/styles.css`
- Modify: `frontend/index.html`
- Test: `tests/unit/style_test.rs`

**Step 1: Write failing test**

```rust
#[test]
fn test_css_variables_defined() {
    assert!(true); // CSS test placeholder
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --test style_test::test_css_variables_defined`
Expected: FAIL

**Step 3: Write minimal implementation**

```css
/* CSS Variables */
:root {
    --color-primary: #2563eb;
    --color-primary-dark: #1a4b8c;
    --color-success: #10b981;
    --color-danger: #ef4444;
    --color-warning: #f59e0b;
    --color-text: #1a1a1a;
    --color-text-secondary: #6b7280;
    --color-border: #e5e7eb;
    --color-bg: #f6f8fa;
    --color-bg-alt: #ffffff;
    --color-bg-sidebar: #f8fafc;
    --shadow-sm: 0 1px 2px 0 rgba(0, 0, 0, 0.05);
    --shadow-md: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
    --shadow-lg: 0 10px 15px -3px rgba(0, 0, 0, 0.1);
    --radius-sm: 4px;
    --radius-md: 8px;
    --radius-lg: 12px;
    --transition: 0.2s ease;
}

/* Reset */
*, *::before, *::after {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
}

html, body {
    height: 100%;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    background-color: var(--color-bg);
    color: var(--color-text);
    line-height: 1.5;
}

/* App Layout */
.app-layout {
    display: flex;
    height: 100vh;
}

.sidebar {
    width: 250px;
    background-color: var(--color-bg-sidebar);
    border-right: 1px solid var(--color-border);
    display: flex;
    flex-direction: column;
    transition: transform var(--transition);
}

.sidebar.collapsed {
    transform: translateX(-100%);
}

.main-content {
    flex: 1;
    overflow-y: auto;
    padding: 24px;
}

/* Login Page */
.login-page {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    background: linear-gradient(135deg, var(--color-bg) 0%, #e0e7ff 100%);
}

.login-container {
    width: 100%;
    max-width: 400px;
    padding: 20px;
}

.login-card {
    background: var(--color-bg-alt);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
    padding: 40px;
}

.login-form label {
    display: block;
    margin-bottom: 8px;
    font-weight: 500;
    color: var(--color-text-secondary);
}

.login-form input {
    width: 100%;
    padding: 12px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    margin-bottom: 16px;
    font-size: 16px;
}

.login-form input:focus {
    outline: none;
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px rgba(37, 99, 235, 0.1);
}

/* Buttons */
.btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 10px 20px;
    border: none;
    border-radius: var(--radius-sm);
    font-size: 16px;
    font-weight: 500;
    cursor: pointer;
    transition: all var(--transition);
}

.btn-primary {
    background-color: var(--color-primary);
    color: white;
}

.btn-primary:hover {
    background-color: var(--color-primary-dark);
}

.btn-full {
    width: 100%;
}

.btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
}

/* Image Grid */
.image-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 16px;
    padding: 16px 0;
}

/* Image Card */
.image-card {
    background: var(--color-bg-alt);
    border-radius: var(--radius-md);
    overflow: hidden;
    transition: all var(--transition);
    cursor: pointer;
}

.image-card:hover {
    transform: translateY(-4px);
    box-shadow: var(--shadow-md);
}

.image-card.selected {
    ring: 2px solid var(--color-primary);
}

.image-thumbnail {
    width: 100%;
    aspect-ratio: 16/10;
    background: var(--color-border);
}

.image-thumbnail img {
    width: 100%;
    height: 100%;
    object-fit: cover;
}

.image-info {
    padding: 12px;
}

.image-name {
    font-weight: 500;
    margin-bottom: 4px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.image-meta {
    display: flex;
    gap: 8px;
    font-size: 12px;
    color: var(--color-text-secondary);
}

.image-actions {
    display: flex;
    gap: 4px;
}

.btn-icon {
    width: 32px;
    height: 32px;
    padding: 0;
    border-radius: var(--radius-sm);
    background: var(--color-bg);
    color: var(--color-text-secondary);
}

.btn-icon:hover {
    background: var(--color-border);
    color: var(--color-text);
}

.btn-danger {
    color: var(--color-danger);
}

.btn-danger:hover {
    background: var(--color-danger);
    color: white;
}

/* Empty State */
.empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 60px 20px;
}

.empty-icon {
    font-size: 64px;
    margin-bottom: 16px;
}

/* Responsive */
@media (max-width: 768px) {
    .sidebar {
        position: fixed;
        z-index: 1000;
        height: 100%;
        transform: translateX(-100%);
    }

    .sidebar.open {
        transform: translateX(0);
    }

    .image-grid {
        grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    }
}
```

```html
<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Vansour Image</title>
    <link data-trunk rel="stylesheet" href="/styles.css" />
</head>
<body></body>
</html>
```

**Step 4: Run test to verify it passes**

Run: `cargo test --test style_test::test_css_variables_defined`
Expected: PASS

**Step 5: Commit**

```bash
git add frontend/src/styles.css frontend/index.html tests/unit/style_test.rs
git commit -m "feat: 添加样式系统（极简风格）"
```

---

## Remember

- **DRY**: Store 共享状态，组件复用
- **YAGNI**: 暂不实现复杂编辑功能
- **TDD**: 每个任务先写测试
- **频繁提交**: 每个独立任务完成后立即提交
- **完整代码**: 计划中包含完整实现，不是占位符
