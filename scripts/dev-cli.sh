#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
REPO_ROOT=$(cd -- "${SCRIPT_DIR}/.." && pwd -P)

# shellcheck source=./lib/worktree.sh
source "${SCRIPT_DIR}/lib/worktree.sh"

usage() {
  cat <<'EOF'
Usage: scripts/dev-cli.sh [options] <occ-subcommand> [args...]

Run the local Rust CLI from a selected worktree. This is useful for quickly
swapping between worktrees when testing the TUI or any other `occ` command.
Build artifacts are shared under `.git/dev/cargo-target` to keep swaps fast.

Options:
  -w, --worktree NAME|PATH
                        Worktree branch, directory name, or explicit path
  --release            Use `cargo run --release`
  --list               List known worktrees
  -h, --help           Show this help

Examples:
  scripts/dev-cli.sh -w sensitive-trigger tui
  scripts/dev-cli.sh -w dev commit --dry-run --text
  scripts/dev-cli.sh --list
EOF
}

WORKTREE_SELECTOR=""
USE_RELEASE=false
LIST_ONLY=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    -w|--worktree)
      if [[ $# -lt 2 || -z "${2:-}" ]]; then
        printf '--worktree requires a value\n' >&2
        exit 1
      fi
      WORKTREE_SELECTOR="${2:-}"
      shift 2
      ;;
    --worktree=*)
      WORKTREE_SELECTOR="${1#*=}"
      shift
      ;;
    --release)
      USE_RELEASE=true
      shift
      ;;
    --list)
      LIST_ONLY=true
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
    -*)
      printf 'unknown option: %s\n\n' "$1" >&2
      usage >&2
      exit 1
      ;;
    *)
      break
      ;;
  esac
done

if [[ "$LIST_ONLY" == true ]]; then
  occ_list_worktrees_pretty "$REPO_ROOT"
  exit 0
fi

if [[ $# -eq 0 ]]; then
  usage >&2
  exit 1
fi

if ! WORKTREE_PATH=$(occ_resolve_worktree "$WORKTREE_SELECTOR" "$REPO_ROOT"); then
  printf '\nKnown worktrees:\n' >&2
  occ_list_worktrees_pretty "$REPO_ROOT" >&2
  exit 1
fi

cd -- "$WORKTREE_PATH"

CARGO_TARGET_DIR="$(occ_git_common_dir "$WORKTREE_PATH")/dev/cargo-target"
mkdir -p "$CARGO_TARGET_DIR"

RUNNER=(env CARGO_TARGET_DIR="$CARGO_TARGET_DIR" cargo run --quiet --manifest-path "${WORKTREE_PATH}/Cargo.toml" --bin occ --)
if [[ "$USE_RELEASE" == true ]]; then
  RUNNER=(env CARGO_TARGET_DIR="$CARGO_TARGET_DIR" cargo run --quiet --release --manifest-path "${WORKTREE_PATH}/Cargo.toml" --bin occ --)
fi

printf 'worktree: %s\n' "$WORKTREE_PATH" >&2
exec "${RUNNER[@]}" "$@"
