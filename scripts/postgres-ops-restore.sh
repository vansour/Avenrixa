#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/postgres-ops-common.sh"

APP_HEALTH_TIMEOUT_SECONDS="${APP_HEALTH_TIMEOUT_SECONDS:-120}"
POSTGRES_HEALTH_TIMEOUT_SECONDS="${POSTGRES_HEALTH_TIMEOUT_SECONDS:-120}"
START_APP_AFTER_RESTORE="${START_APP_AFTER_RESTORE:-1}"
POSTGRES_RESTORE_TARGET_NAME="${POSTGRES_RESTORE_TARGET_NAME:-}"
POSTGRES_RESTORE_TARGET_TIME="${POSTGRES_RESTORE_TARGET_TIME:-}"
POSTGRES_RESTORE_WAL_ARCHIVE_DIR="${POSTGRES_RESTORE_WAL_ARCHIVE_DIR:-}"
POSTGRES_PITR_PROMOTION_TIMEOUT_SECONDS="${POSTGRES_PITR_PROMOTION_TIMEOUT_SECONDS:-180}"

require_postgres_variant
require_commands

restore_manifest_path="${POSTGRES_RESTORE_MANIFEST_PATH:-}"
restore_physical_path="${1:-${POSTGRES_RESTORE_PHYSICAL_PATH:-}}"
restore_data_archive_path="${2:-${POSTGRES_RESTORE_DATA_ARCHIVE:-}}"
manifest_data_archive_sha256=""
manifest_backup_method=""
effective_restore_mode="physical"
manifest_wal_archive_host_dir=""
manifest_wal_archive_mount_path=""
manifest_wal_remote_uri=""
pitr_target_kind=""
pitr_target_value=""
active_restore_data_archive_path="${restore_data_archive_path}"
data_restore_mode="applied"
postgres_requires_recreate=0
pitr_recovery_config_log_path=""

if [[ -n "${restore_manifest_path}" ]]; then
  ensure_json_file "${restore_manifest_path}" "PostgreSQL restore manifest"
  manifest_backup_method="$(jq -r '.backup_method // empty' "${restore_manifest_path}")"

  case "${manifest_backup_method}" in
    physical)
      restore_physical_path="$(jq -r '.physical_backup.path // empty' "${restore_manifest_path}")"
      restore_data_archive_path="$(jq -r '.data_snapshot.path // empty' "${restore_manifest_path}")"
      manifest_data_archive_sha256="$(jq -r '.data_snapshot.sha256 // empty' "${restore_manifest_path}")"
      manifest_wal_archive_host_dir="$(jq -r '.wal_archive.host_dir // empty' "${restore_manifest_path}")"
      manifest_wal_archive_mount_path="$(jq -r '.wal_archive.mount_path // empty' "${restore_manifest_path}")"
      manifest_wal_remote_uri="$(jq -r '.wal_archive.remote_uri // empty' "${restore_manifest_path}")"
      ;;
    logical)
      echo "PostgreSQL ops restore currently only supports physical manifests, not logical SQL dumps." >&2
      exit 1
      ;;
    *)
      echo "Unsupported PostgreSQL manifest backup_method: ${manifest_backup_method}" >&2
      exit 1
      ;;
  esac
fi

if [[ -z "${restore_physical_path}" ]]; then
  echo "Usage: POSTGRES_RESTORE_MANIFEST_PATH=/path/to/postgres_last_physical_backup_manifest.json $0" >&2
  echo "   or: POSTGRES_RESTORE_PHYSICAL_PATH=/path/to/base-backup-dir [POSTGRES_RESTORE_DATA_ARCHIVE=/path/to/data.tar.gz] $0" >&2
  echo "PITR: POSTGRES_RESTORE_TARGET_NAME=name or POSTGRES_RESTORE_TARGET_TIME=timestamp plus POSTGRES_RESTORE_WAL_ARCHIVE_DIR=/path/to/wal-archive" >&2
  if [[ -n "${restore_manifest_path}" ]]; then
    echo "Manifest does not contain a physical backup directory path that this restore can consume." >&2
  fi
  exit 1
fi

if [[ -n "${POSTGRES_RESTORE_TARGET_NAME}" && -n "${POSTGRES_RESTORE_TARGET_TIME}" ]]; then
  echo "POSTGRES_RESTORE_TARGET_NAME and POSTGRES_RESTORE_TARGET_TIME are mutually exclusive." >&2
  exit 1
fi

if [[ -n "${POSTGRES_RESTORE_TARGET_NAME}" ]]; then
  pitr_target_kind="name"
  pitr_target_value="${POSTGRES_RESTORE_TARGET_NAME}"
elif [[ -n "${POSTGRES_RESTORE_TARGET_TIME}" ]]; then
  pitr_target_kind="time"
  pitr_target_value="${POSTGRES_RESTORE_TARGET_TIME}"
fi

if [[ -n "${pitr_target_kind}" ]]; then
  effective_restore_mode="physical-pitr"
  postgres_requires_recreate=1
  if [[ -z "${POSTGRES_RESTORE_WAL_ARCHIVE_DIR}" ]]; then
    POSTGRES_RESTORE_WAL_ARCHIVE_DIR="${manifest_wal_archive_host_dir}"
  fi
  if [[ -z "${POSTGRES_WAL_REMOTE_URI:-}" && -n "${manifest_wal_remote_uri}" ]]; then
    POSTGRES_WAL_REMOTE_URI="${manifest_wal_remote_uri}"
  fi
  if [[ -n "${manifest_wal_archive_mount_path}" ]]; then
    POSTGRES_WAL_ARCHIVE_MOUNT_PATH="${manifest_wal_archive_mount_path}"
  fi
  if [[ -z "${POSTGRES_RESTORE_WAL_ARCHIVE_DIR}" ]]; then
    echo "PITR restore requires POSTGRES_RESTORE_WAL_ARCHIVE_DIR or a wal_archive.host_dir in the manifest." >&2
    exit 1
  fi
  if [[ -n "${POSTGRES_WAL_REMOTE_URI:-}" ]]; then
    POSTGRES_WAL_REMOTE_URI="$(postgres_wal_remote_resolved_uri "${POSTGRES_WAL_REMOTE_URI}")"
  fi

  export POSTGRES_ENABLE_WAL_ARCHIVE=1
  export POSTGRES_WAL_ARCHIVE_HOST_DIR="${POSTGRES_RESTORE_WAL_ARCHIVE_DIR}"
  if [[ -n "${restore_data_archive_path}" ]]; then
    echo "PITR restore will not replay the manifest data snapshot; only PostgreSQL timeline recovery is applied." >&2
    data_restore_mode="skipped_for_pitr"
    active_restore_data_archive_path=""
  else
    data_restore_mode="not_requested"
  fi
else
  active_restore_data_archive_path="${restore_data_archive_path}"
  if [[ -z "${active_restore_data_archive_path}" ]]; then
    data_restore_mode="not_requested"
  fi
fi

ensure_directory_nonempty "${restore_physical_path}" "PostgreSQL physical backup directory"
if [[ -n "${active_restore_data_archive_path}" ]]; then
  if [[ -n "${restore_manifest_path}" ]]; then
    verify_file_sha256 "${active_restore_data_archive_path}" "${manifest_data_archive_sha256}" "PostgreSQL restore data archive"
  else
    ensure_file_nonempty "${active_restore_data_archive_path}" "PostgreSQL restore data archive"
  fi
fi

if [[ -n "${pitr_target_kind}" ]]; then
  POSTGRES_RESTORE_WAL_ARCHIVE_DIR="$(compose_variant_resolved_postgres_wal_archive_host_dir)"
  mkdir -p "${POSTGRES_RESTORE_WAL_ARCHIVE_DIR}"
  chmod 0777 "${POSTGRES_RESTORE_WAL_ARCHIVE_DIR}" || true
  if [[ -n "${POSTGRES_WAL_REMOTE_URI:-}" ]]; then
    log_step "Syncing PostgreSQL WAL archive from remote"
    POSTGRES_WAL_ARCHIVE_DIR="${POSTGRES_RESTORE_WAL_ARCHIVE_DIR}" \
    POSTGRES_WAL_REMOTE_URI="${POSTGRES_WAL_REMOTE_URI}" \
    "${ROOT_DIR}/scripts/postgres-ops-wal-sync.sh" pull
  fi
  ensure_directory_nonempty "${POSTGRES_RESTORE_WAL_ARCHIVE_DIR}" "PostgreSQL WAL archive directory"
fi

tmp_root=""
cleanup_tmp() {
  if [[ -n "${tmp_root}" ]]; then
    rm -rf "${tmp_root}"
  fi
}
trap cleanup_tmp EXIT

staged_restore_data_archive_path=""
if [[ -n "${active_restore_data_archive_path}" ]]; then
  tmp_root="$(mktemp -d /tmp/vansour-postgres-restore-XXXXXX)"
  staged_restore_data_archive_path="${tmp_root}/restore.data.tar.gz"
  cp "${active_restore_data_archive_path}" "${staged_restore_data_archive_path}"
  ensure_file_nonempty "${staged_restore_data_archive_path}" "Staged PostgreSQL restore data archive"
fi

mkdir -p "${ARTIFACT_DIR}"

stamp="$(current_stamp_compact)"
started_at="$(current_timestamp_utc)"
rollback_datadir_archive_path="${ARTIFACT_DIR}/rollback_before_restore_${stamp}.postgres-datadir.tar.gz"
rollback_datadir_restore_log_path="${ARTIFACT_DIR}/rollback_restore_${stamp}.postgres-datadir.log"
rollback_data_archive_path=""
physical_copy_back_log_path="${ARTIFACT_DIR}/restore_${stamp}.postgres-physical-copy-back.log"
pitr_recovery_config_log_path="${ARTIFACT_DIR}/restore_${stamp}.postgres-pitr-config.log"

if [[ -n "${active_restore_data_archive_path}" ]]; then
  rollback_data_archive_path="${ARTIFACT_DIR}/rollback_before_restore_${stamp}.data.tar.gz"
fi

postgres_stopped_for_restore=0
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
      --arg requested_restore_physical_path "${restore_physical_path}" \
      --arg requested_restore_data_archive_path "${restore_data_archive_path}" \
      --arg applied_restore_data_archive_path "${active_restore_data_archive_path}" \
      --arg data_restore_mode "${data_restore_mode}" \
      --arg manifest_data_archive_sha256 "${manifest_data_archive_sha256}" \
      --arg pitr_target_kind "${pitr_target_kind}" \
      --arg pitr_target_value "${pitr_target_value}" \
      --arg pitr_wal_archive_dir "${POSTGRES_RESTORE_WAL_ARCHIVE_DIR}" \
      --arg pitr_wal_archive_mount_path "${POSTGRES_WAL_ARCHIVE_MOUNT_PATH}" \
      --arg pitr_wal_remote_uri "${POSTGRES_WAL_REMOTE_URI:-}" \
      --arg rollback_datadir_archive_path "${rollback_datadir_archive_path}" \
      --arg rollback_datadir_restore_log_path "${rollback_datadir_restore_log_path}" \
      --arg rollback_data_archive_path "${rollback_data_archive_path}" \
      --arg physical_copy_back_log_path "${physical_copy_back_log_path}" \
      --arg pitr_recovery_config_log_path "${pitr_recovery_config_log_path}" \
      --arg compose_project_name "${COMPOSE_PROJECT_NAME}" \
      --argjson compose_files "$(compose_files_json)" \
      --arg app_health_url "${APP_HEALTH_URL}" \
      --arg pgdata_path "$(postgres_service_pgdata)" \
      '{
        status: $status,
        message: $message,
        started_at: $started_at,
        finished_at: $finished_at,
        restore_manifest_path: (if $restore_manifest_path == "" then null else $restore_manifest_path end),
        restore_method: $restore_method,
        manifest_backup_method: (if $manifest_backup_method == "" then null else $manifest_backup_method end),
        requested_restore_physical_path: $requested_restore_physical_path,
        requested_restore_data_archive_path: (if $requested_restore_data_archive_path == "" then null else $requested_restore_data_archive_path end),
        applied_restore_data_archive_path: (if $applied_restore_data_archive_path == "" then null else $applied_restore_data_archive_path end),
        data_restore_mode: $data_restore_mode,
        manifest_data_archive_sha256: (if $manifest_data_archive_sha256 == "" then null else $manifest_data_archive_sha256 end),
        pitr_target_kind: (if $pitr_target_kind == "" then null else $pitr_target_kind end),
        pitr_target_value: (if $pitr_target_value == "" then null else $pitr_target_value end),
        pitr_wal_archive_dir: (if $pitr_wal_archive_dir == "" then null else $pitr_wal_archive_dir end),
        pitr_wal_archive_mount_path: (if $pitr_wal_archive_mount_path == "" then null else $pitr_wal_archive_mount_path end),
        pitr_wal_remote_uri: (if $pitr_wal_remote_uri == "" then null else $pitr_wal_remote_uri end),
        rollback_datadir_archive_path: $rollback_datadir_archive_path,
        rollback_datadir_restore_log_path: $rollback_datadir_restore_log_path,
        rollback_data_archive_path: (if $rollback_data_archive_path == "" then null else $rollback_data_archive_path end),
        physical_copy_back_log_path: $physical_copy_back_log_path,
        pitr_recovery_config_log_path: (if $pitr_recovery_config_log_path == "" then null else $pitr_recovery_config_log_path end),
        compose_project_name: $compose_project_name,
        compose_files: $compose_files,
        pgdata_path: $pgdata_path,
        app_health_url: $app_health_url
      }'
  )"

  write_json_file "${POSTGRES_LAST_RESTORE_RESULT_PATH}" "${payload}"
}

restore_local_data_from_archive() {
  local archive_path="$1"

  compose_remove_host_path "${DATA_DIR}"
  tar -xzf "${archive_path}" -C "${ROOT_DIR}"
}

start_postgres_after_restore() {
  if [[ "${postgres_requires_recreate}" == "1" ]]; then
    compose up -d --no-deps --force-recreate "${POSTGRES_SERVICE}" >/dev/null
  else
    compose start "${POSTGRES_SERVICE}" >/dev/null
  fi
  if [[ -n "${pitr_target_kind}" ]]; then
    wait_for_postgres_not_in_recovery "${POSTGRES_PITR_PROMOTION_TIMEOUT_SECONDS}" || return 1
  fi
  wait_for_compose_service_health "${POSTGRES_SERVICE}" "${POSTGRES_HEALTH_TIMEOUT_SECONDS}" || return 1
}

start_app_after_restore() {
  if [[ "${START_APP_AFTER_RESTORE}" == "1" ]]; then
    compose start "${APP_SERVICE}" >/dev/null
    wait_for_app_health "${APP_HEALTH_TIMEOUT_SECONDS}"
  fi
}

restore_rollback_artifacts() {
  local rollback_error=""

  compose stop "${APP_SERVICE}" >/dev/null || true
  if [[ "${postgres_stopped_for_restore}" == "1" ]]; then
    compose stop "${POSTGRES_SERVICE}" >/dev/null || true
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
      postgres_data_dir_restore_from_archive "${rollback_datadir_archive_path}" "${rollback_datadir_restore_log_path}" || rollback_error="$(
        append_error_message "${rollback_error}" "failed to restore PostgreSQL datadir rollback archive"
      )"
    else
      rollback_error="$(append_error_message "${rollback_error}" "database datadir rollback artifact missing")"
    fi
  fi

  if [[ "${postgres_stopped_for_restore}" == "1" ]]; then
    start_postgres_after_restore || rollback_error="$(
      append_error_message "${rollback_error}" "postgres health check failed after rollback"
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

perform_restore() {
  log_step "Stopping PostgreSQL before physical restore"
  compose stop "${POSTGRES_SERVICE}" >/dev/null || return 1
  postgres_stopped_for_restore=1

  log_step "Creating rollback PostgreSQL datadir snapshot"
  postgres_data_dir_archive_to_file "${rollback_datadir_archive_path}" || return 1

  if [[ -n "${staged_restore_data_archive_path}" ]]; then
    log_step "Creating rollback data snapshot"
    tar -C "${ROOT_DIR}" -czf "${rollback_data_archive_path}" "${DATA_DIR}" || return 1
    if [[ ! -f "${rollback_data_archive_path}" || ! -s "${rollback_data_archive_path}" ]]; then
      echo "Rollback data snapshot is missing or empty: ${rollback_data_archive_path}" >&2
      return 1
    fi
  fi

  log_step "Restoring PostgreSQL physical backup"
  physical_datadir_mutated=1
  postgres_physical_copy_back_from_dir "${restore_physical_path}" "${physical_copy_back_log_path}" || return 1

  if [[ -n "${pitr_target_kind}" ]]; then
    log_step "Writing PostgreSQL PITR recovery target"
    postgres_write_pitr_recovery_config "${pitr_target_kind}" "${pitr_target_value}" "${pitr_recovery_config_log_path}" || return 1
  fi

  if [[ -n "${staged_restore_data_archive_path}" ]]; then
    log_step "Restoring data directory snapshot"
    physical_data_mutated=1
    restore_local_data_from_archive "${staged_restore_data_archive_path}" || return 1
  fi

  log_step "Starting PostgreSQL after restore"
  start_postgres_after_restore || return 1

  if [[ "${START_APP_AFTER_RESTORE}" == "1" ]]; then
    log_step "Starting app after restore"
    start_app_after_restore || return 1
  fi

  return 0
}

log_step "Checking PostgreSQL stack"
ensure_stack_service_running "${POSTGRES_SERVICE}"

log_step "Stopping app before restore"
compose stop "${APP_SERVICE}" >/dev/null || true

set +e
perform_restore
restore_exit_code=$?
set -e

if [[ "${restore_exit_code}" -eq 0 ]]; then
  result_status="completed"
  if [[ -n "${pitr_target_kind}" ]]; then
    result_message="PITR restore completed successfully."
  else
    result_message="Restore completed successfully."
  fi
  write_restore_result "${result_status}" "${result_message}"
  echo "PostgreSQL ops restore completed successfully."
  echo "  result: ${POSTGRES_LAST_RESTORE_RESULT_PATH}"
  echo "  rollback datadir snapshot: ${rollback_datadir_archive_path}"
  if [[ -n "${rollback_data_archive_path}" ]]; then
    echo "  rollback data snapshot: ${rollback_data_archive_path}"
  fi
  echo "  physical copy-back log: ${physical_copy_back_log_path}"
  if [[ -n "${pitr_target_kind}" ]]; then
    echo "  pitr target: ${pitr_target_kind}=${pitr_target_value}"
    echo "  wal archive dir: ${POSTGRES_RESTORE_WAL_ARCHIVE_DIR}"
    echo "  pitr config log: ${pitr_recovery_config_log_path}"
  fi
  exit 0
fi

log_step "Restore failed, applying rollback"
set +e
restore_rollback_artifacts
rollback_exit_code=$?
set -e

write_restore_result "${result_status}" "${result_message}"

echo "${result_message}" >&2
echo "  result: ${POSTGRES_LAST_RESTORE_RESULT_PATH}" >&2
if [[ "${rollback_exit_code}" -eq 0 ]]; then
  exit 1
fi

exit 1
