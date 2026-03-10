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

如果你要直接走 SQLite 部署入口，使用：

```bash
docker compose -f compose.sqlite.yml up --build -d
```

SQLite Compose 默认把宿主机 `./data-sqlite` 挂载到容器内 `/data`。

## 首次启动安装

首次启动时只需要准备基础环境变量：

- `JWT_SECRET`：生产环境请使用足够长的随机字符串

容器启动后，首次打开网页会先进入“数据库引导”页面。可以选择：

- `PostgreSQL`：填写连接信息并保存，然后重启服务
- `SQLite`：填写数据库文件路径或 `sqlite://` 连接并保存，然后重启服务

数据库引导完成后，在安装向导里继续完成：

- 管理员邮箱和密码创建
- 网站名称
- 网站图标（favicon）
- 存储后端配置
- 邮件验证 / 密码重置所需的邮件配置

如果使用 Docker Compose，保存数据库连接后可以执行：

```bash
docker compose restart app
```

如果使用 SQLite，常见写法例如：

```text
/data/sqlite/app.db
sqlite:///data/sqlite/app.db
```

更完整的 SQLite 安装、备份、恢复、停机与回滚说明见 [`docs/sqlite.md`](docs/sqlite.md)。

## 邮件验证与密码重置配置

公开注册、邮箱验证和邮件找回统一通过安装向导或管理员后台配置，不再要求在 `compose.yml` 中写 `MAIL_*` / `SMTP_*` 环境变量。

安装完成后，如需修改，进入“设置 -> 基础设置”，配置以下字段：

- 启用邮件服务
- `SMTP 主机`
- `SMTP 端口`
- `发件邮箱`
- `发件人名称`
- `邮件跳转地址`

如果 SMTP 服务器要求认证，再填写：

- `SMTP 用户名`
- `SMTP 密码`

注意：

- `SMTP 用户名` 和 `SMTP 密码` 必须同时配置或同时留空。
- `邮件跳转地址` 会同时用于密码重置和邮箱验证，必须是用户可访问的 `http` 或 `https` 地址。
- 建议在开启“公开注册”之前先完成这部分配置。

## 运行时设置

管理员页面中的站点名称、邮件配置等通过运行时设置加载。  
如果修改存储后端或底层存储路径，页面会提示是否需要重启服务。

## Cookie 配置建议

- 本地开发可使用 `AUTH_COOKIE_SECURE=false`
- 生产环境应使用 `AUTH_COOKIE_SECURE=true`
- 如果前后端跨站点部署，需要配合 `AUTH_COOKIE_SAME_SITE=None` 和 HTTPS

## 验证命令

```bash
cargo fmt --all --check
cargo check --workspace
cargo test --workspace
```

## Compose 冒烟校验

仓库内提供了一个最小的容器级健康校验脚本，会构建镜像、拉起 Compose 依赖并检查 `/health`：

```bash
./scripts/compose-smoke.sh
```

如果要校验 SQLite Compose 入口：

```bash
COMPOSE_FILE_PATHS="compose.sqlite.yml" \
APP_HOST_PORT=18080 \
DATA_DIR="$(mktemp -d)" \
./scripts/compose-smoke.sh
```

如果本机 `8080` 已被占用，可以覆盖端口和应用数据目录，避免碰到现有运行实例或本地持久化数据：

```bash
APP_HOST_PORT=18080 \
DATA_DIR="$(mktemp -d)" \
./scripts/compose-smoke.sh
```

默认 `compose.yml` 会把 PostgreSQL 数据放到仓库内 `./pg_data`。

为避免 `postgres:trixie` 因宿主目录权限不匹配而启动失败，Compose 会先运行一次性 `postgres-init` 服务，把 `./pg_data` 或 `POSTGRES_DATA_DIR` 指向的目录修正为容器内 PostgreSQL 可写。

如果你显式传入 `POSTGRES_DATA_DIR`，也会复用这套自动修权限逻辑。

GitHub Actions 会自动执行两类校验：

- `cargo fmt --all --check`
- `cargo check --workspace`
- `cargo test --workspace`
- 基于 `docker compose` 的 `/health` 冒烟校验

## SQLite E2E 冒烟

如果要把 SQLite 的安装、邮件、注册、密码重置、上传和管理员查询链路一起跑通，可以使用：

```bash
COMPOSE_PROJECT_NAME=vansour-image-sqlite-e2e \
APP_HOST_PORT=18080 \
MAILPIT_HTTP_PORT=18025 \
./scripts/sqlite-e2e-smoke.sh
```

这个脚本会组合使用 `compose.sqlite.yml` 和 `compose.sqlite.mailpit.yml`，自动完成：

- SQLite 数据库引导
- 安装向导
- Mailpit SMTP 联调
- 公开注册与邮箱验证
- 登录 / 刷新 / 退出
- 密码重置
- 图片上传 / 查询 / 过期回收 / 软删恢复 / 永久删除
- 管理员用户列表 / 角色更新 / 审计分页 / 统计查询

如果希望失败时保留容器与数据目录，方便继续排查，可以额外加上：

```bash
PRESERVE_STACK_ON_FAILURE=1 ./scripts/sqlite-e2e-smoke.sh
```

仓库还提供了一个独立的 GitHub Actions workflow 来跑这套 SQLite E2E：

- 手动触发：`SQLite E2E`
- 定时触发：每天 UTC `02:00`
