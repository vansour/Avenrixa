#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

RELEASE_GATE_INCLUDE_RUST_CHECKS="${RELEASE_GATE_INCLUDE_RUST_CHECKS:-1}"
RELEASE_GATE_INCLUDE_COMPOSE_SMOKE="${RELEASE_GATE_INCLUDE_COMPOSE_SMOKE:-1}"
RELEASE_GATE_INCLUDE_POSTGRES_OPS_DRILL="${RELEASE_GATE_INCLUDE_POSTGRES_OPS_DRILL:-1}"
RELEASE_GATE_INCLUDE_POSTGRES_PITR_DRILL="${RELEASE_GATE_INCLUDE_POSTGRES_PITR_DRILL:-1}"
RELEASE_GATE_PITR_TARGET_MODES="${RELEASE_GATE_PITR_TARGET_MODES:-name time}"
RELEASE_GATE_COMPOSE_PROJECT_PREFIX="${RELEASE_GATE_COMPOSE_PROJECT_PREFIX:-avenrixa-release-ga-gate}"
RELEASE_GATE_ARTIFACT_DIR="${RELEASE_GATE_ARTIFACT_DIR:-ops-backups/release-ga-gate}"
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
  local required_commands=(bash)

  if [[ "${RELEASE_GATE_INCLUDE_RUST_CHECKS}" == "1" ]]; then
    required_commands+=(cargo)
  fi

  if [[ "${RELEASE_GATE_INCLUDE_COMPOSE_SMOKE}" == "1" \
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
  expect_toggle "${RELEASE_GATE_INCLUDE_COMPOSE_SMOKE}" "RELEASE_GATE_INCLUDE_COMPOSE_SMOKE"
  expect_toggle "${RELEASE_GATE_INCLUDE_POSTGRES_OPS_DRILL}" "RELEASE_GATE_INCLUDE_POSTGRES_OPS_DRILL"
  expect_toggle "${RELEASE_GATE_INCLUDE_POSTGRES_PITR_DRILL}" "RELEASE_GATE_INCLUDE_POSTGRES_PITR_DRILL"
  expect_toggle "${PRESERVE_STACK_ON_FAILURE}" "PRESERVE_STACK_ON_FAILURE"
  require_commands

  log_step "Starting 0.1 GA release gate"
  echo "Compose project prefix: ${RELEASE_GATE_COMPOSE_PROJECT_PREFIX}"
  echo "Artifact dir: ${RELEASE_GATE_ARTIFACT_DIR}"

  if [[ "${RELEASE_GATE_INCLUDE_RUST_CHECKS}" == "1" ]]; then
    run_rust_checks
  fi

  if [[ "${RELEASE_GATE_INCLUDE_COMPOSE_SMOKE}" == "1" ]]; then
    run_compose_smoke
  fi

  if [[ "${RELEASE_GATE_INCLUDE_POSTGRES_OPS_DRILL}" == "1" ]]; then
    run_postgres_ops_drill
  fi

  if [[ "${RELEASE_GATE_INCLUDE_POSTGRES_PITR_DRILL}" == "1" ]]; then
    run_postgres_pitr_drills
  fi

  echo
  echo "0.1 GA release gate passed"
}

main "$@"
