#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)

usage() {
  cat <<'USAGE'
Usage: scripts/test-backend-local.sh [-b BACKEND] [-a] [-l LOG_FILE]

Run the OpenCodeCommit e2e suite against a CLI backend on this laptop,
using your locally authenticated CLI (e.g. Codex CLI OAuth). Thin wrapper
around scripts/test-e2e.sh — loads the extension from source so the full
mocha suite (including sinon-stubbed command flows) runs as it does in
normal dev.

Options:
  -b BACKEND   Backend name (default: codex)
  -a           Run all targets: extension + cli + tui
  -l LOG_FILE  Log file path (default: .logs/e2e-<timestamp>.log)
  -h           Show this help
USAGE
}

BACKEND=codex
TARGET=extension
LOG_FILE=

while [ $# -gt 0 ]; do
  case "$1" in
    -b)
      [ $# -ge 2 ] || { echo "-b requires a value" >&2; exit 1; }
      BACKEND=$2
      shift 2
      ;;
    -a)
      TARGET=all
      shift
      ;;
    -l)
      [ $# -ge 2 ] || { echo "-l requires a value" >&2; exit 1; }
      LOG_FILE=$2
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

args=(-b "$BACKEND" -t "$TARGET")
if [ -n "$LOG_FILE" ]; then
  args+=(-l "$LOG_FILE")
fi

exec bash "$REPO_ROOT/scripts/test-e2e.sh" "${args[@]}"
