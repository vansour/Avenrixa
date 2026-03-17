#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/mysql-ops-common.sh"

APP_HEALTH_TIMEOUT_SECONDS="${APP_HEALTH_TIMEOUT_SECONDS:-120}"
MYSQL_HEALTH_TIMEOUT_SECONDS="${MYSQL_HEALTH_TIMEOUT_SECONDS:-120}"
START_APP_AFTER_RESTORE="${START_APP_AFTER_RESTORE:-1}"

require_commands

restore_manifest_path="${MYSQL_RESTORE_MANIFEST_PATH:-}"
restore_sql_path="${1:-${MYSQL_RESTORE_SQL_PATH:-}}"
restore_data_archive_path="${2:-${MYSQL_RESTORE_DATA_ARCHIVE:-}}"
restore_physical_path="${MYSQL_RESTORE_PHYSICAL_PATH:-}"
manifest_dump_sha256=""
manifest_data_archive_sha256=""
manifest_backup_method=""
effective_restore_mode="logical"

if [[ -n "${restore_manifest_path}" ]]; then
  ensure_json_file "${restore_manifest_path}" "MySQL/MariaDB restore manifest"
  manifest_backup_method="$(jq -r '.backup_method // "logical"' "${restore_manifest_path}")"

  case "${manifest_backup_method}" in
    logical)
      effective_restore_mode="logical"
      restore_sql_path="$(jq -r '.dump.path // empty' "${restore_manifest_path}")"
      restore_data_archive_path="$(jq -r '.data_snapshot.path // empty' "${restore_manifest_path}")"
      manifest_dump_sha256="$(jq -r '.dump.sha256 // empty' "${restore_manifest_path}")"
      manifest_data_archive_sha256="$(jq -r '.data_snapshot.sha256 // empty' "${restore_manifest_path}")"
      ;;
    physical)
      effective_restore_mode="physical"
      restore_physical_path="$(jq -r '.physical_backup.path // empty' "${restore_manifest_path}")"
      restore_data_archive_path="$(jq -r '.data_snapshot.path // empty' "${restore_manifest_path}")"
      manifest_data_archive_sha256="$(jq -r '.data_snapshot.sha256 // empty' "${restore_manifest_path}")"
      ;;
    *)
      echo "Unsupported MySQL/MariaDB manifest backup_method: ${manifest_backup_method}" >&2
      exit 1
      ;;
  esac
elif [[ -n "${restore_physical_path}" ]]; then
  effective_restore_mode="physical"
fi

case "${effective_restore_mode}" in
  logical)
    if [[ -z "${restore_sql_path}" ]]; then
      echo "Usage: MYSQL_RESTORE_SQL_PATH=/path/to/backup.mysql.sql [MYSQL_RESTORE_DATA_ARCHIVE=/path/to/data.tar.gz] $0" >&2
      echo "   or: MYSQL_RESTORE_MANIFEST_PATH=/path/to/backup.manifest.json $0" >&2
      if [[ -n "${restore_manifest_path}" ]]; then
        echo "Manifest does not contain a logical dump path that this restore can consume." >&2
      fi
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
    ;;
  physical)
    if [[ -z "${restore_physical_path}" ]]; then
      echo "Usage: MYSQL_RESTORE_MANIFEST_PATH=/path/to/mysql_last_physical_backup_manifest.json $0" >&2
      echo "   or: MYSQL_RESTORE_PHYSICAL_PATH=/path/to/prepared-backup-dir [MYSQL_RESTORE_DATA_ARCHIVE=/path/to/data.tar.gz] $0" >&2
      if [[ -n "${restore_manifest_path}" ]]; then
        echo "Manifest does not contain a physical backup directory path that this restore can consume." >&2
      fi
      exit 1
    fi

    ensure_directory_nonempty "${restore_physical_path}" "MySQL/MariaDB physical backup directory"
    if [[ -n "${restore_data_archive_path}" ]]; then
      if [[ -n "${restore_manifest_path}" ]]; then
        verify_file_sha256 "${restore_data_archive_path}" "${manifest_data_archive_sha256}" "MySQL/MariaDB restore data archive"
      else
        ensure_file_nonempty "${restore_data_archive_path}" "MySQL/MariaDB restore data archive"
      fi
    fi
    ;;
esac

tmp_root=""
cleanup_tmp() {
  if [[ -n "${tmp_root}" ]]; then
    rm -rf "${tmp_root}"
  fi
}
trap cleanup_tmp EXIT

staged_restore_sql_path=""
staged_restore_data_archive_path=""
if [[ "${effective_restore_mode}" == "logical" || -n "${restore_data_archive_path}" ]]; then
  tmp_root="$(mktemp -d /tmp/vansour-mysql-restore-XXXXXX)"

  if [[ "${effective_restore_mode}" == "logical" ]]; then
    staged_restore_sql_path="${tmp_root}/restore.mysql.sql"
    cp "${restore_sql_path}" "${staged_restore_sql_path}"
    ensure_file_nonempty "${staged_restore_sql_path}" "Staged MySQL/MariaDB restore SQL file"
  fi

  if [[ -n "${restore_data_archive_path}" ]]; then
    staged_restore_data_archive_path="${tmp_root}/restore.data.tar.gz"
    cp "${restore_data_archive_path}" "${staged_restore_data_archive_path}"
    ensure_file_nonempty "${staged_restore_data_archive_path}" "Staged MySQL/MariaDB restore data archive"
  fi
fi

mkdir -p "${ARTIFACT_DIR}"

stamp="$(current_stamp_compact)"
started_at="$(current_timestamp_utc)"
rollback_sql_path=""
rollback_datadir_archive_path=""
rollback_datadir_restore_log_path=""
rollback_data_archive_path=""
physical_copy_back_log_path=""

if [[ "${effective_restore_mode}" == "logical" ]]; then
  rollback_sql_path="${ARTIFACT_DIR}/rollback_before_restore_${stamp}.mysql.sql"
else
  rollback_datadir_archive_path="${ARTIFACT_DIR}/rollback_before_restore_${stamp}.mysql-datadir.tar.gz"
  rollback_datadir_restore_log_path="${ARTIFACT_DIR}/rollback_restore_${stamp}.mysql-datadir.log"
  physical_copy_back_log_path="${ARTIFACT_DIR}/restore_${stamp}.physical-copy-back.log"
fi

if [[ -n "${restore_data_archive_path}" ]]; then
  rollback_data_archive_path="${ARTIFACT_DIR}/rollback_before_restore_${stamp}.data.tar.gz"
fi

logical_database_mutated=0
logical_data_mutated=0
mysql_stopped_for_restore=0
physical_datadir_mutated=0
physical_data_mutated=0

result_status="failed"
result_message=""

append_error_message() {
  local current="$1"
  local next="$2"

  if [[ -n "${current}" ]]; then
    printf '%s; %s' "${current}" "${next}"
  else
    printf '%s' "${next}"
  fi
}

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
      --arg restore_method "${effective_restore_mode}" \
      --arg manifest_backup_method "${manifest_backup_method}" \
      --arg requested_restore_sql_path "${restore_sql_path}" \
      --arg requested_restore_physical_path "${restore_physical_path}" \
      --arg requested_restore_data_archive_path "${restore_data_archive_path}" \
      --arg manifest_dump_sha256 "${manifest_dump_sha256}" \
      --arg manifest_data_archive_sha256 "${manifest_data_archive_sha256}" \
      --arg rollback_sql_path "${rollback_sql_path}" \
      --arg rollback_datadir_archive_path "${rollback_datadir_archive_path}" \
      --arg rollback_datadir_restore_log_path "${rollback_datadir_restore_log_path}" \
      --arg rollback_data_archive_path "${rollback_data_archive_path}" \
      --arg physical_copy_back_log_path "${physical_copy_back_log_path}" \
      --arg compose_project_name "${COMPOSE_PROJECT_NAME}" \
      --argjson compose_files "$(compose_files_json)" \
      --arg app_health_url "${APP_HEALTH_URL}" \
      '{
        status: $status,
        message: $message,
        started_at: $started_at,
        finished_at: $finished_at,
        restore_manifest_path: (if $restore_manifest_path == "" then null else $restore_manifest_path end),
        restore_method: $restore_method,
        manifest_backup_method: (if $manifest_backup_method == "" then null else $manifest_backup_method end),
        requested_restore_sql_path: (if $requested_restore_sql_path == "" then null else $requested_restore_sql_path end),
        requested_restore_physical_path: (if $requested_restore_physical_path == "" then null else $requested_restore_physical_path end),
        requested_restore_data_archive_path: (if $requested_restore_data_archive_path == "" then null else $requested_restore_data_archive_path end),
        manifest_dump_sha256: (if $manifest_dump_sha256 == "" then null else $manifest_dump_sha256 end),
        manifest_data_archive_sha256: (if $manifest_data_archive_sha256 == "" then null else $manifest_data_archive_sha256 end),
        rollback_sql_path: (if $rollback_sql_path == "" then null else $rollback_sql_path end),
        rollback_datadir_archive_path: (if $rollback_datadir_archive_path == "" then null else $rollback_datadir_archive_path end),
        rollback_datadir_restore_log_path: (if $rollback_datadir_restore_log_path == "" then null else $rollback_datadir_restore_log_path end),
        rollback_data_archive_path: (if $rollback_data_archive_path == "" then null else $rollback_data_archive_path end),
        physical_copy_back_log_path: (if $physical_copy_back_log_path == "" then null else $physical_copy_back_log_path end),
        compose_project_name: $compose_project_name,
        compose_files: $compose_files,
        app_health_url: $app_health_url
      }'
  )"

  write_json_file "${MYSQL_LAST_RESTORE_RESULT_PATH}" "${payload}"
}

restore_local_data_from_archive() {
  local archive_path="$1"

  rm -rf "${ROOT_DIR:?}/${DATA_DIR}"
  tar -xzf "${archive_path}" -C "${ROOT_DIR}"
}

start_mysql_after_restore() {
  compose start "${MYSQL_SERVICE}" >/dev/null
  wait_for_compose_service_health "${MYSQL_SERVICE}" "${MYSQL_HEALTH_TIMEOUT_SECONDS}"
}

start_app_after_restore() {
  if [[ "${START_APP_AFTER_RESTORE}" == "1" ]]; then
    compose start "${APP_SERVICE}" >/dev/null
    wait_for_app_health "${APP_HEALTH_TIMEOUT_SECONDS}"
  fi
}

restore_rollback_artifacts_logical() {
  local rollback_error=""

  if [[ "${logical_data_mutated}" == "1" ]]; then
    if [[ -f "${rollback_data_archive_path}" && -s "${rollback_data_archive_path}" ]]; then
      restore_local_data_from_archive "${rollback_data_archive_path}" || rollback_error="$(
        append_error_message "${rollback_error}" "failed to restore local data rollback archive"
      )"
    else
      rollback_error="$(append_error_message "${rollback_error}" "data rollback artifact missing")"
    fi
  fi

  if [[ "${logical_database_mutated}" == "1" ]]; then
    if [[ -f "${rollback_sql_path}" && -s "${rollback_sql_path}" ]]; then
      mysql_restore_from_file "${rollback_sql_path}" || rollback_error="$(
        append_error_message "${rollback_error}" "failed to restore database rollback dump"
      )"
    else
      rollback_error="$(append_error_message "${rollback_error}" "database rollback artifact missing")"
    fi
  fi

  if [[ "${START_APP_AFTER_RESTORE}" == "1" ]]; then
    start_app_after_restore || rollback_error="$(
      append_error_message "${rollback_error}" "app health check failed after rollback"
    )"
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

restore_rollback_artifacts_physical() {
  local rollback_error=""

  compose stop "${APP_SERVICE}" >/dev/null || true
  if [[ "${mysql_stopped_for_restore}" == "1" ]]; then
    compose stop "${MYSQL_SERVICE}" >/dev/null || true
  fi

  if [[ "${physical_data_mutated}" == "1" ]]; then
    if [[ -f "${rollback_data_archive_path}" && -s "${rollback_data_archive_path}" ]]; then
      restore_local_data_from_archive "${rollback_data_archive_path}" || rollback_error="$(
        append_error_message "${rollback_error}" "failed to restore local data rollback archive"
      )"
    else
      rollback_error="$(append_error_message "${rollback_error}" "data rollback artifact missing")"
    fi
  fi

  if [[ "${physical_datadir_mutated}" == "1" ]]; then
    if [[ -f "${rollback_datadir_archive_path}" && -s "${rollback_datadir_archive_path}" ]]; then
      mysql_data_dir_restore_from_archive "${rollback_datadir_archive_path}" "${rollback_datadir_restore_log_path}" || rollback_error="$(
        append_error_message "${rollback_error}" "failed to restore MySQL/MariaDB datadir rollback archive"
      )"
    else
      rollback_error="$(append_error_message "${rollback_error}" "database datadir rollback artifact missing")"
    fi
  fi

  if [[ "${mysql_stopped_for_restore}" == "1" ]]; then
    start_mysql_after_restore || rollback_error="$(
      append_error_message "${rollback_error}" "mysql health check failed after rollback"
    )"
  fi

  if [[ "${START_APP_AFTER_RESTORE}" == "1" ]]; then
    start_app_after_restore || rollback_error="$(
      append_error_message "${rollback_error}" "app health check failed after rollback"
    )"
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

perform_logical_restore() {
  log_step "Creating rollback database snapshot"
  mysql_dump_to_file "${rollback_sql_path}" || return 1

  if [[ -n "${staged_restore_data_archive_path}" ]]; then
    log_step "Creating rollback data snapshot"
    tar -C "${ROOT_DIR}" -czf "${rollback_data_archive_path}" "${DATA_DIR}" || return 1
    if [[ ! -f "${rollback_data_archive_path}" || ! -s "${rollback_data_archive_path}" ]]; then
      echo "Rollback data snapshot is missing or empty: ${rollback_data_archive_path}" >&2
      return 1
    fi
  fi

  if [[ -n "${staged_restore_data_archive_path}" ]]; then
    log_step "Restoring data directory snapshot"
    logical_data_mutated=1
    restore_local_data_from_archive "${staged_restore_data_archive_path}" || return 1
  fi

  log_step "Restoring MySQL/MariaDB logical backup"
  logical_database_mutated=1
  mysql_restore_from_file "${staged_restore_sql_path}" || return 1

  if [[ "${START_APP_AFTER_RESTORE}" == "1" ]]; then
    log_step "Starting app after restore"
    start_app_after_restore || return 1
  fi

  return 0
}

perform_physical_restore() {
  log_step "Stopping MySQL/MariaDB before physical restore"
  compose stop "${MYSQL_SERVICE}" >/dev/null || return 1
  mysql_stopped_for_restore=1

  log_step "Creating rollback MySQL/MariaDB datadir snapshot"
  mysql_data_dir_archive_to_file "${rollback_datadir_archive_path}" || return 1

  if [[ -n "${staged_restore_data_archive_path}" ]]; then
    log_step "Creating rollback data snapshot"
    tar -C "${ROOT_DIR}" -czf "${rollback_data_archive_path}" "${DATA_DIR}" || return 1
    if [[ ! -f "${rollback_data_archive_path}" || ! -s "${rollback_data_archive_path}" ]]; then
      echo "Rollback data snapshot is missing or empty: ${rollback_data_archive_path}" >&2
      return 1
    fi
  fi

  log_step "Restoring MySQL/MariaDB physical backup"
  physical_datadir_mutated=1
  mysql_physical_copy_back_from_dir "${restore_physical_path}" "${physical_copy_back_log_path}" || return 1

  if [[ -n "${staged_restore_data_archive_path}" ]]; then
    log_step "Restoring data directory snapshot"
    physical_data_mutated=1
    restore_local_data_from_archive "${staged_restore_data_archive_path}" || return 1
  fi

  log_step "Starting MySQL/MariaDB after restore"
  start_mysql_after_restore || return 1

  if [[ "${START_APP_AFTER_RESTORE}" == "1" ]]; then
    log_step "Starting app after restore"
    start_app_after_restore || return 1
  fi

  return 0
}

perform_restore() {
  case "${effective_restore_mode}" in
    logical)
      perform_logical_restore
      ;;
    physical)
      perform_physical_restore
      ;;
  esac
}

restore_rollback_artifacts() {
  case "${effective_restore_mode}" in
    logical)
      restore_rollback_artifacts_logical
      ;;
    physical)
      restore_rollback_artifacts_physical
      ;;
  esac
}

log_step "Checking MySQL/MariaDB stack"
ensure_stack_service_running "${MYSQL_SERVICE}"

log_step "Stopping app before restore"
compose stop "${APP_SERVICE}" >/dev/null || true

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
  if [[ -n "${rollback_sql_path}" ]]; then
    echo "  rollback database snapshot: ${rollback_sql_path}"
  fi
  if [[ -n "${rollback_datadir_archive_path}" ]]; then
    echo "  rollback datadir snapshot: ${rollback_datadir_archive_path}"
  fi
  if [[ -n "${rollback_data_archive_path}" ]]; then
    echo "  rollback data snapshot: ${rollback_data_archive_path}"
  fi
  if [[ -n "${physical_copy_back_log_path}" ]]; then
    echo "  physical copy-back log: ${physical_copy_back_log_path}"
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
