#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

APP_HOST_PORT="${APP_HOST_PORT:-8080}"
SMOKE_TIMEOUT_SECONDS="${SMOKE_TIMEOUT_SECONDS:-180}"
SMOKE_POLL_INTERVAL_SECONDS="${SMOKE_POLL_INTERVAL_SECONDS:-2}"
COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME:-vansour-image-smoke}"
PRESERVE_STACK_ON_FAILURE="${PRESERVE_STACK_ON_FAILURE:-0}"
SMOKE_FLOW="${SMOKE_FLOW:-auto}"
MYSQL_SMOKE_RESET_DATA_DIR="${MYSQL_SMOKE_RESET_DATA_DIR:-1}"

ADMIN_EMAIL="${ADMIN_EMAIL:-admin@example.com}"
ADMIN_PASSWORD="${ADMIN_PASSWORD:-Password123456!}"
SITE_NAME="${SITE_NAME:-MySQL/MariaDB Compose Smoke}"
LINK_BASE_URL="${LINK_BASE_URL:-http://127.0.0.1:${APP_HOST_PORT}/login}"
MYSQL_DATABASE_URL="${MYSQL_DATABASE_URL:-}"
MYSQL_DATA_DIR="${MYSQL_DATA_DIR:-}"

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

CURRENT_FLOW=""
SCRIPT_FAILED=0
TMP_ROOT=""
ADMIN_COOKIE_JAR=""
TINY_PNG_PATH=""
BACKUP_DOWNLOAD_PATH=""

health_url="http://127.0.0.1:${APP_HOST_PORT}/health"
api_base="http://127.0.0.1:${APP_HOST_PORT}/api/v1"

resolve_smoke_flow() {
  if [[ "${SMOKE_FLOW}" != "auto" ]]; then
    printf '%s' "${SMOKE_FLOW}"
    return 0
  fi

  for compose_file in "${compose_files[@]}"; do
    case "$(basename "${compose_file}")" in
      compose.mysql.yml|compose.mysql.ops.yml|compose.mariadb.yml|compose.mariadb.ops.yml)
      printf 'mysql'
      return 0
        ;;
    esac
  done

  printf 'health'
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

require_commands() {
  local required_commands=(docker curl)

  if [[ "${CURRENT_FLOW}" == "mysql" ]]; then
    required_commands+=(jq base64 mktemp)
  fi

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

expect_eq() {
  local actual="$1"
  local expected="$2"
  local message="$3"

  if [[ "${actual}" != "${expected}" ]]; then
    echo "Assertion failed: ${message}. Expected '${expected}', got '${actual}'" >&2
    exit 1
  fi
}

expect_non_empty() {
  local actual="$1"
  local message="$2"

  if [[ -z "${actual}" || "${actual}" == "null" ]]; then
    echo "Assertion failed: ${message}. Value is empty" >&2
    exit 1
  fi
}

on_error() {
  SCRIPT_FAILED=1
  echo >&2
  echo "Compose smoke failed (flow: ${CURRENT_FLOW:-unknown}). Recent compose state:" >&2
  compose ps >&2 || true
  compose logs --no-color --tail=200 >&2 || true
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

prepare_mysql_fixture() {
  TMP_ROOT="$(mktemp -d /tmp/vansour-compose-mysql-smoke-XXXXXX)"
  ADMIN_COOKIE_JAR="${TMP_ROOT}/admin.cookies.txt"
  TINY_PNG_PATH="${TMP_ROOT}/tiny.png"
  BACKUP_DOWNLOAD_PATH="${TMP_ROOT}/backup.mysql.sql"

  printf '%s' 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAAAAAA6fptVAAAACklEQVR4nGNgAAAAAgABSK+kcQAAAABJRU5ErkJggg==' \
    | base64 -d > "${TINY_PNG_PATH}"
}

reset_mysql_data_dir_if_needed() {
  if [[ "${MYSQL_SMOKE_RESET_DATA_DIR}" != "1" ]]; then
    return 0
  fi

  if ! uses_mysql_compose_file; then
    return 0
  fi

  log_step "Resetting MySQL/MariaDB smoke data directory"
  rm -rf "${MYSQL_DATA_DIR}"
  mkdir -p "${MYSQL_DATA_DIR}"
}

page_has_image() {
  local payload="$1"
  local image_key="$2"

  printf '%s' "${payload}" | jq -r --arg image_key "${image_key}" \
    '.data | map(select(.image_key == $image_key)) | length'
}

page_has_backup() {
  local payload="$1"
  local filename="$2"

  printf '%s' "${payload}" | jq -r --arg filename "${filename}" \
    'map(select(.filename == $filename)) | length'
}

run_health_smoke() {
  log_step "Waiting for application health"
  wait_for_url "${health_url}" "${SMOKE_TIMEOUT_SECONDS}"
  echo "Compose smoke check passed (flow: health)"
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

    if [[ "${configured}" != "true" || "${database_kind}" != "mysql" ]]; then
      log_step "Writing MySQL/MariaDB bootstrap config"
      curl -fsS \
        -X PUT "${api_base}/bootstrap/database-config" \
        -H 'Content-Type: application/json' \
        -d "$(jq -n \
          --arg database_kind "mysql" \
          --arg database_url "${MYSQL_DATABASE_URL}" \
          '{database_kind: $database_kind, database_url: $database_url, database_max_connections: 5}')" \
        >/dev/null
    else
      log_step "Reusing existing MySQL/MariaDB bootstrap config"
    fi

    log_step "Restarting app to enter runtime mode"
    compose restart app >/dev/null
  else
    expect_eq "${mode}" "runtime" "MySQL/MariaDB compose should expose runtime bootstrap status"
    expect_eq "${database_kind}" "mysql" "MySQL/MariaDB compose should run with mysql database kind"
  fi

  wait_for_url "${api_base}/install/status" "${SMOKE_TIMEOUT_SECONDS}"
}

install_mysql_app() {
  local install_status
  local install_payload
  local admin_me

  install_status="$(curl -fsS "${api_base}/install/status")"
  expect_eq \
    "$(printf '%s' "${install_status}" | jq -r '.installed')" \
    "false" \
    "MySQL/MariaDB smoke requires an uninstalled database"

  log_step "Completing installation wizard"
  install_payload="$(
    jq -n \
      --arg admin_email "${ADMIN_EMAIL}" \
      --arg admin_password "${ADMIN_PASSWORD}" \
      --arg site_name "${SITE_NAME}" \
      --arg link_base_url "${LINK_BASE_URL}" \
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
    -c "${ADMIN_COOKIE_JAR}" \
    -X POST "${api_base}/install/bootstrap" \
    -H 'Content-Type: application/json' \
    -d "${install_payload}" \
    >/dev/null

  admin_me="$(curl -fsS -b "${ADMIN_COOKIE_JAR}" "${api_base}/auth/me")"
  expect_eq \
    "$(printf '%s' "${admin_me}" | jq -r '.email')" \
    "${ADMIN_EMAIL}" \
    "admin session should be active after MySQL/MariaDB installation"
}

run_mysql_image_smoke() {
  local upload_response
  local image_key
  local images_page
  local image_detail
  local deleted_page

  log_step "Uploading image"
  upload_response="$(
    curl -fsS \
      -b "${ADMIN_COOKIE_JAR}" \
      -F "file=@${TINY_PNG_PATH};filename=tiny.png;type=image/png" \
      "${api_base}/upload"
  )"
  image_key="$(printf '%s' "${upload_response}" | jq -r '.image_key')"
  expect_non_empty "${image_key}" "upload should return image_key"

  images_page="$(curl -fsS -b "${ADMIN_COOKIE_JAR}" "${api_base}/images?page=1&page_size=20")"
  expect_eq \
    "$(page_has_image "${images_page}" "${image_key}")" \
    "1" \
    "active image list should contain uploaded image"

  image_detail="$(curl -fsS -b "${ADMIN_COOKIE_JAR}" "${api_base}/images/${image_key}")"
  expect_eq \
    "$(printf '%s' "${image_detail}" | jq -r '.image_key')" \
    "${image_key}" \
    "image detail should return uploaded image"

  log_step "Soft deleting and restoring image"
  curl -fsS \
    -b "${ADMIN_COOKIE_JAR}" \
    -X DELETE "${api_base}/images" \
    -H 'Content-Type: application/json' \
    -d "$(jq -n --arg image_key "${image_key}" '{image_keys: [$image_key], permanent: false}')" \
    >/dev/null

  deleted_page="$(curl -fsS -b "${ADMIN_COOKIE_JAR}" "${api_base}/images/deleted?page=1&page_size=20")"
  expect_eq \
    "$(page_has_image "${deleted_page}" "${image_key}")" \
    "1" \
    "deleted image list should contain soft deleted image"

  curl -fsS \
    -b "${ADMIN_COOKIE_JAR}" \
    -X POST "${api_base}/images/restore" \
    -H 'Content-Type: application/json' \
    -d "$(jq -n --arg image_key "${image_key}" '{image_keys: [$image_key]}')" \
    >/dev/null

  images_page="$(curl -fsS -b "${ADMIN_COOKIE_JAR}" "${api_base}/images?page=1&page_size=20")"
  expect_eq \
    "$(page_has_image "${images_page}" "${image_key}")" \
    "1" \
    "active image list should contain restored image"

  log_step "Permanently deleting image"
  curl -fsS \
    -b "${ADMIN_COOKIE_JAR}" \
    -X DELETE "${api_base}/images" \
    -H 'Content-Type: application/json' \
    -d "$(jq -n --arg image_key "${image_key}" '{image_keys: [$image_key], permanent: true}')" \
    >/dev/null

  images_page="$(curl -fsS -b "${ADMIN_COOKIE_JAR}" "${api_base}/images?page=1&page_size=20")"
  expect_eq \
    "$(page_has_image "${images_page}" "${image_key}")" \
    "0" \
    "active image list should not contain permanently deleted image"

  deleted_page="$(curl -fsS -b "${ADMIN_COOKIE_JAR}" "${api_base}/images/deleted?page=1&page_size=20")"
  expect_eq \
    "$(page_has_image "${deleted_page}" "${image_key}")" \
    "0" \
    "deleted image list should not contain permanently deleted image"
}

run_mysql_backup_smoke() {
  local backup_filename

  backup_filename="$(create_mysql_backup)"
  download_mysql_backup "${backup_filename}"
  run_mysql_restore_smoke "${backup_filename}"
}

create_mysql_backup() {
  local backup_response
  local backup_filename
  local backups_page

  log_step "Creating MySQL/MariaDB backup" >&2
  backup_response="$(curl -fsS -b "${ADMIN_COOKIE_JAR}" -X POST "${api_base}/backup")"
  backup_filename="$(printf '%s' "${backup_response}" | jq -r '.filename')"
  expect_non_empty "${backup_filename}" "backup creation should return filename"

  backups_page="$(curl -fsS -b "${ADMIN_COOKIE_JAR}" "${api_base}/backups")"
  expect_eq \
    "$(page_has_backup "${backups_page}" "${backup_filename}")" \
    "1" \
    "backup list should contain created MySQL/MariaDB backup"

  printf '%s' "${backup_filename}"
}

download_mysql_backup() {
  local backup_filename="$1"

  log_step "Downloading MySQL/MariaDB backup"
  curl -fsS \
    -o "${BACKUP_DOWNLOAD_PATH}" \
    -b "${ADMIN_COOKIE_JAR}" \
    "${api_base}/backups/${backup_filename}" \
    >/dev/null

  if [[ ! -s "${BACKUP_DOWNLOAD_PATH}" ]]; then
    echo "Assertion failed: downloaded MySQL/MariaDB backup should not be empty" >&2
    exit 1
  fi
}

delete_mysql_backup() {
  local backup_filename="$1"
  local backups_page

  log_step "Deleting MySQL/MariaDB backup"
  curl -fsS \
    -b "${ADMIN_COOKIE_JAR}" \
    -X DELETE "${api_base}/backups/${backup_filename}" \
    >/dev/null

  backups_page="$(curl -fsS -b "${ADMIN_COOKIE_JAR}" "${api_base}/backups")"
  expect_eq \
    "$(page_has_backup "${backups_page}" "${backup_filename}")" \
    "0" \
    "backup list should not contain deleted MySQL/MariaDB backup"
}

precheck_mysql_restore() {
  local backup_filename="$1"
  local precheck_response

  log_step "Prechecking MySQL/MariaDB restore"
  precheck_response="$(
    curl -fsS \
      -b "${ADMIN_COOKIE_JAR}" \
      -X POST "${api_base}/backups/${backup_filename}/restore/precheck"
  )"

  expect_eq \
    "$(printf '%s' "${precheck_response}" | jq -r '.eligible')" \
    "true" \
    "MySQL/MariaDB restore precheck should pass for freshly created backup"
  expect_eq \
    "$(printf '%s' "${precheck_response}" | jq -r '.current_database_kind')" \
    "mysql" \
    "MySQL/MariaDB restore precheck should report current database kind"
  expect_eq \
    "$(printf '%s' "${precheck_response}" | jq -r '.backup_database_kind')" \
    "mysql" \
    "MySQL/MariaDB restore precheck should report backup database kind"
  expect_eq \
    "$(printf '%s' "${precheck_response}" | jq -r '.object_rollback_anchor.strategy // empty')" \
    "local-directory-snapshot" \
    "MySQL/MariaDB restore precheck should expose local object rollback anchor"
}

schedule_mysql_restore() {
  local backup_filename="$1"
  local restore_response

  log_step "Scheduling MySQL/MariaDB restore"
  restore_response="$(
    curl -fsS \
      -b "${ADMIN_COOKIE_JAR}" \
      -X POST "${api_base}/backups/${backup_filename}/restore"
  )"

  expect_eq \
    "$(printf '%s' "${restore_response}" | jq -r '.scheduled')" \
    "true" \
    "MySQL/MariaDB restore schedule should be accepted"
  expect_eq \
    "$(printf '%s' "${restore_response}" | jq -r '.restart_required')" \
    "true" \
    "MySQL/MariaDB restore schedule should require restart"
  expect_eq \
    "$(printf '%s' "${restore_response}" | jq -r '.pending.filename')" \
    "${backup_filename}" \
    "MySQL/MariaDB restore schedule should persist pending filename"
  expect_eq \
    "$(printf '%s' "${restore_response}" | jq -r '.pending.database_kind')" \
    "mysql" \
    "MySQL/MariaDB restore schedule should persist pending database kind"
}

assert_pending_mysql_restore() {
  local backup_filename="$1"
  local restore_status

  log_step "Checking pending MySQL/MariaDB restore state"
  restore_status="$(curl -fsS -b "${ADMIN_COOKIE_JAR}" "${api_base}/backup-restore/status")"

  expect_eq \
    "$(printf '%s' "${restore_status}" | jq -r '.pending.filename // empty')" \
    "${backup_filename}" \
    "MySQL/MariaDB restore status should expose pending filename"
  expect_eq \
    "$(printf '%s' "${restore_status}" | jq -r '.pending.database_kind // empty')" \
    "mysql" \
    "MySQL/MariaDB restore status should expose pending database kind"
}

assert_mysql_restore_invalidated_session() {
  local http_code

  log_step "Checking old admin session invalidation after restore"
  http_code="$(
    curl -s -o /dev/null -w '%{http_code}' -b "${ADMIN_COOKIE_JAR}" "${api_base}/auth/me"
  )"
  expect_eq "${http_code}" "401" "MySQL/MariaDB restore should invalidate existing admin session"
}

login_mysql_admin() {
  local login_response

  log_step "Logging in as database admin"
  login_response="$(
    curl -fsS \
      -c "${ADMIN_COOKIE_JAR}" \
      -X POST "${api_base}/auth/login" \
      -H 'Content-Type: application/json' \
      -d "$(jq -n --arg email "${ADMIN_EMAIL}" --arg password "${ADMIN_PASSWORD}" '{email: $email, password: $password}')"
  )"
  expect_eq \
    "$(printf '%s' "${login_response}" | jq -r '.email')" \
    "${ADMIN_EMAIL}" \
    "admin should be able to log in after MySQL/MariaDB restore"
}

assert_mysql_restore_completed() {
  local backup_filename="$1"
  local restore_status

  log_step "Checking completed MySQL/MariaDB restore state"
  restore_status="$(curl -fsS -b "${ADMIN_COOKIE_JAR}" "${api_base}/backup-restore/status")"

  expect_eq \
    "$(printf '%s' "${restore_status}" | jq -r '.pending == null')" \
    "true" \
    "pending restore should be cleared after app restart"
  expect_eq \
    "$(printf '%s' "${restore_status}" | jq -r '.last_result.status // empty')" \
    "completed" \
    "MySQL/MariaDB restore should complete after app restart"
  expect_eq \
    "$(printf '%s' "${restore_status}" | jq -r '.last_result.filename // empty')" \
    "${backup_filename}" \
    "completed MySQL/MariaDB restore should reference scheduled backup"
  expect_eq \
    "$(printf '%s' "${restore_status}" | jq -r '.last_result.database_kind // empty')" \
    "mysql" \
    "completed MySQL/MariaDB restore should report mysql database kind"
}

wait_for_mysql_restore_audit() {
  local action="$1"
  local backup_filename="$2"
  local deadline=$((SECONDS + SMOKE_TIMEOUT_SECONDS))
  local audit_logs
  local match_count

  log_step "Checking restore audit log (${action})"
  while (( SECONDS < deadline )); do
    audit_logs="$(curl -fsS -b "${ADMIN_COOKIE_JAR}" "${api_base}/audit-logs?page=1&page_size=100")"
    match_count="$(
      printf '%s' "${audit_logs}" | jq -r \
        --arg action "${action}" \
        --arg filename "${backup_filename}" \
        '[.data[] | select(.action == $action and (.details.filename // "") == $filename)] | length'
    )"

    if [[ "${match_count}" != "0" ]]; then
      return 0
    fi

    sleep "${SMOKE_POLL_INTERVAL_SECONDS}"
  done

  echo "Timed out waiting for restore audit action ${action} for ${backup_filename}" >&2
  exit 1
}

run_mysql_restore_smoke() {
  local backup_filename="$1"

  precheck_mysql_restore "${backup_filename}"
  schedule_mysql_restore "${backup_filename}"
  assert_pending_mysql_restore "${backup_filename}"

  log_step "Restarting app to execute MySQL/MariaDB restore"
  compose restart app >/dev/null
  wait_for_url "${health_url}" "${SMOKE_TIMEOUT_SECONDS}"

  assert_mysql_restore_invalidated_session
  login_mysql_admin
  assert_mysql_restore_completed "${backup_filename}"
  wait_for_mysql_restore_audit "system.database_restore.completed" "${backup_filename}"
  delete_mysql_backup "${backup_filename}"
}

run_mysql_auth_smoke() {
  local post_logout_status

  log_step "Logging out and logging back in"
  curl -fsS \
    -b "${ADMIN_COOKIE_JAR}" \
    -c "${ADMIN_COOKIE_JAR}" \
    -X POST "${api_base}/auth/logout" \
    >/dev/null

  post_logout_status="$(
    curl -s -o /dev/null -w '%{http_code}' -b "${ADMIN_COOKIE_JAR}" "${api_base}/auth/me"
  )"
  expect_eq "${post_logout_status}" "401" "logout should invalidate admin session"
  login_mysql_admin
}

run_mysql_smoke() {
  prepare_mysql_fixture

  log_step "Waiting for bootstrap health"
  wait_for_url "${health_url}" "${SMOKE_TIMEOUT_SECONDS}"

  configure_mysql_runtime
  install_mysql_app
  run_mysql_image_smoke
  run_mysql_backup_smoke
  run_mysql_auth_smoke

  echo "Compose smoke check passed (flow: mysql)"
}

CURRENT_FLOW="$(resolve_smoke_flow)"
if [[ -z "${MYSQL_DATABASE_URL}" ]]; then
  MYSQL_DATABASE_URL="$(default_mysql_database_url)"
fi
if [[ -z "${MYSQL_DATA_DIR}" ]]; then
  MYSQL_DATA_DIR="$(default_mysql_data_dir)"
fi

case "${CURRENT_FLOW}" in
  health|mysql)
    ;;
  *)
    echo "Unsupported SMOKE_FLOW: ${CURRENT_FLOW}" >&2
    exit 1
    ;;
esac

require_commands

if [[ "${CURRENT_FLOW}" == "mysql" ]]; then
  reset_mysql_data_dir_if_needed
fi

echo "Using compose files: ${compose_files[*]}"
echo "Smoke flow: ${CURRENT_FLOW}"
echo "Smoke health URL: ${health_url}"

compose down -v --remove-orphans >/dev/null 2>&1 || true
remove_container_name_conflicts
compose build
compose up -d --remove-orphans

case "${CURRENT_FLOW}" in
  health)
    run_health_smoke
    ;;
  mysql)
    run_mysql_smoke
    ;;
esac
