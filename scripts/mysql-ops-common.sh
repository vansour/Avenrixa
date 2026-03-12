#!/usr/bin/env bash

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME:-vansour-image-mysql-ops}"
COMPOSE_VARIANT="${COMPOSE_VARIANT:-mysql-ops}"
MYSQL_SERVICE="${MYSQL_SERVICE:-mysql}"
APP_SERVICE="${APP_SERVICE:-app}"
APP_HOST_PORT="${APP_HOST_PORT:-8080}"
APP_HEALTH_URL="${APP_HEALTH_URL:-http://127.0.0.1:${APP_HOST_PORT}/health}"

source "${ROOT_DIR}/scripts/compose-runtime.sh"

uses_mariadb_compose_file() {
  compose_variant_uses_mariadb
}

default_data_dir() {
  case "${COMPOSE_VARIANT}" in
    mariadb|mariadb-ops)
      printf 'data-mariadb'
      ;;
    *)
      printf 'data-mysql'
      ;;
  esac
}

default_artifact_dir() {
  case "${COMPOSE_VARIANT}" in
    mariadb|mariadb-ops)
      printf 'ops-backups/mariadb'
      ;;
    *)
      printf 'ops-backups/mysql'
      ;;
  esac
}

DATA_DIR="${DATA_DIR:-$(default_data_dir)}"
ARTIFACT_DIR="${ARTIFACT_DIR:-$(default_artifact_dir)}"
MYSQL_LAST_RESTORE_RESULT_PATH="${MYSQL_LAST_RESTORE_RESULT_PATH:-${DATA_DIR}/backup/mysql_last_restore_result.json}"

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

absolute_existing_dir() {
  local path="$1"

  if [[ ! -d "${path}" ]]; then
    echo "Directory does not exist: ${path}" >&2
    exit 1
  fi

  (
    cd "${path}"
    pwd -P
  )
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

ensure_directory_nonempty() {
  local path="$1"
  local label="$2"

  if [[ ! -d "${path}" ]]; then
    echo "${label} does not exist: ${path}" >&2
    exit 1
  fi
  if ! find "${path}" -mindepth 1 -print -quit | grep -q .; then
    echo "${label} is empty: ${path}" >&2
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

compose_service_container_id() {
  local service="$1"
  compose ps -a -q "${service}" | head -n 1
}

mysql_service_container_id() {
  compose_service_container_id "${MYSQL_SERVICE}"
}

mysql_service_container_image() {
  local container_id
  container_id="$(mysql_service_container_id)"

  if [[ -z "${container_id}" ]]; then
    echo "Unable to resolve running container id for service ${MYSQL_SERVICE}" >&2
    exit 1
  fi

  docker inspect -f '{{.Config.Image}}' "${container_id}"
}

mysql_root_password() {
  local password
  password="$(mysql_env_value "MYSQL_ROOT_PASSWORD" "MARIADB_ROOT_PASSWORD")"

  if [[ -z "${password}" ]]; then
    echo "MYSQL_ROOT_PASSWORD or MARIADB_ROOT_PASSWORD is required" >&2
    exit 1
  fi

  printf '%s' "${password}"
}

mysql_service_image_user_id() {
  local service_image
  service_image="$(mysql_service_container_image)"
  docker run --rm "${service_image}" sh -lc 'id -u mysql'
}

mysql_service_image_group_id() {
  local service_image
  service_image="$(mysql_service_container_image)"
  docker run --rm "${service_image}" sh -lc 'id -g mysql'
}

mysql_physical_tool_family() {
  if uses_mariadb_compose_file; then
    printf 'mariadb-backup'
  else
    printf 'xtrabackup'
  fi
}

default_mysql_physical_helper_image() {
  if uses_mariadb_compose_file; then
    mysql_service_container_image
  else
    printf 'percona/percona-xtrabackup:8.4'
  fi
}

mysql_physical_helper_image() {
  if [[ -n "${MYSQL_PHYSICAL_HELPER_IMAGE:-}" ]]; then
    printf '%s' "${MYSQL_PHYSICAL_HELPER_IMAGE}"
  else
    default_mysql_physical_helper_image
  fi
}

mysql_physical_backup_to_dir() {
  local target_dir="$1"
  local log_path="$2"
  local target_parent
  local target_basename
  local container_id
  local helper_image
  local root_password

  ensure_stack_service_running "${MYSQL_SERVICE}"

  rm -rf "${target_dir}"
  mkdir -p "$(dirname "${target_dir}")"
  target_parent="$(absolute_existing_dir "$(dirname "${target_dir}")")"
  target_basename="$(basename "${target_dir}")"
  container_id="$(mysql_service_container_id)"
  helper_image="$(mysql_physical_helper_image)"
  root_password="$(mysql_root_password)"

  if uses_mariadb_compose_file; then
    docker run --rm \
      --user 0:0 \
      --network "container:${container_id}" \
      --volumes-from "${container_id}:ro" \
      -v "${target_parent}:/backup-root" \
      -e MARIADB_ROOT_PASSWORD="${root_password}" \
      "${helper_image}" \
      sh -lc '
set -eu
backup_bin="$(command -v mariadb-backup || command -v mariabackup || true)"
if [ -z "${backup_bin}" ]; then
  echo "Neither mariadb-backup nor mariabackup is available" >&2
  exit 1
fi
exec "${backup_bin}" \
  --backup \
  --host=127.0.0.1 \
  --port=3306 \
  --user=root \
  --password="${MARIADB_ROOT_PASSWORD}" \
  --target-dir="/backup-root/'"${target_basename}"'"
' > "${log_path}" 2>&1
  else
    docker run --rm \
      --user 0:0 \
      --network "container:${container_id}" \
      --volumes-from "${container_id}:ro" \
      -v "${target_parent}:/backup-root" \
      -e MYSQL_ROOT_PASSWORD="${root_password}" \
      "${helper_image}" \
      sh -lc '
set -eu
exec xtrabackup \
  --backup \
  --host=127.0.0.1 \
  --port=3306 \
  --user=root \
  --password="${MYSQL_ROOT_PASSWORD}" \
  --datadir=/var/lib/mysql \
  --target-dir="/backup-root/'"${target_basename}"'"
' > "${log_path}" 2>&1
  fi

  ensure_directory_nonempty "${target_dir}" "MySQL/MariaDB physical backup directory"
}

mysql_physical_prepare_dir() {
  local target_dir="$1"
  local log_path="$2"
  local target_parent
  local target_basename
  local helper_image

  ensure_directory_nonempty "${target_dir}" "MySQL/MariaDB physical backup directory"

  target_parent="$(absolute_existing_dir "$(dirname "${target_dir}")")"
  target_basename="$(basename "${target_dir}")"
  helper_image="$(mysql_physical_helper_image)"

  if uses_mariadb_compose_file; then
    docker run --rm \
      --user 0:0 \
      -v "${target_parent}:/backup-root" \
      "${helper_image}" \
      sh -lc '
set -eu
prepare_bin="$(command -v mariadb-backup || command -v mariabackup || true)"
if [ -z "${prepare_bin}" ]; then
  echo "Neither mariadb-backup nor mariabackup is available" >&2
  exit 1
fi
exec "${prepare_bin}" \
  --prepare \
  --target-dir="/backup-root/'"${target_basename}"'"
' > "${log_path}" 2>&1
  else
    docker run --rm \
      --user 0:0 \
      -v "${target_parent}:/backup-root" \
      "${helper_image}" \
      sh -lc '
set -eu
exec xtrabackup \
  --prepare \
  --target-dir="/backup-root/'"${target_basename}"'"
' > "${log_path}" 2>&1
  fi
}

mysql_data_dir_archive_to_file() {
  local archive_path="$1"
  local archive_parent
  local archive_name
  local container_id
  local service_image

  container_id="$(mysql_service_container_id)"
  if [[ -z "${container_id}" ]]; then
    echo "Unable to resolve container id for service ${MYSQL_SERVICE}" >&2
    exit 1
  fi

  mkdir -p "$(dirname "${archive_path}")"
  archive_parent="$(absolute_existing_dir "$(dirname "${archive_path}")")"
  archive_name="$(basename "${archive_path}")"
  service_image="$(mysql_service_container_image)"

  docker run --rm \
    --volumes-from "${container_id}" \
    -v "${archive_parent}:/backup-root" \
    "${service_image}" \
    sh -lc '
set -eu
tar -C /var/lib/mysql -czf "/backup-root/'"${archive_name}"'" .
' >/dev/null

  if [[ ! -f "${archive_path}" || ! -s "${archive_path}" ]]; then
    echo "MySQL/MariaDB datadir archive is missing or empty: ${archive_path}" >&2
    return 1
  fi
}

mysql_data_dir_restore_from_archive() {
  local archive_path="$1"
  local log_path="$2"
  local archive_parent
  local archive_name
  local container_id
  local service_image
  local mysql_uid
  local mysql_gid

  ensure_file_nonempty "${archive_path}" "MySQL/MariaDB datadir archive"

  container_id="$(mysql_service_container_id)"
  if [[ -z "${container_id}" ]]; then
    echo "Unable to resolve container id for service ${MYSQL_SERVICE}" >&2
    exit 1
  fi

  archive_parent="$(absolute_existing_dir "$(dirname "${archive_path}")")"
  archive_name="$(basename "${archive_path}")"
  service_image="$(mysql_service_container_image)"
  mysql_uid="$(mysql_service_image_user_id)"
  mysql_gid="$(mysql_service_image_group_id)"

  docker run --rm \
    --volumes-from "${container_id}" \
    -v "${archive_parent}:/backup-root:ro" \
    -e MYSQL_UID="${mysql_uid}" \
    -e MYSQL_GID="${mysql_gid}" \
    "${service_image}" \
    sh -lc '
set -eu
mkdir -p /var/lib/mysql
find /var/lib/mysql -mindepth 1 -maxdepth 1 -exec rm -rf {} +
tar -xzf "/backup-root/'"${archive_name}"'" -C /var/lib/mysql
chown -R "${MYSQL_UID}:${MYSQL_GID}" /var/lib/mysql
' > "${log_path}" 2>&1
}

mysql_physical_copy_back_from_dir() {
  local source_dir="$1"
  local log_path="$2"
  local source_parent
  local source_basename
  local container_id
  local helper_image
  local mysql_uid
  local mysql_gid

  ensure_directory_nonempty "${source_dir}" "MySQL/MariaDB physical backup directory"

  container_id="$(mysql_service_container_id)"
  if [[ -z "${container_id}" ]]; then
    echo "Unable to resolve container id for service ${MYSQL_SERVICE}" >&2
    exit 1
  fi

  source_parent="$(absolute_existing_dir "$(dirname "${source_dir}")")"
  source_basename="$(basename "${source_dir}")"
  helper_image="$(mysql_physical_helper_image)"
  mysql_uid="$(mysql_service_image_user_id)"
  mysql_gid="$(mysql_service_image_group_id)"

  if uses_mariadb_compose_file; then
    docker run --rm \
      --user 0:0 \
      --volumes-from "${container_id}" \
      -v "${source_parent}:/backup-root:ro" \
      -e MYSQL_UID="${mysql_uid}" \
      -e MYSQL_GID="${mysql_gid}" \
      "${helper_image}" \
      sh -lc '
set -eu
copy_back_bin="$(command -v mariadb-backup || command -v mariabackup || true)"
if [ -z "${copy_back_bin}" ]; then
  echo "Neither mariadb-backup nor mariabackup is available" >&2
  exit 1
fi
mkdir -p /var/lib/mysql
find /var/lib/mysql -mindepth 1 -maxdepth 1 -exec rm -rf {} +
exec "${copy_back_bin}" \
  --copy-back \
  --target-dir="/backup-root/'"${source_basename}"'" && \
chown -R "${MYSQL_UID}:${MYSQL_GID}" /var/lib/mysql
' > "${log_path}" 2>&1
  else
    docker run --rm \
      --user 0:0 \
      --volumes-from "${container_id}" \
      -v "${source_parent}:/backup-root:ro" \
      -e MYSQL_UID="${mysql_uid}" \
      -e MYSQL_GID="${mysql_gid}" \
      "${helper_image}" \
      sh -lc '
set -eu
mkdir -p /var/lib/mysql
find /var/lib/mysql -mindepth 1 -maxdepth 1 -exec rm -rf {} +
xtrabackup \
  --copy-back \
  --target-dir="/backup-root/'"${source_basename}"'" \
  --datadir=/var/lib/mysql &&
chown -R "${MYSQL_UID}:${MYSQL_GID}" /var/lib/mysql
' > "${log_path}" 2>&1
  fi
}

wait_for_compose_service_health() {
  local service="$1"
  local timeout_seconds="${2:-120}"
  local container_id
  local deadline
  local status

  container_id="$(compose_service_container_id "${service}")"
  if [[ -z "${container_id}" ]]; then
    echo "Unable to resolve container id for service ${service}" >&2
    return 1
  fi

  deadline=$((SECONDS + timeout_seconds))
  while (( SECONDS < deadline )); do
    status="$(
      docker inspect -f '{{if .State.Health}}{{.State.Health.Status}}{{else}}{{.State.Status}}{{end}}' "${container_id}" 2>/dev/null || true
    )"

    case "${status}" in
      healthy|running)
        return 0
        ;;
      unhealthy|exited|dead)
        echo "Service ${service} entered bad state: ${status}" >&2
        return 1
        ;;
    esac

    sleep 2
  done

  echo "Timed out waiting for service health: ${service}" >&2
  return 1
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
\"\${client_bin}\" --protocol=TCP -h127.0.0.1 -uroot --batch --skip-column-names <<'SQL'
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
exec "${client_bin}" --protocol=TCP -h127.0.0.1 -uroot "${database}"
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
