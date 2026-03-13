#!/usr/bin/env bash

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME:-vansour-image}"
COMPOSE_VARIANT="${COMPOSE_VARIANT:-postgres}"
POSTGRES_SERVICE="${POSTGRES_SERVICE:-postgres}"
APP_SERVICE="${APP_SERVICE:-app}"
APP_HOST_PORT="${APP_HOST_PORT:-8080}"
APP_HEALTH_URL="${APP_HEALTH_URL:-http://127.0.0.1:${APP_HOST_PORT}/health}"

source "${ROOT_DIR}/scripts/compose-runtime.sh"

DATA_DIR="${DATA_DIR:-$(compose_variant_default_data_dir)}"
ARTIFACT_DIR="${ARTIFACT_DIR:-ops-backups/postgres}"
POSTGRES_LAST_PHYSICAL_BACKUP_MANIFEST_PATH="${POSTGRES_LAST_PHYSICAL_BACKUP_MANIFEST_PATH:-${DATA_DIR}/backup/postgres_last_physical_backup_manifest.json}"
POSTGRES_LAST_RESTORE_RESULT_PATH="${POSTGRES_LAST_RESTORE_RESULT_PATH:-${DATA_DIR}/backup/postgres_last_restore_result.json}"

log_step() {
  echo
  echo "==> $1"
}

require_postgres_variant() {
  if [[ "${COMPOSE_VARIANT}" != "postgres" ]]; then
    echo "PostgreSQL ops scripts only support COMPOSE_VARIANT=postgres" >&2
    exit 1
  fi
}

require_commands() {
  local required_commands=(docker curl jq tar sha256sum date mktemp)

  for command in "${required_commands[@]}"; do
    if ! command -v "${command}" >/dev/null 2>&1; then
      echo "Missing required command: ${command}" >&2
      exit 1
    fi
  done
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

absolute_dir_path() {
  local path="$1"

  mkdir -p "${path}"
  (
    cd "${path}"
    pwd -P
  )
}

ensure_directory_nonempty() {
  local path="$1"
  local label="$2"

  if [[ ! -d "${path}" ]]; then
    echo "${label} does not exist: ${path}" >&2
    exit 1
  fi
  if ! find "${path}" -mindepth 1 -print -quit | grep -q .; then
    echo "${label} is empty: ${path}" >&2
    exit 1
  fi
}

ensure_file_nonempty() {
  local path="$1"
  local label="$2"

  if [[ ! -f "${path}" ]]; then
    echo "${label} does not exist: ${path}" >&2
    exit 1
  fi
  if [[ ! -s "${path}" ]]; then
    echo "${label} is empty: ${path}" >&2
    exit 1
  fi
}

ensure_json_file() {
  local path="$1"
  local label="$2"

  if [[ ! -f "${path}" ]]; then
    echo "${label} does not exist: ${path}" >&2
    exit 1
  fi
  if ! jq empty "${path}" >/dev/null 2>&1; then
    echo "${label} is not valid JSON: ${path}" >&2
    exit 1
  fi
}

ensure_stack_service_running() {
  local service="$1"

  if ! compose ps --status running --services | grep -qx "${service}"; then
    echo "Required compose service is not running: ${service}" >&2
    exit 1
  fi
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

verify_file_sha256() {
  local path="$1"
  local expected="$2"
  local label="$3"

  ensure_file_nonempty "${path}" "${label}"
  if [[ -z "${expected}" || "${expected}" == "null" ]]; then
    echo "${label} missing expected sha256" >&2
    exit 1
  fi

  local actual
  actual="$(sha256_file "${path}")"
  if [[ "${actual}" != "${expected}" ]]; then
    echo "${label} sha256 mismatch: expected ${expected}, got ${actual}" >&2
    exit 1
  fi
}

write_json_file() {
  local path="$1"
  local payload="$2"

  compose_write_host_file "$(dirname "${path}")" "${path}" "${payload}"
}

wait_for_compose_service_health() {
  local service="$1"
  local timeout_seconds="${2:-120}"
  local container_id
  local deadline
  local status

  container_id="$(compose_service_container_id "${service}")"
  if [[ -z "${container_id}" ]]; then
    echo "Unable to resolve container id for service ${service}" >&2
    return 1
  fi

  deadline=$((SECONDS + timeout_seconds))
  while (( SECONDS < deadline )); do
    status="$(
      docker inspect -f '{{if .State.Health}}{{.State.Health.Status}}{{else}}{{.State.Status}}{{end}}' "${container_id}" 2>/dev/null || true
    )"

    case "${status}" in
      healthy|running)
        return 0
        ;;
      unhealthy|exited|dead)
        echo "Service ${service} entered bad state: ${status}" >&2
        return 1
        ;;
    esac

    sleep 2
  done

  echo "Timed out waiting for service health: ${service}" >&2
  return 1
}

compose_service_container_id() {
  local service="$1"
  compose ps -a -q "${service}" | head -n 1
}

postgres_service_container_id() {
  compose_service_container_id "${POSTGRES_SERVICE}"
}

postgres_service_container_image() {
  local container_id
  container_id="$(postgres_service_container_id)"

  if [[ -z "${container_id}" ]]; then
    echo "Unable to resolve running container id for service ${POSTGRES_SERVICE}" >&2
    exit 1
  fi

  docker inspect -f '{{.Config.Image}}' "${container_id}"
}

postgres_service_image_user_id() {
  local service_image
  service_image="$(postgres_service_container_image)"
  docker run --rm "${service_image}" sh -lc 'id -u postgres'
}

postgres_service_image_group_id() {
  local service_image
  service_image="$(postgres_service_container_image)"
  docker run --rm "${service_image}" sh -lc 'id -g postgres'
}

postgres_service_pgdata() {
  local container_id
  local pgdata

  container_id="$(postgres_service_container_id)"
  if [[ -z "${container_id}" ]]; then
    echo "Unable to resolve container id for service ${POSTGRES_SERVICE}" >&2
    exit 1
  fi

  pgdata="$(
    docker inspect -f '{{range .Config.Env}}{{println .}}{{end}}' "${container_id}" \
      | awk -F= '$1 == "PGDATA" { print substr($0, index($0, "=") + 1); exit }'
  )"
  if [[ -n "${pgdata}" ]]; then
    printf '%s' "${pgdata}"
    return 0
  fi

  docker run --rm "$(postgres_service_container_image)" sh -lc 'printf "%s" "${PGDATA}"'
}

postgres_env_value() {
  local env_name="$1"
  local fallback="$2"

  compose exec -T "${POSTGRES_SERVICE}" sh -lc "
set -eu
value=\${${env_name}:-}
if [ -n \"\${value}\" ]; then
  printf '%s' \"\${value}\"
else
  printf '%s' \"${fallback}\"
fi
"
}

postgres_primary_user() {
  postgres_env_value "POSTGRES_USER" "postgres"
}

postgres_primary_password() {
  local password
  password="$(postgres_env_value "POSTGRES_PASSWORD" "")"
  if [[ -z "${password}" ]]; then
    echo "POSTGRES_PASSWORD is required" >&2
    exit 1
  fi

  printf '%s' "${password}"
}

postgres_primary_database() {
  postgres_env_value "POSTGRES_DB" "postgres"
}

postgres_wal_archive_enabled() {
  [[ "${POSTGRES_ENABLE_WAL_ARCHIVE:-0}" == "1" ]]
}

postgres_wal_archive_host_dir() {
  compose_variant_resolved_postgres_wal_archive_host_dir
}

postgres_wal_archive_mount_path() {
  printf '%s' "${POSTGRES_WAL_ARCHIVE_MOUNT_PATH:-/wal-archive}"
}

postgres_wal_remote_uri() {
  printf '%s' "${POSTGRES_WAL_REMOTE_URI:-}"
}

postgres_wal_remote_enabled() {
  [[ -n "$(postgres_wal_remote_uri)" ]]
}

postgres_wal_remote_scheme() {
  local remote_uri="$1"

  case "${remote_uri}" in
    s3://*)
      printf 's3'
      ;;
    file://*|/*)
      printf 'file'
      ;;
    *)
      echo "Unsupported PostgreSQL WAL remote URI: ${remote_uri}" >&2
      return 1
      ;;
  esac
}

postgres_wal_remote_file_dir() {
  local remote_uri="$1"
  local path

  case "${remote_uri}" in
    file://*)
      path="${remote_uri#file://}"
      ;;
    /*)
      path="${remote_uri}"
      ;;
    *)
      echo "Unsupported PostgreSQL WAL file remote URI: ${remote_uri}" >&2
      return 1
      ;;
  esac

  if [[ "${path}" != /* ]]; then
    path="${ROOT_DIR}/${path}"
  fi

  printf '%s' "${path}"
}

postgres_wal_remote_resolved_uri() {
  local remote_uri="$1"
  local scheme

  scheme="$(postgres_wal_remote_scheme "${remote_uri}")"
  case "${scheme}" in
    file)
      printf 'file://%s' "$(postgres_wal_remote_file_dir "${remote_uri}")"
      ;;
    s3)
      printf '%s' "${remote_uri}"
      ;;
  esac
}

postgres_wal_remote_s3_bucket() {
  local remote_uri="$1"
  local remainder="${remote_uri#s3://}"
  printf '%s' "${remainder%%/*}"
}

postgres_wal_remote_s3_prefix() {
  local remote_uri="$1"
  local remainder="${remote_uri#s3://}"

  if [[ "${remainder}" == */* ]]; then
    printf '%s' "${remainder#*/}"
  else
    printf '%s' ""
  fi
}

postgres_wal_remote_require_aws_cli() {
  if ! command -v aws >/dev/null 2>&1; then
    echo "PostgreSQL WAL remote sync for s3:// requires aws CLI on the host." >&2
    return 1
  fi
}

postgres_wal_remote_aws() {
  local temp_root=""
  local config_path=""
  local credentials_path=""
  local endpoint="${POSTGRES_WAL_REMOTE_ENDPOINT:-}"
  local region="${POSTGRES_WAL_REMOTE_REGION:-us-east-1}"
  local access_key="${POSTGRES_WAL_REMOTE_ACCESS_KEY:-}"
  local secret_key="${POSTGRES_WAL_REMOTE_SECRET_KEY:-}"
  local force_path_style="${POSTGRES_WAL_REMOTE_FORCE_PATH_STYLE:-1}"
  local aws_command=(aws)

  postgres_wal_remote_require_aws_cli || return 1

  temp_root="$(mktemp -d /tmp/vansour-postgres-wal-aws-XXXXXX)"
  config_path="${temp_root}/config"
  credentials_path="${temp_root}/credentials"

  {
    printf '[default]\n'
    printf 'region = %s\n' "${region}"
    printf 's3 =\n'
    if [[ "${force_path_style}" == "1" ]]; then
      printf '  addressing_style = path\n'
    else
      printf '  addressing_style = auto\n'
    fi
  } > "${config_path}"

  if [[ -n "${access_key}" || -n "${secret_key}" ]]; then
    if [[ -z "${access_key}" || -z "${secret_key}" ]]; then
      echo "POSTGRES_WAL_REMOTE_ACCESS_KEY and POSTGRES_WAL_REMOTE_SECRET_KEY must be provided together." >&2
      rm -rf "${temp_root}"
      return 1
    fi
    {
      printf '[default]\n'
      printf 'aws_access_key_id = %s\n' "${access_key}"
      printf 'aws_secret_access_key = %s\n' "${secret_key}"
    } > "${credentials_path}"
  else
    : > "${credentials_path}"
  fi

  if [[ -n "${endpoint}" ]]; then
    aws_command+=(--endpoint-url "${endpoint}")
  fi

  set +e
  AWS_CONFIG_FILE="${config_path}" \
  AWS_SHARED_CREDENTIALS_FILE="${credentials_path}" \
  AWS_EC2_METADATA_DISABLED=true \
  "${aws_command[@]}" "$@"
  local exit_code=$?
  set -e
  rm -rf "${temp_root}"
  return "${exit_code}"
}

copy_missing_files_between_dirs() {
  local source_dir="$1"
  local target_dir="$2"
  local resolved_source_dir=""
  local resolved_target_dir=""
  local copied=0
  local source_file=""
  local target_file=""
  local file_name=""
  local helper_image=""

  resolved_source_dir="$(absolute_existing_dir "${source_dir}")"
  resolved_target_dir="$(absolute_dir_path "${target_dir}")"
  helper_image="${COMPOSE_FS_HELPER_IMAGE:-busybox:1.37.0}"
  while IFS= read -r -d '' source_file; do
    file_name="$(basename "${source_file}")"
    target_file="${resolved_target_dir}/${file_name}"
    if [[ ! -f "${target_file}" ]]; then
      if ! cp -p "${source_file}" "${target_file}" 2>/dev/null; then
        docker run --rm \
          -v "${resolved_source_dir}:/source:ro" \
          -v "${resolved_target_dir}:/target" \
          -e FILE_NAME="${file_name}" \
          --entrypoint sh \
          "${helper_image}" \
          -lc 'set -eu; if [ ! -f "/target/${FILE_NAME}" ]; then cp -p "/source/${FILE_NAME}" "/target/${FILE_NAME}"; fi'
      fi
      copied=1
    fi
  done < <(find "${resolved_source_dir}" -maxdepth 1 -type f -print0)

  return 0
}

postgres_wal_remote_sync_push() {
  local local_dir="$1"
  local remote_uri="$2"
  local scheme
  local remote_dir

  ensure_directory_nonempty "${local_dir}" "PostgreSQL WAL archive source directory"
  scheme="$(postgres_wal_remote_scheme "${remote_uri}")"

  case "${scheme}" in
    file)
      remote_dir="$(postgres_wal_remote_file_dir "${remote_uri}")"
      mkdir -p "${remote_dir}"
      copy_missing_files_between_dirs "${local_dir}" "${remote_dir}"
      ;;
    s3)
      postgres_wal_remote_aws s3 sync --only-show-errors "${local_dir}/" "${remote_uri%/}/"
      ;;
  esac
}

postgres_wal_remote_sync_pull() {
  local remote_uri="$1"
  local local_dir="$2"
  local scheme
  local remote_dir

  mkdir -p "${local_dir}"
  scheme="$(postgres_wal_remote_scheme "${remote_uri}")"

  case "${scheme}" in
    file)
      remote_dir="$(postgres_wal_remote_file_dir "${remote_uri}")"
      ensure_directory_nonempty "${remote_dir}" "PostgreSQL WAL remote directory"
      copy_missing_files_between_dirs "${remote_dir}" "${local_dir}"
      ;;
    s3)
      postgres_wal_remote_aws s3 sync --only-show-errors "${remote_uri%/}/" "${local_dir}/"
      ;;
  esac
}

wal_segment_name_is_valid() {
  [[ "$1" =~ ^[0-9A-F]{24}$ ]]
}

postgres_backup_label_start_wal_file_from_text() {
  local backup_label_raw="$1"

  sed -nE 's/^START WAL LOCATION: .* \(file ([0-9A-F]{24})\).*$/\1/p' <<<"${backup_label_raw}" | head -n 1
}

postgres_backup_label_start_timeline_from_text() {
  local start_wal_file
  start_wal_file="$(postgres_backup_label_start_wal_file_from_text "$1")"
  if [[ -n "${start_wal_file}" ]]; then
    printf '%s' "${start_wal_file:0:8}"
  fi
}

postgres_prune_local_wal_segments_before() {
  local wal_dir="$1"
  local threshold_segment="$2"
  local dry_run="${3:-0}"
  local threshold_timeline="${threshold_segment:0:8}"
  local file_name
  local deleted_count=0

  if [[ ! -d "${wal_dir}" ]]; then
    return 0
  fi

  while IFS= read -r file_name; do
    if ! wal_segment_name_is_valid "${file_name}"; then
      continue
    fi
    if [[ "${file_name:0:8}" != "${threshold_timeline}" ]]; then
      continue
    fi
    if [[ "${file_name}" < "${threshold_segment}" ]]; then
      if [[ "${dry_run}" == "1" ]]; then
        printf 'would delete local WAL segment: %s/%s\n' "${wal_dir}" "${file_name}" >&2
      else
        rm -f "${wal_dir}/${file_name}"
      fi
      deleted_count=$((deleted_count + 1))
    fi
  done < <(find "${wal_dir}" -maxdepth 1 -type f -printf '%f\n' | sort)

  printf '%s' "${deleted_count}"
}

postgres_prune_remote_wal_segments_before() {
  local remote_uri="$1"
  local threshold_segment="$2"
  local dry_run="${3:-0}"
  local scheme
  local threshold_timeline="${threshold_segment:0:8}"
  local deleted_count=0
  local file_name
  local remote_dir
  local bucket
  local prefix
  local key

  scheme="$(postgres_wal_remote_scheme "${remote_uri}")"

  case "${scheme}" in
    file)
      remote_dir="$(postgres_wal_remote_file_dir "${remote_uri}")"
      if [[ ! -d "${remote_dir}" ]]; then
        printf '0'
        return 0
      fi

      while IFS= read -r file_name; do
        if ! wal_segment_name_is_valid "${file_name}"; then
          continue
        fi
        if [[ "${file_name:0:8}" != "${threshold_timeline}" ]]; then
          continue
        fi
        if [[ "${file_name}" < "${threshold_segment}" ]]; then
          if [[ "${dry_run}" == "1" ]]; then
            printf 'would delete remote WAL segment: %s/%s\n' "${remote_dir}" "${file_name}" >&2
          else
            rm -f "${remote_dir}/${file_name}"
          fi
          deleted_count=$((deleted_count + 1))
        fi
      done < <(find "${remote_dir}" -maxdepth 1 -type f -printf '%f\n' | sort)
      ;;
    s3)
      bucket="$(postgres_wal_remote_s3_bucket "${remote_uri}")"
      prefix="$(postgres_wal_remote_s3_prefix "${remote_uri}")"
      while IFS= read -r key; do
        file_name="$(basename "${key}")"
        if ! wal_segment_name_is_valid "${file_name}"; then
          continue
        fi
        if [[ "${file_name:0:8}" != "${threshold_timeline}" ]]; then
          continue
        fi
        if [[ "${file_name}" < "${threshold_segment}" ]]; then
          if [[ "${dry_run}" == "1" ]]; then
            printf 'would delete remote WAL segment: s3://%s/%s\n' "${bucket}" "${key}" >&2
          else
            postgres_wal_remote_aws s3 rm "s3://${bucket}/${key}" >/dev/null
          fi
          deleted_count=$((deleted_count + 1))
        fi
      done < <(
        if [[ -n "${prefix}" ]]; then
          postgres_wal_remote_aws s3api list-objects-v2 \
            --bucket "${bucket}" \
            --prefix "${prefix%/}/" \
            --output json
        else
          postgres_wal_remote_aws s3api list-objects-v2 \
            --bucket "${bucket}" \
            --output json
        fi | jq -r '.Contents[]?.Key'
      )
      ;;
  esac

  printf '%s' "${deleted_count}"
}

postgres_escape_sql_literal() {
  printf "%s" "$1" | sed "s/'/''/g"
}

postgres_query_value() {
  local sql="$1"

  compose exec -T "${POSTGRES_SERVICE}" sh -lc '
set -eu
user="${POSTGRES_USER:-postgres}"
password="${POSTGRES_PASSWORD:-}"
database="${POSTGRES_DB:-postgres}"
export PGPASSWORD="${password}"
exec psql -h 127.0.0.1 -U "${user}" -d "${database}" -v ON_ERROR_STOP=1 -Atq
' <<<"${sql}" | tr -d '\r'
}

postgres_exec_sql() {
  local sql="$1"
  postgres_query_value "${sql}" >/dev/null
}

postgres_current_setting() {
  local name="$1"
  local escaped_name
  escaped_name="$(postgres_escape_sql_literal "${name}")"
  postgres_query_value "SELECT current_setting('${escaped_name}');"
}

postgres_current_timestamp_utc() {
  postgres_query_value "SELECT to_char(clock_timestamp() AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"');"
}

postgres_create_restore_point() {
  local restore_point_name="$1"
  local escaped_name
  escaped_name="$(postgres_escape_sql_literal "${restore_point_name}")"
  postgres_query_value "SELECT pg_create_restore_point('${escaped_name}');"
}

postgres_switch_wal() {
  postgres_exec_sql "SELECT pg_switch_wal();"
}

directory_regular_file_count() {
  local path="$1"

  if [[ ! -d "${path}" ]]; then
    printf '0'
    return 0
  fi

  find "${path}" -maxdepth 1 -type f | wc -l | awk '{print $1}'
}

wait_for_postgres_not_in_recovery() {
  local timeout_seconds="${1:-120}"
  local deadline=$((SECONDS + timeout_seconds))
  local in_recovery
  local query_exit_code
  local query_output
  local last_error=""

  while (( SECONDS < deadline )); do
    set +e
    query_output="$(postgres_query_value "SELECT CASE WHEN pg_is_in_recovery() THEN 'true' ELSE 'false' END;" 2>&1)"
    query_exit_code=$?
    set -e

    if [[ "${query_exit_code}" -ne 0 ]]; then
      last_error="${query_output}"
      sleep 2
      continue
    fi

    in_recovery="${query_output}"
    case "${in_recovery}" in
      false)
        return 0
        ;;
      true)
        sleep 2
        ;;
      *)
        echo "Unexpected pg_is_in_recovery() result: ${in_recovery}" >&2
        return 1
        ;;
    esac
  done

  if [[ -n "${last_error}" ]]; then
    echo "Last PostgreSQL recovery probe error: ${last_error}" >&2
  fi
  echo "Timed out waiting for PostgreSQL promotion after PITR" >&2
  return 1
}

wait_for_postgres_wal_archive_growth() {
  local wal_archive_dir="$1"
  local initial_count="$2"
  local timeout_seconds="${3:-60}"
  local deadline=$((SECONDS + timeout_seconds))
  local current_count

  while (( SECONDS < deadline )); do
    current_count="$(directory_regular_file_count "${wal_archive_dir}")"
    if (( current_count > initial_count )); then
      return 0
    fi
    sleep 2
  done

  echo "Timed out waiting for PostgreSQL WAL archive growth in ${wal_archive_dir}" >&2
  return 1
}

postgres_force_wal_switch_and_wait() {
  local wal_archive_dir="$1"
  local timeout_seconds="${2:-60}"
  local initial_count

  initial_count="$(directory_regular_file_count "${wal_archive_dir}")"
  postgres_switch_wal
  wait_for_postgres_wal_archive_growth "${wal_archive_dir}" "${initial_count}" "${timeout_seconds}"
}

postgres_physical_helper_image() {
  if [[ -n "${POSTGRES_PHYSICAL_HELPER_IMAGE:-}" ]]; then
    printf '%s' "${POSTGRES_PHYSICAL_HELPER_IMAGE}"
  else
    postgres_service_container_image
  fi
}

read_optional_file() {
  local path="$1"

  compose_read_optional_host_file "$(dirname "${path}")" "${path}"
}

postgres_directory_size_bytes() {
  local path="$1"
  compose_directory_size_bytes "$(dirname "${path}")" "${path}"
}

postgres_physical_backup_to_dir() {
  local target_dir="$1"
  local log_path="$2"
  local label="$3"
  local target_parent
  local target_basename
  local container_id
  local helper_image
  local postgres_user
  local postgres_password

  ensure_stack_service_running "${POSTGRES_SERVICE}"

  rm -rf "${target_dir}"
  mkdir -p "$(dirname "${target_dir}")"
  target_parent="$(absolute_existing_dir "$(dirname "${target_dir}")")"
  target_basename="$(basename "${target_dir}")"
  container_id="$(postgres_service_container_id)"
  helper_image="$(postgres_physical_helper_image)"
  postgres_user="$(postgres_primary_user)"
  postgres_password="$(postgres_primary_password)"

  if [[ -z "${container_id}" ]]; then
    echo "Unable to resolve running container id for service ${POSTGRES_SERVICE}" >&2
    exit 1
  fi

  docker run --rm \
    --user 0:0 \
    --network "container:${container_id}" \
    -v "${target_parent}:/backup-root" \
    -e PGPASSWORD="${postgres_password}" \
    -e POSTGRES_USER="${postgres_user}" \
    -e PG_BASEBACKUP_LABEL="${label}" \
    "${helper_image}" \
    sh -lc '
set -eu
target_dir="/backup-root/'"${target_basename}"'"
rm -rf "${target_dir}"
mkdir -p "${target_dir}"
exec pg_basebackup \
  --pgdata="${target_dir}" \
  --host=127.0.0.1 \
  --port=5432 \
  --username="${POSTGRES_USER}" \
  --format=plain \
  --wal-method=stream \
  --checkpoint=fast \
  --label="${PG_BASEBACKUP_LABEL}" \
  --verbose \
  --no-password
' > "${log_path}" 2>&1

  ensure_directory_nonempty "${target_dir}" "PostgreSQL physical backup directory"
  ensure_file_nonempty "${target_dir}/PG_VERSION" "PostgreSQL PG_VERSION file"
}

postgres_data_dir_archive_to_file() {
  local archive_path="$1"
  local archive_parent
  local archive_name
  local container_id
  local service_image
  local pgdata

  container_id="$(postgres_service_container_id)"
  if [[ -z "${container_id}" ]]; then
    echo "Unable to resolve container id for service ${POSTGRES_SERVICE}" >&2
    exit 1
  fi

  mkdir -p "$(dirname "${archive_path}")"
  archive_parent="$(absolute_existing_dir "$(dirname "${archive_path}")")"
  archive_name="$(basename "${archive_path}")"
  service_image="$(postgres_service_container_image)"
  pgdata="$(postgres_service_pgdata)"

  docker run --rm \
    --user 0:0 \
    --volumes-from "${container_id}" \
    -v "${archive_parent}:/backup-root" \
    -e PGDATA="${pgdata}" \
    "${service_image}" \
    sh -lc '
set -eu
if [ ! -d "${PGDATA}" ]; then
  echo "PGDATA directory does not exist: ${PGDATA}" >&2
  exit 1
fi
tar -C "${PGDATA}" -czf "/backup-root/'"${archive_name}"'" .
' >/dev/null

  if [[ ! -f "${archive_path}" || ! -s "${archive_path}" ]]; then
    echo "PostgreSQL datadir archive is missing or empty: ${archive_path}" >&2
    return 1
  fi
}

postgres_data_dir_restore_from_archive() {
  local archive_path="$1"
  local log_path="$2"
  local archive_parent
  local archive_name
  local container_id
  local service_image
  local pgdata
  local postgres_uid
  local postgres_gid

  ensure_file_nonempty "${archive_path}" "PostgreSQL datadir archive"

  container_id="$(postgres_service_container_id)"
  if [[ -z "${container_id}" ]]; then
    echo "Unable to resolve container id for service ${POSTGRES_SERVICE}" >&2
    exit 1
  fi

  archive_parent="$(absolute_existing_dir "$(dirname "${archive_path}")")"
  archive_name="$(basename "${archive_path}")"
  service_image="$(postgres_service_container_image)"
  pgdata="$(postgres_service_pgdata)"
  postgres_uid="$(postgres_service_image_user_id)"
  postgres_gid="$(postgres_service_image_group_id)"

  docker run --rm \
    --user 0:0 \
    --volumes-from "${container_id}" \
    -v "${archive_parent}:/backup-root:ro" \
    -e PGDATA="${pgdata}" \
    -e POSTGRES_UID="${postgres_uid}" \
    -e POSTGRES_GID="${postgres_gid}" \
    "${service_image}" \
    sh -lc '
set -eu
mkdir -p "${PGDATA}"
find "${PGDATA}" -mindepth 1 -maxdepth 1 -exec rm -rf {} +
tar -xzf "/backup-root/'"${archive_name}"'" -C "${PGDATA}"
chown -R "${POSTGRES_UID}:${POSTGRES_GID}" "${PGDATA}"
' > "${log_path}" 2>&1
}

postgres_physical_copy_back_from_dir() {
  local source_dir="$1"
  local log_path="$2"
  local source_parent
  local source_basename
  local container_id
  local service_image
  local pgdata
  local postgres_uid
  local postgres_gid

  ensure_directory_nonempty "${source_dir}" "PostgreSQL physical backup directory"

  container_id="$(postgres_service_container_id)"
  if [[ -z "${container_id}" ]]; then
    echo "Unable to resolve container id for service ${POSTGRES_SERVICE}" >&2
    exit 1
  fi

  source_parent="$(absolute_existing_dir "$(dirname "${source_dir}")")"
  source_basename="$(basename "${source_dir}")"
  service_image="$(postgres_service_container_image)"
  pgdata="$(postgres_service_pgdata)"
  postgres_uid="$(postgres_service_image_user_id)"
  postgres_gid="$(postgres_service_image_group_id)"

  docker run --rm \
    --user 0:0 \
    --volumes-from "${container_id}" \
    -v "${source_parent}:/backup-root:ro" \
    -e PGDATA="${pgdata}" \
    -e POSTGRES_UID="${postgres_uid}" \
    -e POSTGRES_GID="${postgres_gid}" \
    "${service_image}" \
    sh -lc '
set -eu
source_dir="/backup-root/'"${source_basename}"'"
mkdir -p "${PGDATA}"
find "${PGDATA}" -mindepth 1 -maxdepth 1 -exec rm -rf {} +
tar -C "${source_dir}" -cf - . | tar -C "${PGDATA}" -xf -
chown -R "${POSTGRES_UID}:${POSTGRES_GID}" "${PGDATA}"
' > "${log_path}" 2>&1
}

postgres_config_escape_literal() {
  printf "%s" "$1" | sed "s/'/''/g"
}

postgres_normalize_recovery_target_time() {
  local raw_value="$1"

  if ! date -u -d "${raw_value}" +"%Y-%m-%d %H:%M:%S.%N+00" >/dev/null 2>&1; then
    echo "Invalid PostgreSQL PITR target time: ${raw_value}" >&2
    return 1
  fi

  date -u -d "${raw_value}" +"%Y-%m-%d %H:%M:%S.%N+00"
}

postgres_write_pitr_recovery_config() {
  local target_kind="$1"
  local target_value="$2"
  local log_path="$3"
  local container_id
  local service_image
  local pgdata
  local postgres_uid
  local postgres_gid
  local mount_path
  local target_value_escaped
  local normalized_target_value
  local restore_command_value

  case "${target_kind}" in
    name|time)
      ;;
    *)
      echo "Unsupported PostgreSQL PITR target kind: ${target_kind}" >&2
      return 1
      ;;
  esac

  container_id="$(postgres_service_container_id)"
  if [[ -z "${container_id}" ]]; then
    echo "Unable to resolve container id for service ${POSTGRES_SERVICE}" >&2
    exit 1
  fi

  service_image="$(postgres_service_container_image)"
  pgdata="$(postgres_service_pgdata)"
  postgres_uid="$(postgres_service_image_user_id)"
  postgres_gid="$(postgres_service_image_group_id)"
  mount_path="$(postgres_wal_archive_mount_path)"
  normalized_target_value="${target_value}"
  if [[ "${target_kind}" == "time" ]]; then
    normalized_target_value="$(postgres_normalize_recovery_target_time "${target_value}")" || return 1
  fi
  target_value_escaped="$(postgres_config_escape_literal "${normalized_target_value}")"
  restore_command_value="cp ${mount_path}/%f %p"

  docker run --rm \
    --user 0:0 \
    --volumes-from "${container_id}" \
    -e PGDATA="${pgdata}" \
    -e POSTGRES_UID="${postgres_uid}" \
    -e POSTGRES_GID="${postgres_gid}" \
    -e RESTORE_COMMAND_VALUE="${restore_command_value}" \
    -e RECOVERY_TARGET_KIND="${target_kind}" \
    -e RECOVERY_TARGET_VALUE="${target_value_escaped}" \
    "${service_image}" \
    sh -lc '
set -eu
auto_conf="${PGDATA}/postgresql.auto.conf"
tmp_conf="${auto_conf}.tmp"
mkdir -p "${PGDATA}"
if [ -f "${auto_conf}" ]; then
  grep -v -E "^(restore_command|recovery_target_action|recovery_target_timeline|recovery_target_name|recovery_target_time)[[:space:]]*=" "${auto_conf}" > "${tmp_conf}" || true
  mv "${tmp_conf}" "${auto_conf}"
fi
printf "%s\n" "restore_command = '\''${RESTORE_COMMAND_VALUE}'\''" >> "${auto_conf}"
printf "%s\n" "recovery_target_action = '\''promote'\''" >> "${auto_conf}"
printf "%s\n" "recovery_target_timeline = '\''latest'\''" >> "${auto_conf}"
case "${RECOVERY_TARGET_KIND}" in
  name)
    printf "%s\n" "recovery_target_name = '\''${RECOVERY_TARGET_VALUE}'\''" >> "${auto_conf}"
    ;;
  time)
    printf "%s\n" "recovery_target_time = '\''${RECOVERY_TARGET_VALUE}'\''" >> "${auto_conf}"
    ;;
esac
rm -f "${PGDATA}/standby.signal"
: > "${PGDATA}/recovery.signal"
chown "${POSTGRES_UID}:${POSTGRES_GID}" "${auto_conf}" "${PGDATA}/recovery.signal"
' > "${log_path}" 2>&1
}
