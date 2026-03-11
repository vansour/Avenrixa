#!/usr/bin/env bash

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME:-vansour-image-mysql-ops}"
COMPOSE_FILE_PATHS="${COMPOSE_FILE_PATHS:-compose.mysql.ops.yml}"
MYSQL_SERVICE="${MYSQL_SERVICE:-mysql}"
APP_SERVICE="${APP_SERVICE:-app}"
APP_HOST_PORT="${APP_HOST_PORT:-8080}"
APP_HEALTH_URL="${APP_HEALTH_URL:-http://127.0.0.1:${APP_HOST_PORT}/health}"

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

default_data_dir() {
  if uses_mariadb_compose_file; then
    printf 'data-mariadb'
  else
    printf 'data-mysql'
  fi
}

default_artifact_dir() {
  if uses_mariadb_compose_file; then
    printf 'ops-backups/mariadb'
  else
    printf 'ops-backups/mysql'
  fi
}

DATA_DIR="${DATA_DIR:-$(default_data_dir)}"
ARTIFACT_DIR="${ARTIFACT_DIR:-$(default_artifact_dir)}"
MYSQL_LAST_RESTORE_RESULT_PATH="${MYSQL_LAST_RESTORE_RESULT_PATH:-${DATA_DIR}/backup/mysql_last_restore_result.json}"

compose() {
  docker compose -p "${COMPOSE_PROJECT_NAME}" "${compose_args[@]}" "$@"
}

log_step() {
  echo
  echo "==> $1"
}

require_commands() {
  local required_commands=(docker curl jq tar sha256sum date mktemp)

  for command in "${required_commands[@]}"; do
    if ! command -v "${command}" >/dev/null 2>&1; then
      echo "Missing required command: ${command}" >&2
      exit 1
    fi
  done
}

ensure_json_file() {
  local path="$1"
  local label="$2"

  if [[ ! -f "${path}" ]]; then
    echo "${label} does not exist: ${path}" >&2
    exit 1
  fi
  if ! jq empty "${path}" >/dev/null 2>&1; then
    echo "${label} is not valid JSON: ${path}" >&2
    exit 1
  fi
}

ensure_stack_service_running() {
  local service="$1"

  if ! compose ps --status running --services | grep -qx "${service}"; then
    echo "Required compose service is not running: ${service}" >&2
    exit 1
  fi
}

ensure_file_nonempty() {
  local path="$1"
  local label="$2"

  if [[ ! -f "${path}" ]]; then
    echo "${label} does not exist: ${path}" >&2
    exit 1
  fi
  if [[ ! -s "${path}" ]]; then
    echo "${label} is empty: ${path}" >&2
    exit 1
  fi
}

wait_for_app_health() {
  local timeout_seconds="${1:-120}"
  local deadline=$((SECONDS + timeout_seconds))

  while (( SECONDS < deadline )); do
    if curl -fsS "${APP_HEALTH_URL}" >/dev/null 2>&1; then
      return 0
    fi
    sleep 2
  done

  echo "Timed out waiting for app health: ${APP_HEALTH_URL}" >&2
  return 1
}

current_timestamp_utc() {
  date -u +"%Y-%m-%dT%H:%M:%SZ"
}

current_stamp_compact() {
  date -u +"%Y%m%dT%H%M%SZ"
}

ensure_mysql_identifier_safe() {
  local identifier="$1"
  local label="$2"

  if [[ -z "${identifier}" || "${identifier}" =~ [^A-Za-z0-9_] ]]; then
    echo "${label} contains unsupported characters: ${identifier}" >&2
    exit 1
  fi
}

mysql_env_value() {
  local primary_name="$1"
  local fallback_name="$2"

  compose exec -T "${MYSQL_SERVICE}" sh -lc "
set -eu
primary=\${${primary_name}:-}
fallback=\${${fallback_name}:-}
if [ -n \"\${primary}\" ]; then
  printf '%s' \"\${primary}\"
else
  printf '%s' \"\${fallback}\"
fi
"
}

mysql_dump_to_file() {
  local output_path="$1"

  ensure_stack_service_running "${MYSQL_SERVICE}"
  mkdir -p "$(dirname "${output_path}")"

  compose exec -T "${MYSQL_SERVICE}" sh -lc '
set -eu
dump_bin="$(command -v mysqldump || command -v mariadb-dump || true)"
if [ -z "${dump_bin}" ]; then
  echo "Neither mysqldump nor mariadb-dump is available" >&2
  exit 1
fi
user="${MYSQL_USER:-root}"
if [ -z "${user}" ]; then
  user="${MARIADB_USER:-root}"
fi
password="${MYSQL_PASSWORD:-${MARIADB_PASSWORD:-${MYSQL_ROOT_PASSWORD:-${MARIADB_ROOT_PASSWORD:-}}}}"
database="${MYSQL_DATABASE:-${MARIADB_DATABASE:-}}"
if [ -z "${database}" ]; then
  echo "MYSQL_DATABASE or MARIADB_DATABASE is required" >&2
  exit 1
fi
export MYSQL_PWD="${password}"
exec "${dump_bin}" \
  --protocol=TCP \
  --skip-ssl \
  -h127.0.0.1 \
  -u"${user}" \
  --single-transaction \
  --skip-lock-tables \
  --no-tablespaces \
  --default-character-set=utf8mb4 \
  --routines \
  --triggers \
  --events \
  "${database}"
' > "${output_path}"

  ensure_file_nonempty "${output_path}" "MySQL/MariaDB dump"
}

mysql_restore_from_file() {
  local input_path="$1"

  ensure_file_nonempty "${input_path}" "MySQL/MariaDB restore input"
  ensure_stack_service_running "${MYSQL_SERVICE}"

  local database_name
  database_name="$(mysql_env_value "MYSQL_DATABASE" "MARIADB_DATABASE")"
  ensure_mysql_identifier_safe "${database_name}" "MYSQL_DATABASE/MARIADB_DATABASE"

  compose exec -T "${MYSQL_SERVICE}" sh -lc "
set -eu
client_bin=\"\$(command -v mysql || command -v mariadb || true)\"
if [ -z \"\${client_bin}\" ]; then
  echo 'Neither mysql nor mariadb client is available' >&2
  exit 1
fi
root_password=\"\${MYSQL_ROOT_PASSWORD:-\${MARIADB_ROOT_PASSWORD:-}}\"
if [ -z \"\${root_password}\" ]; then
  echo 'MYSQL_ROOT_PASSWORD or MARIADB_ROOT_PASSWORD is required for restore' >&2
  exit 1
fi
export MYSQL_PWD=\"\${root_password}\"
\"\${client_bin}\" --protocol=TCP --skip-ssl -h127.0.0.1 -uroot --batch --skip-column-names <<'SQL'
DROP DATABASE IF EXISTS \`${database_name}\`;
CREATE DATABASE \`${database_name}\` CHARACTER SET utf8mb4;
SQL
" >/dev/null

  compose exec -T "${MYSQL_SERVICE}" sh -lc '
set -eu
client_bin="$(command -v mysql || command -v mariadb || true)"
if [ -z "${client_bin}" ]; then
  echo "Neither mysql nor mariadb client is available" >&2
  exit 1
fi
database="${MYSQL_DATABASE:-${MARIADB_DATABASE:-}}"
if [ -z "${database}" ]; then
  echo "MYSQL_DATABASE or MARIADB_DATABASE is required for restore" >&2
  exit 1
fi
root_password="${MYSQL_ROOT_PASSWORD:-${MARIADB_ROOT_PASSWORD:-}}"
if [ -z "${root_password}" ]; then
  echo "MYSQL_ROOT_PASSWORD or MARIADB_ROOT_PASSWORD is required for restore" >&2
  exit 1
fi
export MYSQL_PWD="${root_password}"
exec "${client_bin}" --protocol=TCP --skip-ssl -h127.0.0.1 -uroot "${database}"
' < "${input_path}" >/dev/null
}

sha256_file() {
  local path="$1"
  sha256sum "${path}" | awk '{print $1}'
}

verify_file_sha256() {
  local path="$1"
  local expected="$2"
  local label="$3"

  ensure_file_nonempty "${path}" "${label}"
  if [[ -z "${expected}" || "${expected}" == "null" ]]; then
    echo "${label} missing expected sha256" >&2
    exit 1
  fi

  local actual
  actual="$(sha256_file "${path}")"
  if [[ "${actual}" != "${expected}" ]]; then
    echo "${label} sha256 mismatch: expected ${expected}, got ${actual}" >&2
    exit 1
  fi
}

write_json_file() {
  local path="$1"
  local payload="$2"

  mkdir -p "$(dirname "${path}")"
  printf '%s\n' "${payload}" > "${path}"
}
