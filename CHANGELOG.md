# Changelog

## 0.1.1 - 2026-03-16

- 收口安装向导与系统设置的信息架构，移除冗余说明、收紧步骤表达，并统一 General / Storage / Review 等关键配置流。
- 增加共享 `shared-types` crate，迁移 bootstrap、backup/restore、审计与管理侧纯协议类型，进一步清晰前后端边界。
- 完善对象存储配置体验：新增 provider preset、保存前 S3 连通性测试、R2/S3 状态摘要与更稳定的设置页配置加载逻辑。
- 优化上传中心、历史图库与 API 接入页，包括多图网格、图片复制面板、代码块可读性、系统设置精简及若干布局修复。

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
