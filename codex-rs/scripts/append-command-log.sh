#!/usr/bin/env bash
set -euo pipefail

DOCS=(
  "docs/autonomous-accounting-roadmap.md"
  "docs/autonomous-accounting-phase0.md"
)

usage() {
  cat <<'EOF'
Usage:
  append-command-log.sh [--date YYYY-MM-DD] <command summary>
  append-command-log.sh --check

Options:
  --date YYYY-MM-DD  Explicit date label for the entry (default: current UTC date).
  --check            Verify the latest command log line matches across all docs.

Examples:
  append-command-log.sh "just fmt; cargo test -p codex-tui"
  append-command-log.sh --date 2025-10-23 "cargo test -p codex-cli"
  append-command-log.sh --check
EOF
}

require_docs() {
  for doc in "${DOCS[@]}"; do
    if [[ ! -f "${doc}" ]]; then
      echo "append-command-log.sh: missing doc ${doc}" >&2
      exit 1
    fi
  done
}

if [[ $# -eq 0 ]]; then
  usage
  exit 1
fi

date_stamp=$(date -u '+%Y-%m-%d')
check_only=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --date)
      if [[ $# -lt 2 ]]; then
        echo "append-command-log.sh: --date requires an argument" >&2
        exit 1
      fi
      date_stamp="$2"
      shift 2
      ;;
    --check)
      check_only=true
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    --)
      shift
      break
      ;;
    -* )
      echo "append-command-log.sh: unknown option $1" >&2
      usage
      exit 1
      ;;
    *)
      break
      ;;
  esac
done

require_docs

if [[ "${check_only}" == "true" ]]; then
  if [[ $# -ne 0 ]]; then
    echo "append-command-log.sh: --check does not accept additional arguments" >&2
    exit 1
  fi
  last_lines=()
  for doc in "${DOCS[@]}"; do
    last_line=$(grep -n "Commands executed" "${doc}" | tail -n1 || true)
    if [[ -z "${last_line}" ]]; then
      echo "append-command-log.sh: no command log entry found in ${doc}" >&2
      exit 1
    fi
    # Strip leading line number if present (format: N:text)
    line_text=${last_line#*:}
    last_lines+=("${line_text}")
  done
  reference=${last_lines[0]}
  for line in "${last_lines[@]}"; do
    if [[ "${line}" != "${reference}" ]]; then
      echo "Command log mismatch detected:" >&2
      for idx in "${!DOCS[@]}"; do
        echo "  ${DOCS[$idx]} => ${last_lines[$idx]}" >&2
      done
      exit 1
    fi
  done
  exit 0
fi

if [[ $# -eq 0 ]]; then
  echo "append-command-log.sh: missing command summary" >&2
  usage
  exit 1
fi

entry="$*"

for doc in "${DOCS[@]}"; do
  if [[ -s "${doc}" ]] && [[ $(tail -c1 "${doc}") != $'\n' ]]; then
    printf '\n' >> "${doc}"
  fi
  printf '\n- Commands executed (%s): %s\n' "${date_stamp}" "${entry}" >> "${doc}"
done
