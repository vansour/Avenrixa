#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME:-vansour-image-mysql-ops-drill}"
COMPOSE_FILE_PATHS="${COMPOSE_FILE_PATHS:-compose.mysql.yml}"
APP_HOST_PORT="${APP_HOST_PORT:-8080}"
PRESERVE_STACK_ON_FAILURE="${PRESERVE_STACK_ON_FAILURE:-0}"
DRILL_ADMIN_EMAIL="${DRILL_ADMIN_EMAIL:-admin@example.com}"
DRILL_ADMIN_PASSWORD="${DRILL_ADMIN_PASSWORD:-Password123456!}"
DRILL_SITE_NAME_BASELINE="${DRILL_SITE_NAME_BASELINE:-MySQL/MariaDB Ops Drill Baseline}"
DRILL_SITE_NAME_MUTATED="${DRILL_SITE_NAME_MUTATED:-MySQL/MariaDB Ops Drill Mutated}"
DRILL_LINK_BASE_URL="${DRILL_LINK_BASE_URL:-http://127.0.0.1:${APP_HOST_PORT}/login}"
MYSQL_DATABASE_URL="${MYSQL_DATABASE_URL:-}"
DRILL_MARKER_PATH="${DRILL_MARKER_PATH:-}"
ARTIFACT_DIR="${ARTIFACT_DIR:-}"

compose_args=()
read -r -a compose_files <<< "${COMPOSE_FILE_PATHS}"
for compose_file in "${compose_files[@]}"; do
  compose_args+=("-f" "${compose_file}")
done

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

default_data_dir() {
  if uses_mariadb_compose_file; then
    printf 'data-mariadb'
  else
    printf 'data-mysql'
  fi
}

default_artifact_dir() {
  if uses_mariadb_compose_file; then
    printf 'ops-backups/mariadb-drill'
  else
    printf 'ops-backups/mysql-drill'
  fi
}

DATA_DIR="$(default_data_dir)"
if [[ -z "${MYSQL_DATABASE_URL}" ]]; then
  MYSQL_DATABASE_URL="$(default_mysql_database_url)"
fi
if [[ -z "${DRILL_MARKER_PATH}" ]]; then
  DRILL_MARKER_PATH="${DATA_DIR}/images/mysql-ops-drill-marker.txt"
fi
if [[ -z "${ARTIFACT_DIR}" ]]; then
  ARTIFACT_DIR="$(default_artifact_dir)"
fi

compose() {
  docker compose -p "${COMPOSE_PROJECT_NAME}" "${compose_args[@]}" "$@"
}

api_base="http://127.0.0.1:${APP_HOST_PORT}/api/v1"
health_url="http://127.0.0.1:${APP_HOST_PORT}/health"

SCRIPT_FAILED=0
TMP_ROOT=""
COOKIE_JAR=""

log_step() {
  echo
  echo "==> $1"
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

expect_eq() {
  local actual="$1"
  local expected="$2"
  local message="$3"

  if [[ "${actual}" != "${expected}" ]]; then
    echo "Assertion failed: ${message}. Expected '${expected}', got '${actual}'" >&2
    exit 1
  fi
}

require_commands() {
  local required_commands=(docker curl jq mktemp)
  for command in "${required_commands[@]}"; do
    if ! command -v "${command}" >/dev/null 2>&1; then
      echo "Missing required command: ${command}" >&2
      exit 1
    fi
  done
}

on_error() {
  SCRIPT_FAILED=1
  echo >&2
  echo "MySQL/MariaDB ops drill failed. Recent compose state:" >&2
  compose ps >&2 || true
  compose logs --no-color --tail=300 >&2 || true
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
    rm -rf "${DATA_DIR}" >/dev/null 2>&1 || true
    if [[ -n "${TMP_ROOT}" ]]; then
      rm -rf "${TMP_ROOT}"
    fi
  fi
}

trap on_error ERR
trap cleanup EXIT

prepare_fixture() {
  TMP_ROOT="$(mktemp -d /tmp/vansour-mysql-ops-drill-XXXXXX)"
  COOKIE_JAR="${TMP_ROOT}/admin.cookies.txt"
  rm -rf "${DATA_DIR}"
  mkdir -p "${ARTIFACT_DIR}"
}

configure_mysql_runtime() {
  local bootstrap_status
  local mode
  local configured
  local database_kind
  local runtime_error

  bootstrap_status="$(curl -fsS "${api_base}/bootstrap/status")"
  mode="$(printf '%s' "${bootstrap_status}" | jq -r '.mode')"
  configured="$(printf '%s' "${bootstrap_status}" | jq -r '.database_configured')"
  database_kind="$(printf '%s' "${bootstrap_status}" | jq -r '.database_kind')"
  runtime_error="$(printf '%s' "${bootstrap_status}" | jq -r '.runtime_error // empty')"

  if [[ "${mode}" == "bootstrap" ]]; then
    if [[ -n "${runtime_error}" ]]; then
      echo "Bootstrap runtime error: ${runtime_error}" >&2
      exit 1
    fi

    log_step "Writing MySQL/MariaDB bootstrap config"
    curl -fsS \
      -X PUT "${api_base}/bootstrap/database-config" \
      -H 'Content-Type: application/json' \
      -d "$(jq -n \
        --arg database_kind "mysql" \
        --arg database_url "${MYSQL_DATABASE_URL}" \
        '{database_kind: $database_kind, database_url: $database_url, database_max_connections: 5}')" \
      >/dev/null

    log_step "Restarting app to enter runtime mode"
    compose restart app >/dev/null
  else
    expect_eq "${mode}" "runtime" "MySQL/MariaDB drill should expose runtime bootstrap status"
    expect_eq "${database_kind}" "mysql" "MySQL/MariaDB drill should run with mysql database kind"
    expect_eq "${configured}" "true" "MySQL/MariaDB drill should run with configured database"
  fi

  wait_for_url "${api_base}/install/status" 180
}

install_mysql_app() {
  local install_status
  local install_payload
  local admin_me

  install_status="$(curl -fsS "${api_base}/install/status")"
  expect_eq "$(printf '%s' "${install_status}" | jq -r '.installed')" "false" "MySQL/MariaDB ops drill requires an uninstalled database"

  log_step "Completing installation wizard"
  install_payload="$(
    jq -n \
      --arg admin_email "${DRILL_ADMIN_EMAIL}" \
      --arg admin_password "${DRILL_ADMIN_PASSWORD}" \
      --arg site_name "${DRILL_SITE_NAME_BASELINE}" \
      --arg link_base_url "${DRILL_LINK_BASE_URL}" \
      '{
        admin_email: $admin_email,
        admin_password: $admin_password,
        favicon_data_url: null,
        config: {
          site_name: $site_name,
          storage_backend: "local",
          local_storage_path: "/data/images",
          mail_enabled: false,
          mail_smtp_host: "",
          mail_smtp_port: 1025,
          mail_smtp_user: null,
          mail_smtp_password: null,
          mail_from_email: "noreply@example.com",
          mail_from_name: "Vansour Image",
          mail_link_base_url: $link_base_url,
          s3_endpoint: null,
          s3_region: null,
          s3_bucket: null,
          s3_prefix: null,
          s3_access_key: null,
          s3_secret_key: null,
          s3_force_path_style: true
        }
      }'
  )"

  curl -fsS \
    -c "${COOKIE_JAR}" \
    -X POST "${api_base}/install/bootstrap" \
    -H 'Content-Type: application/json' \
    -d "${install_payload}" \
    >/dev/null

  admin_me="$(curl -fsS -b "${COOKIE_JAR}" "${api_base}/auth/me")"
  expect_eq "$(printf '%s' "${admin_me}" | jq -r '.email')" "${DRILL_ADMIN_EMAIL}" "admin session should be active after drill installation"
}

mysql_site_name() {
  docker compose -p "${COMPOSE_PROJECT_NAME}" "${compose_args[@]}" exec -T mysql sh -lc \
    '
set -eu
client_bin="$(command -v mysql || command -v mariadb || true)"
if [ -z "${client_bin}" ]; then
  echo "Neither mysql nor mariadb client is available" >&2
  exit 1
fi
user="${MYSQL_USER:-${MARIADB_USER:-}}"
password="${MYSQL_PASSWORD:-${MARIADB_PASSWORD:-}}"
database="${MYSQL_DATABASE:-${MARIADB_DATABASE:-}}"
export MYSQL_PWD="${password}"
exec "${client_bin}" --skip-ssl -h127.0.0.1 -u"${user}" "${database}" --batch --skip-column-names \
  -e "SELECT value FROM settings WHERE settings.\`key\` = '\''site_name'\'';"
'
}

set_mysql_site_name() {
  local next_value="$1"
  docker compose -p "${COMPOSE_PROJECT_NAME}" "${compose_args[@]}" exec -T mysql sh -lc \
    "
set -eu
client_bin=\"\$(command -v mysql || command -v mariadb || true)\"
if [ -z \"\${client_bin}\" ]; then
  echo 'Neither mysql nor mariadb client is available' >&2
  exit 1
fi
user=\"\${MYSQL_USER:-\${MARIADB_USER:-}}\"
password=\"\${MYSQL_PASSWORD:-\${MARIADB_PASSWORD:-}}\"
database=\"\${MYSQL_DATABASE:-\${MARIADB_DATABASE:-}}\"
export MYSQL_PWD=\"\${password}\"
exec \"\${client_bin}\" --skip-ssl -h127.0.0.1 -u\"\${user}\" \"\${database}\" --batch --skip-column-names \
  -e \"UPDATE settings SET value = '${next_value}' WHERE settings.\\\`key\\\` = 'site_name';\"
"
}

run_drill() {
  local manifest_path

  prepare_fixture

  log_step "Starting MySQL/MariaDB drill stack"
  compose up --build -d

  log_step "Waiting for app health"
  wait_for_url "${health_url}" 180

  configure_mysql_runtime
  install_mysql_app

  log_step "Writing baseline marker"
  mkdir -p "$(dirname "${DRILL_MARKER_PATH}")"
  printf 'baseline-marker\n' > "${DRILL_MARKER_PATH}"
  expect_eq "$(mysql_site_name)" "${DRILL_SITE_NAME_BASELINE}" "baseline site name should be installed value"

  log_step "Creating MySQL/MariaDB ops backup"
  COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME}" \
  COMPOSE_FILE_PATHS="${COMPOSE_FILE_PATHS}" \
  ARTIFACT_DIR="${ARTIFACT_DIR}" \
  ./scripts/mysql-ops-backup.sh

  manifest_path="${DATA_DIR}/backup/mysql_last_backup_manifest.json"
  if [[ ! -f "${manifest_path}" ]]; then
    echo "Drill manifest not found: ${manifest_path}" >&2
    exit 1
  fi

  log_step "Mutating database and local data"
  set_mysql_site_name "${DRILL_SITE_NAME_MUTATED}"
  printf 'mutated-marker\n' > "${DRILL_MARKER_PATH}"
  expect_eq "$(mysql_site_name)" "${DRILL_SITE_NAME_MUTATED}" "mutated site name should be visible before restore"
  expect_eq "$(cat "${DRILL_MARKER_PATH}")" "mutated-marker" "marker should be mutated before restore"

  log_step "Restoring from manifest"
  COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME}" \
  COMPOSE_FILE_PATHS="${COMPOSE_FILE_PATHS}" \
  ARTIFACT_DIR="${ARTIFACT_DIR}" \
  MYSQL_RESTORE_MANIFEST_PATH="${manifest_path}" \
  ./scripts/mysql-ops-restore.sh

  log_step "Validating restored state"
  expect_eq "$(mysql_site_name)" "${DRILL_SITE_NAME_BASELINE}" "site name should return to baseline after restore"
  expect_eq "$(cat "${DRILL_MARKER_PATH}")" "baseline-marker" "marker file should return to baseline after restore"
  expect_eq "$(jq -r '.status' "${DATA_DIR}/backup/mysql_last_restore_result.json")" "completed" "restore result should be completed"
  wait_for_url "${health_url}" 180

  echo "MySQL/MariaDB ops drill passed"
}

require_commands
run_drill
