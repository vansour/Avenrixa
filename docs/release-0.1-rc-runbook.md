# 0.1 RC Runbook

本文档对应 `0.1.x` 候选版的收口阶段。阶段 1 已经把默认 GA 主链路收进统一门禁；阶段 2 的目标不是再扩大功能范围，而是把候选版资产做成可追溯、可复跑、可发布。

## 统一入口

本地执行：

```bash
./scripts/release-rc-preflight.sh
```

GitHub Actions 触发方式：

- `.github/workflows/release-rc-preflight.yml`
- 推送 RC tag 时会自动触发，例如 `v0.1.2-rc.2`
- 手动触发时必须位于 `main`
- 默认发布到 `ghcr.io/<repository_owner>/avenrixa:<version>`，并追加 `ghcr.io/<repository_owner>/avenrixa:rc`
- workflow 成功后会自动创建或更新对应的 GitHub Pre-release
- 可通过 workflow input `image_repository` 覆盖目标仓库

## 阶段 2 关注点

`release-rc-preflight` 会在 `release-ga-gate` 的基础上继续验证：

- 当前工作区版本已经冻结到候选版号
- `CHANGELOG.md` 已经存在对应候选版条目
- Docker 镜像已带上 `org.opencontainers.image.version` / `revision` / `created`
- 应用 `/health` 返回的 `version` 与候选版元数据一致

## 当前候选版

- 版本示例：`0.1.2-rc.2`
- 本地默认镜像引用：`ghcr.io/vansour/avenrixa:0.1.2-rc.2`
- GitHub Actions 默认发布引用：`ghcr.io/<repository_owner>/avenrixa:0.1.2-rc.2`
- GitHub Actions 默认滚动 RC 标签：`ghcr.io/<repository_owner>/avenrixa:rc`

## 常用参数

- `RELEASE_VERSION`：默认取工作区版本；阶段 2 默认要求它与工作区版本完全一致
- `RELEASE_BUILD_REVISION`：默认取当前 `git rev-parse --short=12 HEAD`
- `RELEASE_BUILD_DATE`：默认取当前 UTC 时间
- `RELEASE_IMAGE_REF`：覆盖本次候选版预检构建出的镜像引用
- `RELEASE_IMAGE_PUSH=0|1`：是否在门禁通过后推送主镜像标签；本地推送前需要先 `docker login ghcr.io`
- `RELEASE_IMAGE_ADDITIONAL_TAGS`：为同一镜像额外推送的空格分隔标签列表；GitHub Actions 默认会为候选版补一个 `:rc`
- `RELEASE_RC_INCLUDE_GA_GATE=0|1`：是否先跑完整 GA 门禁
- `RELEASE_RC_INCLUDE_CHANGELOG=0|1`：是否检查 changelog 条目
- `RELEASE_RC_INCLUDE_VERSION_SMOKE=0|1`：是否检查镜像 labels 与 `/health.version`
- `PRESERVE_STACK_ON_FAILURE=1`：失败时保留现场

## 发布结论

只有当下面几项同时满足时，当前 RC 才应进入正式发版窗口：

- [`release-0.1-ga-checklist.md`](release-0.1-ga-checklist.md) 的完整门禁已通过
- `release-rc-preflight` 已通过
- `CHANGELOG.md`、镜像元数据和运行时版本展示没有互相冲突

进入正式版 cutover 后，应继续执行 [`release-0.1-ga-runbook.md`](release-0.1-ga-runbook.md) 里的 `release-ga-ship`。

分支、标签和镜像规则见 [`release-policy.md`](release-policy.md)。
