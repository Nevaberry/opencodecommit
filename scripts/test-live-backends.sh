#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
BACKENDS=("$@")
# Entries may include a model override, for example codex:gpt-6.1-sol.
if [ ${#BACKENDS[@]} -eq 0 ]; then
  BACKENDS=(codex opencode claude agy grok)
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
    agy) echo "agy" ;;
    grok) echo "grok" ;;
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

# Exercise the shipped defaults, independent of the user's saved model choices.
# An explicit config still allows testing a custom provider or CLI path.
SMOKE_CONFIG=${OCC_SMOKE_CONFIG:-$TMP_DIR/config.toml}
if [ -z "${OCC_SMOKE_CONFIG:-}" ]; then
  : > "$SMOKE_CONFIG"
fi

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

for entry in "${BACKENDS[@]}"; do
  backend=${entry%%:*}
  model_args=()
  if [[ "$entry" == *:* ]]; then
    model_args=(--model "${entry#*:}")
  fi
  binary=$(backend_binary "$backend")
  if ! command -v "$binary" >/dev/null 2>&1; then
    printf '[skip] %s: %s not found on PATH\n' "$backend" "$binary"
    SKIP_COUNT=$((SKIP_COUNT + 1))
    continue
  fi

  printf '[run] %s\n' "$entry"
  if output=$(printf '%s' "$DIFF" | "${OCC_RUNNER[@]}" commit --stdin --dry-run --text --config "$SMOKE_CONFIG" --backend "$backend" "${model_args[@]}" 2>&1); then
    printf '[pass] %s\n%s\n' "$entry" "$output"
    PASS_COUNT=$((PASS_COUNT + 1))
  else
    printf '[fail] %s\n%s\n' "$entry" "$output" >&2
    FAIL_COUNT=$((FAIL_COUNT + 1))
  fi
  printf '\n'
done

printf 'Summary: %s passed, %s skipped, %s failed\n' "$PASS_COUNT" "$SKIP_COUNT" "$FAIL_COUNT"

if [ "$FAIL_COUNT" -ne 0 ]; then
  exit 1
fi
