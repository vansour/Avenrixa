# Contributing

`Avenrixa` 仓库采用 `main` 单长期分支模型，日常开发和正式发布都围绕 `main` 展开。

## 分支模型

- `main`：唯一长期分支，所有 RC tag 与 GA tag 都从这里打出。
- `feature/*`：普通功能分支，从 `main` 拉出，完成后通过 PR 回到 `main`。
- `hotfix/*`：正式版紧急修复分支，从 `main` 拉出，完成后通过 PR 回到 `main`。

## 日常开发

1. 从最新 `main` 创建 `feature/<topic>`。
2. 本地完成开发、测试和必要文档更新。
3. 提交 PR 到 `main`，等待 CI 通过和评审完成。
4. 合并后继续在 `main` 上推进后续候选版或正式版标签。

## 发布流程

1. 在 `main` 上冻结版本、更新 `CHANGELOG.md` 并补齐候选版说明。
2. 打 RC tag，例如 `v0.1.2-rc.1`，由 `release-rc-preflight` 执行 RC 预检、发布 `:<version>` 与 `:rc`，并自动创建 GitHub Pre-release。
3. RC 通过后，继续在 `main` 上完成正式版 cutover。
4. 打正式 tag，例如 `v0.1.2`，由 `release-ga-ship` 发布 `:<version>` 与 `:latest`，并自动创建 GitHub Release 与上传附件。

## PR 要求

- `main` 一律通过 PR 合并，不直接推送。
- PR 必须通过 GitHub Actions CI。
- 影响发布、镜像、运维脚本或文档的改动，必须同步更新对应 runbook 或策略文档。
- 涉及数据库 schema、存储行为、备份恢复链路的改动，必须补回归测试或说明测试缺口。

## 合并建议

- `feature/* -> main`：优先 squash merge，保持主线历史清晰。
- `hotfix/* -> main`：优先 merge commit 或 squash merge，根据修复上下文选择。

## 版本与镜像

- RC：候选版本，使用 `x.y.z-rc.n`。
- `main`：正式版本，使用 `x.y.z`。
- GHCR 默认仓库：`ghcr.io/<owner>/avenrixa`

完整规则见 [`docs/release-policy.md`](docs/release-policy.md)。
