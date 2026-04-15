#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
# shellcheck source=scripts/e2e-common.sh
source "$REPO_ROOT/scripts/e2e-common.sh"

PROFILE=${1:-${OCC_E2E_MODE:-dev-local}}

occ_e2e_require_cmd git
occ_e2e_require_cmd cargo
occ_e2e_require_cmd bun
occ_e2e_require_cmd node
occ_e2e_require_cmd curl

if [ "$PROFILE" = "dev-local" ]; then
  occ_e2e_require_cmd "${OCC_E2E_LLAMA_BIN:-llama-server}"
  exit 0
fi

occ_e2e_require_cmd xvfb-run
occ_e2e_require_cmd gh
occ_e2e_require_cmd ollama
occ_e2e_require_cmd "${OCC_E2E_LLAMA_BIN:-llama-server}"
occ_e2e_require_cmd codex
occ_e2e_require_cmd opencode
occ_e2e_require_cmd claude
occ_e2e_require_cmd gemini

occ_e2e_require_env OPENAI_API_KEY
occ_e2e_require_env ANTHROPIC_API_KEY
occ_e2e_require_env GEMINI_API_KEY
occ_e2e_require_env OPENROUTER_API_KEY
occ_e2e_require_env OPENCODE_API_KEY

if ! curl -sf "${OCC_E2E_OLLAMA_BASE_URL:-http://127.0.0.1:11434}/api/tags" >/dev/null 2>&1; then
  echo "[fail] ollama daemon is not responding at ${OCC_E2E_OLLAMA_BASE_URL:-http://127.0.0.1:11434}" >&2
  exit 1
fi

TMP_DIR=$(mktemp -d -t occ-e2e-preflight.XXXXXX)
trap 'occ_e2e_cleanup_dir "$TMP_DIR"' EXIT
OCC_CONFIG="$TMP_DIR/config.toml"
occ_e2e_render_config staging "$OCC_CONFIG"
WORKSPACE="$TMP_DIR/preflight-repo"
occ_e2e_prepare_workspace "$WORKSPACE"
DIFF=$(git -C "$WORKSPACE" diff --cached)

for backend in codex opencode claude gemini; do
  echo "[preflight] CLI backend: $backend"
  (
    cd "$WORKSPACE"
    printf '%s' "$DIFF" | cargo run --quiet --manifest-path "$REPO_ROOT/Cargo.toml" --bin occ -- \
      commit --stdin --dry-run --text --backend "$backend" --config "$OCC_CONFIG" >/dev/null
  )
done
