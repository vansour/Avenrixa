#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

source "${ROOT_DIR}/scripts/release-result-common.sh"

workspace_package_version() {
  sed -n 's/^version = "\(.*\)"/\1/p' "${ROOT_DIR}/Cargo.toml" | head -n 1
}

default_image_repository() {
  local remote_url path owner repo

  if [[ -n "${GITHUB_REPOSITORY:-}" ]]; then
    owner="${GITHUB_REPOSITORY%%/*}"
    repo="${GITHUB_REPOSITORY#*/}"
  else
    remote_url="$(git remote get-url origin 2>/dev/null || true)"
    case "${remote_url}" in
      git@github.com:*)
        path="${remote_url#git@github.com:}"
        path="${path%.git}"
        ;;
      https://github.com/*)
        path="${remote_url#https://github.com/}"
        path="${path%.git}"
        ;;
      *)
        path=""
        ;;
    esac
    if [[ -n "${path}" ]]; then
      owner="${path%%/*}"
      repo="${path#*/}"
    else
      owner="vansour"
      repo="avenrixa"
    fi
  fi

  repo="$(printf '%s' "${repo}" | tr '[:upper:]' '[:lower:]')"
  printf 'ghcr.io/%s/%s' "${owner}" "${repo}"
}

build_version_label() {
  local version="$1"
  printf '%s' "${version}"
}

WORKSPACE_VERSION="$(workspace_package_version)"
RELEASE_VERSION="${RELEASE_VERSION:-${WORKSPACE_VERSION}}"
RELEASE_BUILD_REVISION="${RELEASE_BUILD_REVISION:-$(git rev-parse --short=12 HEAD 2>/dev/null || printf 'dev')}"
RELEASE_BUILD_DATE="${RELEASE_BUILD_DATE:-$(date -u +"%Y-%m-%dT%H:%M:%SZ")}"
RELEASE_IMAGE_REF="${RELEASE_IMAGE_REF:-$(default_image_repository):${RELEASE_VERSION}}"
RELEASE_IMAGE_PUSH="${RELEASE_IMAGE_PUSH:-0}"
RELEASE_IMAGE_ADDITIONAL_TAGS_WAS_SET=0
if [[ -n "${RELEASE_IMAGE_ADDITIONAL_TAGS+x}" ]]; then
  RELEASE_IMAGE_ADDITIONAL_TAGS_WAS_SET=1
fi
RELEASE_IMAGE_ADDITIONAL_TAGS="${RELEASE_IMAGE_ADDITIONAL_TAGS-}"
RELEASE_RC_INCLUDE_GA_GATE="${RELEASE_RC_INCLUDE_GA_GATE:-1}"
RELEASE_RC_INCLUDE_CHANGELOG="${RELEASE_RC_INCLUDE_CHANGELOG:-1}"
RELEASE_RC_INCLUDE_VERSION_SMOKE="${RELEASE_RC_INCLUDE_VERSION_SMOKE:-1}"
RELEASE_RC_ARTIFACT_DIR="${RELEASE_RC_ARTIFACT_DIR:-${ROOT_DIR}/dist/release/${RELEASE_VERSION}}"
RELEASE_RC_RESULT_PATH="${RELEASE_RC_RESULT_PATH:-${RELEASE_RC_ARTIFACT_DIR}/release-rc-preflight-result.json}"
PRESERVE_STACK_ON_FAILURE="${PRESERVE_STACK_ON_FAILURE:-0}"

RELEASE_RC_STARTED_AT="$(release_result_timestamp_utc)"
RELEASE_RC_COMPLETED_STEPS=()

log_step() {
  echo
  echo "==> $1"
}

mark_release_rc_step_completed() {
  RELEASE_RC_COMPLETED_STEPS+=("$1")
}

finalize_release_rc_result() {
  local exit_code="$1"
  local status="failed"
  local summary="0.1 RC preflight failed"

  if [[ "${exit_code}" -eq 0 ]]; then
    status="passed"
    summary="0.1 RC preflight passed"
  fi

  local metadata_json
  metadata_json="$(
    jq -n \
      --arg release_version "${RELEASE_VERSION}" \
      --arg release_revision "${RELEASE_BUILD_REVISION}" \
      --arg release_build_date "${RELEASE_BUILD_DATE}" \
      --arg release_image_ref "${RELEASE_IMAGE_REF}" \
      --arg release_artifact_dir "${RELEASE_RC_ARTIFACT_DIR}" \
      --arg result_path "${RELEASE_RC_RESULT_PATH}" \
      --arg release_image_additional_tags "${RELEASE_IMAGE_ADDITIONAL_TAGS}" \
      --argjson release_image_push "$([[ "${RELEASE_IMAGE_PUSH}" == "1" ]] && echo true || echo false)" \
      --argjson include_ga_gate "$([[ "${RELEASE_RC_INCLUDE_GA_GATE}" == "1" ]] && echo true || echo false)" \
      --argjson include_changelog "$([[ "${RELEASE_RC_INCLUDE_CHANGELOG}" == "1" ]] && echo true || echo false)" \
      --argjson include_version_smoke "$([[ "${RELEASE_RC_INCLUDE_VERSION_SMOKE}" == "1" ]] && echo true || echo false)" \
      '{
        release_version: $release_version,
        release_revision: $release_revision,
        release_build_date: $release_build_date,
        release_image_ref: $release_image_ref,
        release_image_additional_tags: ($release_image_additional_tags | split(" ") | map(select(length > 0))),
        release_image_push: $release_image_push,
        include_ga_gate: $include_ga_gate,
        include_changelog: $include_changelog,
        include_version_smoke: $include_version_smoke,
        release_artifact_dir: $release_artifact_dir,
        result_path: $result_path
      }'
  )"

  write_release_result_file \
    "${RELEASE_RC_RESULT_PATH}" \
    "${status}" \
    "${RELEASE_RC_STARTED_AT}" \
    "$(release_result_timestamp_utc)" \
    "${summary}" \
    "${metadata_json}" \
    "${RELEASE_RC_COMPLETED_STEPS[@]}"
}

trap 'exit_code=$?; finalize_release_rc_result "${exit_code}"; exit "${exit_code}"' EXIT

rolling_rc_image_ref_from_release_ref() {
  local image_ref="$1"
  local image_ref_without_digest="${image_ref%%@*}"

  printf '%s:rc' "${image_ref_without_digest%:*}"
}

apply_default_rolling_rc_tag() {
  if [[ "${RELEASE_IMAGE_ADDITIONAL_TAGS_WAS_SET}" == "0" && "${RELEASE_VERSION}" == *-* ]]; then
    RELEASE_IMAGE_ADDITIONAL_TAGS="$(rolling_rc_image_ref_from_release_ref "${RELEASE_IMAGE_REF}")"
  fi
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
    COMPOSE_PROJECT_NAME="avenrixa-release-rc-preflight-smoke" \
    COMPOSE_VARIANT=postgres \
    SMOKE_FLOW=health \
    CACHE_MODE=dragonfly \
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
  mkdir -p "${RELEASE_RC_ARTIFACT_DIR}"
  apply_default_rolling_rc_tag

  log_step "Starting 0.1 RC preflight"
  echo "Release version: ${RELEASE_VERSION}"
  echo "Release revision: ${RELEASE_BUILD_REVISION}"
  echo "Release image: ${RELEASE_IMAGE_REF}"
  if [[ -n "${RELEASE_IMAGE_ADDITIONAL_TAGS}" ]]; then
    echo "Additional image tags: ${RELEASE_IMAGE_ADDITIONAL_TAGS}"
  fi
  echo "Release build date: ${RELEASE_BUILD_DATE}"

  if [[ "${RELEASE_RC_INCLUDE_CHANGELOG}" == "1" ]]; then
    log_step "Checking changelog entry"
    assert_changelog_has_release
    mark_release_rc_step_completed "changelog"
  fi

  if [[ "${RELEASE_RC_INCLUDE_GA_GATE}" == "1" ]]; then
    run_ga_gate
    mark_release_rc_step_completed "ga_gate"
  fi

  if [[ "${RELEASE_RC_INCLUDE_VERSION_SMOKE}" == "1" ]]; then
    run_version_smoke
    mark_release_rc_step_completed "version_smoke"
    log_step "Checking image labels"
    assert_image_labels
    mark_release_rc_step_completed "image_labels"
  fi

  if [[ "${RELEASE_IMAGE_PUSH}" == "1" ]]; then
    push_release_image
    mark_release_rc_step_completed "image_push"
  fi

  echo
  echo "0.1 RC preflight passed"
}

main "$@"
