#!/usr/bin/env bash
set -euo pipefail

# Navigate to the repo root (parent of scripts/)
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# Find the main repo root (works in both worktrees and main checkout)
MAIN_ROOT="$(git -C "${REPO_ROOT}" rev-parse --path-format=absolute --git-common-dir 2>/dev/null | sed 's|/\.git$||')"
[ -z "${MAIN_ROOT}" ] && MAIN_ROOT="${REPO_ROOT}"

# Extension lives in extension/ subdirectory
cd "${REPO_ROOT}/extension"

VERSION=$(node -p "require('./package.json').version")
NAME=$(node -p "require('./package.json').name")
VSIX="${NAME}-${VERSION}.vsix"

echo "==> Installing extension dependencies"
bun install --frozen-lockfile

echo "==> Building v${VERSION}"
bunx tsc -p ./

echo "==> Packaging ${VSIX}"
bunx @vscode/vsce package

# Open VSX — check main repo root, then worktree root
if [ -f "${MAIN_ROOT}/.ovsx-pat" ]; then
  source "${MAIN_ROOT}/.ovsx-pat"
  echo "==> Publishing to Open VSX"
  bunx ovsx publish "${VSIX}"
elif [ -f "${REPO_ROOT}/.ovsx-pat" ]; then
  source "${REPO_ROOT}/.ovsx-pat"
  echo "==> Publishing to Open VSX"
  bunx ovsx publish "${VSIX}"
else
  echo "==> Skipping Open VSX (no .ovsx-pat found in ${MAIN_ROOT} or ${REPO_ROOT})"
fi

# VS Code Marketplace
if [ -f "${MAIN_ROOT}/.vsce-pat" ]; then
  source "${MAIN_ROOT}/.vsce-pat"
  echo "==> Publishing to VS Code Marketplace"
  bunx @vscode/vsce publish --pat "${VSCE_PAT}"
elif [ -f "${REPO_ROOT}/.vsce-pat" ]; then
  source "${REPO_ROOT}/.vsce-pat"
  echo "==> Publishing to VS Code Marketplace"
  bunx @vscode/vsce publish --pat "${VSCE_PAT}"
else
  echo "==> Skipping VS Code Marketplace (no .vsce-pat)"
fi

echo "==> Done: v${VERSION}"
