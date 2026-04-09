#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
REPO_ROOT=$(cd -- "${SCRIPT_DIR}/.." && pwd -P)

# shellcheck source=./lib/worktree.sh
source "${SCRIPT_DIR}/lib/worktree.sh"

usage() {
  cat <<'EOF'
Usage: scripts/dev-extension.sh [options]

Build the extension from a selected worktree, install it into an isolated
VSCodium profile for that worktree, and launch VSCodium on that worktree.

Options:
  -w, --worktree NAME|PATH
                        Worktree branch, directory name, or explicit path
  --launch-only         Skip build/install and just launch the worktree profile
  --install-only        Build/install, but do not launch VSCodium
  --list                List known worktrees
  -h, --help            Show this help

Examples:
  scripts/dev-extension.sh -w sensitive-trigger
  scripts/dev-extension.sh -w dev --launch-only
  scripts/dev-extension.sh --list
EOF
}

require_vscodium() {
  if command -v flatpak >/dev/null 2>&1 && flatpak info com.vscodium.codium >/dev/null 2>&1; then
    return 0
  fi

  printf 'VSCodium Flatpak not found: com.vscodium.codium\n' >&2
  exit 1
}

run_codium() {
  flatpak run com.vscodium.codium "$@"
}

WORKTREE_SELECTOR=""
LAUNCH_ONLY=false
INSTALL_ONLY=false
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
    --launch-only)
      LAUNCH_ONLY=true
      shift
      ;;
    --install-only)
      INSTALL_ONLY=true
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
    *)
      printf 'unknown option: %s\n\n' "$1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [[ "$LAUNCH_ONLY" == true && "$INSTALL_ONLY" == true ]]; then
  printf '--launch-only and --install-only cannot be used together\n' >&2
  exit 1
fi

if [[ "$LIST_ONLY" == true ]]; then
  occ_list_worktrees_pretty "$REPO_ROOT"
  exit 0
fi

require_vscodium

if ! WORKTREE_PATH=$(occ_resolve_worktree "$WORKTREE_SELECTOR" "$REPO_ROOT"); then
  printf '\nKnown worktrees:\n' >&2
  occ_list_worktrees_pretty "$REPO_ROOT" >&2
  exit 1
fi

STATE_ROOT=$(occ_dev_state_root "$WORKTREE_PATH" "vscodium" "$REPO_ROOT")
USER_DATA_DIR="${STATE_ROOT}/user-data"
EXTENSIONS_DIR="${STATE_ROOT}/extensions"
mkdir -p "$USER_DATA_DIR" "$EXTENSIONS_DIR"

if [[ "$LAUNCH_ONLY" != true ]]; then
  (
    cd -- "${WORKTREE_PATH}/extension"
    bun install
    bunx tsc -p ./
    bunx @vscode/vsce package
  )

  VSIX_PATH=$(find "${WORKTREE_PATH}/extension" -maxdepth 1 -type f -name 'opencodecommit-*.vsix' -printf '%T@ %p\n' | sort -nr | head -n 1 | cut -d' ' -f2-)
  if [[ -z "$VSIX_PATH" ]]; then
    printf 'failed to find packaged VSIX under %s/extension\n' "$WORKTREE_PATH" >&2
    exit 1
  fi

  run_codium \
    --user-data-dir "$USER_DATA_DIR" \
    --extensions-dir "$EXTENSIONS_DIR" \
    --install-extension "$VSIX_PATH" \
    --force
fi

if [[ "$INSTALL_ONLY" != true ]]; then
  run_codium \
    --new-window \
    --user-data-dir "$USER_DATA_DIR" \
    --extensions-dir "$EXTENSIONS_DIR" \
    "$WORKTREE_PATH" >/dev/null 2>&1 &
fi

printf 'worktree: %s\n' "$WORKTREE_PATH"
printf 'user-data-dir: %s\n' "$USER_DATA_DIR"
printf 'extensions-dir: %s\n' "$EXTENSIONS_DIR"
