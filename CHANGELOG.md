# Changelog

## 0.1.2-rc.3 - 2026-04-08

### Changed

- 完成前端主链路收口，安装页与设置页已接通 `favicon` 配置和服务器目录浏览。
- 拆分设置页控制器，按 `general / security / system / maintenance / users` 模块化整理前端状态逻辑。
- 抽取后端 `runtime settings / favicon` 共享 helper，减少安装与管理员设置路径的重复实现。
- 固化发布与运维入口矩阵，`release-ga-gate`、`release-rc-preflight`、`release-ga-ship` 现在会写出统一 JSON 结果文件。

### Verified

- `cargo test --workspace`
- `npm test --prefix frontend`
- `npm run build --prefix frontend`

### Notes

- 当前 RC 仍围绕 `PostgreSQL + Dragonfly + 本地存储` 主链路准备，不扩大支持矩阵。
