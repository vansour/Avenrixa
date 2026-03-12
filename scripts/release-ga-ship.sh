#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

workspace_package_version() {
  sed -n 's/^version = "\(.*\)"/\1/p' "${ROOT_DIR}/Cargo.toml" | head -n 1
}

sha256_file() {
  local path="$1"
  sha256sum "${path}" | awk '{print $1}'
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

WORKSPACE_VERSION="$(workspace_package_version)"
RELEASE_VERSION="${RELEASE_VERSION:-${WORKSPACE_VERSION}}"
RELEASE_BUILD_REVISION="${RELEASE_BUILD_REVISION:-$(git rev-parse --short=12 HEAD 2>/dev/null || printf 'dev')}"
RELEASE_BUILD_DATE="${RELEASE_BUILD_DATE:-$(date -u +"%Y-%m-%dT%H:%M:%SZ")}"
RELEASE_IMAGE_REF="${RELEASE_IMAGE_REF:-ghcr.io/vansour/vansour-image:${RELEASE_VERSION}}"
RELEASE_IMAGE_PUSH="${RELEASE_IMAGE_PUSH:-0}"
RELEASE_IMAGE_ADDITIONAL_TAGS="${RELEASE_IMAGE_ADDITIONAL_TAGS:-}"
RELEASE_GA_INCLUDE_RC_PREFLIGHT="${RELEASE_GA_INCLUDE_RC_PREFLIGHT:-1}"
RELEASE_GA_INCLUDE_ASSET_BUNDLE="${RELEASE_GA_INCLUDE_ASSET_BUNDLE:-1}"
RELEASE_ASSET_DIR="${RELEASE_ASSET_DIR:-${ROOT_DIR}/dist/release/${RELEASE_VERSION}}"
PRESERVE_STACK_ON_FAILURE="${PRESERVE_STACK_ON_FAILURE:-0}"

log_step() {
  echo
  echo "==> $1"
}

expect_toggle() {
  local value="$1"
  local name="$2"

  if [[ "${value}" != "0" && "${value}" != "1" ]]; then
    echo "Invalid ${name}: ${value} (expected 0 or 1)" >&2
    exit 1
  fi
}

require_commands() {
  local required_commands=(bash docker jq date tar sha256sum mktemp cp)
  local command

  if [[ "${RELEASE_GA_INCLUDE_RC_PREFLIGHT}" == "1" ]]; then
    required_commands+=(cargo)
  fi

  for command in "${required_commands[@]}"; do
    if ! command -v "${command}" >/dev/null 2>&1; then
      echo "Missing required command: ${command}" >&2
      exit 1
    fi
  done
}

assert_release_version_matches_workspace() {
  if [[ "${RELEASE_VERSION}" != "${WORKSPACE_VERSION}" ]]; then
    echo "RELEASE_VERSION (${RELEASE_VERSION}) must match workspace version (${WORKSPACE_VERSION}) during GA ship." >&2
    exit 1
  fi
}

assert_release_version_is_stable() {
  if [[ "${RELEASE_VERSION}" == *-* ]]; then
    echo "RELEASE_VERSION (${RELEASE_VERSION}) must be a stable GA version without prerelease suffix." >&2
    exit 1
  fi
}

assert_changelog_has_release() {
  if [[ -z "$(extract_changelog_section "${RELEASE_VERSION}")" ]]; then
    echo "CHANGELOG.md is missing a section for ${RELEASE_VERSION}" >&2
    exit 1
  fi
}

push_release_image() {
  local extra_refs=()
  local ref

  read -r -a extra_refs <<< "${RELEASE_IMAGE_ADDITIONAL_TAGS}"

  log_step "Publishing release image"
  docker push "${RELEASE_IMAGE_REF}"

  for ref in "${extra_refs[@]}"; do
    [[ -n "${ref}" ]] || continue
    if [[ "${ref}" != "${RELEASE_IMAGE_REF}" ]]; then
      docker tag "${RELEASE_IMAGE_REF}" "${ref}"
    fi
    docker push "${ref}"
  done
}

run_rc_preflight() {
  log_step "Running RC preflight as the GA ship foundation"
  env \
    RELEASE_VERSION="${RELEASE_VERSION}" \
    RELEASE_BUILD_REVISION="${RELEASE_BUILD_REVISION}" \
    RELEASE_BUILD_DATE="${RELEASE_BUILD_DATE}" \
    RELEASE_IMAGE_REF="${RELEASE_IMAGE_REF}" \
    RELEASE_IMAGE_PUSH="0" \
    PRESERVE_STACK_ON_FAILURE="${PRESERVE_STACK_ON_FAILURE}" \
    ./scripts/release-rc-preflight.sh
}

write_release_notes() {
  local release_notes_path="$1"
  local generated_at="$2"

  {
    echo "# Vansour Image ${RELEASE_VERSION}"
    echo
    echo "- Release version: \`${RELEASE_VERSION}\`"
    echo "- Release revision: \`${RELEASE_BUILD_REVISION}\`"
    echo "- Release image: \`${RELEASE_IMAGE_REF}\`"
    echo "- Release build date: \`${RELEASE_BUILD_DATE}\`"
    echo "- Generated at: \`${generated_at}\`"
    echo
    echo "## Highlights"
    extract_changelog_section "${RELEASE_VERSION}" | sed '1d'
  } > "${release_notes_path}"
}

write_image_metadata() {
  local image_metadata_path="$1"

  docker image inspect "${RELEASE_IMAGE_REF}" \
    | jq '.[0] | {
        id: .Id,
        repo_tags: (.RepoTags // []),
        repo_digests: (.RepoDigests // []),
        created: .Created,
        architecture: .Architecture,
        os: .Os,
        labels: (.Config.Labels // {})
      }' > "${image_metadata_path}"
}

create_release_bundle() {
  local generated_at="$1"
  local release_notes_path image_metadata_path release_bundle_path release_manifest_path checksums_path
  local release_notes_sha256 image_metadata_sha256 release_bundle_sha256 release_manifest_sha256
  local tmp_dir bundle_root

  mkdir -p "${RELEASE_ASSET_DIR}"
  release_notes_path="${RELEASE_ASSET_DIR}/release-notes.md"
  image_metadata_path="${RELEASE_ASSET_DIR}/image-metadata.json"
  release_bundle_path="${RELEASE_ASSET_DIR}/vansour-image-${RELEASE_VERSION}-release-bundle.tar.gz"
  release_manifest_path="${RELEASE_ASSET_DIR}/release-manifest.json"
  checksums_path="${RELEASE_ASSET_DIR}/SHA256SUMS"

  write_release_notes "${release_notes_path}" "${generated_at}"
  write_image_metadata "${image_metadata_path}"

  tmp_dir="$(mktemp -d)"
  bundle_root="${tmp_dir}/vansour-image-${RELEASE_VERSION}"
  mkdir -p "${bundle_root}/docs"
  cp "${ROOT_DIR}/compose.yml" "${bundle_root}/compose.yml"
  cp "${ROOT_DIR}/README.md" "${bundle_root}/README.md"
  cp "${ROOT_DIR}/CHANGELOG.md" "${bundle_root}/CHANGELOG.md"
  cp "${ROOT_DIR}/docs/release-0.1-ga-runbook.md" "${bundle_root}/docs/release-0.1-ga-runbook.md"
  cp "${release_notes_path}" "${bundle_root}/release-notes.md"
  cp "${image_metadata_path}" "${bundle_root}/image-metadata.json"
  tar -czf "${release_bundle_path}" -C "${tmp_dir}" "vansour-image-${RELEASE_VERSION}"
  rm -rf "${tmp_dir}"

  release_notes_sha256="$(sha256_file "${release_notes_path}")"
  image_metadata_sha256="$(sha256_file "${image_metadata_path}")"
  release_bundle_sha256="$(sha256_file "${release_bundle_path}")"

  jq -n \
    --arg release_version "${RELEASE_VERSION}" \
    --arg release_revision "${RELEASE_BUILD_REVISION}" \
    --arg release_build_date "${RELEASE_BUILD_DATE}" \
    --arg release_image_ref "${RELEASE_IMAGE_REF}" \
    --arg generated_at "${generated_at}" \
    --arg release_notes_path "$(basename "${release_notes_path}")" \
    --arg release_notes_sha256 "${release_notes_sha256}" \
    --arg image_metadata_path "$(basename "${image_metadata_path}")" \
    --arg image_metadata_sha256 "${image_metadata_sha256}" \
    --arg release_bundle_path "$(basename "${release_bundle_path}")" \
    --arg release_bundle_sha256 "${release_bundle_sha256}" \
    '{
      release_version: $release_version,
      release_revision: $release_revision,
      release_build_date: $release_build_date,
      release_image_ref: $release_image_ref,
      generated_at: $generated_at,
      assets: [
        {
          path: $release_notes_path,
          kind: "release_notes",
          sha256: $release_notes_sha256
        },
        {
          path: $image_metadata_path,
          kind: "image_metadata",
          sha256: $image_metadata_sha256
        },
        {
          path: $release_bundle_path,
          kind: "release_bundle",
          sha256: $release_bundle_sha256
        }
      ]
    }' > "${release_manifest_path}"

  release_manifest_sha256="$(sha256_file "${release_manifest_path}")"
  {
    printf '%s  %s\n' "${release_notes_sha256}" "$(basename "${release_notes_path}")"
    printf '%s  %s\n' "${image_metadata_sha256}" "$(basename "${image_metadata_path}")"
    printf '%s  %s\n' "${release_bundle_sha256}" "$(basename "${release_bundle_path}")"
    printf '%s  %s\n' "${release_manifest_sha256}" "$(basename "${release_manifest_path}")"
  } > "${checksums_path}"

  echo "Release asset dir: ${RELEASE_ASSET_DIR}"
  echo "Release bundle: ${release_bundle_path}"
  echo "Release manifest: ${release_manifest_path}"
  echo "Checksums: ${checksums_path}"
}

main() {
  local generated_at

  expect_toggle "${RELEASE_IMAGE_PUSH}" "RELEASE_IMAGE_PUSH"
  expect_toggle "${RELEASE_GA_INCLUDE_RC_PREFLIGHT}" "RELEASE_GA_INCLUDE_RC_PREFLIGHT"
  expect_toggle "${RELEASE_GA_INCLUDE_ASSET_BUNDLE}" "RELEASE_GA_INCLUDE_ASSET_BUNDLE"
  expect_toggle "${PRESERVE_STACK_ON_FAILURE}" "PRESERVE_STACK_ON_FAILURE"

  require_commands
  assert_release_version_matches_workspace
  assert_release_version_is_stable
  assert_changelog_has_release

  generated_at="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

  log_step "Starting 0.1 GA ship"
  echo "Release version: ${RELEASE_VERSION}"
  echo "Release revision: ${RELEASE_BUILD_REVISION}"
  echo "Release image: ${RELEASE_IMAGE_REF}"
  echo "Release build date: ${RELEASE_BUILD_DATE}"
  echo "Release asset dir: ${RELEASE_ASSET_DIR}"

  if [[ "${RELEASE_GA_INCLUDE_RC_PREFLIGHT}" == "1" ]]; then
    run_rc_preflight
  fi

  if [[ "${RELEASE_IMAGE_PUSH}" == "1" ]]; then
    push_release_image
  fi

  if [[ "${RELEASE_GA_INCLUDE_ASSET_BUNDLE}" == "1" ]]; then
    log_step "Generating GA release assets"
    create_release_bundle "${generated_at}"
  fi

  echo
  echo "0.1 GA ship passed"
}

main "$@"
