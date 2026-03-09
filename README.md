# Vansour Image

一个基于 Rust Workspace 的图片管理项目，包含：

- `backend`：Axum + SQLx + Redis/Dragonfly 的后端服务
- `frontend`：Dioxus Web 前端

## 快速启动

使用 Docker Compose：

```bash
docker compose up --build
```

服务默认监听 `http://localhost:8080`。

## 首次启动认证要求

首次启动时必须显式提供管理员初始化信息：

- `JWT_SECRET`：生产环境请使用足够长的随机字符串
- `ADMIN_USERNAME`
- `ADMIN_EMAIL`
- `ADMIN_PASSWORD`

其中 `ADMIN_PASSWORD` 至少需要 12 位，后端会在启动期校验。

## 密码重置配置

项目已支持短时访问令牌 + 刷新令牌，以及邮件密码重置。若需启用邮件找回，请至少配置：

- `MAIL_ENABLED=true`
- `SMTP_HOST`
- `SMTP_PORT`
- `MAIL_FROM`
- `MAIL_FROM_NAME`
- `RESET_LINK_BASE_URL`

如果 SMTP 服务器要求认证，还需要同时设置：

- `SMTP_USER`
- `SMTP_PASSWORD`

注意：

- `SMTP_USER` 和 `SMTP_PASSWORD` 必须同时配置或同时留空。
- `RESET_LINK_BASE_URL` 必须是可被用户访问的 `http` 或 `https` 地址。
- 示例：`http://localhost:8080/reset-password`

## 运行时设置

管理员页面中的站点名称和存储后端配置通过运行时设置加载，保存后会立即生效，不需要重启进程。

## Cookie 配置建议

- 本地开发可使用 `AUTH_COOKIE_SECURE=false`
- 生产环境应使用 `AUTH_COOKIE_SECURE=true`
- 如果前后端跨站点部署，需要配合 `AUTH_COOKIE_SAME_SITE=None` 和 HTTPS

## 验证命令

```bash
cargo fmt --all
cargo check --workspace
cargo test --workspace
```
