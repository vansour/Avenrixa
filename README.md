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

如果你要直接走 MySQL 8.4 的开发 / 联调入口，使用：

```bash
docker compose -f compose.mysql.yml up --build -d
```

如果你要直接走 MariaDB 12 的开发 / 联调入口，使用：

```bash
docker compose -f compose.mariadb.yml up --build -d
```

这两个文件都保留了固定演示口令，适合本地开发、CI 和烟测。

如果你要准备长期运维入口，先复制模板再改密钥：

```bash
cp compose.mysql.ops.yml.example compose.mysql.ops.yml
cp compose.mariadb.ops.yml.example compose.mariadb.ops.yml
```

然后把对应模板里的密钥和数据库密码替换成你自己的正式值，再执行：

```bash
docker compose -f compose.mysql.ops.yml up --build -d
docker compose -f compose.mariadb.ops.yml up --build -d
```

MySQL 8.4 Compose 默认把宿主机 `./data-mysql` 挂载到应用容器 `/data`，并使用 Docker named volume 保存 MySQL 数据。

MariaDB 12 Compose 默认把宿主机 `./data-mariadb` 挂载到应用容器 `/data`，并使用 Docker named volume 保存 MariaDB 数据。

## 首次启动安装

首次启动时只需要准备基础环境变量：

- `JWT_SECRET`：生产环境请使用足够长的随机字符串

容器启动后，首次打开网页会先进入“数据库引导”页面。可以选择：

- `PostgreSQL`：填写连接信息并保存，然后重启服务
- `MySQL`：填写 `mysql://` 或 `mariadb://` 连接信息并保存，然后重启服务
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

如果使用 MySQL / MariaDB Compose，数据库引导页常见写法例如：

```text
mysql://user:pass@mysql:3306/image
mariadb://user:pass@mysql:3306/image
```

如果使用长期运维模板，常见写法则应与模板中的应用账户保持一致，例如：

```text
mysql://vansour_image:replace-with-strong-app-password@mysql:3306/image
```

更完整的 SQLite 安装、备份、恢复、停机与回滚说明见 [`docs/sqlite.md`](docs/sqlite.md)。

MySQL / MariaDB 的部署、后台恢复与备份说明见 [`docs/mysql.md`](docs/mysql.md)。

如果你要执行 MySQL / MariaDB 的运维级备份 / 恢复脚本，分别使用：

```bash
./scripts/mysql-ops-backup.sh
MYSQL_RESTORE_SQL_PATH=ops-backups/mysql/example.mysql.sql \
MYSQL_RESTORE_DATA_ARCHIVE=ops-backups/mysql/example.data.tar.gz \
./scripts/mysql-ops-restore.sh
```

恢复脚本会先停应用、生成回滚快照，再执行导入；结果会写入当前数据库家族对应的数据目录：

- MySQL 8.4：`./data-mysql/backup/mysql_last_restore_result.json`
- MariaDB 12：`./data-mariadb/backup/mysql_last_restore_result.json`

如果你要按 manifest 做校验恢复或跑整套演练，使用：

```bash
MYSQL_RESTORE_MANIFEST_PATH=data-mysql/backup/mysql_last_backup_manifest.json \
./scripts/mysql-ops-restore.sh

MYSQL_RESTORE_MANIFEST_PATH=data-mariadb/backup/mysql_last_backup_manifest.json \
./scripts/mysql-ops-restore.sh

./scripts/mysql-ops-drill.sh
```

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
./scripts/compose-smoke.sh
```

如果要校验 MySQL 8.4 Compose 入口：

```bash
COMPOSE_FILE_PATHS="compose.mysql.yml" \
./scripts/compose-smoke.sh
```

如果要校验 MariaDB 12 Compose 入口：

```bash
COMPOSE_FILE_PATHS="compose.mariadb.yml" \
./scripts/compose-smoke.sh
```

这条命令现在不只是等 `/health`，还会自动串行执行 MySQL / MariaDB 的数据库引导、安装向导、管理员登录、图片上传/删除、备份创建/下载、恢复预检、写入恢复计划、重启执行恢复、旧会话失效校验、重新登录、恢复结果与审计日志校验。

默认情况下，这条脚本会重建当前 Compose 入口对应的数据目录；MySQL 8.4 对应 `./data-mysql`，MariaDB 12 对应 `./data-mariadb`。如果你确实要复用现有目录，可以显式传入 `MYSQL_SMOKE_RESET_DATA_DIR=0`。

## 浏览器点击回归

如果你要对 MySQL / MariaDB 页面链路跑一遍真实浏览器点击回归，使用：

```bash
./scripts/browser-click-regression.sh
```

默认跑 MySQL 8.4；如果要切到 MariaDB 12：

```bash
COMPOSE_FILE_PATHS="compose.mariadb.yml" \
./scripts/browser-click-regression.sh
```

这条脚本会自动完成：

- MySQL 数据库引导页保存连接
- 重启后进入安装向导并完成管理员安装
- 首次进入引导的 4 个按钮逐个点击校对
- 设置页“维护工具”里生成 MySQL 备份并写入恢复计划
- 再次重启后校验旧登录态失效、重新登录、审计页与恢复结果卡片文案

如果当前机器没有可直接使用的 Chrome / Chromium，脚本会自动安装 Playwright Chromium。

默认情况下，这条脚本也会重建当前 Compose 入口对应的数据目录；如需复用当前目录，显式传入 `MYSQL_SMOKE_RESET_DATA_DIR=0`。

注意：

- `compose.mysql.yml` / `compose.mariadb.yml` 是开发 / 烟测入口，不是长期运维入口。
- `compose.mysql.yml`、`compose.mysql.ops.yml`、`compose.mariadb.yml`、`compose.mariadb.ops.yml` 都写了固定 `container_name`，同一台机器一次只能运行一套同名数据库家族栈。

默认 `compose.yml` 会把 PostgreSQL 数据放到 Docker named volume，不再依赖仓库内 `./pg_data` 的宿主目录权限。

如果你确实要把 PostgreSQL 数据绑定到宿主目录，需要自行修改 Compose 配置并确保目录对容器内 PostgreSQL 进程可写。

GitHub Actions 会自动执行两类校验：

- `cargo fmt --all --check`
- `cargo check --workspace`
- `cargo test --workspace`
- 基于 `docker compose` 的 MySQL 8.4 / MariaDB 12 Compose smoke
- 基于 Playwright 的 MySQL 8.4 / MariaDB 12 浏览器点击回归

## SQLite E2E 冒烟

如果要把 SQLite 的安装、邮件、注册、密码重置、上传和管理员查询链路一起跑通，可以使用：

```bash
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
