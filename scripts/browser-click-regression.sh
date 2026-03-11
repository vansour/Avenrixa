#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

APP_HOST_PORT="${APP_HOST_PORT:-8080}"
SMOKE_TIMEOUT_SECONDS="${SMOKE_TIMEOUT_SECONDS:-240}"
SMOKE_POLL_INTERVAL_SECONDS="${SMOKE_POLL_INTERVAL_SECONDS:-2}"
COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME:-vansour-image-browser-regression}"
PRESERVE_STACK_ON_FAILURE="${PRESERVE_STACK_ON_FAILURE:-0}"
MYSQL_SMOKE_RESET_DATA_DIR="${MYSQL_SMOKE_RESET_DATA_DIR:-1}"

ADMIN_EMAIL="${ADMIN_EMAIL:-admin@example.com}"
ADMIN_PASSWORD="${ADMIN_PASSWORD:-Password123456!}"
SITE_NAME="${SITE_NAME:-MySQL/MariaDB Browser Regression}"
MYSQL_DATABASE_URL="${MYSQL_DATABASE_URL:-}"
BROWSER_BASE_URL="${BROWSER_BASE_URL:-http://127.0.0.1:${APP_HOST_PORT}}"
BROWSER_PHASE_TIMEOUT_MS="${BROWSER_PHASE_TIMEOUT_MS:-45000}"
MYSQL_DATA_DIR="${MYSQL_DATA_DIR:-}"

compose_args=()
if [[ -n "${COMPOSE_FILE_PATHS:-}" ]]; then
  read -r -a compose_files <<< "${COMPOSE_FILE_PATHS}"
else
  compose_files=("compose.mysql.yml")
fi

for compose_file in "${compose_files[@]}"; do
  compose_args+=("-f" "${compose_file}")
done

compose() {
  docker compose -p "${COMPOSE_PROJECT_NAME}" "${compose_args[@]}" "$@"
}

configured_container_names() {
  compose config 2>/dev/null | sed -n 's/^[[:space:]]*container_name:[[:space:]]*//p'
}

remove_container_name_conflicts() {
  local container_name
  local container_id

  while IFS= read -r container_name; do
    [[ -n "${container_name}" ]] || continue
    container_id="$(docker ps -aq -f "name=^/${container_name}$")"
    if [[ -n "${container_id}" ]]; then
      log_step "Removing conflicting container ${container_name}"
      docker rm -f "${container_name}" >/dev/null
    fi
  done < <(configured_container_names)
}

SCRIPT_FAILED=0
TMP_ROOT=""
BROWSER_REGRESSION_ARTIFACT_DIR=""
BROWSER_STORAGE_STATE_PATH=""

health_url="http://127.0.0.1:${APP_HOST_PORT}/health"

require_commands() {
  local required_commands=(docker curl jq node npm npx)
  for command in "${required_commands[@]}"; do
    if ! command -v "${command}" >/dev/null 2>&1; then
      echo "Missing required command: ${command}" >&2
      exit 1
    fi
  done
}

log_step() {
  echo
  echo "==> $1"
}

wait_for_url() {
  local url="$1"
  local timeout_seconds="${2:-${SMOKE_TIMEOUT_SECONDS}}"
  local deadline=$((SECONDS + timeout_seconds))

  while (( SECONDS < deadline )); do
    if curl -fsS "${url}" >/dev/null 2>&1; then
      return 0
    fi
    sleep "${SMOKE_POLL_INTERVAL_SECONDS}"
  done

  echo "Timed out waiting for ${url}" >&2
  return 1
}

on_error() {
  SCRIPT_FAILED=1
  echo >&2
  echo "Browser regression failed. Recent compose state:" >&2
  compose ps >&2 || true
  compose logs --no-color --tail=200 >&2 || true
  if [[ -n "${BROWSER_REGRESSION_ARTIFACT_DIR}" && -d "${BROWSER_REGRESSION_ARTIFACT_DIR}" ]]; then
    echo "Browser artifacts: ${BROWSER_REGRESSION_ARTIFACT_DIR}" >&2
  fi
}

cleanup() {
  if [[ "${SCRIPT_FAILED}" == "1" && "${PRESERVE_STACK_ON_FAILURE}" == "1" ]]; then
    echo "Preserving stack because PRESERVE_STACK_ON_FAILURE=1" >&2
    echo "Compose project: ${COMPOSE_PROJECT_NAME}" >&2
    if [[ -n "${TMP_ROOT}" ]]; then
      echo "Workspace tmp dir: ${TMP_ROOT}" >&2
    fi
  else
    compose down -v --remove-orphans >/dev/null 2>&1 || true
    if [[ -n "${TMP_ROOT}" ]]; then
      rm -rf "${TMP_ROOT}"
    fi
  fi
}

trap on_error ERR
trap cleanup EXIT

prepare_workspace() {
  TMP_ROOT="$(mktemp -d /tmp/vansour-browser-regression-XXXXXX)"
  BROWSER_REGRESSION_ARTIFACT_DIR="${TMP_ROOT}/artifacts"
  BROWSER_STORAGE_STATE_PATH="${TMP_ROOT}/browser-storage-state.json"
  mkdir -p "${BROWSER_REGRESSION_ARTIFACT_DIR}"
}

install_browser_regression_deps() {
  log_step "Installing browser regression dependencies"
  if [[ -f "scripts/browser-regression/package-lock.json" ]]; then
    npm ci --prefix scripts/browser-regression --no-fund --no-audit >/dev/null
  else
    npm install --prefix scripts/browser-regression --no-fund --no-audit >/dev/null
  fi
}

detect_browser_executable() {
  local candidate
  for candidate in google-chrome google-chrome-stable chromium chromium-browser; do
    if command -v "${candidate}" >/dev/null 2>&1; then
      command -v "${candidate}"
      return 0
    fi
  done
  return 1
}

uses_mysql_compose_file() {
  local compose_file
  for compose_file in "${compose_files[@]}"; do
    case "$(basename "${compose_file}")" in
      compose.mysql.yml|compose.mysql.ops.yml|compose.mariadb.yml|compose.mariadb.ops.yml)
      return 0
        ;;
    esac
  done

  return 1
}

uses_mariadb_compose_file() {
  local compose_file
  for compose_file in "${compose_files[@]}"; do
    case "$(basename "${compose_file}")" in
      compose.mariadb.yml|compose.mariadb.ops.yml)
      return 0
        ;;
    esac
  done

  return 1
}

default_mysql_database_url() {
  if uses_mariadb_compose_file; then
    printf 'mariadb://user:pass@mysql:3306/image'
  else
    printf 'mysql://user:pass@mysql:3306/image'
  fi
}

default_mysql_data_dir() {
  if uses_mariadb_compose_file; then
    printf '%s/data-mariadb' "${ROOT_DIR}"
  else
    printf '%s/data-mysql' "${ROOT_DIR}"
  fi
}

reset_mysql_data_dir_if_needed() {
  if [[ "${MYSQL_SMOKE_RESET_DATA_DIR}" != "1" ]]; then
    return 0
  fi

  if ! uses_mysql_compose_file; then
    return 0
  fi

  log_step "Resetting MySQL/MariaDB browser regression data directory"
  rm -rf "${MYSQL_DATA_DIR}"
  mkdir -p "${MYSQL_DATA_DIR}"
}

ensure_playwright_browser() {
  if BROWSER_EXECUTABLE_PATH="$(detect_browser_executable)"; then
    export BROWSER_EXECUTABLE_PATH
    return 0
  fi

  log_step "Installing Playwright Chromium"
  (
    cd scripts/browser-regression
    npx playwright install chromium >/dev/null
  )
}

run_browser_phase() {
  local phase="$1"
  shift || true

  env \
    ADMIN_EMAIL="${ADMIN_EMAIL}" \
    ADMIN_PASSWORD="${ADMIN_PASSWORD}" \
    SITE_NAME="${SITE_NAME}" \
    MYSQL_DATABASE_URL="${MYSQL_DATABASE_URL}" \
    BROWSER_BASE_URL="${BROWSER_BASE_URL}" \
    BROWSER_PHASE_TIMEOUT_MS="${BROWSER_PHASE_TIMEOUT_MS}" \
    BROWSER_REGRESSION_ARTIFACT_DIR="${BROWSER_REGRESSION_ARTIFACT_DIR}" \
    BROWSER_STORAGE_STATE_PATH="${BROWSER_STORAGE_STATE_PATH}" \
    "$@" \
    node scripts/browser-regression/run.mjs "${phase}"
}

require_commands
prepare_workspace

if [[ -z "${MYSQL_DATABASE_URL}" ]]; then
  MYSQL_DATABASE_URL="$(default_mysql_database_url)"
fi
if [[ -z "${MYSQL_DATA_DIR}" ]]; then
  MYSQL_DATA_DIR="$(default_mysql_data_dir)"
fi

reset_mysql_data_dir_if_needed

echo "Using compose files: ${compose_files[*]}"
echo "Browser base URL: ${BROWSER_BASE_URL}"

install_browser_regression_deps
ensure_playwright_browser

log_step "Building browser regression stack"
compose down -v --remove-orphans >/dev/null 2>&1 || true
remove_container_name_conflicts
compose build

log_step "Starting browser regression stack"
compose up -d --remove-orphans
wait_for_url "${health_url}" "${SMOKE_TIMEOUT_SECONDS}"

log_step "Browser phase 1: MySQL/MariaDB database bootstrap"
run_browser_phase "bootstrap-mysql" >/dev/null

log_step "Restarting app after database bootstrap"
compose restart app >/dev/null
wait_for_url "${health_url}" "${SMOKE_TIMEOUT_SECONDS}"

log_step "Browser phase 2: install wizard, first-run guide, maintenance restore plan"
phase_two_output="$(run_browser_phase "install-and-restore-plan")"
backup_filename="$(printf '%s' "${phase_two_output}" | jq -r '.backupFilename')"
if [[ -z "${backup_filename}" || "${backup_filename}" == "null" ]]; then
  echo "Browser regression did not return a backup filename" >&2
  exit 1
fi

log_step "Restarting app to execute pending restore"
compose restart app >/dev/null
wait_for_url "${health_url}" "${SMOKE_TIMEOUT_SECONDS}"

log_step "Browser phase 3: login again and verify restore audit/result"
run_browser_phase "verify-after-restore" BROWSER_BACKUP_FILENAME="${backup_filename}" >/dev/null

echo "Browser click regression passed"
