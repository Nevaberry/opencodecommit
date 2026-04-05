#!/usr/bin/env bash
set -euo pipefail

# ---------------------------------------------------------------------------
# Usage
# ---------------------------------------------------------------------------
usage() {
  cat <<EOF
Usage: $(basename "$0") [targets...]

Targets:
  --ovsx     Publish extension to Open VSX
  --vsce     Publish extension to VS Code Marketplace
  --npm      Publish CLI to npm
  --cargo    Publish CLI to crates.io
  --all      All of the above

Extension targets (--ovsx, --vsce) build and package the VSIX automatically.

Examples:
  $(basename "$0") --all
  $(basename "$0") --ovsx --vsce
  $(basename "$0") --npm --cargo
EOF
  exit 1
}

# ---------------------------------------------------------------------------
# Parse flags
# ---------------------------------------------------------------------------
DO_OVSX=false DO_VSCE=false DO_NPM=false DO_CARGO=false

[[ $# -eq 0 ]] && usage

for arg in "$@"; do
  case "$arg" in
    --ovsx)  DO_OVSX=true ;;
    --vsce)  DO_VSCE=true ;;
    --npm)   DO_NPM=true ;;
    --cargo) DO_CARGO=true ;;
    --all)   DO_OVSX=true; DO_VSCE=true; DO_NPM=true; DO_CARGO=true ;;
    *)       echo "Unknown flag: $arg"; usage ;;
  esac
done

# ---------------------------------------------------------------------------
# Common setup
# ---------------------------------------------------------------------------
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MAIN_ROOT="$(git -C "${REPO_ROOT}" rev-parse --path-format=absolute --git-common-dir 2>/dev/null | sed 's|/\.git$||')"
[ -z "${MAIN_ROOT}" ] && MAIN_ROOT="${REPO_ROOT}"

VERSION=$(node -p "require('${REPO_ROOT}/extension/package.json').version")
echo "==> Publishing v${VERSION}"

# Source a token file from MAIN_ROOT or REPO_ROOT, return 1 if missing
source_token() {
  local name="$1"
  if [ -f "${MAIN_ROOT}/${name}" ]; then
    source "${MAIN_ROOT}/${name}"
  elif [ -f "${REPO_ROOT}/${name}" ]; then
    source "${REPO_ROOT}/${name}"
  else
    echo "    ⚠ ${name} not found in ${MAIN_ROOT} or ${REPO_ROOT}"
    return 1
  fi
}

# ---------------------------------------------------------------------------
# Extension build (shared by --ovsx and --vsce)
# ---------------------------------------------------------------------------
VSIX_BUILT=false
build_vsix() {
  if $VSIX_BUILT; then return; fi
  cd "${REPO_ROOT}/extension"
  NAME=$(node -p "require('./package.json').name")
  VSIX="${NAME}-${VERSION}.vsix"

  echo "==> Installing extension dependencies"
  bun install --frozen-lockfile

  echo "==> Building extension"
  bunx tsc -p ./

  echo "==> Packaging ${VSIX}"
  bunx @vscode/vsce package

  VSIX_BUILT=true
  cd "${REPO_ROOT}"
}

# ---------------------------------------------------------------------------
# Open VSX
# ---------------------------------------------------------------------------
if $DO_OVSX; then
  echo "==> Open VSX"
  if source_token .ovsx-pat; then
    build_vsix
    cd "${REPO_ROOT}/extension"
    bunx ovsx publish "${VSIX}"
    cd "${REPO_ROOT}"
    echo "    ✓ Open VSX published"
  fi
fi

# ---------------------------------------------------------------------------
# VS Code Marketplace
# ---------------------------------------------------------------------------
if $DO_VSCE; then
  echo "==> VS Code Marketplace"
  if source_token .vsce-pat; then
    build_vsix
    cd "${REPO_ROOT}/extension"
    bunx @vscode/vsce publish --pat "${VSCE_PAT}"
    cd "${REPO_ROOT}"
    echo "    ✓ VS Code Marketplace published"
  fi
fi

# ---------------------------------------------------------------------------
# npm
# ---------------------------------------------------------------------------
if $DO_NPM; then
  echo "==> npm"
  echo "    Publishing @nevaberry/opencodecommit"
  (cd "${REPO_ROOT}/npm/opencodecommit" && npm publish --access public)
  echo "    ✓ npm published"
fi

# ---------------------------------------------------------------------------
# crates.io
# ---------------------------------------------------------------------------
if $DO_CARGO; then
  echo "==> crates.io"
  (cd "${REPO_ROOT}/crates/opencodecommit" && cargo publish)
  echo "    ✓ crates.io published"
fi

echo "==> Done: v${VERSION}"
