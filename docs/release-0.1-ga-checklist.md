# 0.1 GA 发布清单

本文档用于把 `0.1` 正式版的阻塞项收口成一个可执行的发布门禁，而不是继续分散在 README、CI 和专项演练脚本里。

按 [`release-0.1-scope.md`](release-0.1-scope.md) 当前定义，只有 `PostgreSQL + Redis 8 + 本地存储` 是 `0.1` 的默认 GA 推荐栈；这份清单只对这条主链路负责。

## 统一入口

本地执行：

```bash
./scripts/release-ga-gate.sh
```

GitHub Actions 手动触发：

- `.github/workflows/release-ga-gate.yml`

这两个入口执行的是同一组阻塞项。

## 阻塞项

`0.1` 正式发布前，以下项目必须全部通过：

- `cargo fmt --all --check`
- `cargo check --workspace`
- `cargo test --workspace`
- 默认 `PostgreSQL + Redis 8` Compose smoke：`./scripts/compose-smoke.sh`
- PostgreSQL 物理备份 / 恢复演练：`./scripts/postgres-ops-drill.sh`
- PostgreSQL PITR 演练（named restore point）：`./scripts/postgres-ops-pitr-drill.sh`
- PostgreSQL PITR 演练（time target）：`PITR_TARGET_MODE=time ./scripts/postgres-ops-pitr-drill.sh`

其中：

- PR / push 的常规 `CI` 继续承担 Rust checks 和默认 PostgreSQL smoke
- `release-ga-gate` 负责在发版前把完整 GA 运维主链路一次性串跑
- 周期性 `postgres-ops-drill` / `postgres-pitr-drill` workflow 继续保留，用于日常漂移监控

## 本地执行约定

默认命令会串行执行全部阻塞项：

```bash
./scripts/release-ga-gate.sh
```

常用可调参数：

- `RELEASE_GATE_COMPOSE_PROJECT_PREFIX`：统一修改这次 gate 运行时使用的 Compose project name 前缀
- `RELEASE_GATE_ARTIFACT_DIR`：统一修改 ops / PITR 演练产物目录
- `PRESERVE_STACK_ON_FAILURE=1`：失败时保留容器和现场，方便排查
- `RELEASE_GATE_PITR_TARGET_MODES="name time"`：覆盖要跑的 PITR 目标模式

如果只想局部复跑某个阻塞项，可以临时把不相关步骤关掉：

```bash
RELEASE_GATE_INCLUDE_RUST_CHECKS=0 \
RELEASE_GATE_INCLUDE_POSTGRES_OPS_DRILL=0 \
RELEASE_GATE_INCLUDE_POSTGRES_PITR_DRILL=0 \
./scripts/release-ga-gate.sh
```

这些开关只用于本地排障，不改变正式发布所要求的完整门禁。

## 发布结论

只有在以下条件同时满足时，`0.1` 才应被视为可发布：

- [`release-0.1-scope.md`](release-0.1-scope.md) 的 `GA / Beta / Experimental` 口径没有被新改动打破
- `release-ga-gate` 本地或 GitHub Actions 最近一次完整执行通过
- 本次发版没有引入新的 PostgreSQL 主链路 release blocker

如果你已经进入候选版冻结阶段，还应继续执行 [`release-0.1-rc-runbook.md`](release-0.1-rc-runbook.md) 里的 `release-rc-preflight`。

如果你已经准备把候选版提升为正式版，还应继续执行 [`release-0.1-ga-runbook.md`](release-0.1-ga-runbook.md) 里的 `release-ga-ship`。
