# 0.1 GA Runbook

本文档对应 `0.1.x` 正式版 cutover 阶段。阶段 2 已经把候选版预检收进统一入口；阶段 3 的目标是不再停留在“RC 可发布”，而是把正式版版本、发布说明、镜像元数据和校验和一起固化成可复跑资产。

## 统一入口

本地执行：

```bash
./scripts/release-ga-ship.sh
```

GitHub Actions 入口：

- `.github/workflows/release-ga-ship.yml`
- 推送 `v*` tag 时会自动触发，例如 `v0.1.0`
- 手动触发时必须位于 `main`
- 默认发布到 `ghcr.io/<repository_owner>/avenrixa:<version>`，并追加 `:latest`
- 也可继续手动触发，并通过 workflow input `image_repository` 覆盖目标仓库

## 阶段 3 关注点

`release-ga-ship` 会在 `release-rc-preflight` 的基础上继续验证或产出：

- 当前工作区版本已经提升到稳定正式版号
- `CHANGELOG.md` 已经存在对应正式版条目
- `release-rc-preflight` 的全部门禁已经通过
- 正式版镜像已经推送到目标 GHCR 仓库
- 正式版发布说明、镜像元数据、bundle 和 `SHA256SUMS` 已经生成

## 当前正式版

- 版本示例：`0.1.2`
- 本地默认镜像引用：`ghcr.io/vansour/avenrixa:0.1.2`
- GitHub Actions 默认发布引用：`ghcr.io/<repository_owner>/avenrixa:0.1.2`
- GitHub Actions 默认附加标签：`ghcr.io/<repository_owner>/avenrixa:latest`

## 常用参数

- `RELEASE_VERSION`：默认取工作区版本；阶段 3 默认要求它与工作区版本完全一致
- `RELEASE_BUILD_REVISION`：默认取当前 `git rev-parse --short=12 HEAD`
- `RELEASE_BUILD_DATE`：默认取当前 UTC 时间
- `RELEASE_IMAGE_REF`：覆盖本次正式版演练构建出的镜像引用
- `RELEASE_IMAGE_PUSH=0|1`：是否在门禁通过后推送主镜像标签；本地推送前需要先 `docker login ghcr.io`
- `RELEASE_IMAGE_ADDITIONAL_TAGS`：为同一镜像额外推送的空格分隔标签列表；GitHub Actions 默认追加 `latest`
- `RELEASE_ASSET_DIR`：覆盖发布资产输出目录；默认写入 `dist/release/<version>`
- `RELEASE_GA_INCLUDE_RC_PREFLIGHT=0|1`：是否先跑完整 `release-rc-preflight`
- `RELEASE_GA_INCLUDE_ASSET_BUNDLE=0|1`：是否生成发布说明、bundle、manifest 和校验和
- `PRESERVE_STACK_ON_FAILURE=1`：失败时保留现场

## 发布结论

只有当下面几项同时满足时，当前稳定版才应被视为可正式发版：

- [`release-0.1-rc-runbook.md`](release-0.1-rc-runbook.md) 的全部结论已满足
- `release-ga-ship` 已通过
- `dist/release/<version>` 内的发布说明、镜像元数据、bundle 和 `SHA256SUMS` 已产出且互相一致

分支、标签和镜像规则见 [`release-policy.md`](release-policy.md)。
