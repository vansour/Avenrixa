#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

workspace_package_version() {
  sed -n 's/^version = "\(.*\)"/\1/p' "${ROOT_DIR}/Cargo.toml" | head -n 1
}

extract_changelog_section() {
  local version="$1"

  awk -v version="${version}" '
    $0 == "## " version || index($0, "## " version " -") == 1 {
      capture = 1
      print
      next
    }
    /^## / && capture {
      exit
    }
    capture {
      print
    }
  ' "${ROOT_DIR}/CHANGELOG.md"
}

log_step() {
  echo
  echo "==> $1"
}

require_commands() {
  local required_commands=(bash jq sha256sum tar awk grep)
  local command

  for command in "${required_commands[@]}"; do
    if ! command -v "${command}" >/dev/null 2>&1; then
      echo "Missing required command: ${command}" >&2
      exit 1
    fi
  done
}

expect_file() {
  local path="$1"
  local description="$2"

  if [[ ! -f "${path}" ]]; then
    echo "${description} not found: ${path}" >&2
    exit 1
  fi
}

expect_contains_line() {
  local path="$1"
  local pattern="$2"
  local description="$3"

  if ! grep -F "${pattern}" "${path}" >/dev/null 2>&1; then
    echo "${description} missing expected line in ${path}: ${pattern}" >&2
    exit 1
  fi
}

verify_sha256_entry() {
  local file_path="$1"
  local expected_sha="$2"
  local description="$3"
  local actual_sha

  actual_sha="$(sha256sum "${file_path}" | awk '{print $1}')"
  if [[ "${actual_sha}" != "${expected_sha}" ]]; then
    echo "${description} sha256 mismatch: expected ${expected_sha}, got ${actual_sha}" >&2
    exit 1
  fi
}

verify_bundle_contains() {
  local bundle_path="$1"
  local entry="$2"

  if ! tar -tzf "${bundle_path}" "${entry}" >/dev/null 2>&1; then
    echo "Release bundle is missing entry: ${entry}" >&2
    exit 1
  fi
}

RELEASE_VERSION="${1:-${RELEASE_VERSION:-$(workspace_package_version)}}"
RELEASE_ASSET_DIR="${RELEASE_ASSET_DIR:-${ROOT_DIR}/dist/release/${RELEASE_VERSION}}"

RELEASE_NOTES_PATH="${RELEASE_ASSET_DIR}/release-notes.md"
IMAGE_METADATA_PATH="${RELEASE_ASSET_DIR}/image-metadata.json"
RELEASE_BUNDLE_PATH="${RELEASE_ASSET_DIR}/avenrixa-${RELEASE_VERSION}-release-bundle.tar.gz"
RELEASE_MANIFEST_PATH="${RELEASE_ASSET_DIR}/release-manifest.json"
CHECKSUMS_PATH="${RELEASE_ASSET_DIR}/SHA256SUMS"

main() {
  require_commands

  log_step "Checking expected GA asset files"
  expect_file "${RELEASE_NOTES_PATH}" "release notes"
  expect_file "${IMAGE_METADATA_PATH}" "image metadata"
  expect_file "${RELEASE_BUNDLE_PATH}" "release bundle"
  expect_file "${RELEASE_MANIFEST_PATH}" "release manifest"
  expect_file "${CHECKSUMS_PATH}" "release checksums"

  log_step "Checking release manifest structure"
  local manifest_release_version
  local manifest_assets_len
  manifest_release_version="$(jq -r '.release_version' "${RELEASE_MANIFEST_PATH}")"
  manifest_assets_len="$(jq -r '.assets | length' "${RELEASE_MANIFEST_PATH}")"
  if [[ "${manifest_release_version}" != "${RELEASE_VERSION}" ]]; then
    echo "Release manifest version mismatch: expected ${RELEASE_VERSION}, got ${manifest_release_version}" >&2
    exit 1
  fi
  if [[ "${manifest_assets_len}" -lt 3 ]]; then
    echo "Release manifest must contain at least 3 assets, got ${manifest_assets_len}" >&2
    exit 1
  fi

  log_step "Checking changelog and release notes consistency"
  if [[ -z "$(extract_changelog_section "${RELEASE_VERSION}")" ]]; then
    echo "CHANGELOG.md is missing a section for ${RELEASE_VERSION}" >&2
    exit 1
  fi
  expect_contains_line "${RELEASE_NOTES_PATH}" "# Avenrixa ${RELEASE_VERSION}" "release notes"
  expect_contains_line "${RELEASE_NOTES_PATH}" "## Highlights" "release notes"

  log_step "Checking asset hashes against release manifest"
  local release_notes_sha
  local image_metadata_sha
  local release_bundle_sha
  local release_manifest_sha
  release_notes_sha="$(jq -r '.assets[] | select(.kind == "release_notes") | .sha256' "${RELEASE_MANIFEST_PATH}")"
  image_metadata_sha="$(jq -r '.assets[] | select(.kind == "image_metadata") | .sha256' "${RELEASE_MANIFEST_PATH}")"
  release_bundle_sha="$(jq -r '.assets[] | select(.kind == "release_bundle") | .sha256' "${RELEASE_MANIFEST_PATH}")"
  release_manifest_sha="$(sha256sum "${RELEASE_MANIFEST_PATH}" | awk '{print $1}')"

  verify_sha256_entry "${RELEASE_NOTES_PATH}" "${release_notes_sha}" "release notes"
  verify_sha256_entry "${IMAGE_METADATA_PATH}" "${image_metadata_sha}" "image metadata"
  verify_sha256_entry "${RELEASE_BUNDLE_PATH}" "${release_bundle_sha}" "release bundle"

  log_step "Checking SHA256SUMS coverage"
  expect_contains_line "${CHECKSUMS_PATH}" "${release_notes_sha}  $(basename "${RELEASE_NOTES_PATH}")" "SHA256SUMS"
  expect_contains_line "${CHECKSUMS_PATH}" "${image_metadata_sha}  $(basename "${IMAGE_METADATA_PATH}")" "SHA256SUMS"
  expect_contains_line "${CHECKSUMS_PATH}" "${release_bundle_sha}  $(basename "${RELEASE_BUNDLE_PATH}")" "SHA256SUMS"
  expect_contains_line "${CHECKSUMS_PATH}" "${release_manifest_sha}  $(basename "${RELEASE_MANIFEST_PATH}")" "SHA256SUMS"

  log_step "Checking release bundle contents"
  local bundle_root="avenrixa-${RELEASE_VERSION}"
  verify_bundle_contains "${RELEASE_BUNDLE_PATH}" "${bundle_root}/compose.yml"
  verify_bundle_contains "${RELEASE_BUNDLE_PATH}" "${bundle_root}/README.md"
  verify_bundle_contains "${RELEASE_BUNDLE_PATH}" "${bundle_root}/CHANGELOG.md"
  verify_bundle_contains "${RELEASE_BUNDLE_PATH}" "${bundle_root}/docs/release-0.1-ga-runbook.md"
  verify_bundle_contains "${RELEASE_BUNDLE_PATH}" "${bundle_root}/release-notes.md"
  verify_bundle_contains "${RELEASE_BUNDLE_PATH}" "${bundle_root}/image-metadata.json"

  echo
  echo "Release assets verified: ${RELEASE_ASSET_DIR}"
}

main "$@"
