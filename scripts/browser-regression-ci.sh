#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

SCENARIO="${1:-}"
if [[ -z "${SCENARIO}" ]]; then
  echo "Usage: $0 <runtime-mainline|bootstrap-postgres>" >&2
  exit 1
fi

source "${ROOT_DIR}/scripts/compose-runtime.sh"

ADMIN_EMAIL="${ADMIN_EMAIL:-admin@example.com}"
ADMIN_PASSWORD="${ADMIN_PASSWORD:-Password123456!}"
ADMIN_NEW_PASSWORD="${ADMIN_NEW_PASSWORD:-Password123456!updated}"
SITE_NAME="${SITE_NAME:-Browser Regression}"
BROWSER_HEADLESS="${BROWSER_HEADLESS:-1}"
BROWSER_PHASE_TIMEOUT_MS="${BROWSER_PHASE_TIMEOUT_MS:-45000}"
BROWSER_REGRESSION_ARTIFACT_DIR="${BROWSER_REGRESSION_ARTIFACT_DIR:-${ROOT_DIR}/tmp/browser-regression-artifacts}"

log_step() {
  echo
  echo "==> $1"
}

require_commands() {
  local required_commands=(bash docker jq curl npm node)
  local command

  for command in "${required_commands[@]}"; do
    if ! command -v "${command}" >/dev/null 2>&1; then
      echo "Missing required command: ${command}" >&2
      exit 1
    fi
  done
}

remove_container_name_conflicts() {
  local container_name
  local container_id

  while IFS= read -r container_name; do
    [[ -n "${container_name}" ]] || continue
    container_id="$(docker ps -aq -f "name=^/${container_name}$")"
    if [[ -n "${container_id}" ]]; then
      docker rm -f "${container_name}" >/dev/null
    fi
  done < <(compose config 2>/dev/null | sed -n 's/^[[:space:]]*container_name:[[:space:]]*//p')
}

wait_for_url() {
  local url="$1"
  local timeout_seconds="${2:-180}"
  local deadline=$((SECONDS + timeout_seconds))

  while (( SECONDS < deadline )); do
    if curl -fsS "${url}" >/dev/null 2>&1; then
      return 0
    fi
    sleep 2
  done

  echo "Timed out waiting for ${url}" >&2
  return 1
}

run_browser_phase() {
  local phase="$1"
  shift

  (
    cd "${ROOT_DIR}/scripts/browser-regression"
    env "$@" node ./run.mjs "${phase}"
  )
}

prepare_browser_tooling() {
  log_step "Installing browser regression dependencies"
  npm ci --prefix scripts/browser-regression

  log_step "Installing Chromium for Playwright"
  (
    cd scripts/browser-regression
    ./node_modules/.bin/playwright install --with-deps chromium
  )
}

start_stack() {
  local project_name="$1"
  local app_port="$2"
  local data_dir="$3"
  local database_kind="$4"
  local database_url="$5"

  export COMPOSE_PROJECT_NAME="${project_name}"
  export APP_HOST_PORT="${app_port}"
  export DATA_DIR="${data_dir}"
  export APP_DATABASE_KIND="${database_kind}"
  export APP_DATABASE_URL="${database_url}"
  export APP_CACHE_URL="${APP_CACHE_URL-${CACHE_URL:-dragonfly://cache:6379}}"
  export JWT_SECRET="${JWT_SECRET-your-secret-key-change-in-production}"
  export AUTH_COOKIE_SECURE="${AUTH_COOKIE_SECURE-false}"
  export AUTH_COOKIE_SAME_SITE="${AUTH_COOKIE_SAME_SITE-Strict}"

  compose down -v --remove-orphans >/dev/null 2>&1 || true
  remove_container_name_conflicts
  compose_reset_host_dir "${DATA_DIR}"

  log_step "Starting compose stack (${project_name})"
  compose up -d --build
  wait_for_url "http://127.0.0.1:${APP_HOST_PORT}/health"
}

cleanup_stack() {
  compose down -v --remove-orphans >/dev/null 2>&1 || true
}

run_runtime_mainline() {
  local storage_state_path backup_phase_json backup_filename

  start_stack \
    "avenrixa-browser-runtime" \
    "18080" \
    "${ROOT_DIR}/tmp/browser-runtime-data" \
    "postgresql" \
    "$(compose_variant_default_database_url)"

  storage_state_path="${ROOT_DIR}/tmp/browser-regression/runtime-storage-state.json"

  log_step "Running browser install-and-backup phase"
  backup_phase_json="$(
    run_browser_phase \
      install-and-backup \
      ADMIN_EMAIL="${ADMIN_EMAIL}" \
      ADMIN_PASSWORD="${ADMIN_PASSWORD}" \
      SITE_NAME="${SITE_NAME}" \
      BROWSER_BASE_URL="http://127.0.0.1:18080" \
      BROWSER_STORAGE_STATE_PATH="${storage_state_path}" \
      BROWSER_EXPECT_DATABASE_CONNECTION="postgresql://******" \
      BROWSER_HEADLESS="${BROWSER_HEADLESS}" \
      BROWSER_PHASE_TIMEOUT_MS="${BROWSER_PHASE_TIMEOUT_MS}" \
      BROWSER_REGRESSION_ARTIFACT_DIR="${BROWSER_REGRESSION_ARTIFACT_DIR}"
  )"

  backup_filename="$(printf '%s' "${backup_phase_json}" | jq -r '.backupFilename')"
  if [[ -z "${backup_filename}" || "${backup_filename}" == "null" ]]; then
    echo "Browser install-and-backup phase did not return a backup filename" >&2
    exit 1
  fi

  log_step "Running browser verify-backup-audit phase"
  run_browser_phase \
    verify-backup-audit \
    ADMIN_EMAIL="${ADMIN_EMAIL}" \
    ADMIN_PASSWORD="${ADMIN_PASSWORD}" \
    BROWSER_BASE_URL="http://127.0.0.1:18080" \
    BROWSER_STORAGE_STATE_PATH="${storage_state_path}" \
    BROWSER_BACKUP_FILENAME="${backup_filename}" \
    BROWSER_HEADLESS="${BROWSER_HEADLESS}" \
    BROWSER_PHASE_TIMEOUT_MS="${BROWSER_PHASE_TIMEOUT_MS}" \
    BROWSER_REGRESSION_ARTIFACT_DIR="${BROWSER_REGRESSION_ARTIFACT_DIR}" \
    >/dev/null

  log_step "Running browser auth-semantics phase"
  run_browser_phase \
    auth-semantics \
    ADMIN_EMAIL="${ADMIN_EMAIL}" \
    ADMIN_PASSWORD="${ADMIN_PASSWORD}" \
    ADMIN_NEW_PASSWORD="${ADMIN_NEW_PASSWORD}" \
    BROWSER_BASE_URL="http://127.0.0.1:18080" \
    BROWSER_STORAGE_STATE_PATH="${storage_state_path}" \
    BROWSER_HEADLESS="${BROWSER_HEADLESS}" \
    BROWSER_PHASE_TIMEOUT_MS="${BROWSER_PHASE_TIMEOUT_MS}" \
    BROWSER_REGRESSION_ARTIFACT_DIR="${BROWSER_REGRESSION_ARTIFACT_DIR}" \
    >/dev/null
}

run_bootstrap_postgres() {
  start_stack \
    "avenrixa-browser-bootstrap" \
    "18081" \
    "${ROOT_DIR}/tmp/browser-bootstrap-data" \
    "" \
    ""

  log_step "Running browser bootstrap-postgres phase"
  run_browser_phase \
    bootstrap-postgres \
    ADMIN_EMAIL="${ADMIN_EMAIL}" \
    ADMIN_PASSWORD="${ADMIN_PASSWORD}" \
    BROWSER_DATABASE_URL="$(compose_variant_default_database_url)" \
    BROWSER_BASE_URL="http://127.0.0.1:18081" \
    BROWSER_HEADLESS="${BROWSER_HEADLESS}" \
    BROWSER_PHASE_TIMEOUT_MS="${BROWSER_PHASE_TIMEOUT_MS}" \
    BROWSER_REGRESSION_ARTIFACT_DIR="${BROWSER_REGRESSION_ARTIFACT_DIR}" \
    >/dev/null
}

main() {
  require_commands
  prepare_browser_tooling

  trap cleanup_stack EXIT

  case "${SCENARIO}" in
    runtime-mainline)
      run_runtime_mainline
      ;;
    bootstrap-postgres)
      run_bootstrap_postgres
      ;;
    *)
      echo "Unsupported browser regression scenario: ${SCENARIO}" >&2
      exit 1
      ;;
  esac
}

main "$@"
