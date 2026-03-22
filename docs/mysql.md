# MySQL 8.4 / MariaDB 12 运维说明

当前主线仓库已经不再维护 MySQL / MariaDB 家族的自动化部署、烟测和页面回归入口。

这份文档保留为退役说明，避免继续把旧路径误认为当前仍受支持的主链路。

## 当前状态

- 默认推荐栈已经收口到 `PostgreSQL + Dragonfly + 本地存储`
- `compose.yml`、`compose-runtime.sh`、`compose-smoke.sh` 都围绕这条主链路维护
- `scripts/browser-click-regression.sh` 已停用
- 外部缓存入口已经统一为 `CACHE_URL`

## 如果你仍然需要 MySQL / MariaDB

- 请回看历史 tag 或旧分支中的对应实现
- 不要把当前主线仓库视为仍提供 MySQL / MariaDB 自动化保障
- 当前正式运维说明以 [`postgresql.md`](postgresql.md) 为准
- 当前支持范围以 [`release-0.1-scope.md`](release-0.1-scope.md) 为准
