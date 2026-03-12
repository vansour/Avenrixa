#!/usr/bin/env bash
set -euo pipefail

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/postgres-ops-common.sh"

SYNC_DIRECTION="${1:-${POSTGRES_WAL_SYNC_DIRECTION:-push}}"
WAL_ARCHIVE_DIR="${POSTGRES_WAL_ARCHIVE_DIR:-}"
REMOTE_URI="${POSTGRES_WAL_REMOTE_URI:-}"

require_postgres_variant
require_commands

if [[ -z "${WAL_ARCHIVE_DIR}" ]]; then
  WAL_ARCHIVE_DIR="$(postgres_wal_archive_host_dir)"
fi
if [[ -z "${REMOTE_URI}" ]]; then
  REMOTE_URI="$(postgres_wal_remote_uri)"
fi

if [[ -z "${REMOTE_URI}" ]]; then
  echo "POSTGRES_WAL_REMOTE_URI is required for PostgreSQL WAL sync." >&2
  exit 1
fi

REMOTE_URI="$(postgres_wal_remote_resolved_uri "${REMOTE_URI}")"

case "${SYNC_DIRECTION}" in
  push)
    log_step "Syncing PostgreSQL WAL archive to remote"
    postgres_wal_remote_sync_push "${WAL_ARCHIVE_DIR}" "${REMOTE_URI}"
    echo "PostgreSQL WAL remote sync completed:"
    echo "  direction: push"
    echo "  source: ${WAL_ARCHIVE_DIR}"
    echo "  remote: ${REMOTE_URI}"
    ;;
  pull)
    log_step "Syncing PostgreSQL WAL archive from remote"
    mkdir -p "${WAL_ARCHIVE_DIR}"
    postgres_wal_remote_sync_pull "${REMOTE_URI}" "${WAL_ARCHIVE_DIR}"
    echo "PostgreSQL WAL remote sync completed:"
    echo "  direction: pull"
    echo "  remote: ${REMOTE_URI}"
    echo "  target: ${WAL_ARCHIVE_DIR}"
    ;;
  *)
    echo "Unsupported PostgreSQL WAL sync direction: ${SYNC_DIRECTION}" >&2
    echo "Expected: push | pull" >&2
    exit 1
    ;;
esac
