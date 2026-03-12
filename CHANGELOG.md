# Changelog

## 0.1.0 - 2026-03-12

- 固化 `0.1` 支持范围，只把 `PostgreSQL + Redis 8 + 本地存储` 作为默认 GA 推荐栈。
- 增加统一的 `release-ga-gate`，串行执行 Rust checks、默认 PostgreSQL smoke、PostgreSQL 物理恢复演练和两种 PITR 演练。
- 收敛 PostgreSQL 页面备份语义为“下载型逻辑导出”，企业主恢复统一走物理备份 / PITR 运维脚本。
- 补齐 `release-rc-preflight` 与 `release-ga-ship`，把正式版版本、发布说明、镜像元数据和校验和收进统一发版入口。

## 0.1.0-rc.1 - 2026-03-12

- 固化 `0.1` 支持范围，只把 `PostgreSQL + Redis 8 + 本地存储` 作为默认 GA 推荐栈。
- 增加统一的 `release-ga-gate`，串行执行 Rust checks、默认 PostgreSQL smoke、PostgreSQL 物理恢复演练和两种 PITR 演练。
- 收敛 PostgreSQL 页面备份语义为“下载型逻辑导出”，企业主恢复统一走物理备份 / PITR 运维脚本。
- 补齐 `Release 0.1 RC Preflight`，校验 changelog、镜像元数据以及 `/health` 暴露的候选版版本信息。
