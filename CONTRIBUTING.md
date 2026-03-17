# Contributing

`Avenrixa` 仓库采用固定分支模型，不再直接把所有发布动作堆在单一 `dev` 分支上。

## 分支模型

- `main`：正式版分支，只接受已经完成 RC 验证的变更。
- `dev`：日常集成分支，所有功能先合入这里，自动产出预览镜像。
- `release/*`：发布冻结分支，例如 `release/0.1.2`，用于 RC 收口、修复和签出正式 tag。
- `feature/*`：普通功能分支，从 `dev` 拉出，完成后回合 `dev`。
- `hotfix/*`：正式版紧急修复分支，从 `main` 拉出，完成后同时回合 `main` 和 `dev`。

## 日常开发

1. 从最新 `dev` 创建 `feature/<topic>`。
2. 本地完成开发、测试和必要文档更新。
3. 提交 PR 到 `dev`，等待 CI 通过和评审完成。
4. 合并后由 `preview-dev` 自动发布 `:dev` 和 `:sha-<shortsha>` 预览镜像。

## 发布流程

1. 从 `dev` 切出 `release/<version>`。
2. 在 `release/<version>` 上只接受候选版修复、版本冻结和发布文档更新。
3. 打 RC tag，例如 `v0.1.2-rc.1`，由 `release-rc-preflight` 执行 RC 预检并发布 `:<version>` 与 `:rc`。
4. RC 通过后，把 `release/<version>` 合入 `main`。
5. 在 `main` 上打正式 tag，例如 `v0.1.2`，由 `release-ga-ship` 发布 `:<version>` 与 `:latest`。
6. 正式版完成后，把 `main` 回合到 `dev`，再开始下一个开发周期。

## PR 要求

- `main`、`dev`、`release/*` 一律通过 PR 合并，不直接推送。
- PR 必须通过 GitHub Actions CI。
- 影响发布、镜像、运维脚本或文档的改动，必须同步更新对应 runbook 或策略文档。
- 涉及数据库 schema、存储行为、备份恢复链路的改动，必须补回归测试或说明测试缺口。

## 合并建议

- `feature/* -> dev`：优先 squash merge，保持集成历史清晰。
- `release/* -> main`：优先 merge commit，保留完整 RC 收口上下文。
- `main -> dev`：优先 merge commit，避免丢失正式版修复记录。

## 版本与镜像

- `dev`：开发版本，建议使用 `x.y.z-dev.n`。
- `release/*`：候选版本，使用 `x.y.z-rc.n`。
- `main`：正式版本，使用 `x.y.z`。
- GHCR 默认仓库：`ghcr.io/<owner>/avenrixa`

完整规则见 [`docs/release-policy.md`](docs/release-policy.md)。
