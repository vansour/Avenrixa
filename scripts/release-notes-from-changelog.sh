#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT_DIR}"

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

main() {
  local version section body

  version="${1:-}"
  if [[ -z "${version}" ]]; then
    echo "Usage: $0 <version>" >&2
    exit 1
  fi

  section="$(extract_changelog_section "${version}")"
  if [[ -z "${section}" ]]; then
    echo "CHANGELOG.md is missing a section for ${version}" >&2
    exit 1
  fi

  body="$(printf '%s\n' "${section}" | sed '1d')"

  printf '# Avenrixa %s\n\n' "${version}"
  printf '## Changelog\n\n'
  printf '%s\n' "${body}"
}

main "$@"
