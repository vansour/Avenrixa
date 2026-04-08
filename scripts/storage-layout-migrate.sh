#!/usr/bin/env bash
set -euo pipefail

STORAGE_PATH="${STORAGE_PATH:-/data/images}"
APPLY="${APPLY:-0}"

usage() {
  cat <<'EOF'
Usage:
  STORAGE_PATH=/data/images ./scripts/storage-layout-migrate.sh
  STORAGE_PATH=/data/images APPLY=1 ./scripts/storage-layout-migrate.sh

Description:
  Migrates legacy flat local media files into the sharded storage layout used by stage 3.

Behavior:
  - default is dry-run
  - only top-level files in STORAGE_PATH are considered migration candidates
  - files already stored in shard directories are left untouched
  - only hash-named originals and thumb-<hash>-*.webp thumbnails are migrated
EOF
}

if [[ "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ ! -d "${STORAGE_PATH}" ]]; then
  echo "STORAGE_PATH does not exist or is not a directory: ${STORAGE_PATH}" >&2
  exit 1
fi

if [[ "${APPLY}" != "0" && "${APPLY}" != "1" ]]; then
  echo "APPLY must be 0 or 1" >&2
  exit 1
fi

content_hash_for_key() {
  local file_key="$1"

  if [[ "${file_key}" =~ ^([0-9a-fA-F]{64})\.[^/]+$ ]]; then
    printf '%s' "${BASH_REMATCH[1],,}"
    return 0
  fi

  if [[ "${file_key}" =~ ^thumb-([0-9a-fA-F]{64})-[^/]+\.webp$ ]]; then
    printf '%s' "${BASH_REMATCH[1],,}"
    return 0
  fi

  return 1
}

move_file() {
  local source_path="$1"
  local target_path="$2"

  if [[ "${APPLY}" == "0" ]]; then
    printf '[dry-run] mv %s %s\n' "${source_path}" "${target_path}"
    return 0
  fi

  mkdir -p "$(dirname "${target_path}")"

  if [[ -e "${target_path}" ]]; then
    if cmp -s "${source_path}" "${target_path}"; then
      rm -f "${source_path}"
      printf '[dedupe] rm %s (target already exists with identical content)\n' "${source_path}"
      return 0
    fi

    echo "Refusing to overwrite existing target with different contents: ${target_path}" >&2
    return 1
  fi

  mv "${source_path}" "${target_path}"
  printf '[moved] %s -> %s\n' "${source_path}" "${target_path}"
}

migrated=0
skipped=0
failed=0

while IFS= read -r -d '' source_path; do
  file_key="$(basename "${source_path}")"

  if ! hash="$(content_hash_for_key "${file_key}")"; then
    skipped=$((skipped + 1))
    printf '[skip] %s (not a shardable media key)\n' "${source_path}"
    continue
  fi

  shard_a="${hash:0:2}"
  shard_b="${hash:2:2}"
  target_path="${STORAGE_PATH}/${shard_a}/${shard_b}/${file_key}"

  if move_file "${source_path}" "${target_path}"; then
    migrated=$((migrated + 1))
  else
    failed=$((failed + 1))
  fi
done < <(find "${STORAGE_PATH}" -maxdepth 1 -type f -print0)

printf '\nSummary:\n'
printf '  migrated: %s\n' "${migrated}"
printf '  skipped: %s\n' "${skipped}"
printf '  failed: %s\n' "${failed}"

if [[ "${failed}" -gt 0 ]]; then
  exit 1
fi
