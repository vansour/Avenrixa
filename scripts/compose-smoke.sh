#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

APP_HOST_PORT="${APP_HOST_PORT:-8080}"
SMOKE_TIMEOUT_SECONDS="${SMOKE_TIMEOUT_SECONDS:-180}"
SMOKE_POLL_INTERVAL_SECONDS="${SMOKE_POLL_INTERVAL_SECONDS:-2}"
COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME:-vansour-image-smoke}"

compose_args=()
if [[ -n "${COMPOSE_FILE_PATHS:-}" ]]; then
  read -r -a compose_files <<< "${COMPOSE_FILE_PATHS}"
else
  compose_files=("compose.yml")
fi

for compose_file in "${compose_files[@]}"; do
  compose_args+=("-f" "${compose_file}")
done

compose() {
  docker compose -p "${COMPOSE_PROJECT_NAME}" "${compose_args[@]}" "$@"
}

cleanup() {
  compose down -v --remove-orphans >/dev/null 2>&1 || true
}

trap cleanup EXIT

health_url="http://127.0.0.1:${APP_HOST_PORT}/health"

echo "Using compose files: ${compose_files[*]}"
echo "Smoke health URL: ${health_url}"

compose build
compose up -d --remove-orphans

deadline=$((SECONDS + SMOKE_TIMEOUT_SECONDS))
while (( SECONDS < deadline )); do
  if curl -fsS "${health_url}" >/dev/null; then
    echo "Compose smoke check passed"
    exit 0
  fi
  sleep "${SMOKE_POLL_INTERVAL_SECONDS}"
done

echo "Compose smoke check failed: ${health_url} not ready within ${SMOKE_TIMEOUT_SECONDS}s" >&2
compose ps >&2 || true
compose logs --no-color --tail=200 >&2 || true
exit 1
