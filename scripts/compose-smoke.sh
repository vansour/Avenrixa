#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

APP_HOST_PORT="${APP_HOST_PORT:-8080}"
SMOKE_TIMEOUT_SECONDS="${SMOKE_TIMEOUT_SECONDS:-180}"
SMOKE_POLL_INTERVAL_SECONDS="${SMOKE_POLL_INTERVAL_SECONDS:-2}"
COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME:-avenrixa-smoke}"
COMPOSE_VARIANT="${COMPOSE_VARIANT:-postgres}"
PRESERVE_STACK_ON_FAILURE="${PRESERVE_STACK_ON_FAILURE:-0}"
SMOKE_FLOW="${SMOKE_FLOW:-auto}"
SMOKE_RESET_DATA_DIR="${SMOKE_RESET_DATA_DIR:-1}"
CACHE_MODE="${CACHE_MODE:-dragonfly}"
SMOKE_EXPECT_APP_VERSION_LABEL="${SMOKE_EXPECT_APP_VERSION_LABEL:-}"

ADMIN_EMAIL="${ADMIN_EMAIL:-admin@example.com}"
ADMIN_PASSWORD="${ADMIN_PASSWORD:-Password123456!}"
ADMIN_NEW_PASSWORD="${ADMIN_NEW_PASSWORD:-Password654321!}"
SITE_NAME="${SITE_NAME:-Avenrixa Compose Smoke}"
LINK_BASE_URL="${LINK_BASE_URL:-http://127.0.0.1:${APP_HOST_PORT}/login}"
RUNTIME_DATA_DIR="${RUNTIME_DATA_DIR:-}"
POSTGRES_DATABASE_URL="${POSTGRES_DATABASE_URL:-}"

source "${ROOT_DIR}/scripts/compose-runtime.sh"

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
POSTGRES_BACKUP_FILENAME=""

health_url="http://127.0.0.1:${APP_HOST_PORT}/health"
api_base="http://127.0.0.1:${APP_HOST_PORT}/api/v1"

resolve_smoke_flow() {
  if [[ "${SMOKE_FLOW}" != "auto" ]]; then
    printf '%s' "${SMOKE_FLOW}"
    return 0
  fi

  case "${COMPOSE_VARIANT}" in
    postgres)
      printf 'postgres'
      ;;
    *)
      echo "Unsupported COMPOSE_VARIANT for compose smoke: ${COMPOSE_VARIANT}" >&2
      return 1
      ;;
  esac
}

default_postgres_database_url() {
  compose_variant_default_database_url
}

default_runtime_data_dir() {
  compose_variant_default_data_dir
}

require_commands() {
  local required_commands=(docker curl jq base64)
  local command

  for command in "${required_commands[@]}"; do
    if ! command -v "${command}" >/dev/null 2>&1; then
      echo "Missing required command: ${command}" >&2
      exit 1
    fi
  done
}

expected_cache_component_status() {
  case "${CACHE_MODE}" in
    dragonfly)
      printf 'healthy'
      ;;
    none)
      printf 'disabled'
      ;;
    *)
      echo "Unsupported CACHE_MODE: ${CACHE_MODE}" >&2
      return 1
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

  expect_eq "${overall_status}" "healthy" "overall health status should remain healthy"
  expect_eq \
    "${actual_cache_status}" \
    "${expected_cache_status}" \
    "cache component status should match CACHE_MODE=${CACHE_MODE}"
}

assert_expected_app_version_label() {
  local health_payload
  local actual_version

  if [[ -z "${SMOKE_EXPECT_APP_VERSION_LABEL}" ]]; then
    return 0
  fi

  health_payload="$(curl -fsS "${health_url}")"
  actual_version="$(printf '%s' "${health_payload}" | jq -r '.version // empty')"
  expect_eq \
    "${actual_version}" \
    "${SMOKE_EXPECT_APP_VERSION_LABEL}" \
    "health version should match the expected application version label"
}

start_stack() {
  compose up -d --remove-orphans
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

expect_positive_integer() {
  local actual="$1"
  local message="$2"

  if [[ ! "${actual}" =~ ^-?[0-9]+$ ]] || (( actual <= 0 )); then
    echo "Assertion failed: ${message}. Expected positive integer, got '${actual}'" >&2
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

prepare_api_fixture() {
  TMP_ROOT="$(mktemp -d /tmp/avenrixa-compose-smoke-XXXXXX)"
  ADMIN_COOKIE_JAR="${TMP_ROOT}/admin.cookies.txt"
  TINY_PNG_PATH="${TMP_ROOT}/tiny.png"
  BACKUP_DOWNLOAD_PATH="${TMP_ROOT}/backup.postgresql.sql"

  printf '%s' 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAAAAAA6fptVAAAACklEQVR4nGNgAAAAAgABSK+kcQAAAABJRU5ErkJggg==' \
    | base64 -d > "${TINY_PNG_PATH}"
}

prepare_postgres_fixture() {
  prepare_api_fixture
}

reset_runtime_data_dir_if_needed() {
  if [[ "${SMOKE_RESET_DATA_DIR}" != "1" ]]; then
    return 0
  fi

  log_step "Resetting runtime smoke data directory"
  rm -rf "${RUNTIME_DATA_DIR}"
  mkdir -p "${RUNTIME_DATA_DIR}"
}

page_has_image() {
  local payload="$1"
  local image_key="$2"

  printf '%s' "${payload}" | jq -r --arg image_key "${image_key}" \
    '(.data // .) | map(select(.image_key == $image_key)) | length'
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
  assert_expected_app_version_label
  assert_cache_health_component
  echo "Compose smoke check passed (flow: health)"
}

login_admin() {
  local password="${1:-${ADMIN_PASSWORD}}"
  local login_response

  log_step "Logging in admin"
  rm -f "${ADMIN_COOKIE_JAR}"
  login_response="$(
    curl -fsS \
      -c "${ADMIN_COOKIE_JAR}" \
      -X POST "${api_base}/auth/login" \
      -H 'Content-Type: application/json' \
      -d "$(jq -n \
        --arg email "${ADMIN_EMAIL}" \
        --arg password "${password}" \
        '{email: $email, password: $password}')"
  )"

  expect_eq \
    "$(printf '%s' "${login_response}" | jq -r '.email')" \
    "${ADMIN_EMAIL}" \
    "admin login should return the configured admin email"
}

postgres_updated_site_name() {
  printf '%s' "${SITE_NAME} GA Updated"
}

configure_postgres_runtime() {
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

    if [[ "${configured}" != "true" || "${database_kind}" != "postgresql" ]]; then
      log_step "No DATABASE_URL preset detected, writing PostgreSQL bootstrap fallback config"
      curl -fsS \
        -X PUT "${api_base}/bootstrap/database-config" \
        -H 'Content-Type: application/json' \
        -d "$(jq -n \
          --arg database_kind "postgresql" \
          --arg database_url "${POSTGRES_DATABASE_URL}" \
          '{database_kind: $database_kind, database_url: $database_url}')" \
        >/dev/null
    else
      log_step "Reusing existing PostgreSQL bootstrap fallback config"
    fi

    log_step "Restarting app to enter runtime mode"
    compose restart app >/dev/null
  else
    expect_eq "${mode}" "runtime" "PostgreSQL compose should expose runtime bootstrap status"
    expect_eq "${configured}" "true" "PostgreSQL compose should run with configured database"
    expect_eq "${database_kind}" "postgresql" "PostgreSQL compose should run with postgresql database kind"
  fi

  wait_for_url "${api_base}/install/status" "${SMOKE_TIMEOUT_SECONDS}"
  assert_cache_health_component
}

install_postgres_app() {
  local install_status
  local install_payload
  local install_response
  local install_http_status
  local admin_me

  install_status="$(curl -fsS "${api_base}/install/status")"
  expect_eq \
    "$(printf '%s' "${install_status}" | jq -r '.installed')" \
    "false" \
    "PostgreSQL smoke requires an uninstalled database"

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
          mail_from_name: "Avenrixa",
          mail_link_base_url: $link_base_url
        }
      }'
  )"

  install_http_status="$(
    curl -sS \
      -o "${TMP_ROOT}/install-bootstrap-response.json" \
      -w '%{http_code}' \
      -c "${ADMIN_COOKIE_JAR}" \
      -X POST "${api_base}/install/bootstrap" \
      -H 'Content-Type: application/json' \
      -d "${install_payload}"
  )"
  if [[ "${install_http_status}" != "200" ]]; then
    install_response="$(cat "${TMP_ROOT}/install-bootstrap-response.json" 2>/dev/null || true)"
    echo "Install bootstrap failed with status ${install_http_status}" >&2
    if [[ -n "${install_response}" ]]; then
      echo "${install_response}" >&2
    fi
    exit 1
  fi

  admin_me="$(curl -fsS -b "${ADMIN_COOKIE_JAR}" "${api_base}/auth/me")"
  expect_eq \
    "$(printf '%s' "${admin_me}" | jq -r '.email')" \
    "${ADMIN_EMAIL}" \
    "admin session should be active after PostgreSQL installation"
}

run_postgres_auth_smoke() {
  local auth_me
  local change_password_status
  local post_change_status

  log_step "Refreshing admin session"
  curl -fsS \
    -b "${ADMIN_COOKIE_JAR}" \
    -c "${ADMIN_COOKIE_JAR}" \
    -X POST "${api_base}/auth/refresh" \
    >/dev/null

  auth_me="$(curl -fsS -b "${ADMIN_COOKIE_JAR}" "${api_base}/auth/me")"
  expect_eq \
    "$(printf '%s' "${auth_me}" | jq -r '.email')" \
    "${ADMIN_EMAIL}" \
    "refreshed admin session should stay authenticated"

  log_step "Changing admin password"
  change_password_status="$(
    curl -s -o /dev/null -w '%{http_code}' \
      -b "${ADMIN_COOKIE_JAR}" \
      -c "${ADMIN_COOKIE_JAR}" \
      -X POST "${api_base}/auth/change-password" \
      -H 'Content-Type: application/json' \
      -d "$(jq -n \
        --arg current_password "${ADMIN_PASSWORD}" \
        --arg new_password "${ADMIN_NEW_PASSWORD}" \
        '{current_password: $current_password, new_password: $new_password}')"
  )"
  expect_eq "${change_password_status}" "200" "password change should succeed for the admin"

  post_change_status="$(
    curl -s -o /dev/null -w '%{http_code}' -b "${ADMIN_COOKIE_JAR}" "${api_base}/auth/me"
  )"
  expect_eq \
    "${post_change_status}" \
    "401" \
    "password change should invalidate the previous authenticated session"

  login_admin "${ADMIN_NEW_PASSWORD}"
}

run_postgres_image_smoke() {
  local upload_response
  local image_key
  local images_page
  local image_detail
  local expires_at
  local stats
  local health_payload

  log_step "Uploading image"
  upload_response="$(
    curl -fsS \
      -b "${ADMIN_COOKIE_JAR}" \
      -F "file=@${TINY_PNG_PATH};filename=tiny.png;type=image/png" \
      "${api_base}/upload"
  )"
  image_key="$(printf '%s' "${upload_response}" | jq -r '.image_key')"
  expect_non_empty "${image_key}" "upload should return image_key"

  images_page="$(curl -fsS -b "${ADMIN_COOKIE_JAR}" "${api_base}/images?limit=20")"
  expect_eq \
    "$(page_has_image "${images_page}" "${image_key}")" \
    "1" \
    "active image list should contain uploaded image"

  image_detail="$(curl -fsS -b "${ADMIN_COOKIE_JAR}" "${api_base}/images/${image_key}")"
  expect_eq \
    "$(printf '%s' "${image_detail}" | jq -r '.image_key')" \
    "${image_key}" \
    "image detail should return uploaded image"

  expires_at="$(date -u -d '+1 day' '+%Y-%m-%dT%H:%M:%SZ')"
  log_step "Setting image expiry"
  curl -fsS \
    -b "${ADMIN_COOKIE_JAR}" \
    -X PUT "${api_base}/images/${image_key}/expiry" \
    -H 'Content-Type: application/json' \
    -d "$(jq -n --arg expires_at "${expires_at}" '{expires_at: $expires_at}')" \
    >/dev/null

  image_detail="$(curl -fsS -b "${ADMIN_COOKIE_JAR}" "${api_base}/images/${image_key}")"
  expect_eq \
    "$(printf '%s' "${image_detail}" | jq -r '.expires_at')" \
    "${expires_at}" \
    "image detail should expose the configured expiry timestamp"

  log_step "Deleting image"
  curl -fsS \
    -b "${ADMIN_COOKIE_JAR}" \
    -X DELETE "${api_base}/images" \
    -H 'Content-Type: application/json' \
    -d "$(jq -n --arg image_key "${image_key}" '{image_keys: [$image_key]}')" \
    >/dev/null

  images_page="$(curl -fsS -b "${ADMIN_COOKIE_JAR}" "${api_base}/images?limit=20")"
  expect_eq \
    "$(page_has_image "${images_page}" "${image_key}")" \
    "0" \
    "active image list should not contain deleted image"

  stats="$(curl -fsS -b "${ADMIN_COOKIE_JAR}" "${api_base}/stats")"
  expect_eq "$(printf '%s' "${stats}" | jq -r '.total_users')" "1" "system stats should report a single admin user"
  expect_eq "$(printf '%s' "${stats}" | jq -r '.total_images')" "0" "system stats should report zero active images after delete"
  expect_eq "$(printf '%s' "${stats}" | jq -r '.images_last_24h')" "1" "system stats should count the new image in the last 24 hours"
  expect_eq "$(printf '%s' "${stats}" | jq -r '.images_last_7d')" "1" "system stats should count the new image in the last 7 days"
  expect_eq "$(printf '%s' "${stats}" | jq -r '.total_storage')" "0" "system stats should report zero active storage after delete"

  health_payload="$(curl -fsS "${health_url}")"
  expect_eq "$(printf '%s' "${health_payload}" | jq -r '.database.status')" "healthy" "database health should remain healthy after image operations"
  expect_eq "$(printf '%s' "${health_payload}" | jq -r '.cache.status')" "$(expected_cache_component_status)" "cache health should remain healthy after image operations"
  expect_eq "$(printf '%s' "${health_payload}" | jq -r '.storage.status')" "healthy" "storage health should remain healthy after image operations"
  expect_eq "$(printf '%s' "${health_payload}" | jq -r '.metrics.users_count')" "1" "health metrics should report one user"
  expect_eq "$(printf '%s' "${health_payload}" | jq -r '.metrics.images_count')" "0" "health metrics should report zero active images after delete"
}

run_postgres_settings_smoke() {
  local settings_config
  local updated_site_name
  local update_payload
  local updated_config
  local reloaded_config
  local install_status

  updated_site_name="$(postgres_updated_site_name)"

  log_step "Loading admin settings config"
  settings_config="$(curl -fsS -b "${ADMIN_COOKIE_JAR}" "${api_base}/settings/config")"
  expect_eq "$(printf '%s' "${settings_config}" | jq -r '.site_name')" "${SITE_NAME}" "settings config should expose the installed site name"
  expect_eq "$(printf '%s' "${settings_config}" | jq -r '.storage_backend')" "local" "settings config should expose local storage backend"
  expect_eq "$(printf '%s' "${settings_config}" | jq -r '.local_storage_path')" "/data/images" "settings config should expose the local storage path"
  expect_eq "$(printf '%s' "${settings_config}" | jq -r '.mail_enabled')" "false" "settings config should reflect disabled mail"
  expect_eq "$(printf '%s' "${settings_config}" | jq -r '.restart_required')" "false" "site-name only config changes should not require restart"

  update_payload="$(
    printf '%s' "${settings_config}" | jq -c --arg site_name "${updated_site_name}" '{
      site_name: $site_name,
      storage_backend,
      local_storage_path,
      mail_enabled,
      mail_smtp_host,
      mail_smtp_port,
      mail_smtp_user,
      mail_smtp_password: null,
      mail_from_email,
      mail_from_name,
      mail_link_base_url,
      expected_settings_version
    }'
  )"

  log_step "Updating site name through structured settings config"
  updated_config="$(
    curl -fsS \
      -b "${ADMIN_COOKIE_JAR}" \
      -X PUT "${api_base}/settings/config" \
      -H 'Content-Type: application/json' \
      -d "${update_payload}"
  )"
  expect_eq "$(printf '%s' "${updated_config}" | jq -r '.site_name')" "${updated_site_name}" "settings update should persist the new site name"
  expect_eq "$(printf '%s' "${updated_config}" | jq -r '.restart_required')" "false" "site-name update should stay hot-reloadable"

  reloaded_config="$(curl -fsS -b "${ADMIN_COOKIE_JAR}" "${api_base}/settings/config")"
  expect_eq "$(printf '%s' "${reloaded_config}" | jq -r '.site_name')" "${updated_site_name}" "reloading settings config should keep the updated site name"

  install_status="$(curl -fsS "${api_base}/install/status")"
  expect_eq "$(printf '%s' "${install_status}" | jq -r '.config.site_name')" "${updated_site_name}" "public install status should expose the updated site name"
}

create_postgres_backup() {
  local backup_response
  local backup_filename
  local backups_page

  log_step "Creating PostgreSQL logical backup" >&2
  backup_response="$(curl -fsS -b "${ADMIN_COOKIE_JAR}" -X POST "${api_base}/backup")"
  backup_filename="$(printf '%s' "${backup_response}" | jq -r '.filename')"
  expect_non_empty "${backup_filename}" "backup creation should return filename"
  expect_eq \
    "$(printf '%s' "${backup_response}" | jq -r '.semantics.database_family')" \
    "postgresql" \
    "PostgreSQL backup should report postgresql database family"
  expect_eq \
    "$(printf '%s' "${backup_response}" | jq -r '.semantics.backup_kind')" \
    "postgresql-logical-dump" \
    "PostgreSQL backup should report logical dump semantics"
  expect_eq \
    "$(printf '%s' "${backup_response}" | jq -r '.semantics.restore_mode')" \
    "download-only" \
    "PostgreSQL backup should report download-only restore mode"
  expect_eq \
    "$(printf '%s' "${backup_response}" | jq -r '.semantics.ui_restore_supported')" \
    "false" \
    "PostgreSQL backup should report page restore unsupported"

  backups_page="$(curl -fsS -b "${ADMIN_COOKIE_JAR}" "${api_base}/backups")"
  expect_eq \
    "$(page_has_backup "${backups_page}" "${backup_filename}")" \
    "1" \
    "backup list should contain created PostgreSQL backup"
  expect_eq \
    "$(printf '%s' "${backups_page}" | jq -r --arg filename "${backup_filename}" '[.[] | select(.filename == $filename and .semantics.restore_mode == "download-only" and .semantics.ui_restore_supported == false)] | length')" \
    "1" \
    "backup list should expose PostgreSQL download-only semantics"

  printf '%s' "${backup_filename}"
}

download_postgres_backup() {
  local backup_filename="$1"

  log_step "Downloading PostgreSQL logical backup"
  curl -fsS \
    -o "${BACKUP_DOWNLOAD_PATH}" \
    -b "${ADMIN_COOKIE_JAR}" \
    "${api_base}/backups/${backup_filename}" \
    >/dev/null

  if [[ ! -s "${BACKUP_DOWNLOAD_PATH}" ]]; then
    echo "Assertion failed: downloaded PostgreSQL backup should not be empty" >&2
    exit 1
  fi
}

delete_postgres_backup() {
  local backup_filename="$1"
  local backups_page

  log_step "Deleting PostgreSQL logical backup"
  curl -fsS \
    -b "${ADMIN_COOKIE_JAR}" \
    -X DELETE "${api_base}/backups/${backup_filename}" \
    >/dev/null

  backups_page="$(curl -fsS -b "${ADMIN_COOKIE_JAR}" "${api_base}/backups")"
  expect_eq \
    "$(page_has_backup "${backups_page}" "${backup_filename}")" \
    "0" \
    "backup list should not contain deleted PostgreSQL backup"
}

assert_postgres_page_restore_routes_removed() {
  local backup_filename="$1"
  local restore_status_http_code
  local precheck_http_code
  local schedule_http_code

  log_step "Verifying PostgreSQL page restore routes are not exposed"
  restore_status_http_code="$(
    curl -sS \
      -o /dev/null \
      -w '%{http_code}' \
      -b "${ADMIN_COOKIE_JAR}" \
      "${api_base}/backup-restore/status"
  )"
  expect_eq \
    "${restore_status_http_code}" \
    "404" \
    "PostgreSQL logical backup smoke should not expose restore status route"

  precheck_http_code="$(
    curl -sS \
      -o /dev/null \
      -w '%{http_code}' \
      -b "${ADMIN_COOKIE_JAR}" \
      -X POST "${api_base}/backups/${backup_filename}/restore/precheck"
  )"
  expect_eq \
    "${precheck_http_code}" \
    "404" \
    "PostgreSQL logical backup smoke should not expose restore precheck route"

  schedule_http_code="$(
    curl -sS \
      -o /dev/null \
      -w '%{http_code}' \
      -b "${ADMIN_COOKIE_JAR}" \
      -X POST "${api_base}/backups/${backup_filename}/restore"
  )"
  expect_eq \
    "${schedule_http_code}" \
    "404" \
    "PostgreSQL logical backup smoke should not expose restore scheduling route"
}

run_postgres_backup_smoke() {
  POSTGRES_BACKUP_FILENAME="$(create_postgres_backup)"
  download_postgres_backup "${POSTGRES_BACKUP_FILENAME}"
  assert_postgres_page_restore_routes_removed "${POSTGRES_BACKUP_FILENAME}"
  delete_postgres_backup "${POSTGRES_BACKUP_FILENAME}"
}

run_postgres_audit_smoke() {
  local audit_page

  expect_non_empty "${POSTGRES_BACKUP_FILENAME}" "PostgreSQL audit smoke requires a backup filename"

  log_step "Verifying audit log coverage for the GA flow"
  audit_page="$(curl -fsS -b "${ADMIN_COOKIE_JAR}" "${api_base}/audit-logs?page=1&page_size=100")"
  expect_positive_integer "$(printf '%s' "${audit_page}" | jq -r '.total')" "audit logs should contain GA smoke entries"
  expect_eq \
    "$(printf '%s' "${audit_page}" | jq -r '[.data[] | select(.action == "system.install_completed" and (.details.storage_backend // "") == "local")] | length > 0')" \
    "true" \
    "audit logs should contain the installation completion event"
  expect_eq \
    "$(printf '%s' "${audit_page}" | jq -r '[.data[] | select(.action == "user.password_changed")] | length > 0')" \
    "true" \
    "audit logs should contain the password rotation event"
  expect_eq \
    "$(printf '%s' "${audit_page}" | jq -r '[.data[] | select(.action == "admin.settings.config_updated" and ((.details.changed_keys // []) | index("site_name")) != null and (.details.restart_required == false))] | length > 0')" \
    "true" \
    "audit logs should contain the structured settings update event"
  expect_eq \
    "$(printf '%s' "${audit_page}" | jq -r --arg filename "${POSTGRES_BACKUP_FILENAME}" '[.data[] | select(.action == "admin.maintenance.database_backup.created" and (.details.filename // "") == $filename and (.details.database_kind // "") == "postgresql" and (.details.restore_mode // "") == "download-only" and (.details.ui_restore_supported == false))] | length > 0')" \
    "true" \
    "audit logs should contain the PostgreSQL logical backup creation event"
}

run_postgres_logout_smoke() {
  local post_logout_status

  log_step "Logging out at the end of the GA smoke"
  curl -fsS \
    -b "${ADMIN_COOKIE_JAR}" \
    -c "${ADMIN_COOKIE_JAR}" \
    -X POST "${api_base}/auth/logout" \
    >/dev/null

  post_logout_status="$(
    curl -s -o /dev/null -w '%{http_code}' -b "${ADMIN_COOKIE_JAR}" "${api_base}/auth/me"
  )"
  expect_eq "${post_logout_status}" "401" "logout should invalidate the final admin session"
}

run_postgres_smoke() {
  prepare_postgres_fixture

  log_step "Waiting for bootstrap health"
  wait_for_url "${health_url}" "${SMOKE_TIMEOUT_SECONDS}"
  assert_expected_app_version_label
  assert_cache_health_component

  configure_postgres_runtime
  install_postgres_app
  run_postgres_auth_smoke
  run_postgres_image_smoke
  run_postgres_settings_smoke
  run_postgres_backup_smoke
  run_postgres_audit_smoke
  run_postgres_logout_smoke

  echo "Compose smoke check passed (flow: postgres)"
}

require_commands

CURRENT_FLOW="$(resolve_smoke_flow)"
if [[ -z "${POSTGRES_DATABASE_URL}" ]]; then
  POSTGRES_DATABASE_URL="$(default_postgres_database_url)"
fi
if [[ -z "${RUNTIME_DATA_DIR}" ]]; then
  if [[ -n "${DATA_DIR:-}" ]]; then
    RUNTIME_DATA_DIR="${DATA_DIR}"
  else
    RUNTIME_DATA_DIR="$(default_runtime_data_dir)"
  fi
fi

case "${CURRENT_FLOW}" in
  health|postgres)
    ;;
  *)
    echo "Unsupported SMOKE_FLOW: ${CURRENT_FLOW}" >&2
    exit 1
    ;;
esac

echo "Cache mode: ${CACHE_MODE}"
echo "Smoke health URL: ${health_url}"
echo "Runtime data dir: ${RUNTIME_DATA_DIR}"

compose down -v --remove-orphans >/dev/null 2>&1 || true
remove_container_name_conflicts
reset_runtime_data_dir_if_needed
compose build

case "${CURRENT_FLOW}" in
  health)
    start_stack
    run_health_smoke
    ;;
  postgres)
    start_stack
    run_postgres_smoke
    ;;
esac
