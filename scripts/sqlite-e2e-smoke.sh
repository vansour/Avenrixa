#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

for command in docker curl jq base64 date mktemp; do
  if ! command -v "${command}" >/dev/null 2>&1; then
    echo "Missing required command: ${command}" >&2
    exit 1
  fi
done

APP_HOST_PORT="8080"
MAILPIT_HTTP_PORT="18025"
MAILPIT_SMTP_PORT="11025"
COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME:-vansour-image-sqlite-e2e}"
COMPOSE_VARIANT="${COMPOSE_VARIANT:-sqlite}"
COMPOSE_ENABLE_MAILPIT="${COMPOSE_ENABLE_MAILPIT:-1}"
PRESERVE_STACK_ON_FAILURE="${PRESERVE_STACK_ON_FAILURE:-0}"
CACHE_MODE="${CACHE_MODE:-redis8}"
ADMIN_EMAIL="${ADMIN_EMAIL:-admin@example.com}"
ADMIN_PASSWORD="${ADMIN_PASSWORD:-Password123456!}"
USER_EMAIL="${USER_EMAIL:-user@example.com}"
USER_PASSWORD="${USER_PASSWORD:-Password123456!}"
USER_NEW_PASSWORD="${USER_NEW_PASSWORD:-Password654321!}"
SITE_NAME="${SITE_NAME:-SQLite E2E Smoke}"
MAIL_FROM_EMAIL="${MAIL_FROM_EMAIL:-noreply@example.com}"
MAIL_FROM_NAME="${MAIL_FROM_NAME:-Vansour Image}"
LINK_BASE_URL="${LINK_BASE_URL:-http://127.0.0.1:${APP_HOST_PORT}/login}"
SQLITE_DATABASE_URL="${SQLITE_DATABASE_URL:-/data/sqlite/app.db}"

TEMP_DATA_DIR=0
if [[ -z "${DATA_DIR:-}" ]]; then
  DATA_DIR="$(mktemp -d /tmp/vansour-sqlite-e2e-data-XXXXXX)"
  TEMP_DATA_DIR=1
fi
export DATA_DIR

source "${ROOT_DIR}/scripts/compose-runtime.sh"

TMP_ROOT="$(mktemp -d /tmp/vansour-sqlite-e2e-work-XXXXXX)"
ADMIN_COOKIE_JAR="${TMP_ROOT}/admin.cookies.txt"
USER_COOKIE_JAR="${TMP_ROOT}/user.cookies.txt"
USER_COOKIE_JAR_NEW="${TMP_ROOT}/user-new.cookies.txt"
TINY_PNG_PATH="${TMP_ROOT}/tiny.png"
SCRIPT_FAILED=0

start_stack() {
  compose up -d --build
}

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

  expect_eq "${overall_status}" "healthy" "overall health status should remain healthy"
  expect_eq \
    "${actual_cache_status}" \
    "${expected_cache_status}" \
    "cache component status should match CACHE_MODE=${CACHE_MODE}"
}

on_error() {
  SCRIPT_FAILED=1
  echo >&2
  echo "SQLite E2E smoke failed. Recent compose state:" >&2
  compose ps >&2 || true
  compose logs --no-color --tail=200 >&2 || true
}

cleanup() {
  if [[ "${SCRIPT_FAILED}" == "1" && "${PRESERVE_STACK_ON_FAILURE}" == "1" ]]; then
    echo "Preserving stack because PRESERVE_STACK_ON_FAILURE=1" >&2
    echo "Compose project: ${COMPOSE_PROJECT_NAME}" >&2
    echo "Data dir: ${DATA_DIR}" >&2
    echo "Workspace tmp dir: ${TMP_ROOT}" >&2
  else
    compose down -v --remove-orphans >/dev/null 2>&1 || true
    rm -rf "${TMP_ROOT}"
    if [[ "${TEMP_DATA_DIR}" == "1" ]]; then
      rm -rf "${DATA_DIR}"
    fi
  fi
}

trap on_error ERR
trap cleanup EXIT

health_url="http://127.0.0.1:${APP_HOST_PORT}/health"
mailpit_url="http://127.0.0.1:${MAILPIT_HTTP_PORT}/api/v1"
api_base="http://127.0.0.1:${APP_HOST_PORT}/api/v1"

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

expect_non_200() {
  local status="$1"
  local message="$2"
  if [[ "${status}" == "200" ]]; then
    echo "Assertion failed: ${message}. Unexpected HTTP 200" >&2
    exit 1
  fi
}

mailpit_message_id() {
  local recipient="$1"
  local subject="$2"
  local timeout_seconds="${3:-60}"
  local deadline=$((SECONDS + timeout_seconds))

  while (( SECONDS < deadline )); do
    local message_id
    message_id="$(
      curl -fsS "${mailpit_url}/messages" | jq -r \
        --arg recipient "${recipient}" \
        --arg subject "${subject}" \
        '.messages
         | map(select(.Subject == $subject and ([.To[].Address] | index($recipient))))
         | first
         | .ID // empty'
    )"
    if [[ -n "${message_id}" ]]; then
      printf '%s' "${message_id}"
      return 0
    fi
    sleep 2
  done

  echo "Timed out waiting for Mailpit message: ${subject} -> ${recipient}" >&2
  return 1
}

mailpit_extract_token() {
  local message_id="$1"
  local token
  token="$(
    curl -fsS "${mailpit_url}/message/${message_id}" \
      | jq -r '.Text // .HTML // ""' \
      | tr '\r' '\n' \
      | grep -Eo 'token=[^&[:space:]]+' \
      | head -n 1 \
      | cut -d '=' -f 2-
  )"
  if [[ -z "${token}" ]]; then
    echo "Failed to extract token from Mailpit message ${message_id}" >&2
    exit 1
  fi
  printf '%s' "${token}"
}

log_step "Preparing tiny PNG fixture"
printf '%s' 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAAAAAA6fptVAAAACklEQVR4nGNgAAAAAgABSK+kcQAAAABJRU5ErkJggg==' \
  | base64 -d > "${TINY_PNG_PATH}"

log_step "Starting SQLite + Mailpit stack"
echo "Cache mode: ${CACHE_MODE}"
start_stack
wait_for_url "${health_url}" 180
assert_cache_health_component
wait_for_url "${mailpit_url}/info" 120

BOOTSTRAP_STATUS="$(
  curl -fsS "${api_base}/bootstrap/status"
)"
BOOTSTRAP_MODE="$(printf '%s' "${BOOTSTRAP_STATUS}" | jq -r '.mode')"

if [[ "${BOOTSTRAP_MODE}" == "bootstrap" ]]; then
  log_step "No DATABASE_URL preset detected, writing SQLite bootstrap fallback config"
  curl -fsS \
    -X PUT "${api_base}/bootstrap/database-config" \
    -H 'Content-Type: application/json' \
    -d "$(jq -n \
      --arg database_kind "sqlite" \
      --arg database_url "${SQLITE_DATABASE_URL}" \
      '{database_kind: $database_kind, database_url: $database_url}')" \
    >/dev/null

  compose restart app >/dev/null
else
  log_step "SQLite runtime already preconfigured by compose"
fi

wait_for_url "${api_base}/install/status" 180
assert_cache_health_component

log_step "Completing installation wizard with runtime mail config"
INSTALL_PAYLOAD="$(
  jq -n \
    --arg admin_email "${ADMIN_EMAIL}" \
    --arg admin_password "${ADMIN_PASSWORD}" \
    --arg site_name "${SITE_NAME}" \
    --arg local_storage_path "/data/images" \
    --arg smtp_host "mailpit" \
    --arg from_email "${MAIL_FROM_EMAIL}" \
    --arg from_name "${MAIL_FROM_NAME}" \
    --arg link_base_url "${LINK_BASE_URL}" \
    '{
      admin_email: $admin_email,
      admin_password: $admin_password,
      favicon_data_url: null,
      config: {
        site_name: $site_name,
        storage_backend: "local",
        local_storage_path: $local_storage_path,
        mail_enabled: true,
        mail_smtp_host: $smtp_host,
        mail_smtp_port: 1025,
        mail_smtp_user: null,
        mail_smtp_password: null,
        mail_from_email: $from_email,
        mail_from_name: $from_name,
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
  -d "${INSTALL_PAYLOAD}" \
  >/dev/null

ADMIN_ME="$(curl -fsS -b "${ADMIN_COOKIE_JAR}" "${api_base}/auth/me")"
expect_eq \
  "$(printf '%s' "${ADMIN_ME}" | jq -r '.email')" \
  "${ADMIN_EMAIL}" \
  "admin session after installation should be active"

log_step "Registering and verifying a normal user through Mailpit"
curl -fsS \
  -X POST "${api_base}/auth/register" \
  -H 'Content-Type: application/json' \
  -d "$(jq -n --arg email "${USER_EMAIL}" --arg password "${USER_PASSWORD}" '{email: $email, password: $password}')" \
  >/dev/null

VERIFY_MESSAGE_ID="$(mailpit_message_id "${USER_EMAIL}" "Vansour Image 邮箱验证" 60)"
VERIFY_TOKEN="$(mailpit_extract_token "${VERIFY_MESSAGE_ID}")"

curl -fsS \
  -X POST "${api_base}/auth/register/verify" \
  -H 'Content-Type: application/json' \
  -d "$(jq -n --arg token "${VERIFY_TOKEN}" '{token: $token}')" \
  >/dev/null

log_step "Exercising login, refresh, and auth-protected image flows"
curl -fsS \
  -c "${USER_COOKIE_JAR}" \
  -X POST "${api_base}/auth/login" \
  -H 'Content-Type: application/json' \
  -d "$(jq -n --arg email "${USER_EMAIL}" --arg password "${USER_PASSWORD}" '{email: $email, password: $password}')" \
  >/dev/null

curl -fsS \
  -b "${USER_COOKIE_JAR}" \
  -c "${USER_COOKIE_JAR}" \
  -X POST "${api_base}/auth/refresh" \
  >/dev/null

USER_ME="$(curl -fsS -b "${USER_COOKIE_JAR}" "${api_base}/auth/me")"
expect_eq \
  "$(printf '%s' "${USER_ME}" | jq -r '.email')" \
  "${USER_EMAIL}" \
  "verified user should stay logged in after refresh"

UPLOAD_RESPONSE="$(
  curl -fsS \
    -b "${USER_COOKIE_JAR}" \
    -F "file=@${TINY_PNG_PATH};filename=tiny.png;type=image/png" \
    "${api_base}/upload"
)"
IMAGE_KEY="$(printf '%s' "${UPLOAD_RESPONSE}" | jq -r '.image_key')"
if [[ -z "${IMAGE_KEY}" || "${IMAGE_KEY}" == "null" ]]; then
  echo "Upload did not return image_key" >&2
  exit 1
fi

IMAGES_PAGE="$(curl -fsS -b "${USER_COOKIE_JAR}" "${api_base}/images?page=1&page_size=20")"
expect_eq \
  "$(printf '%s' "${IMAGES_PAGE}" | jq -r '.total')" \
  "1" \
  "image list should contain the uploaded image"

IMAGE_DETAIL="$(curl -fsS -b "${USER_COOKIE_JAR}" "${api_base}/images/${IMAGE_KEY}")"
expect_eq \
  "$(printf '%s' "${IMAGE_DETAIL}" | jq -r '.image_key')" \
  "${IMAGE_KEY}" \
  "image detail should return the requested image"

curl -fsS \
  -b "${USER_COOKIE_JAR}" \
  -X PUT "${api_base}/images/${IMAGE_KEY}" \
  -H 'Content-Type: application/json' \
  -d '{"tags":["sqlite","smoke"]}' \
  >/dev/null

EXPIRED_AT="$(date -u -Iseconds -d '2 minutes ago')"
curl -fsS \
  -b "${USER_COOKIE_JAR}" \
  -X PUT "${api_base}/images/${IMAGE_KEY}/expiry" \
  -H 'Content-Type: application/json' \
  -d "$(jq -n --arg expires_at "${EXPIRED_AT}" '{expires_at: $expires_at}')" \
  >/dev/null

EXPIRED_CLEANUP_COUNT="$(
  curl -fsS -b "${ADMIN_COOKIE_JAR}" -X POST "${api_base}/cleanup/expired"
)"
expect_eq \
  "${EXPIRED_CLEANUP_COUNT}" \
  "1" \
  "expired image cleanup should move one image to trash"

DELETED_PAGE="$(curl -fsS -b "${USER_COOKIE_JAR}" "${api_base}/images/deleted?page=1&page_size=20")"
expect_eq \
  "$(printf '%s' "${DELETED_PAGE}" | jq -r '.total')" \
  "1" \
  "deleted list should contain the expired image"

curl -fsS \
  -b "${USER_COOKIE_JAR}" \
  -X POST "${api_base}/images/restore" \
  -H 'Content-Type: application/json' \
  -d "$(jq -n --arg image_key "${IMAGE_KEY}" '{image_keys: [$image_key]}')" \
  >/dev/null

curl -fsS \
  -b "${USER_COOKIE_JAR}" \
  -X DELETE "${api_base}/images" \
  -H 'Content-Type: application/json' \
  -d "$(jq -n --arg image_key "${IMAGE_KEY}" '{image_keys: [$image_key], permanent: false}')" \
  >/dev/null

DELETED_PAGE_AFTER_SOFT_DELETE="$(curl -fsS -b "${USER_COOKIE_JAR}" "${api_base}/images/deleted?page=1&page_size=20")"
expect_eq \
  "$(printf '%s' "${DELETED_PAGE_AFTER_SOFT_DELETE}" | jq -r '.total')" \
  "1" \
  "soft delete should move the image into deleted list"

curl -fsS \
  -b "${USER_COOKIE_JAR}" \
  -X POST "${api_base}/images/restore" \
  -H 'Content-Type: application/json' \
  -d "$(jq -n --arg image_key "${IMAGE_KEY}" '{image_keys: [$image_key]}')" \
  >/dev/null

curl -fsS \
  -b "${USER_COOKIE_JAR}" \
  -X DELETE "${api_base}/images" \
  -H 'Content-Type: application/json' \
  -d "$(jq -n --arg image_key "${IMAGE_KEY}" '{image_keys: [$image_key], permanent: true}')" \
  >/dev/null

IMAGES_PAGE_AFTER_DELETE="$(curl -fsS -b "${USER_COOKIE_JAR}" "${api_base}/images?page=1&page_size=20")"
expect_eq \
  "$(printf '%s' "${IMAGES_PAGE_AFTER_DELETE}" | jq -r '.total')" \
  "0" \
  "permanent delete should remove the image from active list"

log_step "Exercising logout and password reset"
curl -fsS \
  -b "${USER_COOKIE_JAR}" \
  -c "${USER_COOKIE_JAR}" \
  -X POST "${api_base}/auth/logout" \
  >/dev/null

POST_LOGOUT_STATUS="$(
  curl -s -o /dev/null -w '%{http_code}' -b "${USER_COOKIE_JAR}" "${api_base}/auth/me"
)"
expect_eq "${POST_LOGOUT_STATUS}" "401" "logout should invalidate the active user session"

curl -fsS \
  -c "${USER_COOKIE_JAR}" \
  -X POST "${api_base}/auth/login" \
  -H 'Content-Type: application/json' \
  -d "$(jq -n --arg email "${USER_EMAIL}" --arg password "${USER_PASSWORD}" '{email: $email, password: $password}')" \
  >/dev/null

curl -fsS \
  -X POST "${api_base}/auth/password-reset/request" \
  -H 'Content-Type: application/json' \
  -d "$(jq -n --arg email "${USER_EMAIL}" '{email: $email}')" \
  >/dev/null

RESET_MESSAGE_ID="$(mailpit_message_id "${USER_EMAIL}" "Vansour Image 密码重置" 60)"
RESET_TOKEN="$(mailpit_extract_token "${RESET_MESSAGE_ID}")"

curl -fsS \
  -b "${USER_COOKIE_JAR}" \
  -c "${USER_COOKIE_JAR}" \
  -X POST "${api_base}/auth/password-reset/confirm" \
  -H 'Content-Type: application/json' \
  -d "$(jq -n --arg token "${RESET_TOKEN}" --arg new_password "${USER_NEW_PASSWORD}" '{token: $token, new_password: $new_password}')" \
  >/dev/null

POST_RESET_STATUS="$(
  curl -s -o /dev/null -w '%{http_code}' -b "${USER_COOKIE_JAR}" "${api_base}/auth/me"
)"
expect_eq \
  "${POST_RESET_STATUS}" \
  "401" \
  "password reset should invalidate the previous session token version"

OLD_PASSWORD_STATUS="$(
  curl -s -o /dev/null -w '%{http_code}' \
    -X POST "${api_base}/auth/login" \
    -H 'Content-Type: application/json' \
    -d "$(jq -n --arg email "${USER_EMAIL}" --arg password "${USER_PASSWORD}" '{email: $email, password: $password}')"
)"
expect_non_200 "${OLD_PASSWORD_STATUS}" "old password should stop working after password reset"

curl -fsS \
  -c "${USER_COOKIE_JAR_NEW}" \
  -X POST "${api_base}/auth/login" \
  -H 'Content-Type: application/json' \
  -d "$(jq -n --arg email "${USER_EMAIL}" --arg password "${USER_NEW_PASSWORD}" '{email: $email, password: $password}')" \
  >/dev/null

RESET_LOGIN_ME="$(curl -fsS -b "${USER_COOKIE_JAR_NEW}" "${api_base}/auth/me")"
expect_eq \
  "$(printf '%s' "${RESET_LOGIN_ME}" | jq -r '.email')" \
  "${USER_EMAIL}" \
  "new password should allow login"

log_step "Exercising admin-side user queries, role update, stats, and audit pagination"
USERS_BEFORE_ROLE_UPDATE="$(curl -fsS -b "${ADMIN_COOKIE_JAR}" "${api_base}/users")"
expect_eq \
  "$(printf '%s' "${USERS_BEFORE_ROLE_UPDATE}" | jq -r 'length')" \
  "2" \
  "admin users list should contain admin and registered user"

REGISTERED_USER_ID="$(
  printf '%s' "${USERS_BEFORE_ROLE_UPDATE}" | jq -r --arg user_email "${USER_EMAIL}" '.[] | select(.email == $user_email) | .id'
)"
if [[ -z "${REGISTERED_USER_ID}" || "${REGISTERED_USER_ID}" == "null" ]]; then
  echo "Failed to find registered user in admin users list" >&2
  exit 1
fi

curl -fsS \
  -b "${ADMIN_COOKIE_JAR}" \
  -X PUT "${api_base}/users/${REGISTERED_USER_ID}" \
  -H 'Content-Type: application/json' \
  -d '{"role":"admin"}' \
  >/dev/null

curl -fsS \
  -b "${ADMIN_COOKIE_JAR}" \
  -X PUT "${api_base}/users/${REGISTERED_USER_ID}" \
  -H 'Content-Type: application/json' \
  -d '{"role":"user"}' \
  >/dev/null

SYSTEM_STATS="$(curl -fsS -b "${ADMIN_COOKIE_JAR}" "${api_base}/stats")"
expect_eq \
  "$(printf '%s' "${SYSTEM_STATS}" | jq -r '.total_users')" \
  "2" \
  "system stats should report two users"

AUDIT_PAGE="$(curl -fsS -b "${ADMIN_COOKIE_JAR}" "${api_base}/audit-logs?page=1&page_size=30")"
expect_eq \
  "$(printf '%s' "${AUDIT_PAGE}" | jq -r '.page')" \
  "1" \
  "audit logs pagination should respond with page metadata"

HAS_ROLE_UPDATE_AUDIT="$(
  printf '%s' "${AUDIT_PAGE}" | jq -r '[.data[].action == "admin.user.role_updated"] | any'
)"
expect_eq \
  "${HAS_ROLE_UPDATE_AUDIT}" \
  "true" \
  "audit log page should include the role update event"

echo
echo "SQLite E2E smoke passed"
