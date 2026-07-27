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
editor profile for that worktree, and launch a compatible VS Code/VSCodium.

Options:
  -w, --worktree NAME|PATH
                        Worktree branch, directory name, or explicit path
  --launch-only         Skip build/install and just launch the worktree profile
  --install-only        Build/install, but do not launch the editor
  --list                List known worktrees
  -h, --help            Show this help

Examples:
  scripts/dev-extension.sh -w sensitive-trigger
  scripts/dev-extension.sh -w dev --launch-only
  scripts/dev-extension.sh --list
EOF
}

resolve_editor() {
  local extension_directory=$1
  local cache_directory=$2
  local engine version version_output managed_output
  local managed_lines=()

  engine=$(node "${SCRIPT_DIR}/resolve-vscode-editor.mjs" engine "$extension_directory")
  if command -v flatpak >/dev/null 2>&1 && flatpak info com.vscodium.codium >/dev/null 2>&1; then
    if version_output=$(flatpak run com.vscodium.codium --version 2>/dev/null); then
      version=$(awk '/^[0-9]+\.[0-9]+\.[0-9]+/ { print $1; exit }' <<<"$version_output")
    fi
    if [[ -n "${version:-}" ]] && node "${SCRIPT_DIR}/resolve-vscode-editor.mjs" supports "$extension_directory" "$version"; then
      EDITOR_KIND=vscodium-flatpak
      EDITOR_NAME=VSCodium
      EDITOR_VERSION=$version
      EDITOR_STATE_NAME=vscodium
      EDITOR_GUI=flatpak
      EDITOR_CLI=flatpak
      return 0
    fi

    if [[ -n "${version:-}" ]]; then
      printf 'Installed VSCodium %s does not satisfy engines.vscode %s; using managed VS Code.\n' "$version" "$engine"
    else
      printf 'Could not determine the installed VSCodium version; using managed VS Code.\n'
    fi
  else
    printf 'VSCodium Flatpak not found; using managed VS Code.\n'
  fi

  managed_output=$(node "${SCRIPT_DIR}/resolve-vscode-editor.mjs" managed "$extension_directory" "$cache_directory")
  mapfile -t managed_lines <<<"$managed_output"
  if [[ ${#managed_lines[@]} -ne 3 ]]; then
    printf 'failed to resolve managed VS Code paths\n' >&2
    exit 1
  fi

  EDITOR_KIND=vscode-managed
  EDITOR_NAME='Visual Studio Code'
  EDITOR_VERSION=${managed_lines[0]}
  EDITOR_STATE_NAME=vscode
  EDITOR_GUI=${managed_lines[1]}
  EDITOR_CLI=${managed_lines[2]}
}

run_editor_cli() {
  if [[ "$EDITOR_KIND" == vscodium-flatpak ]]; then
    flatpak run \
      --env=ELECTRON_RUN_AS_NODE=1 \
      --command=/app/bin/com.vscodium.codium-wrapper \
      com.vscodium.codium \
      /app/share/codium/resources/app/out/cli.js \
      "$@"
    return
  fi

  "$EDITOR_CLI" "$@"
}

launch_editor() {
  if [[ "$EDITOR_KIND" == vscodium-flatpak ]]; then
    flatpak run com.vscodium.codium "$@"
    return
  fi

  "$EDITOR_GUI" "$@"
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

if ! WORKTREE_PATH=$(occ_resolve_worktree "$WORKTREE_SELECTOR" "$REPO_ROOT"); then
  printf '\nKnown worktrees:\n' >&2
  occ_list_worktrees_pretty "$REPO_ROOT" >&2
  exit 1
fi

EXTENSION_DIR="${WORKTREE_PATH}/extension"
if [[ "$LAUNCH_ONLY" != true || ! -f "${EXTENSION_DIR}/node_modules/@vscode/test-electron/package.json" ]]; then
  (
    cd -- "$EXTENSION_DIR"
    bun install --frozen-lockfile
  )
fi

resolve_editor "$EXTENSION_DIR" "${REPO_ROOT}/.vscode-test"

STATE_ROOT=$(occ_dev_state_root "$WORKTREE_PATH" "$EDITOR_STATE_NAME" "$REPO_ROOT")
USER_DATA_DIR="${STATE_ROOT}/user-data"
EXTENSIONS_DIR="${STATE_ROOT}/extensions"
mkdir -p "$USER_DATA_DIR" "$EXTENSIONS_DIR"

if [[ "$LAUNCH_ONLY" != true ]]; then
  MANIFEST_VERSION=$(node -e 'const manifest = require(process.argv[1]); process.stdout.write(manifest.version)' "${EXTENSION_DIR}/package.json")
  EXPECTED_EXTENSION=$(node -e 'const manifest = require(process.argv[1]); process.stdout.write(`${manifest.publisher}.${manifest.name}@${manifest.version}`)' "${EXTENSION_DIR}/package.json")
  VSIX_PATH="${EXTENSION_DIR}/opencodecommit-${MANIFEST_VERSION}.vsix"
  (
    cd -- "$EXTENSION_DIR"
    bun run build:vsix
    bunx @vscode/vsce package --out "$VSIX_PATH"
  )

  if [[ ! -f "$VSIX_PATH" ]]; then
    printf 'failed to find packaged VSIX at %s\n' "$VSIX_PATH" >&2
    exit 1
  fi

  run_editor_cli \
    --user-data-dir "$USER_DATA_DIR" \
    --extensions-dir "$EXTENSIONS_DIR" \
    --install-extension "$VSIX_PATH" \
    --force

  INSTALLED_EXTENSIONS=$(run_editor_cli \
    --user-data-dir "$USER_DATA_DIR" \
    --extensions-dir "$EXTENSIONS_DIR" \
    --list-extensions \
    --show-versions)
  if ! grep -Fxiq -- "$EXPECTED_EXTENSION" <<<"$INSTALLED_EXTENSIONS"; then
    printf '%s did not report %s after installation\nInstalled extensions:\n%s\n' \
      "$EDITOR_NAME" "$EXPECTED_EXTENSION" "$INSTALLED_EXTENSIONS" >&2
    exit 1
  fi
  printf 'verified-extension: %s\n' "$EXPECTED_EXTENSION"
fi

if [[ "$INSTALL_ONLY" != true ]]; then
  launch_editor \
    --new-window \
    --user-data-dir "$USER_DATA_DIR" \
    --extensions-dir "$EXTENSIONS_DIR" \
    "$WORKTREE_PATH" >/dev/null 2>&1 &
fi

printf 'editor: %s %s\n' "$EDITOR_NAME" "$EDITOR_VERSION"
printf 'worktree: %s\n' "$WORKTREE_PATH"
printf 'user-data-dir: %s\n' "$USER_DATA_DIR"
printf 'extensions-dir: %s\n' "$EXTENSIONS_DIR"
