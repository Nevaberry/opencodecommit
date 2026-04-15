#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)

FIXTURE_DIR=$(mktemp -d -t occ-dev-tui.XXXXXX)
trap 'rm -rf -- "$FIXTURE_DIR"' EXIT

mkdir -p "$FIXTURE_DIR/src" "$FIXTURE_DIR/docs"

git_in() {
  git -C "$FIXTURE_DIR" \
    -c user.name="OpenCodeCommit Dev TUI" \
    -c user.email="dev-tui@example.com" \
    "$@"
}

git_in init -q
cat >"$FIXTURE_DIR/src/app.ts" <<'EOF'
export function add(left: number, right: number): number {
  return left + right
}
EOF
printf '# Dev TUI Fixture\n' >"$FIXTURE_DIR/README.md"
git_in add README.md src/app.ts
git_in commit -q -m "chore: seed dev-tui fixture"
git_in checkout -q -b feature/dev-tui

cat >"$FIXTURE_DIR/src/app.ts" <<'EOF'
export function add(left: number, right: number): number {
  return left + right
}

export function subtract(left: number, right: number): number {
  return left - right
}
EOF
cat >"$FIXTURE_DIR/docs/notes.md" <<'EOF'
- add subtract helper
- dev-tui fixture for manual resize testing
EOF
git_in add src/app.ts docs/notes.md

cat >"$FIXTURE_DIR/src/app.ts" <<'EOF'
export function add(left: number, right: number): number {
  return left + right
}

export function subtract(left: number, right: number): number {
  return left - right
}

export function multiply(left: number, right: number): number {
  return left * right
}
EOF

printf 'dev-tui fixture: %s\n' "$FIXTURE_DIR"
printf 'launching: cargo run -p opencodecommit -- tui --backend codex\n'

cd -- "$FIXTURE_DIR"
exec cargo run --manifest-path "$REPO_ROOT/Cargo.toml" -p opencodecommit -- tui --backend codex "$@"
