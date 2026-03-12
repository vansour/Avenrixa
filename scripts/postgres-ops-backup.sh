#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/postgres-ops-common.sh"

INCLUDE_DATA_SNAPSHOT="${INCLUDE_DATA_SNAPSHOT:-1}"
STOP_APP_DURING_DATA_SNAPSHOT="${STOP_APP_DURING_DATA_SNAPSHOT:-1}"
APP_HEALTH_TIMEOUT_SECONDS="${APP_HEALTH_TIMEOUT_SECONDS:-120}"
POSTGRES_WAL_ARCHIVE_WAIT_TIMEOUT_SECONDS="${POSTGRES_WAL_ARCHIVE_WAIT_TIMEOUT_SECONDS:-60}"
POSTGRES_WAL_PRUNE_AFTER_BACKUP="${POSTGRES_WAL_PRUNE_AFTER_BACKUP:-0}"

require_postgres_variant
require_commands

mkdir -p "${ARTIFACT_DIR}"
mkdir -p "${DATA_DIR}/backup"

data_dir_abs="$(absolute_existing_dir "${DATA_DIR}")"
artifact_dir_abs="$(absolute_existing_dir "${ARTIFACT_DIR}")"
data_archive_parent="$(dirname "${data_dir_abs}")"
data_archive_root="$(basename "${data_dir_abs}")"

case "${artifact_dir_abs}/" in
  "${data_dir_abs}/"*)
    echo "ARTIFACT_DIR must not be inside DATA_DIR to avoid recursive archives" >&2
    exit 1
    ;;
esac

stamp="$(current_stamp_compact)"
backup_prefix="${POSTGRES_BACKUP_PREFIX:-postgres_ops_backup_${stamp}}"
backup_label="vansour-postgres-ops-${stamp}"
physical_backup_root="${artifact_dir_abs}/${backup_prefix}.physical"
physical_target_dir="${physical_backup_root}/base"
physical_logs_dir="${physical_backup_root}/logs"
physical_backup_log_path="${physical_logs_dir}/backup.log"
manifest_path="${artifact_dir_abs}/${backup_prefix}.physical.manifest.json"
latest_manifest_path="${POSTGRES_LAST_PHYSICAL_BACKUP_MANIFEST_PATH}"
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
  tar -C "${data_archive_parent}" -czf "${data_archive_path}" "${data_archive_root}"
  ensure_file_nonempty "${data_archive_path}" "PostgreSQL data snapshot"
  data_archive_sha256="$(sha256_file "${data_archive_path}")"
}

log_step "Checking PostgreSQL stack"
ensure_stack_service_running "${POSTGRES_SERVICE}"

if [[ "${INCLUDE_DATA_SNAPSHOT}" == "1" && "${STOP_APP_DURING_DATA_SNAPSHOT}" == "1" && "${app_was_running}" == "1" ]]; then
  log_step "Stopping app for consistent data snapshot"
  compose stop "${APP_SERVICE}" >/dev/null
  app_stopped_for_snapshot=1
fi

app_stopped_for_snapshot_actual="${app_stopped_for_snapshot}"
rm -rf "${physical_backup_root}"
mkdir -p "${physical_logs_dir}"

log_step "Creating PostgreSQL physical base backup"
postgres_physical_backup_to_dir "${physical_target_dir}" "${physical_backup_log_path}" "${backup_label}"

create_data_snapshot
restore_app_if_needed

helper_image="$(postgres_physical_helper_image)"
created_at="$(current_timestamp_utc)"
pg_version_path="${physical_target_dir}/PG_VERSION"
backup_manifest_path="${physical_target_dir}/backup_manifest"
backup_label_path="${physical_target_dir}/backup_label"
tablespace_map_path="${physical_target_dir}/tablespace_map"
backup_size_bytes="$(postgres_directory_size_bytes "${physical_target_dir}")"
backup_label_raw="$(read_optional_file "${backup_label_path}")"
start_wal_file="$(postgres_backup_label_start_wal_file_from_text "${backup_label_raw}")"
start_timeline="$(postgres_backup_label_start_timeline_from_text "${backup_label_raw}")"
wal_archive_mode_setting="$(postgres_current_setting "archive_mode")"
wal_level_setting="$(postgres_current_setting "wal_level")"
wal_archive_enabled_actual=0
wal_archive_host_dir=""
wal_archive_mount_path=""
wal_archive_file_count=0
wal_remote_uri=""
restore_point_name=""
restore_point_lsn=""
restore_point_created_at=""

if [[ "${wal_archive_mode_setting}" == "on" ]]; then
  wal_archive_enabled_actual=1
  if postgres_wal_archive_enabled; then
    wal_archive_host_dir="$(postgres_wal_archive_host_dir)"
    wal_archive_mount_path="$(postgres_wal_archive_mount_path)"
    mkdir -p "${wal_archive_host_dir}"
    chmod 0777 "${wal_archive_host_dir}"
  fi

  restore_point_name="${POSTGRES_BACKUP_RESTORE_POINT_NAME:-vansour_postgres_backup_${stamp}}"
  log_step "Creating PostgreSQL restore point for PITR"
  restore_point_lsn="$(postgres_create_restore_point "${restore_point_name}")"
  restore_point_created_at="$(current_timestamp_utc)"

  if [[ -n "${wal_archive_host_dir}" ]]; then
    postgres_force_wal_switch_and_wait "${wal_archive_host_dir}" "${POSTGRES_WAL_ARCHIVE_WAIT_TIMEOUT_SECONDS}"
    wal_archive_file_count="$(directory_regular_file_count "${wal_archive_host_dir}")"
  else
    postgres_switch_wal
  fi

  if postgres_wal_remote_enabled; then
    wal_remote_uri="$(postgres_wal_remote_resolved_uri "$(postgres_wal_remote_uri)")"
    log_step "Syncing PostgreSQL WAL archive to remote"
    POSTGRES_WAL_REMOTE_URI="${wal_remote_uri}" \
    "${ROOT_DIR}/scripts/postgres-ops-wal-sync.sh" push
  fi
fi

if [[ "${POSTGRES_WAL_PRUNE_AFTER_BACKUP}" == "1" && "${wal_archive_enabled_actual}" == "1" ]]; then
  log_step "Pruning PostgreSQL WAL archive after backup"
  if [[ -n "${wal_remote_uri}" ]]; then
    POSTGRES_WAL_REMOTE_URI="${wal_remote_uri}" \
    "${ROOT_DIR}/scripts/postgres-ops-wal-prune.sh"
  else
    "${ROOT_DIR}/scripts/postgres-ops-wal-prune.sh"
  fi
fi

manifest_payload="$(
  jq -n \
    --arg created_at "${created_at}" \
    --arg backup_method "physical" \
    --arg backup_kind "postgresql-physical-basebackup" \
    --arg tool_family "pg_basebackup" \
    --arg helper_image "${helper_image}" \
    --arg compose_project_name "${COMPOSE_PROJECT_NAME}" \
    --arg compose_variant "${COMPOSE_VARIANT}" \
    --arg cache_mode "${CACHE_MODE}" \
    --arg backup_label "${backup_label}" \
    --arg physical_target_dir "${physical_target_dir}" \
    --arg physical_backup_log_path "${physical_backup_log_path}" \
    --arg pg_version_path "${pg_version_path}" \
    --arg backup_manifest_path "${backup_manifest_path}" \
    --arg backup_label_path "${backup_label_path}" \
    --arg tablespace_map_path "${tablespace_map_path}" \
    --arg pg_version_raw "$(read_optional_file "${pg_version_path}")" \
    --arg backup_label_raw "${backup_label_raw}" \
    --arg start_wal_file "${start_wal_file}" \
    --arg start_timeline "${start_timeline}" \
    --arg data_archive_path "${data_archive_path}" \
    --arg data_archive_sha256 "${data_archive_sha256:-}" \
    --arg wal_archive_mode_setting "${wal_archive_mode_setting}" \
    --arg wal_level_setting "${wal_level_setting}" \
    --arg wal_archive_host_dir "${wal_archive_host_dir}" \
    --arg wal_archive_mount_path "${wal_archive_mount_path}" \
    --arg wal_remote_uri "${wal_remote_uri}" \
    --arg restore_point_name "${restore_point_name}" \
    --arg restore_point_lsn "${restore_point_lsn}" \
    --arg restore_point_created_at "${restore_point_created_at}" \
    --argjson compose_files "$(printf '%s\n' "${compose_files[@]}" | jq -R . | jq -s .)" \
    --argjson backup_size_bytes "${backup_size_bytes}" \
    --argjson wal_archive_file_count "${wal_archive_file_count}" \
    --argjson backup_manifest_exists "$([[ -f "${backup_manifest_path}" ]] && echo true || echo false)" \
    --argjson backup_label_exists "$([[ -f "${backup_label_path}" ]] && echo true || echo false)" \
    --argjson tablespace_map_exists "$([[ -f "${tablespace_map_path}" ]] && echo true || echo false)" \
    --argjson include_data_snapshot "$([[ "${INCLUDE_DATA_SNAPSHOT}" == "1" ]] && echo true || echo false)" \
    --argjson app_was_running "$([[ "${app_was_running}" == "1" ]] && echo true || echo false)" \
    --argjson app_stopped_for_snapshot "$([[ "${app_stopped_for_snapshot_actual}" == "1" ]] && echo true || echo false)" \
    --argjson wal_archive_enabled "$([[ "${wal_archive_enabled_actual}" == "1" ]] && echo true || echo false)" \
    '{
      created_at: $created_at,
      backup_method: $backup_method,
      backup_kind: $backup_kind,
      compose_project_name: $compose_project_name,
      compose_variant: $compose_variant,
      compose_files: $compose_files,
      cache_mode: $cache_mode,
      physical_backup: {
        tool_family: $tool_family,
        helper_image: $helper_image,
        path: $physical_target_dir,
        ready_for_restore: true,
        wal_method: "stream",
        checkpoint_mode: "fast",
        backup_log_path: $physical_backup_log_path,
        metadata: {
          backup_label: $backup_label,
          pg_version_path: $pg_version_path,
          backup_manifest_path: $backup_manifest_path,
          backup_label_path: $backup_label_path,
          tablespace_map_path: $tablespace_map_path,
          pg_version_raw: (if $pg_version_raw == "" then null else $pg_version_raw end),
          backup_label_raw: (if $backup_label_raw == "" then null else $backup_label_raw end),
          start_wal_file: (if $start_wal_file == "" then null else $start_wal_file end),
          start_timeline: (if $start_timeline == "" then null else $start_timeline end),
          backup_manifest_exists: $backup_manifest_exists,
          backup_label_exists: $backup_label_exists,
          tablespace_map_exists: $tablespace_map_exists,
          backup_size_bytes: $backup_size_bytes
        }
      },
      wal_archive: (
        if $wal_archive_enabled then
          {
            enabled: true,
            archive_mode_setting: $wal_archive_mode_setting,
            wal_level_setting: $wal_level_setting,
            host_dir: (if $wal_archive_host_dir == "" then null else $wal_archive_host_dir end),
            mount_path: (if $wal_archive_mount_path == "" then null else $wal_archive_mount_path end),
            remote_uri: (if $wal_remote_uri == "" then null else $wal_remote_uri end),
            file_count: $wal_archive_file_count,
            restore_point: (
              if $restore_point_name == "" then
                null
              else
                {
                  name: $restore_point_name,
                  lsn: (if $restore_point_lsn == "" then null else $restore_point_lsn end),
                  created_at: (if $restore_point_created_at == "" then null else $restore_point_created_at end)
                }
              end
            )
          }
        else
          {
            enabled: false,
            archive_mode_setting: $wal_archive_mode_setting,
            wal_level_setting: $wal_level_setting,
            host_dir: null,
            mount_path: null,
            file_count: 0,
            restore_point: null
          }
        end
      ),
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
      app_stopped_for_snapshot: $app_stopped_for_snapshot,
      notes: [
        "This base backup is produced by pg_basebackup against the running PostgreSQL service.",
        "Physical restore is available through scripts/postgres-ops-restore.sh.",
        "If wal_archive.enabled=true, this manifest can also be used as the base backup input for PITR.",
        "If storage_backend uses S3 or MinIO, external object data is not included in the optional /data snapshot."
      ]
    }'
)"

write_json_file "${manifest_path}" "${manifest_payload}"
write_json_file "${latest_manifest_path}" "${manifest_payload}"

echo "PostgreSQL ops physical backup created:"
echo "  base backup dir: ${physical_target_dir}"
echo "  backup log: ${physical_backup_log_path}"
if [[ "${INCLUDE_DATA_SNAPSHOT}" == "1" ]]; then
  echo "  data snapshot: ${data_archive_path}"
fi
if [[ "${wal_archive_enabled_actual}" == "1" ]]; then
  echo "  restore point: ${restore_point_name}"
  if [[ -n "${wal_archive_host_dir}" ]]; then
    echo "  wal archive dir: ${wal_archive_host_dir}"
  fi
  if [[ -n "${wal_remote_uri}" ]]; then
    echo "  wal remote uri: ${wal_remote_uri}"
  fi
fi
echo "  manifest: ${manifest_path}"
