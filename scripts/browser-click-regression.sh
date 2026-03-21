#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

APP_HOST_PORT="${APP_HOST_PORT:-8080}"
SMOKE_TIMEOUT_SECONDS="${SMOKE_TIMEOUT_SECONDS:-240}"
SMOKE_POLL_INTERVAL_SECONDS="${SMOKE_POLL_INTERVAL_SECONDS:-2}"
COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME:-avenrixa-browser-regression}"
COMPOSE_VARIANT="${COMPOSE_VARIANT:-mysql}"
PRESERVE_STACK_ON_FAILURE="${PRESERVE_STACK_ON_FAILURE:-0}"
BROWSER_BASE_URL="${BROWSER_BASE_URL:-http://127.0.0.1:${APP_HOST_PORT}}"
BROWSER_PHASE_TIMEOUT_MS="${BROWSER_PHASE_TIMEOUT_MS:-45000}"
MYSQL_DATA_DIR="${MYSQL_DATA_DIR:-}"
BROWSER_TMP_PARENT_DIR="${BROWSER_TMP_PARENT_DIR:-${TMPDIR:-${ROOT_DIR}/tmp}}"

source "${ROOT_DIR}/scripts/compose-runtime.sh"

configured_container_names() {
  local container_name
  local container_id

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

expected_cache_component_status() {
  case "${CACHE_MODE}" in
    redis8|dragonfly)
      printf 'healthy'
      ;;
    none)
      printf 'disabled'
      ;;
  esac
}

assert_cache_health_component() {
  local health_payload
  local expected_cache_status
  local actual_cache_status
  local overall_status

  health_payload="$(curl -fsS "${health_url}")"
  expected_cache_status="$(expected_cache_component_status)"
  actual_cache_status="$(printf '%s' "${health_payload}" | jq -r '.cache.status')"
  overall_status="$(printf '%s' "${health_payload}" | jq -r '.status')"

  if [[ "${overall_status}" != "healthy" ]]; then
    echo "Unexpected overall health status: ${overall_status}" >&2
    exit 1
  fi
  if [[ "${actual_cache_status}" != "${expected_cache_status}" ]]; then
    echo "Unexpected cache health status: expected ${expected_cache_status}, got ${actual_cache_status}" >&2
    exit 1
  fi
}

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

escape_sql_string() {
  printf "%s" "$1" | sed "s/'/''/g"
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

start_stack() {
  compose up -d --remove-orphans
}

trap on_error ERR
trap cleanup EXIT

prepare_workspace() {
  mkdir -p "${BROWSER_TMP_PARENT_DIR}"
  TMP_ROOT="$(mktemp -d "${BROWSER_TMP_PARENT_DIR%/}/vansour-browser-regression-XXXXXX")"
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
  compose_variant_uses_mysql
}

uses_mariadb_compose_file() {
  compose_variant_uses_mariadb
}

default_mysql_database_url() {
reset_mysql_data_dir_if_needed() {
  if [[ "${MYSQL_SMOKE_RESET_DATA_DIR}" != "1" ]]; then
    return 0
  fi

  if ! uses_mysql_compose_file; then
    return 0
  fi
  if BROWSER_EXECUTABLE_PATH="$(detect_browser_executable)"; then
    export BROWSER_EXECUTABLE_PATH
    return 0
  fi

  log_step "Installing Playwright Chromium"
  (
    cd scripts/browser-regression
      ;;
    none)
      printf '未配置 REDIS_URL'
      ;;
    *)
      printf '******'
      ;;
  esac
}

run_browser_phase() {
  local phase="$1"
  shift || true

  env \
    ADMIN_EMAIL="${ADMIN_EMAIL}" \
    ADMIN_PASSWORD="${ADMIN_PASSWORD}" \
    ADMIN_NEW_PASSWORD="${ADMIN_NEW_PASSWORD}" \
    SITE_NAME="${SITE_NAME}" \
    MYSQL_DATABASE_URL="${MYSQL_DATABASE_URL}" \
    BROWSER_BASE_URL="${BROWSER_BASE_URL}" \
    BROWSER_PHASE_TIMEOUT_MS="${BROWSER_PHASE_TIMEOUT_MS}" \
    BROWSER_REGRESSION_ARTIFACT_DIR="${BROWSER_REGRESSION_ARTIFACT_DIR}" \
    BROWSER_STORAGE_STATE_PATH="${BROWSER_STORAGE_STATE_PATH}" \
    BROWSER_EXPECT_DATABASE_CONNECTION="$(expected_browser_database_connection)" \
    BROWSER_EXPECT_CACHE_CONNECTION="$(expected_browser_cache_connection)" \
    "$@" \
    node scripts/browser-regression/run.mjs "${phase}"
}

seed_second_admin_for_demotion_regression() {
  local second_admin_uuid
  local escaped_email
  local database_cli

  second_admin_uuid="$(cat /proc/sys/kernel/random/uuid)"
  escaped_email="$(escape_sql_string "${SECOND_ADMIN_EMAIL}")"
  if compose_variant_uses_mariadb; then
    database_cli="mariadb"
  else
    database_cli="mysql"
  fi

  compose exec -T mysql "${database_cli}" -uuser -ppass image -e "
INSERT INTO users (id, email, email_verified_at, password_hash, role, created_at)
VALUES (UNHEX(REPLACE('${second_admin_uuid}', '-', '')), '${escaped_email}', UTC_TIMESTAMP(6), 'browser-regression-placeholder-hash', 'admin', UTC_TIMESTAMP(6));
"
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

echo "Using compose variant: ${COMPOSE_VARIANT}"
echo "Cache mode: ${CACHE_MODE}"
echo "Browser base URL: ${BROWSER_BASE_URL}"

install_browser_regression_deps
ensure_playwright_browser

log_step "Building browser regression stack"
compose down -v --remove-orphans >/dev/null 2>&1 || true
remove_container_name_conflicts
compose build

log_step "Starting browser regression stack"
start_stack
wait_for_url "${health_url}" "${SMOKE_TIMEOUT_SECONDS}"
assert_cache_health_component

log_step "Browser phase 1: check preconfigured database or run bootstrap fallback"
phase_one_output="$(run_browser_phase "bootstrap-mysql")"
phase_one_skipped="$(printf '%s' "${phase_one_output}" | jq -r '.skipped // false')"
log_step "Seeding second admin for demotion regression"
seed_second_admin_for_demotion_regression

log_step "Browser phase 4: verify auth semantics on settings and login page"
run_browser_phase "auth-semantics" >/dev/null

echo "Browser click regression passed"
