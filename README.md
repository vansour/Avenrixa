# Vansour Image

一个基于 Rust Workspace 的图片管理项目，包含：

- `backend`：Axum + SQLx + Redis 8 / Dragonfly 可选缓存的后端服务
- `frontend`：Dioxus Web 前端

## 0.1 范围

当前仓库已经具备多数据库、多缓存和多存储能力，但 `0.1` 不会把所有组合都当成正式版承诺。

`0.1` 当前的默认推荐栈是：

- `PostgreSQL + Redis 8 + 本地存储`

当前支持级别按下面收口：

- `GA`：PostgreSQL、Redis 8、本地存储，以及 PostgreSQL 物理备份 / 恢复 / PITR 主路径
- `Beta`：PostgreSQL + S3 / MinIO、SQLite、MySQL 8.4、MariaDB 12
- `Experimental`：Dragonfly、无外部缓存、SQLite 页面恢复、子路径部署

正式版边界、非目标和后续阶段的统一口径见 [`docs/release-0.1-scope.md`](docs/release-0.1-scope.md)。

## 快速启动

使用 Docker Compose：

```bash
docker compose up --build -d
```

手工部署现在统一编辑仓库根目录的 [`compose.yml`](compose.yml)：

- 默认已激活 `PostgreSQL + Redis 8`
- 同一个文件里也写好了 `SQLite`、`MySQL 8.4`、`MariaDB 12`
- `Dragonfly` 和“无外部缓存”模式也写在同一个文件的注释里
- `MySQL / MariaDB` 的长期运维口令预设也在同一个文件里，启用后记得替换所有占位密码和 `JWT_SECRET`

切换时只需要按 `compose.yml` 顶部注释做 3 件事：

- 给 `services.app` 保留一组预设
- 给数据库服务保留一组 `postgres` 或 `mysql`
- 按需保留 `cache`（Redis 8 / Dragonfly / 无外部缓存）

服务默认监听 `http://localhost:8080`。

仓库现在只保留这一个 `compose.yml`。脚本、CI 和专项演练会在运行时基于同一个入口自动生成临时 compose 配置，不再依赖仓库内的其他 compose 文件。

## 首次启动安装

首次启动时只需要准备基础环境变量：

- `JWT_SECRET`：生产环境请使用足够长的随机字符串

统一 `compose.yml` 里的 PostgreSQL / SQLite / MySQL / MariaDB 预设都已经直接写好了数据库连接。

因此只要你使用这个文件里的任一预设，容器启动后都会直接进入“安装向导”，不再默认先进入“数据库引导”页面。

数据库引导页现在只作为兜底入口：当你的自定义部署没有通过环境变量提供 `DATABASE_KIND` / `DATABASE_URL` 时，页面才会出现。此时可以选择：

- `PostgreSQL`：填写连接信息并保存，然后重启服务
- `MySQL`：填写 `mysql://` 或 `mariadb://` 连接信息并保存，然后重启服务
- `SQLite`：填写数据库文件路径或 `sqlite://` 连接并保存，然后重启服务

进入安装向导后，继续完成：

- 管理员邮箱和密码创建
- 网站名称
- 网站图标（favicon）
- 存储后端配置
- 邮件验证 / 密码重置所需的邮件配置

如果你走的是未预设数据库连接的自定义部署，保存数据库连接后可以执行：

```bash
docker compose restart app
```

如果你需要手动填写 SQLite 数据库连接，常见写法例如：

```text
/data/sqlite/app.db
sqlite:///data/sqlite/app.db
```

如果你需要手动填写 MySQL / MariaDB 数据库连接，常见写法例如：

```text
mysql://user:pass@mysql:3306/image
mariadb://user:pass@mysql:3306/image
```

如果你使用长期运维模板且改成手动填写，常见写法则应与模板中的应用账户保持一致，例如：

```text
mysql://vansour_image:replace-with-strong-app-password@mysql:3306/image
```

`REDIS_URL` 现在也是可选项。默认 Compose 会带上 Redis 8 `cache` 服务；如果你的自定义部署没有提供 `REDIS_URL`，应用会以“无外部缓存”模式启动，登录态与核心功能仍然可用，只是列表/哈希缓存与健康状态会显示为 `disabled` 或 `degraded`。

更完整的 SQLite 安装、备份、恢复、停机与回滚说明见 [`docs/sqlite.md`](docs/sqlite.md)。

如果你要做 SQLite 整站冷备，直接使用：

```bash
./scripts/sqlite-ops-backup.sh
```

默认归档会写到 `ops-backups/sqlite/`，最近一次 manifest 会写到 `./data-sqlite/backup/sqlite_last_cold_backup_manifest.json`。

PostgreSQL 的部署与运维备份说明见 [`docs/postgresql.md`](docs/postgresql.md)。

如果你使用默认的 PostgreSQL 部署栈，并且要做企业主路径物理备份，直接使用：

```bash
./scripts/postgres-ops-backup.sh
```

这会使用 `pg_basebackup` 生成物理 base backup 目录，并把最近一次 manifest 写到：

- PostgreSQL：`./data/backup/postgres_last_physical_backup_manifest.json`

按最近一次 physical manifest 恢复时，直接执行：

```bash
POSTGRES_RESTORE_MANIFEST_PATH=data/backup/postgres_last_physical_backup_manifest.json \
./scripts/postgres-ops-restore.sh
```

恢复脚本会先停应用，再停 PostgreSQL，自动生成恢复前回滚快照，并在恢复完成后把结果写到：

- PostgreSQL：`./data/backup/postgres_last_restore_result.json`

如果你要跑整套 PostgreSQL 物理备份/恢复演练，使用：

```bash
./scripts/postgres-ops-drill.sh
```

如果你还要启用 WAL 归档并验证 PostgreSQL PITR 主链路，先用运行时生成入口带上：

```bash
POSTGRES_ENABLE_WAL_ARCHIVE=1 \
POSTGRES_WAL_ARCHIVE_HOST_DIR=ops-backups/postgres-wal-archive \
./scripts/compose-runtime.sh up --build -d
```

然后可以按 restore point / 时间点恢复：

```bash
POSTGRES_RESTORE_MANIFEST_PATH=data/backup/postgres_last_physical_backup_manifest.json \
POSTGRES_RESTORE_TARGET_NAME=your_restore_point \
POSTGRES_RESTORE_WAL_ARCHIVE_DIR=ops-backups/postgres-wal-archive \
./scripts/postgres-ops-restore.sh
```

如果要按时间点恢复，可以继续传 ISO UTC 时间，脚本会自动正规化成 PostgreSQL 可接受的配置值：

```bash
POSTGRES_RESTORE_MANIFEST_PATH=data/backup/postgres_last_physical_backup_manifest.json \
POSTGRES_RESTORE_TARGET_TIME=2026-03-11T11:05:44Z \
POSTGRES_RESTORE_WAL_ARCHIVE_DIR=ops-backups/postgres-wal-archive \
./scripts/postgres-ops-restore.sh
```

如果你还要把 WAL 归档同步到远端：

```bash
POSTGRES_WAL_REMOTE_URI=file:///srv/vansour/wal-remote \
./scripts/postgres-ops-backup.sh
```

或者：

```bash
POSTGRES_WAL_REMOTE_URI=s3://my-bucket/vansour/postgres-wal \
POSTGRES_WAL_REMOTE_ENDPOINT=https://minio.example.com \
POSTGRES_WAL_REMOTE_ACCESS_KEY=... \
POSTGRES_WAL_REMOTE_SECRET_KEY=... \
./scripts/postgres-ops-backup.sh
```

保守 WAL 清理脚本：

```bash
POSTGRES_WAL_RETENTION_KEEP_BASE_BACKUPS=2 \
POSTGRES_WAL_REMOTE_URI=file:///srv/vansour/wal-remote \
./scripts/postgres-ops-wal-prune.sh
```

对应的 PITR 演练脚本是：

```bash
./scripts/postgres-ops-pitr-drill.sh
```

时间点 PITR 演练：

```bash
PITR_TARGET_MODE=time ./scripts/postgres-ops-pitr-drill.sh
```

PITR 当前只回放 PostgreSQL 时间线，不会自动把 `/data` 回放到中间时间点。

后台 `backup_*.sql` 现在仍然只承担下载型逻辑导出，不作为 PostgreSQL 企业主恢复路径。

MySQL / MariaDB 的部署与运维备份恢复说明见 [`docs/mysql.md`](docs/mysql.md)。

当前页面内不再支持 MySQL / MariaDB 备份直接恢复；MySQL / MariaDB 的恢复入口统一走运维脚本。

如果你要执行 MySQL / MariaDB 的企业主路径物理备份，直接使用：

```bash
./scripts/mysql-ops-backup.sh
```

这会生成 prepared 物理备份目录，并把最近一次 manifest 写到：

- MySQL 8.4：`./data-mysql/backup/mysql_last_physical_backup_manifest.json`
- MariaDB 12：`./data-mariadb/backup/mysql_last_physical_backup_manifest.json`

按物理 manifest 恢复时，直接执行：

```bash
MYSQL_RESTORE_MANIFEST_PATH=data-mysql/backup/mysql_last_physical_backup_manifest.json \
./scripts/mysql-ops-restore.sh

MYSQL_RESTORE_MANIFEST_PATH=data-mariadb/backup/mysql_last_physical_backup_manifest.json \
./scripts/mysql-ops-restore.sh
```

恢复脚本会先停应用，并在物理恢复时额外停掉 MySQL / MariaDB，自带回滚快照；结果会写入当前数据库家族对应的数据目录：

- MySQL 8.4：`./data-mysql/backup/mysql_last_restore_result.json`
- MariaDB 12：`./data-mariadb/backup/mysql_last_restore_result.json`

如果你还需要旧的逻辑导出兼容路径，显式指定：

```bash
MYSQL_BACKUP_MODE=logical ./scripts/mysql-ops-backup.sh
MYSQL_RESTORE_SQL_PATH=ops-backups/mysql/example.mysql.sql \
MYSQL_RESTORE_DATA_ARCHIVE=ops-backups/mysql/example.data.tar.gz \
./scripts/mysql-ops-restore.sh
```

如果你要跑整套演练，使用：

```bash
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

## 0.1 GA 发布门禁

如果你要在发版前把 `0.1` 默认推荐栈的阻塞项一次性跑完，直接执行：

```bash
./scripts/release-ga-gate.sh
```

这条入口会串行执行：

- Rust checks
- 默认 `PostgreSQL + Redis 8` Compose smoke
- PostgreSQL 物理备份 / 恢复演练
- PostgreSQL PITR 演练（restore point / time）

完整清单见 [`docs/release-0.1-ga-checklist.md`](docs/release-0.1-ga-checklist.md)。

## 0.1 RC 预检

如果你已经把 GA 主链路收口完成，接下来要验证候选版版本号、镜像元数据和 changelog 是否真正进入发布产物，直接执行：

```bash
./scripts/release-rc-preflight.sh
```

这条入口会在 `release-ga-gate` 之上继续检查：

- 当前工作区版本是否已经冻结到候选版
- `CHANGELOG.md` 是否有对应版本条目
- Docker 镜像 labels 是否写入 version / revision / created
- `/health` 返回的运行版本是否和候选版一致
- GitHub Actions 工作流会默认发布到 `ghcr.io/<repository_owner>/vansour-image:<version>`，并为预发布镜像追加 `:dev`

当前 RC runbook 见 [`docs/release-0.1-rc-runbook.md`](docs/release-0.1-rc-runbook.md)。

## 0.1 正式版发布演练

如果你已经确认候选版预检通过，并且要把版本真正切到正式版，直接执行：

```bash
./scripts/release-ga-ship.sh
```

这条入口会在 `release-rc-preflight` 之上继续完成：

- 校验当前工作区版本已经是稳定正式版号
- 校验 `CHANGELOG.md` 已存在正式版条目
- 推送 `v*` tag 时会自动触发对应的 GitHub Actions 正式发布 workflow
- GitHub Actions 工作流会默认发布到 `ghcr.io/<repository_owner>/vansour-image:<version>` 并追加 `:latest`
- 生成正式版 `release-notes.md`
- 导出镜像元数据、发布 bundle、`release-manifest.json` 与 `SHA256SUMS`

当前 GA runbook 见 [`docs/release-0.1-ga-runbook.md`](docs/release-0.1-ga-runbook.md)。

## Compose 冒烟校验

按 [`docs/release-0.1-scope.md`](docs/release-0.1-scope.md) 当前定义：

- `COMPOSE_VARIANT=postgres` 且 `CACHE_MODE=redis8` 是 `0.1` 的默认 GA 主链路冒烟
- `SQLite`、`MySQL 8.4`、`MariaDB 12` 属于补充覆盖，用于 Beta 栈持续回归
- `dragonfly`、`none` 属于补充覆盖，用于 Experimental 路径观察，不作为 `0.1` GA 发布门槛
仓库内提供了一个容器级冒烟脚本，会构建镜像、拉起 Compose 依赖，并按具体变体执行健康检查或主链路校验：

```bash
./scripts/compose-smoke.sh
```

如果要校验 SQLite 变体：

```bash
COMPOSE_VARIANT=sqlite \
./scripts/compose-smoke.sh
```

如果要校验 MySQL 8.4 变体：

```bash
COMPOSE_VARIANT=mysql \
./scripts/compose-smoke.sh
```

如果要校验 MariaDB 12 变体：

```bash
COMPOSE_VARIANT=mariadb \
./scripts/compose-smoke.sh
```

如果要切换缓存模式，统一使用 `CACHE_MODE`：

```bash
CACHE_MODE=dragonfly ./scripts/compose-smoke.sh
CACHE_MODE=none ./scripts/compose-smoke.sh

COMPOSE_VARIANT=mysql CACHE_MODE=dragonfly ./scripts/compose-smoke.sh
COMPOSE_VARIANT=mariadb CACHE_MODE=redis8 ./scripts/compose-smoke.sh
```

可选值：

- `redis8`：默认值，使用 Compose 内置 Redis 8 `cache`
- `dragonfly`：运行时自动切到 Dragonfly `cache`
- `none`：运行时自动切到无外部缓存模式

默认 `COMPOSE_VARIANT=postgres CACHE_MODE=redis8` 现在会跑 `0.1` GA 主链路：运行时检查、安装向导、管理员登录/刷新/登出/改密、图片上传/删除、结构化设置持久化、系统统计、审计日志，以及 PostgreSQL 逻辑备份“仅下载、不支持页面恢复”的语义校验。

MySQL / MariaDB 变体仍然会自动串行执行运行时检查、安装向导、管理员登录、图片上传/删除、备份创建/下载，并确认维护页和恢复预检都已经把 MySQL / MariaDB 逻辑导出收口为“仅运维脚本恢复”；如果当前入口没有预设数据库连接，则会自动回退到数据库引导页完成配置。

默认情况下，这条脚本会重建当前 Compose 入口对应的数据目录；PostgreSQL 对应 `./data`，MySQL 8.4 对应 `./data-mysql`，MariaDB 12 对应 `./data-mariadb`。如果你确实要复用现有目录，可以显式传入 `SMOKE_RESET_DATA_DIR=0`。

## 浏览器点击回归

这条浏览器回归当前主要用于 MySQL / MariaDB 的补充覆盖，帮助持续观察 `Beta` 栈的页面行为；它不是 `0.1` GA 发布的默认阻塞门槛。
如果你要对 MySQL / MariaDB 页面链路跑一遍真实浏览器点击回归，使用：

```bash
./scripts/browser-click-regression.sh
```

默认跑 MySQL 8.4；如果要切到 MariaDB 12：

```bash
COMPOSE_VARIANT=mariadb \
./scripts/browser-click-regression.sh
```

如果要在浏览器点击回归里切换缓存模式，也使用同一个环境变量：

```bash
CACHE_MODE=dragonfly ./scripts/browser-click-regression.sh
CACHE_MODE=none ./scripts/browser-click-regression.sh
```

这条脚本会自动完成：

- 检查 Compose 是否已预设 MySQL / MariaDB 数据库连接；如未预设则自动走数据库引导页保存连接
- 进入安装向导并完成管理员安装
- 首次进入引导的 4 个按钮逐个点击校对
- 设置页“维护工具”里生成 MySQL 备份，并校验备份行已明确标记为“不支持页面恢复”
- 审计页与维护页中核对备份创建文案和运维恢复提示

如果当前机器没有可直接使用的 Chrome / Chromium，脚本会自动安装 Playwright Chromium。

默认情况下，这条脚本也会重建当前 Compose 入口对应的数据目录；如需复用当前目录，显式传入 `MYSQL_SMOKE_RESET_DATA_DIR=0`。

注意：

- 单仓库入口现在只有 `compose.yml`。
- 运行时生成的 MySQL / MariaDB 栈仍然写了固定 `container_name`，同一台机器一次只能运行一套同名数据库家族栈。

默认 `compose.yml` 会把 PostgreSQL 数据放到 Docker named volume，不再依赖仓库内 `./pg_data` 的宿主目录权限。

如果你确实要把 PostgreSQL 数据绑定到宿主目录，需要自行修改 Compose 配置并确保目录对容器内 PostgreSQL 进程可写。

GitHub Actions 当前同时承担两类职责：

- `GA` 主链路校验：Rust checks、默认 PostgreSQL Compose smoke、PostgreSQL 运维演练
- `补充覆盖`：SQLite、MySQL 8.4、MariaDB 12、Dragonfly、无外部缓存及对应浏览器/运维演练

另外，仓库现在提供一个专门的手动发布门禁 workflow：

- `Release 0.1 GA Gate`：串行执行 Rust checks、默认 PostgreSQL smoke、PostgreSQL ops drill 和两种 PITR drill
- `Release 0.1 RC Preflight`：在 GA gate 之上继续校验 changelog、镜像 labels 和运行时版本元数据

## SQLite E2E 冒烟

这套 SQLite E2E 当前按 `Beta / Supplemental` 覆盖理解：它会继续保留并定时跑通，但不是 `0.1` GA 默认推荐栈的发布门槛。

如果要把 SQLite 的安装、邮件、注册、密码重置、上传和管理员查询链路一起跑通，可以使用：

```bash
./scripts/sqlite-e2e-smoke.sh
```

这个脚本会基于同一个 `compose.yml` 运行时生成 SQLite + Mailpit 栈，并自动完成：

- 如果当前入口没有预设数据库连接则自动完成 SQLite 数据库引导
- 安装向导
- Mailpit SMTP 联调
- 公开注册与邮箱验证
- 登录 / 刷新 / 退出
- 密码重置
- 图片上传 / 查询 / 过期回收 / 永久删除
- 管理员用户列表 / 角色更新 / 审计分页 / 统计查询

如果希望失败时保留容器与数据目录，方便继续排查，可以额外加上：

```bash
PRESERVE_STACK_ON_FAILURE=1 ./scripts/sqlite-e2e-smoke.sh
```

如果你要验证 SQLite 在不同缓存模式下的行为：

```bash
CACHE_MODE=redis8 ./scripts/sqlite-e2e-smoke.sh
CACHE_MODE=none ./scripts/sqlite-e2e-smoke.sh
CACHE_MODE=dragonfly ./scripts/sqlite-e2e-smoke.sh
```

仓库还提供了一个独立的 GitHub Actions workflow 来跑这套 SQLite E2E：

- 手动触发：`SQLite E2E (Supplemental)`
- 定时触发：每天 UTC `02:00`
