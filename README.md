# Avenrixa

`Avenrixa` 是一个图片管理项目，当前仓库由以下部分组成：

- `backend`：Rust / Axum / SQLx 后端
- `frontend`：Vue 3 / TypeScript / Vite 前端
- `shared-types`：前后端共享类型
- `scripts`：Compose、回归、备份恢复、发布相关脚本

当前主线默认围绕这一条栈收口：

- `PostgreSQL + Dragonfly + 本地存储`

## 当前定位

当前仓库的真实交付面以“单主链路可用、可回归、可恢复”为主，不再把所有历史组合都视为正式承诺。

当前已经覆盖的主链路包括：

- 数据库引导
- 安装向导
- 管理员登录 / 刷新 / 登出 / 改密
- 注册 / 邮箱验证 / 密码重置
- 图片上传 / 列表 / 过期时间管理 / 永久删除
- 网站名称、favicon、邮件配置、本地存储路径等运行时设置
- 健康检查、系统状态、审计日志、系统统计
- PostgreSQL 物理备份 / 恢复 / PITR 运维主路径

当前补充路径：

- 无外部缓存模式可以运行，但不是默认推荐栈

当前不在主线承诺里的方向：

- MySQL / MariaDB
- 对象存储正式支持
- 更宽的数据库 / 存储 / 缓存支持矩阵
- 完整子路径部署承诺

## 快速启动

直接使用仓库根目录的 `compose.yml`：

```bash
docker compose up --build -d
```

默认服务：

- 应用：`http://localhost:8080`
- 数据库：PostgreSQL
- 缓存：Dragonfly

默认情况下只需要准备一个基础环境变量：

```bash
JWT_SECRET=replace-with-a-long-random-secret
```

如果你直接用仓库自带的 `compose.yml`，应用启动后通常会直接进入安装向导，而不是数据库引导页。

## 首次安装

首次启动时，按页面完成以下内容：

- 管理员邮箱和密码
- 网站名称
- 网站图标（favicon）
- 本地存储路径
- 邮件服务配置

如果你的自定义部署没有通过环境变量提供数据库连接，前端会进入数据库引导页。此时手动填写 PostgreSQL URL，保存后重启应用即可。

常见 PostgreSQL URL 示例：

```text
postgresql://user:pass@postgres:5432/image
```

## 运行时行为

当前关键运行时语义如下：

- `CACHE_URL` 可选；为空时系统以无外部缓存模式启动
- 无缓存模式下，登录态和核心功能仍可用，但缓存相关健康状态会显示为 `disabled` 或 `degraded`
- 网站设置通过数据库 `settings` 表持久化，修改后由后端运行时加载
- favicon 通过运行时设置保存，不需要手动改静态资源

常见环境变量：

- `JWT_SECRET`
- `DATABASE_KIND`
- `DATABASE_URL`
- `CACHE_URL`
- `AUTH_COOKIE_SECURE`
- `AUTH_COOKIE_SAME_SITE`
- `SERVER_HOST`
- `SERVER_PORT`
- `STORAGE_PATH`

## 本地开发

后端常用命令：

```bash
cargo fmt --all --check
cargo check --workspace
cargo test --workspace
```

前端常用命令：

```bash
npm ci --prefix frontend
npm run test --prefix frontend
npm run build --prefix frontend
npm run dev --prefix frontend
```

如果前端本地开发需要代理后端，Vite dev server 默认代理到：

```text
http://127.0.0.1:8080
```

如需覆盖，可设置：

```bash
VITE_DEV_PROXY_TARGET=http://127.0.0.1:8080
```

## 验证入口

常规代码验证：

```bash
cargo test --workspace
npm test --prefix frontend
npm run build --prefix frontend
```

容器主链路 smoke：

```bash
./scripts/compose-smoke.sh
```

无外部缓存模式 smoke：

```bash
CACHE_MODE=none ./scripts/compose-smoke.sh
```

浏览器回归：

```bash
bash ./scripts/browser-regression-ci.sh runtime-mainline
bash ./scripts/browser-regression-ci.sh bootstrap-postgres
```

## 备份与恢复

当前需要区分两类备份语义：

### 应用内逻辑备份

后台产生的 `backup_*.sql` 主要用于下载型逻辑导出。

它的定位是：

- 便于下载
- 便于人工检查
- 不作为企业主路径恢复方案

### PostgreSQL 企业主路径

物理备份：

```bash
./scripts/postgres-ops-backup.sh
```

按最近一次 manifest 恢复：

```bash
POSTGRES_RESTORE_MANIFEST_PATH=data/backup/postgres_last_physical_backup_manifest.json \
./scripts/postgres-ops-restore.sh
```

完整演练：

```bash
./scripts/postgres-ops-drill.sh
```

PITR 演练：

```bash
./scripts/postgres-ops-pitr-drill.sh
PITR_TARGET_MODE=time ./scripts/postgres-ops-pitr-drill.sh
```

WAL 清理：

```bash
./scripts/postgres-ops-wal-prune.sh
```

当前 PITR 边界：

- 回放的是 PostgreSQL 时间线
- 不会自动把 `/data` 回放到某个中间时间点

## 发布入口

当前仓库把发布前后入口分成 5 层：

- `smoke`：快速验证主链路还能跑
- `gate`：发版前串行执行默认 `GA` 阻塞项
- `drill`：专项恢复 / 备份 / PITR 演练
- `preflight`：候选版版本与元数据冻结检查
- `ship`：正式版发布与资产生成

常用入口：

```bash
./scripts/release-ga-gate.sh
./scripts/release-rc-preflight.sh
./scripts/release-ga-ship.sh
```

当前这些脚本会产出结果文件：

- `ops-backups/release-ga-gate/release-ga-gate-result.json`
- `dist/release/<version>/release-rc-preflight-result.json`
- `dist/release/<version>/release-ga-ship-result.json`

## 仓库结构

```text
backend/         Rust 后端
frontend/        Vue 3 + Vite 前端
shared-types/    共享类型
scripts/         运维、回归、发布脚本
compose.yml      默认手工部署入口
Dockerfile       多阶段构建
```

## 当前版本

当前 workspace 版本：

- `0.1.2-rc.3`

以 `Cargo.toml` 为准。

## License

见仓库根目录 [`LICENSE`](./LICENSE)。
