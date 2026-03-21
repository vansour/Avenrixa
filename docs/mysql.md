# MySQL 8.4 / MariaDB 12 运维说明

本文档对应当前项目的 MySQL 8.4 与 MariaDB 12 运行模式，重点说明开发入口、长期运维入口、运维脚本、停机要求和当前限制。

按 [`release-0.1-scope.md`](release-0.1-scope.md) 当前定义，MySQL 8.4 与 MariaDB 12 在 `0.1` 范围内按 `Beta` 处理：相关链路会继续保留并维护，但不作为 `0.1` 首页默认推荐栈，也不应与 PostgreSQL GA 主路径使用同等级发布承诺。

手工部署现在直接编辑仓库根目录的 `compose.yml`，并切到其中对应的 MySQL / MariaDB 注释片段。
烟测脚本、CI 和专项运维流程也不再依赖仓库内额外的 compose 文件，而是运行时基于同一个 `compose.yml` 生成临时栈定义。

当前项目的外部缓存实现已经按 Redis 协议统一处理：

- 仓库内 Compose 默认提供 `cache` 服务，镜像为 Redis 8
- 如果你更偏好 Dragonfly，可以把同一个 `compose.yml` 里的 `cache` 服务改成 Dragonfly
- 如果你完全不提供 `REDIS_URL`，应用也能启动，只是以无外部缓存模式运行

## 1. 入口区分

当前只有一个入口文件：`compose.yml`。

- 开发 / 联调 / 烟测：在 `compose.yml` 里启用 MySQL 8.4 或 MariaDB 12 的开发预设
- 长期运维：在 `compose.yml` 里启用对应的运维口令预设，并替换所有占位密钥与密码

## 2. 开发 / 联调入口

开发或联调 MySQL 8.4 时，先把 `compose.yml` 切到 MySQL 8.4 开发预设，再执行：

```bash
docker compose up --build -d
```

开发或联调 MariaDB 12 时，先把 `compose.yml` 切到 MariaDB 12 开发预设，再执行：

```bash
docker compose up --build -d
```

这两个入口都适合：

- 本地功能联调
- Docker 冒烟脚本
- CI 中的 MySQL 主链路校验

它们默认都会额外拉起一个名为 `cache` 的 Redis 8 服务，并把应用环境变量预设为：

```text
redis://cache:6379
```

如果你想切到 Dragonfly，直接把同一个 `compose.yml` 里的 `cache` 服务改成 Dragonfly 即可。

## 3. 长期运维入口

先在 `compose.yml` 里启用对应的 MySQL / MariaDB 运维预设，然后至少修改下面这些值：

- `JWT_SECRET`
- 应用账户密码
- 数据库 root 密码

修改完成后再启动：

```bash
docker compose up --build -d
```

注意：

- 当前 MySQL / MariaDB 运行时生成的栈都使用固定 `container_name`
- 这意味着同一台机器一次只能运行一套同名数据库家族栈

当前这些 Compose 入口默认都已经通过环境变量预设了数据库连接：

```text
mysql://user:pass@mysql:3306/image
mariadb://user:pass@mysql:3306/image
```

如果你使用 MySQL 8.4 长期运维模板，对应写法应改为：

```text
mysql://avenrixa:replace-with-strong-app-password@mysql:3306/image
```

如果你使用 MariaDB 12 长期运维模板，对应写法应改为：

```text
mariadb://avenrixa:replace-with-strong-app-password@mysql:3306/image
```

因此使用这些 Compose 入口时，首次打开网页会直接进入安装向导，而不是数据库引导页。然后继续完成：

- 创建管理员邮箱和密码
- 配置站点标题 / favicon
- 配置存储后端
- 配置公开注册、邮箱验证、密码重置所需的邮件服务

只有当你删除了 `DATABASE_KIND` / `DATABASE_URL`，或者在自定义部署里没有预设连接时，系统才会回退到“数据库引导”页；此时可以手动填写上面的连接地址并重启 `app`。

同理，只有在你明确删除 `REDIS_URL`，或让 `cache` 服务不可达时，应用才会回到无外部缓存模式；这不会破坏登录与主流程，但后台健康状态会显示 `disabled` 或 `degraded`。

如果要跑项目内置的 MySQL 8.4 Docker 联调烟测，可以直接执行：

```bash
COMPOSE_VARIANT=mysql \
./scripts/compose-smoke.sh
```

如果你要切换缓存实现：

```bash
COMPOSE_VARIANT=mysql CACHE_MODE=redis8 ./scripts/compose-smoke.sh
COMPOSE_VARIANT=mysql CACHE_MODE=dragonfly ./scripts/compose-smoke.sh
COMPOSE_VARIANT=mysql CACHE_MODE=none ./scripts/compose-smoke.sh
```

这条脚本会自动完成：

- 检查 Compose 是否已预设 MySQL / MariaDB 数据库连接；如未预设则自动写入数据库引导兜底配置
- 进入安装向导
- 管理员创建与登录校验
- 图片上传、永久删除
- MySQL 备份创建、下载
- MySQL 恢复预检会明确返回“仅运维脚本恢复”
- 后台列表与恢复语义字段会标记这类备份不支持页面恢复

默认情况下，`scripts/compose-smoke.sh` 会先重建当前 Compose 入口对应的数据目录；MySQL 8.4 对应 `./data-mysql`，MariaDB 12 对应 `./data-mariadb`。如果你要复用已有目录，再显式加上 `MYSQL_SMOKE_RESET_DATA_DIR=0`。

如果要跑 MariaDB 12 Docker 联调烟测，可以直接执行：

```bash
COMPOSE_VARIANT=mariadb \
./scripts/compose-smoke.sh
```

MariaDB 12 同样支持：

```bash
COMPOSE_VARIANT=mariadb CACHE_MODE=redis8 ./scripts/compose-smoke.sh
COMPOSE_VARIANT=mariadb CACHE_MODE=dragonfly ./scripts/compose-smoke.sh
COMPOSE_VARIANT=mariadb CACHE_MODE=none ./scripts/compose-smoke.sh
```

默认情况下，MariaDB 12 这条 smoke 会先重建 `./data-mariadb`。

如果你要把前端页面链路也一起回归，直接执行：

```bash
./scripts/browser-click-regression.sh
```

如果要切到 MariaDB 12：

```bash
COMPOSE_VARIANT=mariadb \
./scripts/browser-click-regression.sh
```

浏览器点击回归也支持相同的缓存模式切换：

```bash
CACHE_MODE=dragonfly ./scripts/browser-click-regression.sh
CACHE_MODE=none ./scripts/browser-click-regression.sh
```

这条浏览器脚本会真实点击：

- 如果当前入口未预设数据库连接，则自动完成数据库引导页的 `MySQL / MariaDB` 选择与保存；否则直接进入安装向导
- 安装向导
- 首次进入引导的 4 个快捷按钮
- 设置页维护工具中的“生成备份”
- 备份行上的“不支持页面恢复”按钮与运维恢复提示
- 审计日志中的备份创建记录

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
docker compose down -v --remove-orphans
rm -rf data-mysql
```

如果你要清空本地 MariaDB 12 开发现场，可以执行：

```bash
docker compose down -v --remove-orphans
rm -rf data-mariadb
```

## 5. 备份策略

当前 MySQL / MariaDB 运维脚本已经分成两个模式：

1. `MYSQL_BACKUP_MODE=physical`
2. `MYSQL_BACKUP_MODE=logical`

默认值现在已经切到 `physical`；`logical` 只保留给兼容导出和旧流程。

### 5.1 物理全量备份基座

企业主路径现在是物理备份：

```bash
./scripts/mysql-ops-backup.sh
```

默认行为：

- MySQL 8.4 使用 `percona/percona-xtrabackup:8.4`
- MariaDB 12 使用当前 MariaDB 容器镜像中的 `mariadb-backup`
- 先执行一次全量物理备份，再执行 `prepare`
- 产物输出到 `ops-backups/mysql/` 或 `ops-backups/mariadb/`
- 最近一次 manifest 写入当前数据目录下的 `backup/mysql_last_physical_backup_manifest.json`

物理 manifest 会记录：

- `tool_family` 与 helper 镜像
- prepared 物理备份目录路径
- `backup.log` / `prepare.log`
- `xtrabackup_checkpoints` / `xtrabackup_info` / `xtrabackup_binlog_info`
- `from_lsn` / `to_lsn` / `last_lsn`
- binlog 坐标与原始元数据文本

如果你需要改 helper 镜像源或内网镜像地址，可以覆盖：

```bash
MYSQL_PHYSICAL_HELPER_IMAGE=registry.example.com/percona/percona-xtrabackup:8.4 \
MYSQL_BACKUP_MODE=physical ./scripts/mysql-ops-backup.sh
```

### 5.2 逻辑导出兼容路径

如果你仍然需要逻辑导出兼容路径，显式执行：

```bash
MYSQL_BACKUP_MODE=logical ./scripts/mysql-ops-backup.sh
```

特点：

- 备份文件后缀：`*.mysql.sql`
- 同时生成 sidecar manifest：`*.manifest.json`
- 只覆盖数据库结构和数据
- 不包含本地图片文件、S3 对象或其他 `/data` 附件
- 导出后会校验备份文件真实存在且非空
- 如果 `mysqldump` 或 `mariadb-dump` 成功但打印 warning，warning 会写入应用日志，并作为审计附加信息保留

逻辑 manifest 仍会额外记录：

- 备份数据库类型
- 当前存储配置签名
- 备份创建时的对象 / 文件回滚锚点
- 如果当前是 S3 / MinIO，则记录 bucket / prefix / backup timestamp / bucket versioning 状态

因此它适合兼容导出、审计留档和旧流程过渡，不再是企业主备份方式。

如果站点使用本地文件存储，并且希望数据库与图片文件一起回滚，无论 physical 还是 logical，都可以继续带上 `INCLUDE_DATA_SNAPSHOT=1` 额外归档 `/data`。

默认情况下，脚本会在打本地数据快照前暂时停止 `app` 服务，完成后再拉起，以避免本地图片或 bootstrap 文件在归档过程中继续变化。

## 6. 当前覆盖与 0.1 定位

当前版本已经覆盖：

- 未预设环境变量时的数据库引导兜底流程
- 安装向导
- 登录鉴权
- 图片上传 / 永久删除
- 后台创建、下载、删除 MySQL 备份
- 后台页面把 MySQL 备份明确标记为“仅运维脚本恢复”
- `scripts/compose-smoke.sh` 中的 MySQL 8.4 / MariaDB 12 主链路校验
- `scripts/mysql-ops-backup.sh` 的 physical / logical 双模式运维备份脚本
- `scripts/mysql-ops-restore.sh` 运维级恢复脚本
- `scripts/browser-click-regression.sh` 的 MySQL 8.4 / MariaDB 12 页面回归
- `scripts/mysql-ops-drill.sh` 的 MySQL 8.4 / MariaDB 12 运维演练

MySQL 8.4 / MariaDB 12 当前仍保留在仓库和文档中，但在 `0.1` 里按 `Beta` 处理：

- 接受 `mariadb://` 连接串
- 手工入口：`compose.yml`
- 运行时具体变体：`COMPOSE_VARIANT=mysql|mariadb|mysql-ops|mariadb-ops`
- 备份优先使用 `mysqldump`，没有时回退 `mariadb-dump`
- 恢复优先使用 `mysql`，没有时回退 `mariadb`
- 运行时数据库类型仍统一按 `mysql` 家族处理

这表示：

- 它们会继续参与专项烟测、浏览器回归和运维演练
- 但不会作为 `0.1` 首页默认推荐生产栈
- 如果后续阶段要提升到 GA，需要先补齐更严格的测试、升级和恢复验收标准

## 7. 恢复方式与停机要求

当前 MySQL / MariaDB 的恢复入口统一是运维脚本，不再提供页面内恢复。

当前 `mysql-ops-restore.sh` 已经支持两条恢复链路：

- 物理主路径：读取 prepared physical manifest，停 `app` 和数据库服务，先归档当前 datadir 作为回滚点，再执行 copy-back
- 逻辑兼容路径：读取 SQL dump 或 logical manifest，先导出当前数据库为回滚 dump，再导入目标 SQL

如果你需要连同本地数据目录一起恢复，继续带上数据归档即可。

逻辑兼容恢复示例：

```bash
MYSQL_RESTORE_SQL_PATH=ops-backups/mysql/mysql_ops_backup_xxx.mysql.sql \
MYSQL_RESTORE_DATA_ARCHIVE=ops-backups/mysql/mysql_ops_backup_xxx.data.tar.gz \
./scripts/mysql-ops-restore.sh

MYSQL_RESTORE_SQL_PATH=ops-backups/mariadb/mysql_ops_backup_xxx.mysql.sql \
MYSQL_RESTORE_DATA_ARCHIVE=ops-backups/mariadb/mysql_ops_backup_xxx.data.tar.gz \
./scripts/mysql-ops-restore.sh
```

物理主路径恢复示例：

```bash
MYSQL_RESTORE_MANIFEST_PATH=./data-mysql/backup/mysql_last_physical_backup_manifest.json \
./scripts/mysql-ops-restore.sh

MYSQL_RESTORE_MANIFEST_PATH=./data-mariadb/backup/mysql_last_physical_backup_manifest.json \
./scripts/mysql-ops-restore.sh
```

如果你希望按 logical manifest 自动带出 SQL 文件路径并校验 SHA256，可以继续执行：

```bash
MYSQL_RESTORE_MANIFEST_PATH=./data-mysql/backup/mysql_last_backup_manifest.json \
./scripts/mysql-ops-restore.sh

MYSQL_RESTORE_MANIFEST_PATH=./data-mariadb/backup/mysql_last_backup_manifest.json \
./scripts/mysql-ops-restore.sh
```

物理恢复脚本流程如下：

1. 停止 `app` 服务，阻断新的写流量
2. 停止 MySQL / MariaDB 服务，冻结当前 datadir
3. 归档当前 datadir 为 `rollback_before_restore_*.mysql-datadir.tar.gz`
4. 如果你提供了数据目录归档，再额外生成 `rollback_before_restore_*.data.tar.gz`
5. 对 prepared physical backup 执行 copy-back
6. 重新拉起 MySQL / MariaDB 并等待健康检查
7. 重新拉起应用并等待 `/health`
8. 如果恢复后启动失败，脚本会自动尝试导回 datadir 回滚快照

逻辑兼容恢复流程如下：

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
- 本次目标恢复文件或 physical backup 目录
- manifest 中记录的 SHA256
- 本次生成的回滚快照路径
- 最终状态是 `completed`、`rolled_back` 还是 `failed`
- 应用健康检查地址

这意味着当前 MySQL / MariaDB 已经具备“物理备份 + 物理恢复 + 逻辑兼容恢复 + 自动回滚前快照 + 结果留痕”的基础运维能力。

后台页面中的 `*.mysql.sql` 备份现在只承担两件事：

- 供管理员下载逻辑导出文件
- 给运维脚本提供一份可追溯、带 manifest 的数据库逻辑快照

## 8. 演练脚本与 Workflow

为了避免恢复脚本长期无人验证，仓库内新增了一条完整演练脚本：

```bash
./scripts/mysql-ops-drill.sh
```

这条脚本会自动完成：

1. 启动基于 `compose.yml` 运行时生成的 MySQL / MariaDB 栈
2. 若当前入口未预设数据库连接，则写入 MySQL bootstrap 兜底配置；否则直接进入安装向导
3. 创建站点基线状态和本地 marker 文件
4. 调用 `MYSQL_BACKUP_MODE=physical ./scripts/mysql-ops-backup.sh` 生成物理备份基座并校验 manifest
5. 人为篡改数据库和本地文件
6. 调用 `mysql-ops-restore.sh` 按 physical manifest 恢复
7. 校验数据库、文件和 `/health` 是否都回到基线

同时新增了 GitHub Actions workflow `Avenrixa MySQL Family Ops Drill (Supplemental)`，支持：

- 手动触发
- 每周定时跑一次演练

这样 MySQL 的物理备份与物理恢复主链路都不再只是“文档存在”，而是有独立演练入口和自动化回归。

此外，主 CI 里还会自动执行：

- `scripts/compose-smoke.sh` 的 MySQL 8.4 / MariaDB 12 备份导出与恢复语义收口校验
- `scripts/browser-click-regression.sh` 的 MySQL 8.4 / MariaDB 12 浏览器点击回归

## 9. 当前限制

当前版本对 MySQL / MariaDB 的运维边界如下：

- 后台页面不再提供 MySQL / MariaDB 的恢复入口
- PostgreSQL 导出备份仍不支持页面内恢复
- 逻辑 `*.mysql.sql` 仍然是兼容路径，不是企业主恢复手段
- 旧的、没有 `*.manifest.json` 的 MySQL 备份仍可下载，但不具备同等的运维追溯信息

原因很直接：

- 当前产品不再为任何数据库家族承诺页面内“一键恢复”
- MySQL 是服务型数据库，恢复需要停机窗口、导入策略和失败回滚策略
- 物理备份和物理恢复需要成对设计，所以页面内“一键恢复”不适合作为默认入口

## 10. 对象存储回滚策略

如果站点使用 S3 / MinIO，当前版本不再要求你自己定义一套对象回滚策略；产品内置策略如下：

1. 每次后台生成数据库备份时，同时记录对象回滚锚点
2. 这个锚点固定包含 bucket、prefix、备份时间，以及当时探测到的 bucket versioning 状态
3. 运维恢复前应按同一个锚点回退对象版本，而不是另找一个时间点
4. 如果 bucket versioning 没开，产品记录会明确提示“数据库可恢复，但对象无法按产品锚点精确回溯”

也就是说，数据库恢复与对象恢复现在共享同一个“备份时间点”语义，而不是让你自己再额外定义另一套版本策略。
