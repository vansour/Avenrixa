# SQLite 运维说明

本文档对应当前项目的 SQLite 运行模式，重点说明正式部署入口、备份恢复约束、停机要求和回滚策略。

按 [`release-0.1-scope.md`](release-0.1-scope.md) 当前定义，SQLite 在 `0.1` 范围内按 `Beta` 处理：适合单机或低运维成本场景，但不是 `0.1` 首页默认推荐栈，也不应和 PostgreSQL GA 主路径使用同一套稳定性承诺。

手工部署现在直接编辑仓库根目录的 `compose.yml`，并切到其中对应的 SQLite 注释片段。
SQLite 专项脚本和 E2E 也不再依赖单独的 compose 文件，而是运行时基于同一个 `compose.yml` 生成临时栈定义。

当前项目的外部缓存实现已经按 Redis 协议统一处理：

- 仓库内 SQLite Compose 默认提供 `cache` 服务，镜像为 Redis 8
- 如果你更偏好 Dragonfly，可以把同一个 `compose.yml` 里的 `cache` 服务改成 Dragonfly
- 如果你完全不提供 `REDIS_URL`，应用也能启动，只是以无外部缓存模式运行

## 1. 正式部署入口

先把 `compose.yml` 切到 SQLite 预设，然后启动：

```bash
docker compose up --build -d
```

SQLite 预设默认已经通过环境变量预设了 SQLite 连接：

```text
sqlite:///data/sqlite/app.db
```

因此使用 SQLite 预设时，首次打开网页会直接进入安装向导。然后继续完成：

- 创建管理员邮箱和密码
- 配置站点标题 / favicon
- 配置存储后端
- 配置公开注册、邮箱验证、密码重置所需的邮件服务

邮件配置应在安装向导或管理员设置页中保存，不需要写进 Compose 环境变量。

同一个 Compose 文件还会默认把 `REDIS_URL` 预设为：

```text
redis://cache:6379
```

如果你删掉了 `DATABASE_KIND` / `DATABASE_URL`，或是在自定义部署中没有提供 SQLite 连接，系统才会回退到“数据库引导”页；此时可以手动填写例如：

```text
/data/sqlite/app.db
sqlite:///data/sqlite/app.db
```

保存后再执行：

```bash
docker compose restart app
```

如果你删掉了 `REDIS_URL`，应用仍然会继续启动，只是不再使用外部缓存。

如果你要用项目内置脚本验证 SQLite 在不同缓存模式下的行为，直接执行：

```bash
COMPOSE_VARIANT=sqlite CACHE_MODE=redis8 ./scripts/sqlite-e2e-smoke.sh
COMPOSE_VARIANT=sqlite CACHE_MODE=dragonfly ./scripts/sqlite-e2e-smoke.sh
COMPOSE_VARIANT=sqlite CACHE_MODE=none ./scripts/sqlite-e2e-smoke.sh
```

## 2. 默认数据路径

SQLite Compose 默认把宿主机 `./data-sqlite` 挂载到容器内 `/data`。

关键文件位置如下：

- Bootstrap 配置：`/data/bootstrap/config.json`
- SQLite 数据库：`/data/sqlite/app.db`
- 后台创建的 SQLite 备份：`/data/backup/backup_*.sqlite3`
- 整站冷备最近一次 manifest：`/data/backup/sqlite_last_cold_backup_manifest.json`
- 待执行恢复计划：`/data/backup/pending_restore.json`
- 最近一次恢复结果：`/data/backup/last_restore_result.json`
- 自动回滚快照：`/data/backup/rollback_before_restore_*.sqlite3`

## 3. 备份策略

当前项目有两类备份思路：

### 3.1 后台文件级数据库备份

管理员在后台“维护 / 备份恢复”中创建备份时，系统会生成 `backup_*.sqlite3`。

特点：

- 只备份 SQLite 数据库文件
- 通过 `VACUUM INTO` 导出，适合日常快速备份
- 不包含本地图片文件、S3 对象或其他 `/data` 附件

因此它适合“元数据回滚”，不等同于“整站冷备”。

### 3.2 整站冷备

如果站点使用本地文件存储，并且你希望数据库和图片文件一起回滚，应该在停机状态下备份整个 `DATA_DIR`：

```bash
./scripts/sqlite-ops-backup.sh
```

默认行为：

- 归档输出到 `ops-backups/sqlite/`
- 生成 `*.tar.gz`、对应的 `*.sha256` 和 `*.manifest.json`
- 最近一次 manifest 额外写入 `/data/backup/sqlite_last_cold_backup_manifest.json`
- 如果 `app` 当前正在运行，脚本会先停 `app`，打包完成后再自动拉起

这类冷备能同时覆盖：

- `/data/sqlite/app.db`
- `/data/images`
- `/data/bootstrap`
- `/data/backup`

如果你已经手工停好服务，也可以显式关闭脚本内的停机控制：

```bash
STOP_APP_DURING_BACKUP=0 ./scripts/sqlite-ops-backup.sh
```

要特别注意：

- 这条脚本备份的是整个挂载进容器的 SQLite `DATA_DIR`，不是单独的数据库文件
- 如果当前存储后端是 S3 / MinIO，外部对象数据不在这个归档里
- 后台页面恢复只覆盖 3.1 里的 SQLite 数据库快照；3.2 这类整站冷备属于运维恢复路径

## 4. 恢复约束

当前版本的后台恢复功能只支持 SQLite，并且有明确限制：

- 当前运行实例必须已经使用 SQLite
- 当前数据库地址必须是可落到单个文件的 SQLite 路径
- 备份必须通过 `integrity_check`
- 备份里必须包含核心表：`users`、`settings`、`images`
- 备份必须已经完成安装，并且包含管理员账户
- 备份中的存储配置必须与当前运行配置一致

另外要特别注意：

- 恢复只替换数据库文件，不会自动回滚本地图片文件或 S3 对象
- 恢复计划写入后，必须尽快重启服务，真正的文件替换发生在下次启动前
- 恢复成功后，现有登录会话和缓存都会失效，所有用户需要重新登录

## 5. 停机要求

SQLite 恢复是“启动前替换数据库文件”的流程，不是在线热切换。

实际过程是：

1. 管理员在后台先做恢复预检查
2. 预检查通过后，写入 `pending_restore.json`
3. 运维重启应用容器
4. 应用在启动阶段先创建恢复前快照，再替换数据库文件
5. 运行时初始化成功后，记录恢复成功审计并清理旧会话

因此停机要求应按下面执行：

- 计划恢复前，先通知有短时重启窗口
- 写入恢复计划后，不要长时间继续对外提供写入流量
- 立刻执行一次应用重启，避免“待恢复计划”和线上写入长期并存

推荐命令：

```bash
docker compose restart app
```

## 6. 回滚策略

当前实现的回滚策略是“恢复前自动创建快照，启动失败时自动回滚”。

流程如下：

1. 启动恢复前，系统先创建 `rollback_before_restore_*.sqlite3`
2. 然后执行数据库文件替换
3. 如果替换后应用启动失败，系统会尝试自动把数据库切回该回滚快照
4. 若回滚成功，会记录 `rolled_back` 结果，并继续尝试启动
5. 若回滚后仍无法启动，应用会回退到 bootstrap 模式，并把错误暴露在启动页

这意味着：

- 回滚保护只覆盖数据库文件
- 回滚并不恢复本地图片文件或对象存储内容
- 恢复成功后的最终结果，以 `last_restore_result.json` 和恢复后的审计日志为准

## 7. 审计与结果查看

恢复相关的最终状态可以从两处查看：

- 后台“维护 / 备份恢复”状态接口读取 `last_restore_result.json`
- 审计日志中的最终事件：
  - `system.database_restore.completed`
  - `system.database_restore.rollback_applied`
  - `system.database_restore.failed`

有一个容易误解的点：

- `prechecked` / `scheduled` 这类“恢复前”审计事件写在旧数据库里
- 一旦恢复到较早快照，这些记录可能随旧数据库一起消失
- 这是当前文件级恢复模型的正常结果，不代表前端展示有误

## 8. 建议的运维动作

如果是 SQLite 正式运行，建议至少做到：

- 把 `DATA_DIR` 放在独立磁盘或稳定挂载点
- 定期导出后台数据库备份
- 重要变更前做一次整站冷备
- 恢复演练时同时验证“重新登录”“后台状态页”“站点标题/配置是否回退”

## 9. SQLite 功能回归脚本

仓库内还提供了一套更完整的 SQLite E2E 冒烟脚本，用于验证当前主链路是否真的可用：

```bash
./scripts/sqlite-e2e-smoke.sh
```

它会在运行时基于同一个 `compose.yml` 生成 SQLite + Mailpit 栈，并自动验证：

- 直接进入安装向导；如果当前入口未预设数据库连接，则自动回退到 SQLite 数据库引导
- Mailpit SMTP 投递
- 公开注册、邮箱验证、密码重置
- 登录、刷新、退出
- 图片上传、查询、过期回收、永久删除
- 管理员用户列表、角色变更、审计分页、统计接口

这套脚本更适合发布前或大改后的人肉回归，不建议替代正式的备份恢复演练。

如果你要在本机复盘失败现场，可以这样执行：

```bash
PRESERVE_STACK_ON_FAILURE=1 \
./scripts/sqlite-e2e-smoke.sh
```

失败时脚本会：

- 打印当前 compose `ps`
- 打印最近的容器日志
- 保留容器、数据目录和临时 cookie/workspace 路径，方便继续手动排查

另外，仓库内已经新增独立的 GitHub Actions workflow `Avenrixa SQLite E2E (Supplemental)`，用于手动或定时执行这套全链路回归。
