#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/postgres-ops-common.sh"

POSTGRES_WAL_RETENTION_KEEP_BASE_BACKUPS="${POSTGRES_WAL_RETENTION_KEEP_BASE_BACKUPS:-2}"
POSTGRES_WAL_PRUNE_REMOTE="${POSTGRES_WAL_PRUNE_REMOTE:-1}"
POSTGRES_WAL_PRUNE_DRY_RUN="${POSTGRES_WAL_PRUNE_DRY_RUN:-0}"
WAL_ARCHIVE_DIR="${POSTGRES_WAL_ARCHIVE_DIR:-}"
REMOTE_URI="${POSTGRES_WAL_REMOTE_URI:-}"

require_postgres_variant
require_commands

if [[ "${POSTGRES_WAL_RETENTION_KEEP_BASE_BACKUPS}" =~ ^[0-9]+$ ]] && (( POSTGRES_WAL_RETENTION_KEEP_BASE_BACKUPS > 0 )); then
  :
else
  echo "POSTGRES_WAL_RETENTION_KEEP_BASE_BACKUPS must be a positive integer." >&2
  exit 1
fi

if [[ -z "${WAL_ARCHIVE_DIR}" ]]; then
  WAL_ARCHIVE_DIR="$(postgres_wal_archive_host_dir)"
fi
if [[ -z "${REMOTE_URI}" ]]; then
  REMOTE_URI="$(postgres_wal_remote_uri)"
fi

if [[ -n "${REMOTE_URI}" ]]; then
  REMOTE_URI="$(postgres_wal_remote_resolved_uri "${REMOTE_URI}")"
fi

mapfile -t manifest_entries < <(
  find "${ARTIFACT_DIR}" -maxdepth 1 -type f -name '*.physical.manifest.json' -print \
    | while IFS= read -r manifest_path; do
        created_at="$(jq -r '.created_at // empty' "${manifest_path}")"
        start_wal_file="$(jq -r '.physical_backup.metadata.start_wal_file // empty' "${manifest_path}")"
        if [[ -n "${created_at}" && -n "${start_wal_file}" ]]; then
          printf '%s\t%s\t%s\n' "${created_at}" "${start_wal_file}" "${manifest_path}"
        fi
      done | sort -r
)

if (( ${#manifest_entries[@]} == 0 )); then
  echo "No PostgreSQL physical manifests with start_wal_file metadata found in ${ARTIFACT_DIR}." >&2
  exit 1
fi

earliest_required_segment="$(
  printf '%s\n' "${manifest_entries[@]}" \
    | head -n "${POSTGRES_WAL_RETENTION_KEEP_BASE_BACKUPS}" \
    | awk -F '\t' '{print $2}' \
    | sort \
    | head -n 1
)"

if [[ -z "${earliest_required_segment}" ]]; then
  echo "Unable to resolve earliest required WAL segment from retained manifests." >&2
  exit 1
fi

log_step "Pruning local PostgreSQL WAL archive"
local_deleted_count="$(postgres_prune_local_wal_segments_before "${WAL_ARCHIVE_DIR}" "${earliest_required_segment}" "${POSTGRES_WAL_PRUNE_DRY_RUN}")"

remote_deleted_count="0"
if [[ "${POSTGRES_WAL_PRUNE_REMOTE}" == "1" && -n "${REMOTE_URI}" ]]; then
  log_step "Pruning remote PostgreSQL WAL archive"
  remote_deleted_count="$(postgres_prune_remote_wal_segments_before "${REMOTE_URI}" "${earliest_required_segment}" "${POSTGRES_WAL_PRUNE_DRY_RUN}")"
fi

echo "PostgreSQL WAL prune completed:"
echo "  retained manifests: ${POSTGRES_WAL_RETENTION_KEEP_BASE_BACKUPS}"
echo "  earliest required WAL segment: ${earliest_required_segment}"
echo "  local wal dir: ${WAL_ARCHIVE_DIR}"
echo "  local deleted segments: ${local_deleted_count}"
if [[ -n "${REMOTE_URI}" ]]; then
  echo "  remote uri: ${REMOTE_URI}"
  echo "  remote deleted segments: ${remote_deleted_count}"
fi
echo "  dry run: ${POSTGRES_WAL_PRUNE_DRY_RUN}"
