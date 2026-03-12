#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/mysql-ops-common.sh"

MYSQL_BACKUP_MODE="${MYSQL_BACKUP_MODE:-physical}"
INCLUDE_DATA_SNAPSHOT="${INCLUDE_DATA_SNAPSHOT:-1}"
STOP_APP_DURING_DATA_SNAPSHOT="${STOP_APP_DURING_DATA_SNAPSHOT:-1}"
APP_HEALTH_TIMEOUT_SECONDS="${APP_HEALTH_TIMEOUT_SECONDS:-120}"

normalize_backup_mode() {
  case "${MYSQL_BACKUP_MODE}" in
    logical|physical)
      printf '%s' "${MYSQL_BACKUP_MODE}"
      ;;
    *)
      echo "Unsupported MYSQL_BACKUP_MODE: ${MYSQL_BACKUP_MODE}" >&2
      exit 1
      ;;
  esac
}

metadata_value_from_key() {
  local path="$1"
  local key="$2"

  if [[ ! -f "${path}" ]]; then
    return 0
  fi

  awk -F' *= *' -v key_name="${key}" '$1 == key_name { print $2; exit }' "${path}"
}

read_optional_file() {
  local path="$1"

  if [[ -f "${path}" ]]; then
    tr -d '\r' < "${path}"
  fi
}

require_commands

mkdir -p "${ARTIFACT_DIR}"
mkdir -p "${DATA_DIR}/backup"

data_dir_abs="$(absolute_existing_dir "${DATA_DIR}")"
artifact_dir_abs="$(absolute_existing_dir "${ARTIFACT_DIR}")"
backup_mode="$(normalize_backup_mode)"

stamp="$(current_stamp_compact)"
backup_prefix="${MYSQL_BACKUP_PREFIX:-mysql_ops_backup_${stamp}}"
data_archive_path="${artifact_dir_abs}/${backup_prefix}.data.tar.gz"

app_was_running=0
if compose ps --status running --services | grep -qx "${APP_SERVICE}"; then
  app_was_running=1
fi

app_stopped_for_snapshot=0
restore_app_if_needed() {
  if [[ "${app_stopped_for_snapshot}" == "1" ]]; then
    log_step "Starting app after backup"
    compose start "${APP_SERVICE}" >/dev/null
    wait_for_app_health "${APP_HEALTH_TIMEOUT_SECONDS}"
    app_stopped_for_snapshot=0
  fi
}

trap restore_app_if_needed EXIT

create_data_snapshot() {
  data_archive_sha256=""

  if [[ "${INCLUDE_DATA_SNAPSHOT}" != "1" ]]; then
    return 0
  fi

  log_step "Creating data directory snapshot"
  tar -C "${ROOT_DIR}" -czf "${data_archive_path}" "${DATA_DIR}"
  ensure_file_nonempty "${data_archive_path}" "MySQL/MariaDB data snapshot"
  data_archive_sha256="$(sha256_file "${data_archive_path}")"
}

log_step "Checking MySQL/MariaDB stack"
ensure_stack_service_running "${MYSQL_SERVICE}"

if [[ "${INCLUDE_DATA_SNAPSHOT}" == "1" && "${STOP_APP_DURING_DATA_SNAPSHOT}" == "1" && "${app_was_running}" == "1" ]]; then
  log_step "Stopping app for consistent data snapshot"
  compose stop "${APP_SERVICE}" >/dev/null
  app_stopped_for_snapshot=1
fi

app_stopped_for_snapshot_actual="${app_stopped_for_snapshot}"

case "${backup_mode}" in
  logical)
    dump_path="${artifact_dir_abs}/${backup_prefix}.mysql.sql"
    manifest_path="${artifact_dir_abs}/${backup_prefix}.manifest.json"
    latest_manifest_path="${data_dir_abs}/backup/mysql_last_backup_manifest.json"

    log_step "Creating MySQL/MariaDB logical backup"
    mysql_dump_to_file "${dump_path}"
    create_data_snapshot
    restore_app_if_needed

    dump_sha256="$(sha256_file "${dump_path}")"
    created_at="$(current_timestamp_utc)"
    manifest_payload="$(
      jq -n \
        --arg created_at "${created_at}" \
        --arg backup_method "logical" \
        --arg backup_kind "mysql-logical-dump" \
        --arg compose_project_name "${COMPOSE_PROJECT_NAME}" \
        --argjson compose_files "$(printf '%s\n' "${compose_files[@]}" | jq -R . | jq -s .)" \
        --arg dump_path "${dump_path}" \
        --arg dump_sha256 "${dump_sha256}" \
        --arg data_archive_path "${data_archive_path}" \
        --arg data_archive_sha256 "${data_archive_sha256:-}" \
        --argjson include_data_snapshot "$([[ "${INCLUDE_DATA_SNAPSHOT}" == "1" ]] && echo true || echo false)" \
        --argjson app_was_running "$([[ "${app_was_running}" == "1" ]] && echo true || echo false)" \
        --argjson app_stopped_for_snapshot "$([[ "${app_stopped_for_snapshot_actual}" == "1" ]] && echo true || echo false)" \
        '{
          created_at: $created_at,
          backup_method: $backup_method,
          backup_kind: $backup_kind,
          compose_project_name: $compose_project_name,
          compose_files: $compose_files,
          dump: {
            path: $dump_path,
            sha256: $dump_sha256
          },
          include_data_snapshot: $include_data_snapshot,
          data_snapshot: (
            if $include_data_snapshot then
              {
                path: $data_archive_path,
                sha256: $data_archive_sha256
              }
            else
              null
            end
          ),
          app_was_running: $app_was_running,
          app_stopped_for_snapshot: $app_stopped_for_snapshot
        }'
    )"

    write_json_file "${manifest_path}" "${manifest_payload}"
    write_json_file "${latest_manifest_path}" "${manifest_payload}"

    echo "MySQL/MariaDB ops logical backup created:"
    echo "  dump: ${dump_path}"
    if [[ "${INCLUDE_DATA_SNAPSHOT}" == "1" ]]; then
      echo "  data snapshot: ${data_archive_path}"
    fi
    echo "  manifest: ${manifest_path}"
    ;;
  physical)
    physical_backup_root="${artifact_dir_abs}/${backup_prefix}.physical"
    physical_target_dir="${physical_backup_root}/base"
    physical_logs_dir="${physical_backup_root}/logs"
    physical_backup_log_path="${physical_logs_dir}/backup.log"
    physical_prepare_log_path="${physical_logs_dir}/prepare.log"
    manifest_path="${artifact_dir_abs}/${backup_prefix}.physical.manifest.json"
    latest_manifest_path="${data_dir_abs}/backup/mysql_last_physical_backup_manifest.json"

    rm -rf "${physical_backup_root}"
    mkdir -p "${physical_logs_dir}"

    log_step "Creating MySQL/MariaDB physical backup"
    mysql_physical_backup_to_dir "${physical_target_dir}" "${physical_backup_log_path}"

    log_step "Preparing MySQL/MariaDB physical backup"
    mysql_physical_prepare_dir "${physical_target_dir}" "${physical_prepare_log_path}"

    create_data_snapshot
    restore_app_if_needed

    tool_family="$(mysql_physical_tool_family)"
    helper_image="$(mysql_physical_helper_image)"
    created_at="$(current_timestamp_utc)"
    checkpoints_path="${physical_target_dir}/xtrabackup_checkpoints"
    info_path="${physical_target_dir}/xtrabackup_info"
    binlog_info_path="${physical_target_dir}/xtrabackup_binlog_info"
    backup_my_cnf_path="${physical_target_dir}/backup-my.cnf"
    backup_type="$(metadata_value_from_key "${checkpoints_path}" "backup_type")"
    from_lsn="$(metadata_value_from_key "${checkpoints_path}" "from_lsn")"
    to_lsn="$(metadata_value_from_key "${checkpoints_path}" "to_lsn")"
    last_lsn="$(metadata_value_from_key "${checkpoints_path}" "last_lsn")"
    binlog_file=""
    binlog_position=""
    binlog_gtid=""
    if [[ -f "${binlog_info_path}" ]]; then
      read -r binlog_file binlog_position binlog_gtid < "${binlog_info_path}" || true
    fi

    manifest_payload="$(
      jq -n \
        --arg created_at "${created_at}" \
        --arg backup_method "physical" \
        --arg backup_kind "mysql-physical-full-backup" \
        --arg tool_family "${tool_family}" \
        --arg helper_image "${helper_image}" \
        --arg compose_project_name "${COMPOSE_PROJECT_NAME}" \
        --arg compose_variant "${COMPOSE_VARIANT}" \
        --argjson compose_files "$(printf '%s\n' "${compose_files[@]}" | jq -R . | jq -s .)" \
        --arg physical_target_dir "${physical_target_dir}" \
        --arg physical_backup_log_path "${physical_backup_log_path}" \
        --arg physical_prepare_log_path "${physical_prepare_log_path}" \
        --arg checkpoints_path "${checkpoints_path}" \
        --arg info_path "${info_path}" \
        --arg binlog_info_path "${binlog_info_path}" \
        --arg backup_my_cnf_path "${backup_my_cnf_path}" \
        --arg backup_type "${backup_type}" \
        --arg from_lsn "${from_lsn}" \
        --arg to_lsn "${to_lsn}" \
        --arg last_lsn "${last_lsn}" \
        --arg binlog_file "${binlog_file}" \
        --arg binlog_position "${binlog_position}" \
        --arg binlog_gtid "${binlog_gtid}" \
        --arg xtrabackup_info_raw "$(read_optional_file "${info_path}")" \
        --arg xtrabackup_checkpoints_raw "$(read_optional_file "${checkpoints_path}")" \
        --arg xtrabackup_binlog_info_raw "$(read_optional_file "${binlog_info_path}")" \
        --arg data_archive_path "${data_archive_path}" \
        --arg data_archive_sha256 "${data_archive_sha256:-}" \
        --argjson include_data_snapshot "$([[ "${INCLUDE_DATA_SNAPSHOT}" == "1" ]] && echo true || echo false)" \
        --argjson app_was_running "$([[ "${app_was_running}" == "1" ]] && echo true || echo false)" \
        --argjson app_stopped_for_snapshot "$([[ "${app_stopped_for_snapshot_actual}" == "1" ]] && echo true || echo false)" \
        '{
          created_at: $created_at,
          backup_method: $backup_method,
          backup_kind: $backup_kind,
          compose_project_name: $compose_project_name,
          compose_variant: $compose_variant,
          compose_files: $compose_files,
          physical_backup: {
            tool_family: $tool_family,
            helper_image: $helper_image,
            path: $physical_target_dir,
            prepared: true,
            prepare_strategy: "in-place-prepare",
            backup_log_path: $physical_backup_log_path,
            prepare_log_path: $physical_prepare_log_path,
            metadata: {
              checkpoints_path: $checkpoints_path,
              info_path: $info_path,
              binlog_info_path: $binlog_info_path,
              backup_my_cnf_path: $backup_my_cnf_path,
              backup_type: (if $backup_type == "" then null else $backup_type end),
              from_lsn: (if $from_lsn == "" then null else $from_lsn end),
              to_lsn: (if $to_lsn == "" then null else $to_lsn end),
              last_lsn: (if $last_lsn == "" then null else $last_lsn end),
              binlog_file: (if $binlog_file == "" then null else $binlog_file end),
              binlog_position: (if $binlog_position == "" then null else $binlog_position end),
              binlog_gtid: (if $binlog_gtid == "" then null else $binlog_gtid end),
              xtrabackup_info_raw: (if $xtrabackup_info_raw == "" then null else $xtrabackup_info_raw end),
              xtrabackup_checkpoints_raw: (if $xtrabackup_checkpoints_raw == "" then null else $xtrabackup_checkpoints_raw end),
              xtrabackup_binlog_info_raw: (if $xtrabackup_binlog_info_raw == "" then null else $xtrabackup_binlog_info_raw end)
            }
          },
          include_data_snapshot: $include_data_snapshot,
          data_snapshot: (
            if $include_data_snapshot then
              {
                path: $data_archive_path,
                sha256: $data_archive_sha256
              }
            else
              null
            end
          ),
          app_was_running: $app_was_running,
          app_stopped_for_snapshot: $app_stopped_for_snapshot
        }'
    )"

    write_json_file "${manifest_path}" "${manifest_payload}"
    write_json_file "${latest_manifest_path}" "${manifest_payload}"

    echo "MySQL/MariaDB ops physical backup created:"
    echo "  prepared backup dir: ${physical_target_dir}"
    echo "  backup log: ${physical_backup_log_path}"
    echo "  prepare log: ${physical_prepare_log_path}"
    if [[ "${INCLUDE_DATA_SNAPSHOT}" == "1" ]]; then
      echo "  data snapshot: ${data_archive_path}"
    fi
    echo "  manifest: ${manifest_path}"
    ;;
esac
