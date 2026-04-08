#!/usr/bin/env bash

release_result_timestamp_utc() {
  date -u +"%Y-%m-%dT%H:%M:%SZ"
}

release_result_steps_json() {
  if [[ "$#" -eq 0 ]]; then
    printf '[]'
    return 0
  fi

  printf '%s\n' "$@" \
    | jq -R . \
    | jq -s 'map(select(length > 0))'
}

write_release_result_file() {
  local path="$1"
  local status="$2"
  local started_at="$3"
  local finished_at="$4"
  local summary="$5"
  local metadata_json
  if [[ "$#" -ge 6 && -n "${6}" ]]; then
    metadata_json="$6"
  else
    metadata_json='{}'
  fi
  shift 6

  local steps_json
  steps_json="$(release_result_steps_json "$@")"

  mkdir -p "$(dirname "${path}")"
  jq -n \
    --arg status "${status}" \
    --arg started_at "${started_at}" \
    --arg finished_at "${finished_at}" \
    --arg summary "${summary}" \
    --argjson metadata "${metadata_json}" \
    --argjson completed_steps "${steps_json}" \
    '{
      status: $status,
      started_at: $started_at,
      finished_at: $finished_at,
      summary: $summary,
      metadata: $metadata,
      completed_steps: $completed_steps
    }' > "${path}"
}
