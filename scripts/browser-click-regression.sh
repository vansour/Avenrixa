#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

echo "scripts/browser-click-regression.sh 已停用。" >&2
echo "当前仓库的主回归入口是 ./scripts/compose-smoke.sh（PostgreSQL + Dragonfly）。" >&2
exit 1
