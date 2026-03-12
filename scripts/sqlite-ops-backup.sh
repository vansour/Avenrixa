#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME:-vansour-image-sqlite-ops}"
COMPOSE_VARIANT="${COMPOSE_VARIANT:-sqlite}"
APP_SERVICE="${APP_SERVICE:-app}"
APP_HOST_PORT="${APP_HOST_PORT:-8080}"
APP_HEALTH_URL="${APP_HEALTH_URL:-http://127.0.0.1:${APP_HOST_PORT}/health}"
STOP_APP_DURING_BACKUP="${STOP_APP_DURING_BACKUP:-1}"
APP_HEALTH_TIMEOUT_SECONDS="${APP_HEALTH_TIMEOUT_SECONDS:-120}"

source "${ROOT_DIR}/scripts/compose-runtime.sh"

if [[ "${COMPOSE_VARIANT}" != "sqlite" ]]; then
  echo "sqlite-ops-backup.sh only supports COMPOSE_VARIANT=sqlite" >&2
  exit 1
fi

DATA_DIR="${DATA_DIR:-$(compose_variant_default_data_dir)}"
ARTIFACT_DIR="${ARTIFACT_DIR:-ops-backups/sqlite}"

log_step() {
  echo
  echo "==> $1"
}

require_commands() {
  local required_commands=(docker curl jq tar sha256sum date)

  for command in "${required_commands[@]}"; do
    if ! command -v "${command}" >/dev/null 2>&1; then
      echo "Missing required command: ${command}" >&2
      exit 1
    fi
  done
}

current_timestamp_utc() {
  date -u +"%Y-%m-%dT%H:%M:%SZ"
}

current_stamp_compact() {
  date -u +"%Y%m%dT%H%M%SZ"
}

sha256_file() {
  local path="$1"
  sha256sum "${path}" | awk '{print $1}'
}

write_json_file() {
  local path="$1"
  local payload="$2"

  mkdir -p "$(dirname "${path}")"
  printf '%s\n' "${payload}" > "${path}"
}

absolute_existing_dir() {
  local path="$1"

  if [[ ! -d "${path}" ]]; then
    echo "Directory does not exist: ${path}" >&2
    exit 1
  fi

  (
    cd "${path}"
    pwd -P
  )
}

wait_for_app_health() {
  local timeout_seconds="${1:-120}"
  local deadline=$((SECONDS + timeout_seconds))

  while (( SECONDS < deadline )); do
    if curl -fsS "${APP_HEALTH_URL}" >/dev/null 2>&1; then
      return 0
    fi
    sleep 2
  done

  echo "Timed out waiting for app health: ${APP_HEALTH_URL}" >&2
  return 1
}

require_commands

mkdir -p "${ARTIFACT_DIR}"
mkdir -p "${DATA_DIR}/backup"

data_dir_abs="$(absolute_existing_dir "${DATA_DIR}")"
artifact_dir_abs="$(absolute_existing_dir "${ARTIFACT_DIR}")"

case "${artifact_dir_abs}/" in
  "${data_dir_abs}/"*)
    echo "ARTIFACT_DIR must not be inside DATA_DIR to avoid recursive archives" >&2
    exit 1
    ;;
esac

stamp="$(current_stamp_compact)"
backup_prefix="${SQLITE_BACKUP_PREFIX:-sqlite_ops_backup_${stamp}}"
archive_path="${artifact_dir_abs}/${backup_prefix}.tar.gz"
checksum_path="${archive_path}.sha256"
manifest_path="${artifact_dir_abs}/${backup_prefix}.manifest.json"
latest_manifest_path="${data_dir_abs}/backup/sqlite_last_cold_backup_manifest.json"

archive_parent="$(dirname "${data_dir_abs}")"
archive_root="$(basename "${data_dir_abs}")"

declare -a included_standard_entries=()
for entry in bootstrap sqlite images backup; do
  if [[ -e "${data_dir_abs}/${entry}" ]]; then
    included_standard_entries+=("${entry}")
  fi
done

app_was_running=0
if compose ps --status running --services 2>/dev/null | grep -qx "${APP_SERVICE}"; then
  app_was_running=1
fi

app_stopped_for_backup=0
restore_app_if_needed() {
  if [[ "${app_stopped_for_backup}" == "1" ]]; then
    log_step "Starting app after cold backup"
    compose start "${APP_SERVICE}" >/dev/null
    wait_for_app_health "${APP_HEALTH_TIMEOUT_SECONDS}"
    app_stopped_for_backup=0
  fi
}

trap restore_app_if_needed EXIT

if [[ "${STOP_APP_DURING_BACKUP}" == "1" && "${app_was_running}" == "1" ]]; then
  log_step "Stopping app for SQLite cold backup"
  compose stop "${APP_SERVICE}" >/dev/null
  app_stopped_for_backup=1
fi

log_step "Creating SQLite cold backup archive"
tar -C "${archive_parent}" -czf "${archive_path}" "${archive_root}"

if [[ ! -s "${archive_path}" ]]; then
  echo "SQLite cold backup archive is empty: ${archive_path}" >&2
  exit 1
fi

restore_app_if_needed

archive_sha256="$(sha256_file "${archive_path}")"
printf '%s  %s\n' "${archive_sha256}" "$(basename "${archive_path}")" > "${checksum_path}"

created_at="$(current_timestamp_utc)"
manifest_payload="$(
  jq -n \
    --arg created_at "${created_at}" \
    --arg compose_project_name "${COMPOSE_PROJECT_NAME}" \
    --arg compose_variant "${COMPOSE_VARIANT}" \
    --arg cache_mode "${CACHE_MODE}" \
    --arg data_dir "${data_dir_abs}" \
    --arg archive_root "${archive_root}" \
    --arg archive_path "${archive_path}" \
    --arg archive_sha256 "${archive_sha256}" \
    --arg checksum_path "${checksum_path}" \
    --argjson compose_files "$(printf '%s\n' "${compose_files[@]}" | jq -R . | jq -s .)" \
    --argjson included_standard_entries "$(printf '%s\n' "${included_standard_entries[@]}" | jq -R . | jq -s .)" \
    --argjson app_was_running "$([[ "${app_was_running}" == "1" ]] && echo true || echo false)" \
    --argjson app_stopped_for_backup "$([[ "${STOP_APP_DURING_BACKUP}" == "1" && "${app_was_running}" == "1" ]] && echo true || echo false)" \
    '{
      created_at: $created_at,
      backup_kind: "sqlite-full-site-cold-backup",
      backup_scope: "data-dir",
      restore_mode: "ops-data-dir-replace",
      compose_project_name: $compose_project_name,
      compose_variant: $compose_variant,
      compose_files: $compose_files,
      cache_mode: $cache_mode,
      data_dir: $data_dir,
      archive: {
        root: $archive_root,
        path: $archive_path,
        sha256: $archive_sha256
      },
      checksum_path: $checksum_path,
      included_standard_entries: $included_standard_entries,
      app_was_running: $app_was_running,
      app_stopped_for_backup: $app_stopped_for_backup,
      notes: [
        "This archive captures the mounted SQLite DATA_DIR, not just the database file.",
        "Restore requires stopping app and replacing the whole DATA_DIR from the archive.",
        "If storage_backend uses S3 or MinIO, external object data is not included."
      ]
    }'
)"

write_json_file "${manifest_path}" "${manifest_payload}"
write_json_file "${latest_manifest_path}" "${manifest_payload}"

echo "SQLite cold backup created:"
echo "  archive: ${archive_path}"
echo "  checksum: ${checksum_path}"
echo "  manifest: ${manifest_path}"
