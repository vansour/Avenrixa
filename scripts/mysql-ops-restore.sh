#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/mysql-ops-common.sh"

APP_HEALTH_TIMEOUT_SECONDS="${APP_HEALTH_TIMEOUT_SECONDS:-120}"
START_APP_AFTER_RESTORE="${START_APP_AFTER_RESTORE:-1}"

require_commands

restore_manifest_path="${MYSQL_RESTORE_MANIFEST_PATH:-}"
restore_sql_path="${1:-${MYSQL_RESTORE_SQL_PATH:-}}"
restore_data_archive_path="${2:-${MYSQL_RESTORE_DATA_ARCHIVE:-}}"
manifest_dump_sha256=""
manifest_data_archive_sha256=""

if [[ -n "${restore_manifest_path}" ]]; then
  ensure_json_file "${restore_manifest_path}" "MySQL/MariaDB restore manifest"
  restore_sql_path="$(jq -r '.dump.path // empty' "${restore_manifest_path}")"
  restore_data_archive_path="$(jq -r '.data_snapshot.path // empty' "${restore_manifest_path}")"
  manifest_dump_sha256="$(jq -r '.dump.sha256 // empty' "${restore_manifest_path}")"
  manifest_data_archive_sha256="$(jq -r '.data_snapshot.sha256 // empty' "${restore_manifest_path}")"
fi

if [[ -z "${restore_sql_path}" ]]; then
  echo "Usage: MYSQL_RESTORE_SQL_PATH=/path/to/backup.mysql.sql [MYSQL_RESTORE_DATA_ARCHIVE=/path/to/data.tar.gz] $0" >&2
  echo "   or: MYSQL_RESTORE_MANIFEST_PATH=/path/to/backup.manifest.json $0" >&2
  exit 1
fi

if [[ -n "${restore_manifest_path}" ]]; then
  verify_file_sha256 "${restore_sql_path}" "${manifest_dump_sha256}" "MySQL/MariaDB restore SQL file"
  if [[ -n "${restore_data_archive_path}" ]]; then
    verify_file_sha256 "${restore_data_archive_path}" "${manifest_data_archive_sha256}" "MySQL/MariaDB restore data archive"
  fi
else
  ensure_file_nonempty "${restore_sql_path}" "MySQL/MariaDB restore SQL file"
  if [[ -n "${restore_data_archive_path}" ]]; then
    ensure_file_nonempty "${restore_data_archive_path}" "MySQL/MariaDB restore data archive"
  fi
fi

tmp_root="$(mktemp -d /tmp/vansour-mysql-restore-XXXXXX)"
cleanup_tmp() {
  rm -rf "${tmp_root}"
}
trap cleanup_tmp EXIT

staged_restore_sql_path="${tmp_root}/restore.mysql.sql"
cp "${restore_sql_path}" "${staged_restore_sql_path}"
ensure_file_nonempty "${staged_restore_sql_path}" "Staged MySQL/MariaDB restore SQL file"

staged_restore_data_archive_path=""
if [[ -n "${restore_data_archive_path}" ]]; then
  staged_restore_data_archive_path="${tmp_root}/restore.data.tar.gz"
  cp "${restore_data_archive_path}" "${staged_restore_data_archive_path}"
  ensure_file_nonempty "${staged_restore_data_archive_path}" "Staged MySQL/MariaDB restore data archive"
fi

mkdir -p "${ARTIFACT_DIR}"

stamp="$(current_stamp_compact)"
started_at="$(current_timestamp_utc)"
rollback_sql_path="${ARTIFACT_DIR}/rollback_before_restore_${stamp}.mysql.sql"
rollback_data_archive_path=""
if [[ -n "${restore_data_archive_path}" ]]; then
  rollback_data_archive_path="${ARTIFACT_DIR}/rollback_before_restore_${stamp}.data.tar.gz"
fi

result_status="failed"
result_message=""
finished_at=""

write_restore_result() {
  local status="$1"
  local message="$2"
  local finished
  finished="$(current_timestamp_utc)"

  local payload
  payload="$(
    jq -n \
      --arg status "${status}" \
      --arg message "${message}" \
      --arg started_at "${started_at}" \
      --arg finished_at "${finished}" \
      --arg restore_manifest_path "${restore_manifest_path}" \
      --arg requested_restore_sql_path "${restore_sql_path}" \
      --arg requested_restore_data_archive_path "${restore_data_archive_path}" \
      --arg manifest_dump_sha256 "${manifest_dump_sha256}" \
      --arg manifest_data_archive_sha256 "${manifest_data_archive_sha256}" \
      --arg rollback_sql_path "${rollback_sql_path}" \
      --arg rollback_data_archive_path "${rollback_data_archive_path}" \
      --arg compose_project_name "${COMPOSE_PROJECT_NAME}" \
      --argjson compose_files "$(printf '%s\n' "${compose_files[@]}" | jq -R . | jq -s .)" \
      --arg app_health_url "${APP_HEALTH_URL}" \
      '{
        status: $status,
        message: $message,
        started_at: $started_at,
        finished_at: $finished_at,
        restore_manifest_path: (if $restore_manifest_path == "" then null else $restore_manifest_path end),
        requested_restore_sql_path: $requested_restore_sql_path,
        requested_restore_data_archive_path: (if $requested_restore_data_archive_path == "" then null else $requested_restore_data_archive_path end),
        manifest_dump_sha256: (if $manifest_dump_sha256 == "" then null else $manifest_dump_sha256 end),
        manifest_data_archive_sha256: (if $manifest_data_archive_sha256 == "" then null else $manifest_data_archive_sha256 end),
        rollback_sql_path: $rollback_sql_path,
        rollback_data_archive_path: (if $rollback_data_archive_path == "" then null else $rollback_data_archive_path end),
        compose_project_name: $compose_project_name,
        compose_files: $compose_files,
        app_health_url: $app_health_url
      }'
  )"

  write_json_file "${MYSQL_LAST_RESTORE_RESULT_PATH}" "${payload}"
}

restore_rollback_artifacts() {
  local rollback_error=""

  if [[ -n "${rollback_data_archive_path}" ]]; then
    if [[ -f "${rollback_data_archive_path}" && -s "${rollback_data_archive_path}" ]]; then
      rm -rf "${ROOT_DIR}/${DATA_DIR}"
      tar -xzf "${rollback_data_archive_path}" -C "${ROOT_DIR}"
    else
      rollback_error="data rollback artifact missing"
    fi
  fi

  if [[ -z "${rollback_error}" ]]; then
    if [[ -f "${rollback_sql_path}" && -s "${rollback_sql_path}" ]]; then
      mysql_restore_from_file "${rollback_sql_path}"
    else
      rollback_error="database rollback artifact missing"
    fi
  fi

  if [[ "${START_APP_AFTER_RESTORE}" == "1" ]]; then
    compose start "${APP_SERVICE}" >/dev/null
    if ! wait_for_app_health "${APP_HEALTH_TIMEOUT_SECONDS}"; then
      if [[ -n "${rollback_error}" ]]; then
        rollback_error="${rollback_error}; app health check failed after rollback"
      else
        rollback_error="app health check failed after rollback"
      fi
    fi
  fi

  if [[ -n "${rollback_error}" ]]; then
    result_status="failed"
    result_message="Restore failed and rollback did not complete cleanly: ${rollback_error}"
    return 1
  fi

  result_status="rolled_back"
  result_message="Restore failed. Rollback snapshot has been applied."
  return 0
}

log_step "Checking MySQL/MariaDB stack"
ensure_stack_service_running "${MYSQL_SERVICE}"

log_step "Stopping app before restore"
compose stop "${APP_SERVICE}" >/dev/null || true

log_step "Creating rollback database snapshot"
mysql_dump_to_file "${rollback_sql_path}"

if [[ -n "${restore_data_archive_path}" ]]; then
  log_step "Creating rollback data snapshot"
  tar -C "${ROOT_DIR}" -czf "${rollback_data_archive_path}" "${DATA_DIR}"
  ensure_file_nonempty "${rollback_data_archive_path}" "Rollback data snapshot"
fi

perform_restore() {
  if [[ -n "${restore_data_archive_path}" ]]; then
    log_step "Restoring data directory snapshot"
    rm -rf "${ROOT_DIR}/${DATA_DIR}"
    tar -xzf "${staged_restore_data_archive_path}" -C "${ROOT_DIR}" || return 1
  fi

  log_step "Restoring MySQL/MariaDB logical backup"
  mysql_restore_from_file "${staged_restore_sql_path}" || return 1

  if [[ "${START_APP_AFTER_RESTORE}" == "1" ]]; then
    log_step "Starting app after restore"
    compose start "${APP_SERVICE}" >/dev/null || return 1
    wait_for_app_health "${APP_HEALTH_TIMEOUT_SECONDS}" || return 1
  fi

  return 0
}

set +e
perform_restore
restore_exit_code=$?
set -e

if [[ "${restore_exit_code}" -eq 0 ]]; then
  result_status="completed"
  result_message="Restore completed successfully."
  write_restore_result "${result_status}" "${result_message}"
  echo "MySQL/MariaDB ops restore completed successfully."
  echo "  result: ${MYSQL_LAST_RESTORE_RESULT_PATH}"
  echo "  rollback database snapshot: ${rollback_sql_path}"
  if [[ -n "${rollback_data_archive_path}" ]]; then
    echo "  rollback data snapshot: ${rollback_data_archive_path}"
  fi
  exit 0
fi

log_step "Restore failed, applying rollback"
set +e
restore_rollback_artifacts
rollback_exit_code=$?
set -e

write_restore_result "${result_status}" "${result_message}"

if [[ "${rollback_exit_code}" -eq 0 ]]; then
  echo "${result_message}" >&2
  echo "  result: ${MYSQL_LAST_RESTORE_RESULT_PATH}" >&2
  exit 1
fi

echo "${result_message}" >&2
echo "  result: ${MYSQL_LAST_RESTORE_RESULT_PATH}" >&2
exit 1
