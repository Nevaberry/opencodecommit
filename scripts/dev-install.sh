#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

cd extension
bun install --frozen-lockfile
bunx tsc -p ./
bunx @vscode/vsce package

VSIX=$(ls -t opencodecommit-*.vsix | head -1)
echo "==> Installing ${VSIX}"
flatpak run com.vscodium.codium --install-extension "${VSIX}" --force
flatpak run com.vscodium.codium &
