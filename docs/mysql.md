# MySQL 8.4 / MariaDB 12 运维说明

本文档对应当前项目的 MySQL 8.4 与 MariaDB 12 运行模式，重点说明开发入口、长期运维入口、后台恢复、运维脚本、停机要求和当前限制。

## 1. 入口区分

当前仓库内有 4 个 MySQL-family 入口角色：

- `compose.mysql.yml`
  - 用途：MySQL 8.4 的本地开发、联调、CI、冒烟校验
  - 特点：保留固定演示口令，方便直接起栈
  - 结论：不要直接作为长期运维配置使用
- `compose.mariadb.yml`
  - 用途：MariaDB 12 的本地开发、联调、CI、冒烟校验
  - 特点：保留固定演示口令，方便直接起栈
  - 结论：不要直接作为长期运维配置使用
- `compose.mysql.ops.yml.example`
  - 用途：MySQL 8.4 的长期运维模板
  - 特点：要求你先复制、再替换密钥和数据库密码
  - 结论：正式长期运行请基于它生成你自己的 `compose.mysql.ops.yml`
- `compose.mariadb.ops.yml.example`
  - 用途：MariaDB 12 的长期运维模板
  - 特点：要求你先复制、再替换密钥和数据库密码
  - 结论：正式长期运行请基于它生成你自己的 `compose.mariadb.ops.yml`

## 2. 开发 / 联调入口

开发或联调 MySQL 8.4 时，直接使用：

```bash
docker compose -f compose.mysql.yml up --build -d
```

开发或联调 MariaDB 12 时，直接使用：

```bash
docker compose -f compose.mariadb.yml up --build -d
```

这两个入口都适合：

- 本地功能联调
- Docker 冒烟脚本
- CI 中的 MySQL 主链路校验

## 3. 长期运维入口

先复制对应模板：

```bash
cp compose.mysql.ops.yml.example compose.mysql.ops.yml
cp compose.mariadb.ops.yml.example compose.mariadb.ops.yml
```

然后至少修改下面这些值：

- `JWT_SECRET`
- 应用账户密码
- 数据库 root 密码

修改完成后再启动：

```bash
docker compose -f compose.mysql.ops.yml up --build -d
docker compose -f compose.mariadb.ops.yml up --build -d
```

注意：

- `compose.mysql.ops.yml` 和 `compose.mariadb.ops.yml` 都已加入 `.gitignore`
- 当前 MySQL / MariaDB Compose 文件都使用固定 `container_name`
- 这意味着同一台机器一次只能运行一套同名数据库家族栈

首次打开网页时，系统会进入“数据库引导”页。选择 `MySQL` 后填写连接地址，例如：

```text
mysql://user:pass@mysql:3306/image
```

如果你接的是 MariaDB，也可以直接填写：

```text
mariadb://user:pass@mysql:3306/image
```

如果你使用 MySQL 8.4 长期运维模板，对应写法应改为：

```text
mysql://vansour_image:replace-with-strong-app-password@mysql:3306/image
```

如果你使用 MariaDB 12 长期运维模板，对应写法应改为：

```text
mariadb://vansour_image:replace-with-strong-app-password@mysql:3306/image
```

保存后需要重启应用容器，让运行时真正切换到 MySQL / MariaDB：

```bash
docker compose -f compose.mysql.yml restart app
```

如果你当前运行的是 MySQL 8.4 长期运维模板，则对应执行：

```bash
docker compose -f compose.mysql.ops.yml restart app
```

如果你当前运行的是 MariaDB 12 对应模板，则执行：

```bash
docker compose -f compose.mariadb.yml restart app
docker compose -f compose.mariadb.ops.yml restart app
```

然后继续完成安装向导：

- 创建管理员邮箱和密码
- 配置站点标题 / favicon
- 配置存储后端
- 配置公开注册、邮箱验证、密码重置所需的邮件服务

如果要跑项目内置的 MySQL 8.4 Docker 联调烟测，可以直接执行：

```bash
COMPOSE_FILE_PATHS="compose.mysql.yml" \
./scripts/compose-smoke.sh
```

这条脚本会自动完成：

- MySQL 数据库引导写入
- 应用重启并进入安装向导
- 管理员创建与登录校验
- 图片上传、软删除、恢复、永久删除
- MySQL 备份创建、下载
- MySQL 恢复预检、写入恢复计划、重启执行恢复
- 旧会话失效、重新登录、恢复结果与审计日志校验

默认情况下，`scripts/compose-smoke.sh` 会先重建当前 Compose 入口对应的数据目录；MySQL 8.4 对应 `./data-mysql`，MariaDB 12 对应 `./data-mariadb`。如果你要复用已有目录，再显式加上 `MYSQL_SMOKE_RESET_DATA_DIR=0`。

如果要跑 MariaDB 12 Docker 联调烟测，可以直接执行：

```bash
COMPOSE_FILE_PATHS="compose.mariadb.yml" \
./scripts/compose-smoke.sh
```

默认情况下，MariaDB 12 这条 smoke 会先重建 `./data-mariadb`。

如果你要把前端页面链路也一起回归，直接执行：

```bash
./scripts/browser-click-regression.sh
```

如果要切到 MariaDB 12：

```bash
COMPOSE_FILE_PATHS="compose.mariadb.yml" \
./scripts/browser-click-regression.sh
```

这条浏览器脚本会真实点击：

- 数据库引导页的 `MySQL / MariaDB` 选择与保存
- 安装向导
- 首次进入引导的 4 个快捷按钮
- 设置页维护工具中的“生成备份”与“恢复到此备份”
- 恢复后的重新登录、审计日志核对、恢复结果卡片核对

默认情况下，这条浏览器脚本会重建当前 Compose 入口对应的数据目录；MySQL 8.4 对应 `./data-mysql`，MariaDB 12 对应 `./data-mariadb`。同样可以用 `MYSQL_SMOKE_RESET_DATA_DIR=0` 关闭。

## 4. 默认数据路径

MySQL 8.4 Compose 默认把宿主机 `./data-mysql` 挂载到应用容器内 `/data`。

MariaDB 12 Compose 默认把宿主机 `./data-mariadb` 挂载到应用容器内 `/data`。

关键文件位置如下：

- Bootstrap 配置：`/data/bootstrap/config.json`
- 后台创建的 MySQL 备份：`/data/backup/backup_*.mysql.sql`
- 本地图片目录：`/data/images`

MySQL 8.4 数据库自身数据文件保存在 Docker named volume `mysql_data` 中，不直接暴露到仓库宿主机目录。

MariaDB 12 数据库自身数据文件保存在 Docker named volume `mariadb_data` 中，不直接暴露到仓库宿主机目录。

仓库根目录下的 `./data-mysql` 与 `./data-mariadb` 都已加入 `.gitignore`，属于本地运行数据目录，不应提交。

如果你要清空本地 MySQL 8.4 开发现场，可以执行：

```bash
docker compose -f compose.mysql.yml down -v --remove-orphans
rm -rf data-mysql
```

如果你要清空本地 MariaDB 12 开发现场，可以执行：

```bash
docker compose -f compose.mariadb.yml down -v --remove-orphans
rm -rf data-mariadb
```

## 5. 备份策略

当前后台“维护 / 备份恢复”中的 MySQL 备份，使用 `mysqldump` 生成逻辑 SQL 导出文件：

- 备份文件后缀：`*.mysql.sql`
- 同时生成 sidecar manifest：`*.mysql.sql.manifest.json`
- 只覆盖数据库结构和数据
- 不包含本地图片文件、S3 对象或其他 `/data` 附件
- 导出后会校验备份文件真实存在且非空
- 如果 `mysqldump` 或 `mariadb-dump` 成功但打印 warning，warning 会写入应用日志，并作为审计附加信息保留

这个 sidecar manifest 会额外记录：

- 备份数据库类型
- 当前存储配置签名
- 备份创建时的对象 / 文件回滚锚点
- 如果当前是 S3 / MinIO，则记录 bucket / prefix / backup timestamp / bucket versioning 状态

因此它适合“数据库逻辑快照”，不等同于“整站冷备”。

如果站点使用本地文件存储，并且希望数据库与图片文件一起回滚，仍然应在停机状态下额外备份整个 `DATA_DIR`。

当前仓库已经补上运维级脚本：

```bash
./scripts/mysql-ops-backup.sh
```

默认行为：

- MySQL 8.4 时生成到 `ops-backups/mysql/`
- MariaDB 12 时生成到 `ops-backups/mariadb/`
- 同时把最近一次 manifest 写入当前数据目录下的 `backup/mysql_last_backup_manifest.json`

如果你只想导出数据库逻辑备份，不打本地数据快照，可以执行：

```bash
INCLUDE_DATA_SNAPSHOT=0 ./scripts/mysql-ops-backup.sh
```

默认情况下，脚本会在打本地数据快照前暂时停止 `app` 服务，完成后再拉起，以避免本地图片或 bootstrap 文件在归档过程中继续变化。

## 6. 当前支持范围

当前版本已经覆盖：

- 数据库引导
- 安装向导
- 登录鉴权
- 图片上传 / 软删 / 恢复 / 永久删除
- 后台创建、下载、删除 MySQL 备份
- 后台页面内的 MySQL 备份预检、计划恢复、启动前导入恢复、自动回滚
- `scripts/compose-smoke.sh` 中的 MySQL 8.4 / MariaDB 12 主链路校验
- `scripts/mysql-ops-backup.sh` 运维级备份脚本
- `scripts/mysql-ops-restore.sh` 运维级恢复脚本
- `scripts/browser-click-regression.sh` 的 MySQL 8.4 / MariaDB 12 页面回归
- `scripts/mysql-ops-drill.sh` 的 MySQL 8.4 / MariaDB 12 运维演练

MariaDB 12 现在按正式支持处理：

- 接受 `mariadb://` 连接串
- 开发 / 烟测入口：`compose.mariadb.yml`
- 长期运维模板：`compose.mariadb.ops.yml.example`
- 备份优先使用 `mysqldump`，没有时回退 `mariadb-dump`
- 恢复优先使用 `mysql`，没有时回退 `mariadb`
- 运行时数据库类型仍统一按 `mysql` 家族处理

## 7. 恢复方式与停机要求

当前有两种 MySQL / MariaDB 恢复入口：

1. 后台页面恢复
2. 运维脚本恢复

后台页面恢复适用范围：

- 目标文件必须是后台生成的 `*.mysql.sql`
- 对应 `*.manifest.json` 必须存在
- 只恢复数据库，不替换当前数据库家族对应的本地数据目录
- 写入恢复计划后，需要尽快重启 `app`

后台页面恢复的实际流程是：

1. 管理员在后台对目标备份做预检
2. 预检通过后，写入 pending restore
3. 运维重启 `app`
4. 应用在启动前先导出 `rollback_before_restore_*.mysql.sql`
5. 应用清空当前 schema 并导入目标 SQL
6. 如果导入后启动失败，自动再导回 rollback SQL

如果你需要连同本地数据目录一起恢复，则继续使用独立恢复脚本：

```bash
MYSQL_RESTORE_SQL_PATH=ops-backups/mysql/mysql_ops_backup_xxx.mysql.sql \
MYSQL_RESTORE_DATA_ARCHIVE=ops-backups/mysql/mysql_ops_backup_xxx.data.tar.gz \
./scripts/mysql-ops-restore.sh

MYSQL_RESTORE_SQL_PATH=ops-backups/mariadb/mysql_ops_backup_xxx.mysql.sql \
MYSQL_RESTORE_DATA_ARCHIVE=ops-backups/mariadb/mysql_ops_backup_xxx.data.tar.gz \
./scripts/mysql-ops-restore.sh
```

如果你希望按 manifest 自动带出备份文件路径并校验 SHA256，可以直接执行：

```bash
MYSQL_RESTORE_MANIFEST_PATH=./data-mysql/backup/mysql_last_backup_manifest.json \
./scripts/mysql-ops-restore.sh

MYSQL_RESTORE_MANIFEST_PATH=./data-mariadb/backup/mysql_last_backup_manifest.json \
./scripts/mysql-ops-restore.sh
```

脚本流程如下：

1. 停止 `app` 服务，阻断新的写流量
2. 先导出当前数据库为回滚快照 `rollback_before_restore_*.mysql.sql`
3. 如果你提供了数据目录归档，再额外生成 `rollback_before_restore_*.data.tar.gz`
4. 重建目标数据库并导入指定 SQL 备份
5. 如果提供了数据目录归档，同时把当前数据库家族对应的数据目录回滚到该归档版本
6. 重新拉起应用并等待 `/health`
7. 如果恢复后启动失败，脚本会自动尝试导回回滚快照

恢复结果会写到：

- `./data-mysql/backup/mysql_last_restore_result.json`
- `./data-mariadb/backup/mysql_last_restore_result.json`

这个结果文件至少会说明：

- 本次是否按 manifest 发起恢复
- 本次目标恢复文件
- manifest 中记录的 SHA256
- 本次生成的回滚快照路径
- 最终状态是 `completed`、`rolled_back` 还是 `failed`
- 应用健康检查地址

这意味着当前 MySQL / MariaDB 已经具备“脚本化恢复 + 自动回滚前快照 + 结果留痕”的基础运维能力。

## 8. 演练脚本与 Workflow

为了避免恢复脚本长期无人验证，仓库内新增了一条完整演练脚本：

```bash
./scripts/mysql-ops-drill.sh
```

这条脚本会自动完成：

1. 启动 `compose.mysql.yml` 或 `compose.mariadb.yml`
2. 写入 MySQL bootstrap 配置并完成安装向导
3. 创建站点基线状态和本地 marker 文件
4. 调用 `mysql-ops-backup.sh` 生成备份与 manifest
5. 人为篡改数据库和本地文件
6. 调用 `mysql-ops-restore.sh` 按 manifest 恢复
7. 校验数据库、文件和 `/health` 是否都回到基线

同时新增了 GitHub Actions workflow `MySQL Family Ops Drill`，支持：

- 手动触发
- 每周定时跑一次演练

这样 MySQL 的备份恢复链路不再只是“文档存在”，而是有独立演练入口和自动化回归。

此外，主 CI 里还会自动执行：

- `scripts/compose-smoke.sh` 的 MySQL 8.4 / MariaDB 12 restore 链路
- `scripts/browser-click-regression.sh` 的 MySQL 8.4 / MariaDB 12 浏览器点击回归

## 9. 当前限制

当前版本对 MySQL / MariaDB 的运维边界如下：

- 不支持类似 SQLite 那样的“启动前文件级替换恢复”
- 后台页面恢复只覆盖数据库，不直接替换当前数据库家族对应的本地数据目录
- PostgreSQL 导出备份仍不支持页面内恢复
- 旧的、没有 `*.manifest.json` 的 MySQL 备份不能直接走后台页面恢复

原因很直接：

- SQLite 恢复是单文件替换模型
- MySQL 是服务型数据库，恢复需要停机窗口、导入策略和失败回滚策略
- 页面恢复与整站冷备恢复不是同一层能力

## 10. 对象存储回滚策略

如果站点使用 S3 / MinIO，当前版本不再要求你自己定义一套对象回滚策略；产品内置策略如下：

1. 每次后台生成数据库备份时，同时记录对象回滚锚点
2. 这个锚点固定包含 bucket、prefix、备份时间，以及当时探测到的 bucket versioning 状态
3. 后台恢复预检会把这个锚点展示出来，要求你按同一个锚点回退对象版本
4. 如果 bucket versioning 没开，页面会明确提示“数据库可恢复，但对象无法按产品锚点精确回溯”

也就是说，数据库恢复与对象恢复现在共享同一个“备份时间点”语义，而不是让你自己再额外定义另一套版本策略。
