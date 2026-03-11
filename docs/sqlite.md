# SQLite 运维说明

本文档对应当前项目的 SQLite 运行模式，重点说明正式部署入口、备份恢复约束、停机要求和回滚策略。

## 1. 正式部署入口

使用独立的 SQLite Compose 文件启动：

```bash
docker compose -f compose.sqlite.yml up --build -d
```

首次打开网页时，系统会进入“数据库引导”页。选择 `SQLite` 后填写数据库文件路径，例如：

```text
/data/sqlite/app.db
sqlite:///data/sqlite/app.db
```

保存后需要重启应用容器，让运行时真正切换到 SQLite：

```bash
docker compose -f compose.sqlite.yml restart app
```

然后继续完成安装向导：

- 创建管理员邮箱和密码
- 配置站点标题 / favicon
- 配置存储后端
- 配置公开注册、邮箱验证、密码重置所需的邮件服务

邮件配置应在安装向导或管理员设置页中保存，不需要写进 Compose 环境变量。

## 2. 默认数据路径

SQLite Compose 默认把宿主机 `./data-sqlite` 挂载到容器内 `/data`。

关键文件位置如下：

- Bootstrap 配置：`/data/bootstrap/config.json`
- SQLite 数据库：`/data/sqlite/app.db`
- 后台创建的 SQLite 备份：`/data/backup/backup_*.sqlite3`
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
docker compose -f compose.sqlite.yml stop app
tar -C . -czf vansour-data-sqlite-backup.tgz data-sqlite
docker compose -f compose.sqlite.yml start app
```

这类备份能同时覆盖：

- `/data/sqlite/app.db`
- `/data/images`
- `/data/bootstrap`
- `/data/backup`

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
docker compose -f compose.sqlite.yml restart app
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

它会在 `compose.sqlite.yml` 之外再叠加 `compose.sqlite.mailpit.yml`，并自动验证：

- SQLite 数据库引导与安装向导
- Mailpit SMTP 投递
- 公开注册、邮箱验证、密码重置
- 登录、刷新、退出
- 图片上传、查询、过期回收、软删恢复、永久删除
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

另外，仓库内已经新增独立的 GitHub Actions workflow `SQLite E2E`，用于手动或定时执行这套全链路回归。
