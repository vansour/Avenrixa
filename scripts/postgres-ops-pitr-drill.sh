#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

COMPOSE_VARIANT="${COMPOSE_VARIANT:-postgres}"
APP_HOST_PORT="${APP_HOST_PORT:-8080}"
PRESERVE_STACK_ON_FAILURE="${PRESERVE_STACK_ON_FAILURE:-0}"
DRILL_ADMIN_EMAIL="${DRILL_ADMIN_EMAIL:-admin@example.com}"
DRILL_ADMIN_PASSWORD="${DRILL_ADMIN_PASSWORD:-Password123456!}"
DRILL_SITE_NAME_BASELINE="${DRILL_SITE_NAME_BASELINE:-PostgreSQL PITR Drill Baseline}"
DRILL_SITE_NAME_TARGET="${DRILL_SITE_NAME_TARGET:-PostgreSQL PITR Drill Target}"
DRILL_SITE_NAME_AFTER_TARGET="${DRILL_SITE_NAME_AFTER_TARGET:-PostgreSQL PITR Drill After Target}"
DRILL_LINK_BASE_URL="${DRILL_LINK_BASE_URL:-http://127.0.0.1:${APP_HOST_PORT}/login}"
DATA_DIR="${DATA_DIR:-data}"
PITR_TARGET_MODE="${PITR_TARGET_MODE:-name}"
COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME:-vansour-image-postgres-pitr-drill-${PITR_TARGET_MODE}}"
ARTIFACT_DIR="${ARTIFACT_DIR:-ops-backups/postgres-pitr-drill-${PITR_TARGET_MODE}}"
POSTGRES_ENABLE_WAL_ARCHIVE="${POSTGRES_ENABLE_WAL_ARCHIVE:-1}"
POSTGRES_WAL_ARCHIVE_HOST_DIR="${POSTGRES_WAL_ARCHIVE_HOST_DIR:-${ARTIFACT_DIR}/wal-archive}"
POSTGRES_WAL_REMOTE_URI="${POSTGRES_WAL_REMOTE_URI:-file://${ARTIFACT_DIR}/wal-remote}"
TMP_PARENT_DIR="${TMP_PARENT_DIR:-${TMPDIR:-${ROOT_DIR}/tmp}}"

source "${ROOT_DIR}/scripts/postgres-ops-common.sh"

api_base="http://127.0.0.1:${APP_HOST_PORT}/api/v1"
health_url="http://127.0.0.1:${APP_HOST_PORT}/health"

SCRIPT_FAILED=0
TMP_ROOT=""
COOKIE_JAR=""

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

on_error() {
  SCRIPT_FAILED=1
  echo >&2
  echo "PostgreSQL PITR drill failed. Recent compose state:" >&2
  compose ps >&2 || true
  compose logs --no-color --tail=300 >&2 || true
}

cleanup() {
  if [[ "${SCRIPT_FAILED}" == "1" && "${PRESERVE_STACK_ON_FAILURE}" == "1" ]]; then
    echo "Preserving stack because PRESERVE_STACK_ON_FAILURE=1" >&2
    echo "Compose project: ${COMPOSE_PROJECT_NAME}" >&2
    echo "Artifact dir: ${ARTIFACT_DIR}" >&2
    echo "WAL archive dir: ${POSTGRES_WAL_ARCHIVE_HOST_DIR}" >&2
    if [[ -n "${TMP_ROOT}" ]]; then
      echo "Workspace tmp dir: ${TMP_ROOT}" >&2
    fi
  else
    compose down -v --remove-orphans >/dev/null 2>&1 || true
    compose_remove_host_path "${DATA_DIR}" >/dev/null 2>&1 || true
    compose_remove_host_path "${ARTIFACT_DIR}" >/dev/null 2>&1 || true
    if [[ -n "${TMP_ROOT}" ]]; then
      rm -rf "${TMP_ROOT}"
    fi
  fi
}

trap on_error ERR
trap cleanup EXIT

prepare_fixture() {
  mkdir -p "${TMP_PARENT_DIR}"
  TMP_ROOT="$(mktemp -d "${TMP_PARENT_DIR%/}/vansour-postgres-pitr-drill-XXXXXX")"
  COOKIE_JAR="${TMP_ROOT}/admin.cookies.txt"
  compose_reset_host_dir "${DATA_DIR}"
  compose_reset_host_dir "${ARTIFACT_DIR}"
  mkdir -p "${POSTGRES_WAL_ARCHIVE_HOST_DIR}"
  chmod 0777 "${POSTGRES_WAL_ARCHIVE_HOST_DIR}"
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

    log_step "No DATABASE_URL preset detected, writing PostgreSQL bootstrap fallback config"
    curl -fsS \
      -X PUT "${api_base}/bootstrap/database-config" \
      -H 'Content-Type: application/json' \
      -d "$(jq -n \
        --arg database_kind "postgresql" \
        --arg database_url "$(compose_variant_default_database_url)" \
        '{database_kind: $database_kind, database_url: $database_url}')" \
      >/dev/null

    log_step "Restarting app to enter runtime mode"
    compose restart app >/dev/null
  else
    expect_eq "${mode}" "runtime" "PostgreSQL PITR drill should expose runtime bootstrap status"
    expect_eq "${database_kind}" "postgresql" "PostgreSQL PITR drill should run with postgresql database kind"
    expect_eq "${configured}" "true" "PostgreSQL PITR drill should run with configured database"
  fi

  wait_for_url "${api_base}/install/status" 180
}

install_postgres_app() {
  local install_status
  local install_payload
  local admin_me

  install_status="$(curl -fsS "${api_base}/install/status")"
  expect_eq "$(printf '%s' "${install_status}" | jq -r '.installed')" "false" "PostgreSQL PITR drill requires an uninstalled database"

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
  expect_eq "$(printf '%s' "${admin_me}" | jq -r '.email')" "${DRILL_ADMIN_EMAIL}" "admin session should be active after PITR drill installation"
}

postgres_site_name() {
  postgres_query_value "SELECT value FROM settings WHERE settings.\"key\" = 'site_name';"
}

set_postgres_site_name() {
  local next_value="$1"
  local escaped_value
  escaped_value="$(postgres_escape_sql_literal "${next_value}")"
  postgres_exec_sql "UPDATE settings SET value = '${escaped_value}' WHERE settings.\"key\" = 'site_name';"
}

run_drill() {
  local physical_manifest_path
  local pitr_target_restore_point
  local resolved_wal_archive_dir
  local pitr_target_time
  local resolved_wal_remote_dir

  prepare_fixture
  resolved_wal_archive_dir="$(cd "${POSTGRES_WAL_ARCHIVE_HOST_DIR}" && pwd -P)"
  resolved_wal_remote_dir="$(cd "${ARTIFACT_DIR}" && mkdir -p wal-remote && cd wal-remote && pwd -P)"

  log_step "Starting PostgreSQL PITR drill stack"
  compose up --build -d

  log_step "Waiting for app health"
  wait_for_url "${health_url}" 180

  configure_postgres_runtime
  install_postgres_app

  expect_eq "$(postgres_site_name)" "${DRILL_SITE_NAME_BASELINE}" "baseline site name should be installed value"

  log_step "Creating PostgreSQL physical backup base with WAL archive enabled"
  COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME}" \
  COMPOSE_VARIANT="${COMPOSE_VARIANT}" \
  ARTIFACT_DIR="${ARTIFACT_DIR}" \
  POSTGRES_ENABLE_WAL_ARCHIVE="${POSTGRES_ENABLE_WAL_ARCHIVE}" \
  POSTGRES_WAL_ARCHIVE_HOST_DIR="${POSTGRES_WAL_ARCHIVE_HOST_DIR}" \
  POSTGRES_WAL_REMOTE_URI="${POSTGRES_WAL_REMOTE_URI}" \
  INCLUDE_DATA_SNAPSHOT=0 \
  ./scripts/postgres-ops-backup.sh

  physical_manifest_path="${DATA_DIR}/backup/postgres_last_physical_backup_manifest.json"
  if [[ ! -f "${physical_manifest_path}" ]]; then
    echo "Physical backup manifest not found: ${physical_manifest_path}" >&2
    exit 1
  fi

  expect_eq "$(jq -r '.backup_method' "${physical_manifest_path}")" "physical" "physical manifest should report physical backup method"
  expect_eq "$(jq -r '.wal_archive.enabled' "${physical_manifest_path}")" "true" "physical manifest should report WAL archive enabled"
  expect_eq "$(jq -r '.wal_archive.host_dir' "${physical_manifest_path}")" "${resolved_wal_archive_dir}" "physical manifest should record WAL archive dir"
  expect_eq "$(jq -r '.wal_archive.remote_uri' "${physical_manifest_path}")" "file://${resolved_wal_remote_dir}" "physical manifest should record WAL remote uri"
  if [[ "$(jq -r '.wal_archive.restore_point.name // empty' "${physical_manifest_path}")" == "" ]]; then
    echo "Physical backup manifest is missing the PITR restore point metadata" >&2
    exit 1
  fi

  log_step "Mutating database to PITR target state"
  set_postgres_site_name "${DRILL_SITE_NAME_TARGET}"
  expect_eq "$(postgres_site_name)" "${DRILL_SITE_NAME_TARGET}" "target site name should be visible before creating restore point"

  if [[ "${PITR_TARGET_MODE}" == "name" ]]; then
    pitr_target_restore_point="vansour_postgres_pitr_target_$(current_stamp_compact)"
    log_step "Creating named restore point ${pitr_target_restore_point}"
    postgres_create_restore_point "${pitr_target_restore_point}" >/dev/null
  elif [[ "${PITR_TARGET_MODE}" == "time" ]]; then
    # Capture the target from PostgreSQL itself so recovery_target_time lands
    # after the target-state commit instead of racing the host clock boundary.
    pitr_target_time="$(postgres_current_timestamp_utc)"
    log_step "Captured PITR target time ${pitr_target_time}"
    sleep 1
  else
    echo "Unsupported PITR_TARGET_MODE: ${PITR_TARGET_MODE}" >&2
    exit 1
  fi
  postgres_force_wal_switch_and_wait "${POSTGRES_WAL_ARCHIVE_HOST_DIR}" 60
  COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME}" \
  COMPOSE_VARIANT="${COMPOSE_VARIANT}" \
  POSTGRES_WAL_ARCHIVE_DIR="${POSTGRES_WAL_ARCHIVE_HOST_DIR}" \
  POSTGRES_WAL_REMOTE_URI="${POSTGRES_WAL_REMOTE_URI}" \
  ./scripts/postgres-ops-wal-sync.sh push

  log_step "Applying changes after PITR target"
  set_postgres_site_name "${DRILL_SITE_NAME_AFTER_TARGET}"
  expect_eq "$(postgres_site_name)" "${DRILL_SITE_NAME_AFTER_TARGET}" "site name should advance past PITR target before restore"

  log_step "Clearing local WAL archive to force remote pull during restore"
  find "${POSTGRES_WAL_ARCHIVE_HOST_DIR}" -mindepth 1 -maxdepth 1 -type f -delete

  if [[ "${PITR_TARGET_MODE}" == "name" ]]; then
    log_step "Restoring to named PITR restore point"
    COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME}" \
    COMPOSE_VARIANT="${COMPOSE_VARIANT}" \
    ARTIFACT_DIR="${ARTIFACT_DIR}" \
    POSTGRES_ENABLE_WAL_ARCHIVE="${POSTGRES_ENABLE_WAL_ARCHIVE}" \
    POSTGRES_WAL_ARCHIVE_HOST_DIR="${POSTGRES_WAL_ARCHIVE_HOST_DIR}" \
    POSTGRES_WAL_REMOTE_URI="${POSTGRES_WAL_REMOTE_URI}" \
    POSTGRES_RESTORE_MANIFEST_PATH="${physical_manifest_path}" \
    POSTGRES_RESTORE_TARGET_NAME="${pitr_target_restore_point}" \
    POSTGRES_RESTORE_WAL_ARCHIVE_DIR="${POSTGRES_WAL_ARCHIVE_HOST_DIR}" \
    ./scripts/postgres-ops-restore.sh
  else
    log_step "Restoring to PITR target time ${pitr_target_time}"
    COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME}" \
    COMPOSE_VARIANT="${COMPOSE_VARIANT}" \
    ARTIFACT_DIR="${ARTIFACT_DIR}" \
    POSTGRES_ENABLE_WAL_ARCHIVE="${POSTGRES_ENABLE_WAL_ARCHIVE}" \
    POSTGRES_WAL_ARCHIVE_HOST_DIR="${POSTGRES_WAL_ARCHIVE_HOST_DIR}" \
    POSTGRES_WAL_REMOTE_URI="${POSTGRES_WAL_REMOTE_URI}" \
    POSTGRES_RESTORE_MANIFEST_PATH="${physical_manifest_path}" \
    POSTGRES_RESTORE_TARGET_TIME="${pitr_target_time}" \
    POSTGRES_RESTORE_WAL_ARCHIVE_DIR="${POSTGRES_WAL_ARCHIVE_HOST_DIR}" \
    ./scripts/postgres-ops-restore.sh
  fi

  log_step "Validating PITR-restored state"
  expect_eq "$(postgres_site_name)" "${DRILL_SITE_NAME_TARGET}" "site name should return to PITR target after restore"
  expect_eq "$(jq -r '.status' "${DATA_DIR}/backup/postgres_last_restore_result.json")" "completed" "restore result should be completed"
  expect_eq "$(jq -r '.restore_method' "${DATA_DIR}/backup/postgres_last_restore_result.json")" "physical-pitr" "restore result should report physical-pitr restore method"
  if [[ "${PITR_TARGET_MODE}" == "name" ]]; then
    expect_eq "$(jq -r '.pitr_target_kind' "${DATA_DIR}/backup/postgres_last_restore_result.json")" "name" "restore result should report PITR target kind"
    expect_eq "$(jq -r '.pitr_target_value' "${DATA_DIR}/backup/postgres_last_restore_result.json")" "${pitr_target_restore_point}" "restore result should report PITR target value"
  else
    expect_eq "$(jq -r '.pitr_target_kind' "${DATA_DIR}/backup/postgres_last_restore_result.json")" "time" "restore result should report PITR target kind"
    expect_eq "$(jq -r '.pitr_target_value' "${DATA_DIR}/backup/postgres_last_restore_result.json")" "${pitr_target_time}" "restore result should report PITR target value"
  fi
  expect_eq "$(jq -r '.pitr_wal_remote_uri' "${DATA_DIR}/backup/postgres_last_restore_result.json")" "file://${resolved_wal_remote_dir}" "restore result should report PITR WAL remote uri"
  expect_eq "$(jq -r '.data_restore_mode' "${DATA_DIR}/backup/postgres_last_restore_result.json")" "not_requested" "PITR drill should not restore a data snapshot"
  wait_for_url "${health_url}" 180

  echo "PostgreSQL PITR drill passed (${PITR_TARGET_MODE})"
}

require_postgres_variant
require_commands
run_drill
