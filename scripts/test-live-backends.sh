#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
BACKENDS=("$@")
if [ ${#BACKENDS[@]} -eq 0 ]; then
  BACKENDS=(codex opencode claude gemini)
fi

case "${OCC_RUNNER:-}" in
  "")
    OCC_RUNNER=(cargo run --manifest-path "$REPO_ROOT/Cargo.toml" --quiet --bin occ --)
    ;;
  *)
    # shellcheck disable=SC2206
    OCC_RUNNER=(${OCC_RUNNER})
    ;;
esac

backend_binary() {
  case "$1" in
    codex) echo "codex" ;;
    opencode) echo "opencode" ;;
    claude) echo "claude" ;;
    gemini) echo "gemini" ;;
    *)
      echo "unknown backend: $1" >&2
      return 1
      ;;
  esac
}

if ! command -v git >/dev/null 2>&1; then
  echo "git is required" >&2
  exit 1
fi

TMP_DIR=$(mktemp -d)
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

TEST_REPO="$TMP_DIR/live-backend-smoke"
mkdir -p "$TEST_REPO"
cd "$TEST_REPO"

git init -q
git config user.name "OpenCodeCommit Smoke"
git config user.email "smoke@example.com"

cat <<'EOF' > README.md
# Live backend smoke test
EOF

git add README.md
git commit -q -m "chore: seed smoke repo"

cat <<'EOF' > README.md
# Live backend smoke test

Backend matrix validation.
EOF

git add README.md

DIFF=$(git diff --cached)
if [ -z "$DIFF" ]; then
  echo "expected a staged diff in smoke repo" >&2
  exit 1
fi

printf 'Repo: %s\n' "$TEST_REPO"
printf 'Backends: %s\n' "${BACKENDS[*]}"

PASS_COUNT=0
SKIP_COUNT=0
FAIL_COUNT=0

for backend in "${BACKENDS[@]}"; do
  binary=$(backend_binary "$backend")
  if ! command -v "$binary" >/dev/null 2>&1; then
    printf '[skip] %s: %s not found on PATH\n' "$backend" "$binary"
    SKIP_COUNT=$((SKIP_COUNT + 1))
    continue
  fi

  printf '[run] %s\n' "$backend"
  if output=$(printf '%s' "$DIFF" | "${OCC_RUNNER[@]}" commit --stdin --dry-run --text --backend "$backend" 2>&1); then
    printf '[pass] %s\n%s\n' "$backend" "$output"
    PASS_COUNT=$((PASS_COUNT + 1))
  else
    printf '[fail] %s\n%s\n' "$backend" "$output" >&2
    FAIL_COUNT=$((FAIL_COUNT + 1))
  fi
  printf '\n'
done

printf 'Summary: %s passed, %s skipped, %s failed\n' "$PASS_COUNT" "$SKIP_COUNT" "$FAIL_COUNT"

if [ "$FAIL_COUNT" -ne 0 ]; then
  exit 1
fi
