#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

source "${ROOT_DIR}/scripts/release-result-common.sh"

RELEASE_GATE_INCLUDE_RUST_CHECKS="${RELEASE_GATE_INCLUDE_RUST_CHECKS:-1}"
RELEASE_GATE_INCLUDE_FRONTEND_CHECKS="${RELEASE_GATE_INCLUDE_FRONTEND_CHECKS:-1}"
RELEASE_GATE_INCLUDE_COMPOSE_SMOKE="${RELEASE_GATE_INCLUDE_COMPOSE_SMOKE:-1}"
RELEASE_GATE_INCLUDE_BROWSER_REGRESSION="${RELEASE_GATE_INCLUDE_BROWSER_REGRESSION:-1}"
RELEASE_GATE_INCLUDE_POSTGRES_OPS_DRILL="${RELEASE_GATE_INCLUDE_POSTGRES_OPS_DRILL:-1}"
RELEASE_GATE_INCLUDE_POSTGRES_PITR_DRILL="${RELEASE_GATE_INCLUDE_POSTGRES_PITR_DRILL:-1}"
RELEASE_GATE_PITR_TARGET_MODES="${RELEASE_GATE_PITR_TARGET_MODES:-name time}"
RELEASE_GATE_COMPOSE_PROJECT_PREFIX="${RELEASE_GATE_COMPOSE_PROJECT_PREFIX:-avenrixa-release-ga-gate}"
RELEASE_GATE_ARTIFACT_DIR="${RELEASE_GATE_ARTIFACT_DIR:-ops-backups/release-ga-gate}"
RELEASE_GATE_RESULT_PATH="${RELEASE_GATE_RESULT_PATH:-${RELEASE_GATE_ARTIFACT_DIR}/release-ga-gate-result.json}"
PRESERVE_STACK_ON_FAILURE="${PRESERVE_STACK_ON_FAILURE:-0}"

RELEASE_GATE_STARTED_AT="$(release_result_timestamp_utc)"
RELEASE_GATE_COMPLETED_STEPS=()

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
  local required_commands=(bash jq)

  if [[ "${RELEASE_GATE_INCLUDE_RUST_CHECKS}" == "1" ]]; then
    required_commands+=(cargo)
  fi
  if [[ "${RELEASE_GATE_INCLUDE_FRONTEND_CHECKS}" == "1" ]]; then
    required_commands+=(npm)
  fi

  if [[ "${RELEASE_GATE_INCLUDE_COMPOSE_SMOKE}" == "1" \
    || "${RELEASE_GATE_INCLUDE_BROWSER_REGRESSION}" == "1" \
    || "${RELEASE_GATE_INCLUDE_POSTGRES_OPS_DRILL}" == "1" \
    || "${RELEASE_GATE_INCLUDE_POSTGRES_PITR_DRILL}" == "1" ]]; then
    required_commands+=(docker)
  fi

  local command
  for command in "${required_commands[@]}"; do
    if ! command -v "${command}" >/dev/null 2>&1; then
      echo "Missing required command: ${command}" >&2
      exit 1
    fi
  done
}

run_frontend_checks() {
  log_step "Installing frontend dependencies"
  npm ci --prefix frontend

  log_step "Running frontend unit tests"
  npm run test --prefix frontend

  log_step "Running frontend build"
  npm run build --prefix frontend
}

mark_release_gate_step_completed() {
  RELEASE_GATE_COMPLETED_STEPS+=("$1")
}

finalize_release_gate_result() {
  local exit_code="$1"
  local status="failed"
  local summary="0.1 GA release gate failed"

  if [[ "${exit_code}" -eq 0 ]]; then
    status="passed"
    summary="0.1 GA release gate passed"
  fi

  local metadata_json
  metadata_json="$(
    jq -n \
      --arg compose_project_prefix "${RELEASE_GATE_COMPOSE_PROJECT_PREFIX}" \
      --arg artifact_dir "${RELEASE_GATE_ARTIFACT_DIR}" \
      --arg result_path "${RELEASE_GATE_RESULT_PATH}" \
      --arg pitr_target_modes "${RELEASE_GATE_PITR_TARGET_MODES}" \
      --argjson include_rust_checks "$([[ "${RELEASE_GATE_INCLUDE_RUST_CHECKS}" == "1" ]] && echo true || echo false)" \
      --argjson include_frontend_checks "$([[ "${RELEASE_GATE_INCLUDE_FRONTEND_CHECKS}" == "1" ]] && echo true || echo false)" \
      --argjson include_compose_smoke "$([[ "${RELEASE_GATE_INCLUDE_COMPOSE_SMOKE}" == "1" ]] && echo true || echo false)" \
      --argjson include_browser_regression "$([[ "${RELEASE_GATE_INCLUDE_BROWSER_REGRESSION}" == "1" ]] && echo true || echo false)" \
      --argjson include_postgres_ops_drill "$([[ "${RELEASE_GATE_INCLUDE_POSTGRES_OPS_DRILL}" == "1" ]] && echo true || echo false)" \
      --argjson include_postgres_pitr_drill "$([[ "${RELEASE_GATE_INCLUDE_POSTGRES_PITR_DRILL}" == "1" ]] && echo true || echo false)" \
      '{
        compose_project_prefix: $compose_project_prefix,
        artifact_dir: $artifact_dir,
        result_path: $result_path,
        include_rust_checks: $include_rust_checks,
        include_frontend_checks: $include_frontend_checks,
        include_compose_smoke: $include_compose_smoke,
        include_browser_regression: $include_browser_regression,
        include_postgres_ops_drill: $include_postgres_ops_drill,
        include_postgres_pitr_drill: $include_postgres_pitr_drill,
        pitr_target_modes: ($pitr_target_modes | split(" ") | map(select(length > 0)))
      }'
  )"

  write_release_result_file \
    "${RELEASE_GATE_RESULT_PATH}" \
    "${status}" \
    "${RELEASE_GATE_STARTED_AT}" \
    "$(release_result_timestamp_utc)" \
    "${summary}" \
    "${metadata_json}" \
    "${RELEASE_GATE_COMPLETED_STEPS[@]}"
}

trap 'exit_code=$?; finalize_release_gate_result "${exit_code}"; exit "${exit_code}"' EXIT

run_rust_checks() {
  log_step "Running Rust formatting check"
  cargo fmt --all --check

  log_step "Running Rust workspace check"
  cargo check --workspace

  log_step "Running Rust workspace tests"
  cargo test --workspace
}

run_compose_smoke() {
  log_step "Running default PostgreSQL GA compose smoke"
  env \
    COMPOSE_PROJECT_NAME="${RELEASE_GATE_COMPOSE_PROJECT_PREFIX}-smoke" \
    COMPOSE_VARIANT=postgres \
    SMOKE_FLOW=postgres \
    CACHE_MODE=dragonfly \
    PRESERVE_STACK_ON_FAILURE="${PRESERVE_STACK_ON_FAILURE}" \
    DOCKER_BUILDKIT=1 \
    COMPOSE_DOCKER_CLI_BUILD=1 \
    ./scripts/compose-smoke.sh
}

run_browser_regression() {
  log_step "Running browser regression runtime mainline"
  env \
    ADMIN_EMAIL="admin@example.com" \
    ADMIN_PASSWORD="Password123456!" \
    ADMIN_NEW_PASSWORD="Password123456!updated" \
    SITE_NAME="Browser Regression" \
    BROWSER_HEADLESS="1" \
    BROWSER_PHASE_TIMEOUT_MS="60000" \
    BROWSER_REGRESSION_ARTIFACT_DIR="${RELEASE_GATE_ARTIFACT_DIR}/browser-regression/runtime-mainline" \
    DOCKER_BUILDKIT="1" \
    COMPOSE_DOCKER_CLI_BUILD="1" \
    ./scripts/browser-regression-ci.sh runtime-mainline

  log_step "Running browser regression bootstrap postgres"
  env \
    ADMIN_EMAIL="admin@example.com" \
    ADMIN_PASSWORD="Password123456!" \
    ADMIN_NEW_PASSWORD="Password123456!updated" \
    SITE_NAME="Browser Regression" \
    BROWSER_HEADLESS="1" \
    BROWSER_PHASE_TIMEOUT_MS="60000" \
    BROWSER_REGRESSION_ARTIFACT_DIR="${RELEASE_GATE_ARTIFACT_DIR}/browser-regression/bootstrap-postgres" \
    DOCKER_BUILDKIT="1" \
    COMPOSE_DOCKER_CLI_BUILD="1" \
    ./scripts/browser-regression-ci.sh bootstrap-postgres
}

run_postgres_ops_drill() {
  log_step "Running PostgreSQL GA ops drill"
  env \
    COMPOSE_PROJECT_NAME="${RELEASE_GATE_COMPOSE_PROJECT_PREFIX}-ops" \
    COMPOSE_VARIANT=postgres \
    ARTIFACT_DIR="${RELEASE_GATE_ARTIFACT_DIR}/postgres-ops-drill" \
    PRESERVE_STACK_ON_FAILURE="${PRESERVE_STACK_ON_FAILURE}" \
    DOCKER_BUILDKIT=1 \
    COMPOSE_DOCKER_CLI_BUILD=1 \
    ./scripts/postgres-ops-drill.sh
}

run_postgres_pitr_drills() {
  local pitr_target_modes=()
  local pitr_target_mode

  read -r -a pitr_target_modes <<< "${RELEASE_GATE_PITR_TARGET_MODES}"
  if [[ "${#pitr_target_modes[@]}" -eq 0 ]]; then
    echo "RELEASE_GATE_PITR_TARGET_MODES must not be empty" >&2
    exit 1
  fi

  for pitr_target_mode in "${pitr_target_modes[@]}"; do
    case "${pitr_target_mode}" in
      name|time)
        ;;
      *)
        echo "Unsupported PITR target mode: ${pitr_target_mode}" >&2
        exit 1
        ;;
    esac

    log_step "Running PostgreSQL GA PITR drill (${pitr_target_mode})"
    env \
      COMPOSE_PROJECT_NAME="${RELEASE_GATE_COMPOSE_PROJECT_PREFIX}-pitr-${pitr_target_mode}" \
      COMPOSE_VARIANT=postgres \
      PITR_TARGET_MODE="${pitr_target_mode}" \
      ARTIFACT_DIR="${RELEASE_GATE_ARTIFACT_DIR}/postgres-pitr-${pitr_target_mode}" \
      PRESERVE_STACK_ON_FAILURE="${PRESERVE_STACK_ON_FAILURE}" \
      DOCKER_BUILDKIT=1 \
      COMPOSE_DOCKER_CLI_BUILD=1 \
      ./scripts/postgres-ops-pitr-drill.sh
  done
}

main() {
  expect_toggle "${RELEASE_GATE_INCLUDE_RUST_CHECKS}" "RELEASE_GATE_INCLUDE_RUST_CHECKS"
  expect_toggle "${RELEASE_GATE_INCLUDE_FRONTEND_CHECKS}" "RELEASE_GATE_INCLUDE_FRONTEND_CHECKS"
  expect_toggle "${RELEASE_GATE_INCLUDE_COMPOSE_SMOKE}" "RELEASE_GATE_INCLUDE_COMPOSE_SMOKE"
  expect_toggle "${RELEASE_GATE_INCLUDE_BROWSER_REGRESSION}" "RELEASE_GATE_INCLUDE_BROWSER_REGRESSION"
  expect_toggle "${RELEASE_GATE_INCLUDE_POSTGRES_OPS_DRILL}" "RELEASE_GATE_INCLUDE_POSTGRES_OPS_DRILL"
  expect_toggle "${RELEASE_GATE_INCLUDE_POSTGRES_PITR_DRILL}" "RELEASE_GATE_INCLUDE_POSTGRES_PITR_DRILL"
  expect_toggle "${PRESERVE_STACK_ON_FAILURE}" "PRESERVE_STACK_ON_FAILURE"
  require_commands

  log_step "Starting 0.1 GA release gate"
  echo "Compose project prefix: ${RELEASE_GATE_COMPOSE_PROJECT_PREFIX}"
  echo "Artifact dir: ${RELEASE_GATE_ARTIFACT_DIR}"

  if [[ "${RELEASE_GATE_INCLUDE_FRONTEND_CHECKS}" == "1" ]]; then
    run_frontend_checks
    mark_release_gate_step_completed "frontend_checks"
  fi

  if [[ "${RELEASE_GATE_INCLUDE_RUST_CHECKS}" == "1" ]]; then
    run_rust_checks
    mark_release_gate_step_completed "rust_checks"
  fi

  if [[ "${RELEASE_GATE_INCLUDE_COMPOSE_SMOKE}" == "1" ]]; then
    run_compose_smoke
    mark_release_gate_step_completed "compose_smoke"
  fi

  if [[ "${RELEASE_GATE_INCLUDE_BROWSER_REGRESSION}" == "1" ]]; then
    run_browser_regression
    mark_release_gate_step_completed "browser_regression"
  fi

  if [[ "${RELEASE_GATE_INCLUDE_POSTGRES_OPS_DRILL}" == "1" ]]; then
    run_postgres_ops_drill
    mark_release_gate_step_completed "postgres_ops_drill"
  fi

  if [[ "${RELEASE_GATE_INCLUDE_POSTGRES_PITR_DRILL}" == "1" ]]; then
    run_postgres_pitr_drills
    mark_release_gate_step_completed "postgres_pitr_drill"
  fi

  echo
  echo "0.1 GA release gate passed"
}

main "$@"
