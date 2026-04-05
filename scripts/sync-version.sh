#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# Version source: argument or root package.json
if [ $# -ge 1 ]; then
  VERSION="$1"
  # Update source of truth first
  node -e "
    const f = '${REPO_ROOT}/package.json';
    const pkg = JSON.parse(require('fs').readFileSync(f, 'utf-8'));
    pkg.version = '${VERSION}';
    require('fs').writeFileSync(f, JSON.stringify(pkg, null, 2) + '\n');
  "
else
  VERSION=$(node -p "require('${REPO_ROOT}/package.json').version")
fi

echo "Syncing all manifests to v${VERSION}"

# Extension package.json
node -e "
  const f = '${REPO_ROOT}/extension/package.json';
  const pkg = JSON.parse(require('fs').readFileSync(f, 'utf-8'));
  pkg.version = '${VERSION}';
  require('fs').writeFileSync(f, JSON.stringify(pkg, null, 2) + '\n');
"

# Cargo.toml
sed -i "s/^version = \".*\"/version = \"${VERSION}\"/" "${REPO_ROOT}/crates/opencodecommit/Cargo.toml"

# npm platform packages
for dir in linux-x64 linux-arm64 darwin-x64 darwin-arm64 win32-x64; do
  node -e "
    const f = '${REPO_ROOT}/npm/${dir}/package.json';
    const pkg = JSON.parse(require('fs').readFileSync(f, 'utf-8'));
    pkg.version = '${VERSION}';
    require('fs').writeFileSync(f, JSON.stringify(pkg, null, 2) + '\n');
  "
done

# npm wrapper package + its optionalDependencies
node -e "
  const f = '${REPO_ROOT}/npm/opencodecommit/package.json';
  const pkg = JSON.parse(require('fs').readFileSync(f, 'utf-8'));
  pkg.version = '${VERSION}';
  if (pkg.optionalDependencies) {
    for (const dep of Object.keys(pkg.optionalDependencies)) {
      pkg.optionalDependencies[dep] = '${VERSION}';
    }
  }
  require('fs').writeFileSync(f, JSON.stringify(pkg, null, 2) + '\n');
"

# Update Cargo.lock
(cd "${REPO_ROOT}" && cargo check --quiet 2>/dev/null) || true

echo "Done: $(grep -c '"version"' <<< "$(git diff --stat)" 2>/dev/null || echo "all") files updated to v${VERSION}"
