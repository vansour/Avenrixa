#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

workspace_package_version() {
  sed -n 's/^version = "\(.*\)"/\1/p' "${ROOT_DIR}/Cargo.toml" | head -n 1
}

build_version_label() {
  local version="$1"
  printf '%s' "${version}"
}

WORKSPACE_VERSION="$(workspace_package_version)"
RELEASE_VERSION="${RELEASE_VERSION:-${WORKSPACE_VERSION}}"
RELEASE_BUILD_REVISION="${RELEASE_BUILD_REVISION:-$(git rev-parse --short=12 HEAD 2>/dev/null || printf 'dev')}"
RELEASE_BUILD_DATE="${RELEASE_BUILD_DATE:-$(date -u +"%Y-%m-%dT%H:%M:%SZ")}"
RELEASE_IMAGE_REF="${RELEASE_IMAGE_REF:-ghcr.io/vansour/vansour-image:${RELEASE_VERSION}}"
RELEASE_IMAGE_PUSH="${RELEASE_IMAGE_PUSH:-0}"
RELEASE_IMAGE_ADDITIONAL_TAGS="${RELEASE_IMAGE_ADDITIONAL_TAGS:-}"
RELEASE_RC_INCLUDE_GA_GATE="${RELEASE_RC_INCLUDE_GA_GATE:-1}"
RELEASE_RC_INCLUDE_CHANGELOG="${RELEASE_RC_INCLUDE_CHANGELOG:-1}"
RELEASE_RC_INCLUDE_VERSION_SMOKE="${RELEASE_RC_INCLUDE_VERSION_SMOKE:-1}"
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
  local required_commands=(bash docker jq date)
  local command

  if [[ "${RELEASE_RC_INCLUDE_GA_GATE}" == "1" ]]; then
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
    echo "RELEASE_VERSION (${RELEASE_VERSION}) must match workspace version (${WORKSPACE_VERSION}) during RC freeze." >&2
    exit 1
  fi
}

assert_changelog_has_release() {
  local escaped_release_version

  escaped_release_version="$(printf '%s' "${RELEASE_VERSION}" | sed 's/[][(){}.^$+*?|\\/]/\\&/g')"
  if ! grep -Eq "^##[[:space:]]+${escaped_release_version}([[:space:]]+-.*)?$" "${ROOT_DIR}/CHANGELOG.md"; then
    echo "CHANGELOG.md is missing a section for ${RELEASE_VERSION}" >&2
    exit 1
  fi
}

assert_image_labels() {
  local version_label
  local revision_label

  version_label="$(
    docker image inspect "${RELEASE_IMAGE_REF}" \
      --format '{{ index .Config.Labels "org.opencontainers.image.version" }}'
  )"
  revision_label="$(
    docker image inspect "${RELEASE_IMAGE_REF}" \
      --format '{{ index .Config.Labels "org.opencontainers.image.revision" }}'
  )"

  if [[ "${version_label}" != "${RELEASE_VERSION}" ]]; then
    echo "Image version label mismatch: expected ${RELEASE_VERSION}, got ${version_label}" >&2
    exit 1
  fi
  if [[ "${revision_label}" != "${RELEASE_BUILD_REVISION}" ]]; then
    echo "Image revision label mismatch: expected ${RELEASE_BUILD_REVISION}, got ${revision_label}" >&2
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

run_ga_gate() {
  log_step "Running GA release gate as the RC foundation"
  env \
    APP_VERSION="${RELEASE_VERSION}" \
    APP_REVISION="${RELEASE_BUILD_REVISION}" \
    BUILD_DATE="${RELEASE_BUILD_DATE}" \
    APP_IMAGE_REF="${RELEASE_IMAGE_REF}" \
    PRESERVE_STACK_ON_FAILURE="${PRESERVE_STACK_ON_FAILURE}" \
    ./scripts/release-ga-gate.sh
}

run_version_smoke() {
  local expected_version_label

  expected_version_label="$(build_version_label "${RELEASE_VERSION}" "${RELEASE_BUILD_REVISION}")"

  log_step "Running release metadata smoke"
  env \
    APP_VERSION="${RELEASE_VERSION}" \
    APP_REVISION="${RELEASE_BUILD_REVISION}" \
    BUILD_DATE="${RELEASE_BUILD_DATE}" \
    APP_IMAGE_REF="${RELEASE_IMAGE_REF}" \
    COMPOSE_PROJECT_NAME="vansour-image-release-rc-preflight-smoke" \
    COMPOSE_VARIANT=postgres \
    SMOKE_FLOW=health \
    CACHE_MODE=redis8 \
    SMOKE_EXPECT_APP_VERSION_LABEL="${expected_version_label}" \
    PRESERVE_STACK_ON_FAILURE="${PRESERVE_STACK_ON_FAILURE}" \
    ./scripts/compose-smoke.sh
}

main() {
  expect_toggle "${RELEASE_IMAGE_PUSH}" "RELEASE_IMAGE_PUSH"
  expect_toggle "${RELEASE_RC_INCLUDE_GA_GATE}" "RELEASE_RC_INCLUDE_GA_GATE"
  expect_toggle "${RELEASE_RC_INCLUDE_CHANGELOG}" "RELEASE_RC_INCLUDE_CHANGELOG"
  expect_toggle "${RELEASE_RC_INCLUDE_VERSION_SMOKE}" "RELEASE_RC_INCLUDE_VERSION_SMOKE"
  expect_toggle "${PRESERVE_STACK_ON_FAILURE}" "PRESERVE_STACK_ON_FAILURE"

  require_commands
  assert_release_version_matches_workspace

  log_step "Starting 0.1 RC preflight"
  echo "Release version: ${RELEASE_VERSION}"
  echo "Release revision: ${RELEASE_BUILD_REVISION}"
  echo "Release image: ${RELEASE_IMAGE_REF}"
  echo "Release build date: ${RELEASE_BUILD_DATE}"

  if [[ "${RELEASE_RC_INCLUDE_CHANGELOG}" == "1" ]]; then
    log_step "Checking changelog entry"
    assert_changelog_has_release
  fi

  if [[ "${RELEASE_RC_INCLUDE_GA_GATE}" == "1" ]]; then
    run_ga_gate
  fi

  if [[ "${RELEASE_RC_INCLUDE_VERSION_SMOKE}" == "1" ]]; then
    run_version_smoke
    log_step "Checking image labels"
    assert_image_labels
  fi

  if [[ "${RELEASE_IMAGE_PUSH}" == "1" ]]; then
    push_release_image
  fi

  echo
  echo "0.1 RC preflight passed"
}

main "$@"
