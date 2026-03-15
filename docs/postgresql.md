# PostgreSQL 运维说明

本文档对应当前项目默认的 PostgreSQL 部署栈，重点说明企业主路径运维备份、当前页面能力边界，以及仓库里已经落地和暂未落地的部分。

按 [`release-0.1-scope.md`](release-0.1-scope.md) 当前定义，`PostgreSQL + Redis 8 + 本地存储` 是 `0.1` 的默认 GA 推荐栈；如果你要优先建设正式版发布路径，应先覆盖这里描述的主链路。

## 1. 当前定位

仓库默认 `compose.yml` 入口就是 `PostgreSQL + Redis 8`。

当前 PostgreSQL 相关能力分成两条线：

- 后台页面里的 `backup_*.sql`：继续作为下载型逻辑导出
- `scripts/postgres-ops-backup.sh` / `scripts/postgres-ops-restore.sh`：企业主路径物理备份、恢复与 PITR

页面内恢复当前仍不支持 PostgreSQL；企业主路径恢复统一走运维脚本。

### 1.1 GA 发布 smoke

如果你要在本地先按 `0.1` GA 门槛把默认主链路跑一遍，直接执行：

```bash
./scripts/compose-smoke.sh
```

这条默认 smoke 现在会覆盖：

- `PostgreSQL + Redis 8 + 本地存储` 运行时健康检查
- 安装向导与管理员会话初始化
- 管理员登录、刷新、登出、改密
- 图片上传、过期时间写入、永久删除
- `settings/config` 结构化设置更新与持久化
- `/stats`、`/health`、`/audit-logs` 基本验收
- PostgreSQL 逻辑备份下载链路，以及“仅下载、不支持页面恢复”的语义校验

默认情况下它会重建 `./data`。如果你要复用现有目录，再显式加上：

```bash
SMOKE_RESET_DATA_DIR=0 ./scripts/compose-smoke.sh
```

### 1.2 完整 GA 发布门禁

如果你要把 `0.1` 的 PostgreSQL 默认推荐栈阻塞项一次性跑完，而不是只跑 smoke，直接执行：

```bash
./scripts/release-ga-gate.sh
```

这条入口会继续串行执行：

- 默认 PostgreSQL GA smoke
- PostgreSQL 物理备份 / 恢复演练
- PostgreSQL PITR 演练（named restore point）
- PostgreSQL PITR 演练（time target）

完整发布清单见 [`release-0.1-ga-checklist.md`](release-0.1-ga-checklist.md)。

## 2. 企业主路径备份

直接执行：

```bash
./scripts/postgres-ops-backup.sh
```

脚本行为：

- 要求当前运行 `COMPOSE_VARIANT=postgres`
- 对运行中的 PostgreSQL 服务执行 `pg_basebackup`
- 默认额外归档应用侧 `/data`，用于把本地图片、bootstrap 文件和后台备份目录一起回退到同一时间点
- 如果归档 `/data` 时应用正在运行，默认会先停 `app`，归档完成后再自动拉起

默认产物：

- 物理 base backup 目录：`ops-backups/postgres/postgres_ops_backup_<stamp>.physical/base/`
- 可选 `/data` 归档：`ops-backups/postgres/postgres_ops_backup_<stamp>.data.tar.gz`
- sidecar manifest：`ops-backups/postgres/postgres_ops_backup_<stamp>.physical.manifest.json`
- 最近一次 manifest：`./data/backup/postgres_last_physical_backup_manifest.json`
- 如果启用了 WAL 归档，manifest 还会带上 `wal_archive` 信息和一个可直接用于 PITR 的 restore point
- 如果同时配置了 `POSTGRES_WAL_REMOTE_URI`，backup 结束后还会把本地 WAL 归档同步到远端

## 3. 企业主路径恢复

按最近一次 physical manifest 恢复：

```bash
POSTGRES_RESTORE_MANIFEST_PATH=data/backup/postgres_last_physical_backup_manifest.json \
./scripts/postgres-ops-restore.sh
```

如果你已经拿到了具体的 base backup 目录，也可以直接指定：

```bash
POSTGRES_RESTORE_PHYSICAL_PATH=ops-backups/postgres/postgres_ops_backup_<stamp>.physical/base \
POSTGRES_RESTORE_DATA_ARCHIVE=ops-backups/postgres/postgres_ops_backup_<stamp>.data.tar.gz \
./scripts/postgres-ops-restore.sh
```

脚本行为：

- 先停 `app`
- 再停 `postgres`
- 对当前 `PGDATA` 生成回滚快照
- 如果本次恢复包含 `/data` 快照，也会先生成本地 `./data` 回滚包
- 将 physical backup 目录直接 copy-back 到 `PGDATA`
- 如有 `/data` 归档，则一并恢复
- 自动重启 `postgres` 并等待健康检查
- 默认继续拉起 `app` 并等待 `/health`
- 如果恢复失败，会自动尝试把 `PGDATA` 和 `./data` 回滚到恢复前状态

默认结果文件：

- 最近一次恢复结果：`./data/backup/postgres_last_restore_result.json`

默认恢复时还会生成：

- 恢复前 `PGDATA` 回滚包：`ops-backups/postgres/rollback_before_restore_<stamp>.postgres-datadir.tar.gz`
- 可选恢复前 `./data` 回滚包：`ops-backups/postgres/rollback_before_restore_<stamp>.data.tar.gz`
- physical copy-back 日志：`ops-backups/postgres/restore_<stamp>.postgres-physical-copy-back.log`

## 4. PITR / WAL 归档

如果你要把 PostgreSQL 恢复到 base backup 之后的某个 restore point 或时间点，需要先启用 WAL 归档。

对仓库内基于 `compose.yml` 运行时生成的 PostgreSQL 栈，直接带上：

```bash
POSTGRES_ENABLE_WAL_ARCHIVE=1 \
POSTGRES_WAL_ARCHIVE_HOST_DIR=ops-backups/postgres-wal-archive \
./scripts/compose-runtime.sh up --build -d
```

启用后：

- PostgreSQL 会以 `archive_mode=on`
- `archive_command` 会把 WAL 文件写到宿主机 `POSTGRES_WAL_ARCHIVE_HOST_DIR`
- `postgres-ops-backup.sh` 会在 backup 完成后自动创建一个 restore point 并写入 manifest

按 named restore point 做 PITR：

```bash
POSTGRES_RESTORE_MANIFEST_PATH=data/backup/postgres_last_physical_backup_manifest.json \
POSTGRES_RESTORE_TARGET_NAME=vansour_postgres_backup_20260311T110541Z \
POSTGRES_RESTORE_WAL_ARCHIVE_DIR=ops-backups/postgres-wal-archive \
./scripts/postgres-ops-restore.sh
```

按时间点做 PITR：

```bash
POSTGRES_RESTORE_MANIFEST_PATH=data/backup/postgres_last_physical_backup_manifest.json \
POSTGRES_RESTORE_TARGET_TIME=2026-03-11T11:05:44Z \
POSTGRES_RESTORE_WAL_ARCHIVE_DIR=ops-backups/postgres-wal-archive \
./scripts/postgres-ops-restore.sh
```

`POSTGRES_RESTORE_TARGET_TIME` 可以继续传标准 ISO UTC 时间，例如 `2026-03-11T11:05:44Z`；脚本会在写入 `postgresql.auto.conf` 前自动正规化成 PostgreSQL 可接受的时间格式。

当前仓库里 PITR 的行为边界必须明确：

- PITR 会先恢复 physical base backup，再通过 `recovery.signal` 和 `restore_command` 回放 WAL
- PITR 只回放 PostgreSQL 时间线
- 即使 manifest 里带了 `/data` snapshot，PITR 也不会自动把 `/data` 回退到某个中间时间点
- 如果你还要让本地图片或对象存储与 PITR 后的数据库时间点保持一致，必须额外依赖对象版本、文件系统快照或独立的附件回滚策略

也就是说，`/data` snapshot 仍适合“整站回到 base backup 那个时刻”，不适合“数据库回放到 base backup 之后的某个中间时刻”。

### 4.1 远端 WAL 同步

如果你不希望 WAL 只落在本机目录，还可以给 backup / restore / drill 脚本补上：

```bash
POSTGRES_WAL_REMOTE_URI=file:///srv/vansour/wal-remote
```

或者：

```bash
POSTGRES_WAL_REMOTE_URI=s3://my-bucket/vansour/postgres-wal
POSTGRES_WAL_REMOTE_ENDPOINT=https://minio.example.com
POSTGRES_WAL_REMOTE_REGION=us-east-1
POSTGRES_WAL_REMOTE_ACCESS_KEY=...
POSTGRES_WAL_REMOTE_SECRET_KEY=...
POSTGRES_WAL_REMOTE_FORCE_PATH_STYLE=1
```

当前远端语义：

- `file://...` 或绝对路径：适合本机、挂载盘和测试环境
- `s3://bucket/prefix`：通过宿主机 `aws` CLI 推送与拉取，兼容 MinIO 一类 S3 API
- PostgreSQL 自身仍然先把 WAL 归档到本地 `POSTGRES_WAL_ARCHIVE_HOST_DIR`
- 远端同步由独立运维脚本完成，而不是把 `archive_command` 直接指向对象存储

独立执行同步：

```bash
POSTGRES_WAL_ARCHIVE_DIR=ops-backups/postgres-wal-archive \
POSTGRES_WAL_REMOTE_URI=file:///srv/vansour/wal-remote \
./scripts/postgres-ops-wal-sync.sh push
```

恢复前如果本地 WAL 目录为空，也可以显式拉回：

```bash
POSTGRES_WAL_ARCHIVE_DIR=ops-backups/postgres-wal-archive \
POSTGRES_WAL_REMOTE_URI=file:///srv/vansour/wal-remote \
./scripts/postgres-ops-wal-sync.sh pull
```

### 4.2 WAL 清理

当前仓库已经提供基于 retained base backup 数量的保守 WAL 清理：

```bash
POSTGRES_WAL_RETENTION_KEEP_BASE_BACKUPS=2 \
POSTGRES_WAL_REMOTE_URI=file:///srv/vansour/wal-remote \
./scripts/postgres-ops-wal-prune.sh
```

也可以让 physical backup 成功后自动触发：

```bash
POSTGRES_WAL_PRUNE_AFTER_BACKUP=1 \
./scripts/postgres-ops-backup.sh
```

当前清理策略边界：

- 以 manifest 里的 `physical_backup.metadata.start_wal_file` 为保留基准
- 默认同时清理本地和远端；`POSTGRES_WAL_PRUNE_REMOTE=0` 可关闭远端删除
- `POSTGRES_WAL_PRUNE_DRY_RUN=1` 只打印计划，不实际删除
- 只删除同一 timeline 上、严格早于保留阈值的 24 位十六进制 WAL segment
- `.history`、`.backup` 等非标准 segment 文件会保留
- 这是一套保守清理，不是完整的多 timeline 生命周期管理器

## 5. Manifest 内容

当前 physical manifest 至少记录：

- `backup_method=physical`
- `backup_kind=postgresql-physical-basebackup`
- `tool_family=pg_basebackup`
- helper 镜像
- 物理备份目录路径
- `wal_method=stream`
- `checkpoint_mode=fast`
- `backup.log`
- `PG_VERSION`、`backup_manifest`、`backup_label`、`tablespace_map` 等元数据文件位置与存在状态
- 备份目录大小
- 可选 `/data` 归档路径与 SHA256
- 如果 WAL 归档已开启，还会记录：
- `wal_archive.enabled`
- 宿主机 WAL 归档目录
- 容器内挂载路径
- 远端 WAL URI（如果配置）
- 当前 `archive_mode` / `wal_level`
- backup 完成后自动创建的 restore point 名称与 LSN
- 本次 physical base backup 的 `start_wal_file` / `start_timeline`

这使 PostgreSQL 物理备份不再只是一个目录，而是有可追溯 manifest 的运维产物。

## 6. 恢复演练

直接执行：

```bash
./scripts/postgres-ops-drill.sh
```

演练脚本会自动完成：

- 启动默认 PostgreSQL Compose 栈
- 必要时通过 bootstrap fallback 写入 `DATABASE_KIND=postgresql` / `DATABASE_URL`
- 完成安装向导
- 写入基线数据库内容和本地 marker 文件
- 执行一次 `pg_basebackup` 物理备份
- 人为篡改数据库与本地文件
- 按 manifest 执行物理恢复
- 校验数据库内容、本地 marker、恢复结果 JSON 和应用健康状态都回到基线

它的定位不是页面功能测试，而是企业主路径恢复链路的真实演练。

如果你还要验证 WAL 归档 + named restore point 的 PITR 主链路，直接执行：

```bash
./scripts/postgres-ops-pitr-drill.sh
```

它会自动完成：

- 启动开启 WAL 归档的 PostgreSQL 栈
- 安装应用并生成 physical base backup
- 把数据库变更到目标状态
- 创建一个 named restore point 并强制切 WAL
- 再继续写入新的数据库变化
- 按 restore point 做 PITR
- 校验数据库最终回到目标状态，而不是 base backup 时刻，也不是 PITR 之后的新状态

如果你还要验证按时间点恢复，直接执行：

```bash
PITR_TARGET_MODE=time ./scripts/postgres-ops-pitr-drill.sh
```

## 7. 当前边界

当前仓库已经落地：

- 默认 Compose 栈的 PostgreSQL 物理备份基座
- PostgreSQL 物理 restore 编排脚本
- PostgreSQL 物理恢复演练脚本
- PostgreSQL 本地 WAL 归档与 PITR 主链路
- PostgreSQL WAL 远端同步（`file://` / `s3://`）
- PostgreSQL 基于 retained base backup 数量的保守 WAL pruning
- PostgreSQL PITR 演练脚本
- 后台页面 PostgreSQL 逻辑导出下载
- `/data` 同步归档能力

当前仓库还没有落地：

- 页面内 PostgreSQL 恢复
- 基于逻辑 SQL dump 的 PostgreSQL 运维恢复
- 多 timeline 管理与更完整的 PITR runbook 编排

因此现在的边界很明确：

- 如果你要企业主路径备份，走 `./scripts/postgres-ops-backup.sh`
- 如果你要企业主路径恢复，走 `./scripts/postgres-ops-restore.sh`
- 如果你要把数据库恢复到 base backup 之后的某个 restore point / 时间点，也还是走 `./scripts/postgres-ops-restore.sh`
- 如果你要下载一个逻辑 SQL 文件，继续用后台页面
- 页面导出的 `backup_*.sql` 不是企业主恢复路径
- 当前 restore 只支持 physical manifest / physical backup 目录，不支持把逻辑 SQL dump 当成主恢复入口

## 8. 可调参数

常用环境变量：

- `ARTIFACT_DIR`：修改备份输出目录，默认 `ops-backups/postgres`
- `INCLUDE_DATA_SNAPSHOT=0|1`：是否同时归档 `/data`
- `STOP_APP_DURING_DATA_SNAPSHOT=0|1`：归档 `/data` 前是否停 `app`
- `POSTGRES_BACKUP_PREFIX`：自定义备份文件名前缀
- `POSTGRES_PHYSICAL_HELPER_IMAGE`：覆盖默认 helper 镜像
- `POSTGRES_ENABLE_WAL_ARCHIVE=0|1`：是否为运行时生成的 PostgreSQL 栈开启 WAL 归档
- `POSTGRES_WAL_ARCHIVE_HOST_DIR`：宿主机 WAL 归档目录
- `POSTGRES_WAL_ARCHIVE_MOUNT_PATH`：容器内 WAL 归档挂载路径，默认 `/wal-archive`
- `POSTGRES_WAL_REMOTE_URI`：可选远端 WAL 目录，支持 `file://...`、绝对路径、`s3://bucket/prefix`
- `POSTGRES_WAL_REMOTE_ENDPOINT`：S3 / MinIO 端点
- `POSTGRES_WAL_REMOTE_REGION`：S3 区域，默认 `us-east-1`
- `POSTGRES_WAL_REMOTE_ACCESS_KEY` / `POSTGRES_WAL_REMOTE_SECRET_KEY`：S3 凭证
- `POSTGRES_WAL_REMOTE_FORCE_PATH_STYLE=0|1`：是否使用 path-style S3 地址
- `POSTGRES_WAL_PRUNE_AFTER_BACKUP=0|1`：backup 成功后是否自动做一次 WAL 清理
- `POSTGRES_WAL_RETENTION_KEEP_BASE_BACKUPS`：WAL 清理时保留最近多少份 base backup
- `POSTGRES_WAL_PRUNE_REMOTE=0|1`：清理时是否删除远端 WAL
- `POSTGRES_WAL_PRUNE_DRY_RUN=0|1`：只打印清理计划，不实际删除
- `POSTGRES_RESTORE_MANIFEST_PATH`：按 physical manifest 恢复
- `POSTGRES_RESTORE_PHYSICAL_PATH`：直接指定 physical backup 目录
- `POSTGRES_RESTORE_DATA_ARCHIVE`：恢复时额外指定 `/data` 归档
- `POSTGRES_RESTORE_TARGET_NAME`：按 named restore point 做 PITR
- `POSTGRES_RESTORE_TARGET_TIME`：按时间点做 PITR
- `POSTGRES_RESTORE_WAL_ARCHIVE_DIR`：PITR 时使用的宿主机 WAL 归档目录
- `START_APP_AFTER_RESTORE=0|1`：恢复完成后是否自动拉起 `app`

如果你把 `ARTIFACT_DIR` 指到 `DATA_DIR` 内部，脚本会直接拒绝，以避免递归打包。
