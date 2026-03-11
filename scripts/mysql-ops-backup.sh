#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/mysql-ops-common.sh"

INCLUDE_DATA_SNAPSHOT="${INCLUDE_DATA_SNAPSHOT:-1}"
STOP_APP_DURING_DATA_SNAPSHOT="${STOP_APP_DURING_DATA_SNAPSHOT:-1}"
APP_HEALTH_TIMEOUT_SECONDS="${APP_HEALTH_TIMEOUT_SECONDS:-120}"

require_commands

mkdir -p "${ARTIFACT_DIR}"
mkdir -p "${DATA_DIR}/backup"

stamp="$(current_stamp_compact)"
backup_prefix="${MYSQL_BACKUP_PREFIX:-mysql_ops_backup_${stamp}}"
dump_path="${ARTIFACT_DIR}/${backup_prefix}.mysql.sql"
data_archive_path="${ARTIFACT_DIR}/${backup_prefix}.data.tar.gz"
manifest_path="${ARTIFACT_DIR}/${backup_prefix}.manifest.json"
latest_manifest_path="${DATA_DIR}/backup/mysql_last_backup_manifest.json"

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

log_step "Checking MySQL/MariaDB stack"
ensure_stack_service_running "${MYSQL_SERVICE}"

if [[ "${INCLUDE_DATA_SNAPSHOT}" == "1" && "${STOP_APP_DURING_DATA_SNAPSHOT}" == "1" && "${app_was_running}" == "1" ]]; then
  log_step "Stopping app for consistent data snapshot"
  compose stop "${APP_SERVICE}" >/dev/null
  app_stopped_for_snapshot=1
fi

app_stopped_for_snapshot_actual="${app_stopped_for_snapshot}"

log_step "Creating MySQL/MariaDB logical backup"
mysql_dump_to_file "${dump_path}"

data_archive_sha256=""
if [[ "${INCLUDE_DATA_SNAPSHOT}" == "1" ]]; then
  log_step "Creating data directory snapshot"
  tar -C "${ROOT_DIR}" -czf "${data_archive_path}" "${DATA_DIR}"
  ensure_file_nonempty "${data_archive_path}" "MySQL/MariaDB data snapshot"
  data_archive_sha256="$(sha256_file "${data_archive_path}")"
fi

restore_app_if_needed

dump_sha256="$(sha256_file "${dump_path}")"
created_at="$(current_timestamp_utc)"
manifest_payload="$(
  jq -n \
    --arg created_at "${created_at}" \
    --arg compose_project_name "${COMPOSE_PROJECT_NAME}" \
    --argjson compose_files "$(printf '%s\n' "${compose_files[@]}" | jq -R . | jq -s .)" \
    --arg dump_path "${dump_path}" \
    --arg dump_sha256 "${dump_sha256}" \
    --arg data_archive_path "${data_archive_path}" \
    --arg data_archive_sha256 "${data_archive_sha256}" \
    --argjson include_data_snapshot "$([[ "${INCLUDE_DATA_SNAPSHOT}" == "1" ]] && echo true || echo false)" \
    --argjson app_was_running "$([[ "${app_was_running}" == "1" ]] && echo true || echo false)" \
    --argjson app_stopped_for_snapshot "$([[ "${app_stopped_for_snapshot_actual}" == "1" ]] && echo true || echo false)" \
    '{
      created_at: $created_at,
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

echo "MySQL/MariaDB ops backup created:"
echo "  dump: ${dump_path}"
if [[ "${INCLUDE_DATA_SNAPSHOT}" == "1" ]]; then
  echo "  data snapshot: ${data_archive_path}"
fi
echo "  manifest: ${manifest_path}"
