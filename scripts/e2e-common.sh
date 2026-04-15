#!/usr/bin/env bash
set -euo pipefail

occ_e2e_repo_root() {
  cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd
}

occ_e2e_require_cmd() {
  local cmd=$1
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "[fail] required command not found: $cmd" >&2
    exit 1
  fi
}

occ_e2e_require_env() {
  local name=$1
  if [ -z "${!name:-}" ]; then
    echo "[fail] required environment variable is not set: $name" >&2
    exit 1
  fi
}

occ_e2e_default_backends_for_profile() {
  case "${1:-dev-local}" in
    dev-local)
      echo "custom-api,lm-studio-api"
      ;;
    staging)
      echo "codex,opencode,claude,gemini,openai-api,anthropic-api,gemini-api,openrouter-api,opencode-api,ollama-api,lm-studio-api,custom-api"
      ;;
    *)
      echo "unknown profile: $1" >&2
      return 1
      ;;
  esac
}

occ_e2e_all_backends_csv() {
  occ_e2e_default_backends_for_profile staging
}

occ_e2e_trim() {
  local value=${1:-}
  value=${value#"${value%%[![:space:]]*}"}
  value=${value%"${value##*[![:space:]]}"}
  printf '%s' "$value"
}

occ_e2e_is_known_backend() {
  case "${1:-}" in
    codex|opencode|claude|gemini|openai-api|anthropic-api|gemini-api|openrouter-api|opencode-api|ollama-api|lm-studio-api|custom-api)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

occ_e2e_normalize_backends() {
  local requested=${1:-all}
  requested=$(occ_e2e_trim "$requested")
  requested=${requested,,}

  if [ -z "$requested" ] || [ "$requested" = "all" ]; then
    occ_e2e_all_backends_csv
    return 0
  fi

  local normalized=""
  local seen="," item
  local items=()
  IFS=',' read -r -a items <<<"$requested"
  for item in "${items[@]}"; do
    item=$(occ_e2e_trim "$item")
    item=${item,,}
    if [ -z "$item" ]; then
      continue
    fi
    if ! occ_e2e_is_known_backend "$item"; then
      echo "[fail] unknown backend: $item" >&2
      return 1
    fi
    if [[ "$seen" == *",$item,"* ]]; then
      continue
    fi
    seen="${seen}${item},"
    if [ -z "$normalized" ]; then
      normalized=$item
    else
      normalized="${normalized},${item}"
    fi
  done

  if [ -z "$normalized" ]; then
    echo "[fail] no backends selected" >&2
    return 1
  fi

  printf '%s\n' "$normalized"
}

occ_e2e_first_backend() {
  local backends=${1:-}
  printf '%s\n' "${backends%%,*}"
}

occ_e2e_backends_match_all() {
  local backends=${1:-}
  [ "$backends" = "$(occ_e2e_all_backends_csv)" ]
}

occ_e2e_mode_for_backends() {
  local backends=${1:-}
  if occ_e2e_backends_match_all "$backends"; then
    echo "staging"
  else
    echo "targeted"
  fi
}

occ_e2e_has_backend() {
  local backends=${1:-}
  local needle=${2:-}
  case ",${backends}," in
    *,"${needle}",*)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

occ_e2e_toml_string_array() {
  local csv=${1:-}
  local item
  local first=1
  local items=()
  IFS=',' read -r -a items <<<"$csv"
  printf '['
  for item in "${items[@]}"; do
    if [ -z "$item" ]; then
      continue
    fi
    if [ $first -eq 0 ]; then
      printf ', '
    fi
    printf '"%s"' "$item"
    first=0
  done
  printf ']'
}

occ_e2e_render_config_for_backends() {
  local backends_csv=$1
  local output_path=$2
  local backend_order
  backend_order=$(occ_e2e_toml_string_array "$backends_csv")
  local primary_backend
  primary_backend=$(occ_e2e_first_backend "$backends_csv")

  local llama_base=${OCC_E2E_LLAMA_BASE_URL:-http://127.0.0.1:8080}
  local llama_model=${OCC_E2E_LLAMA_MODEL_ID:-${OCC_E2E_LLAMA_MODEL_REPO:-unsloth/Qwen3.5-2B-GGUF}:${OCC_E2E_LLAMA_MODEL_QUANT:-Q4_K_M}}
  local ollama_base=${OCC_E2E_OLLAMA_BASE_URL:-http://127.0.0.1:11434}
  local ollama_model=${OCC_E2E_OLLAMA_MODEL:-qwen3.5:latest}

  local opencode_provider=${OCC_E2E_OPENCODE_PROVIDER:-openai}
  local opencode_model=${OCC_E2E_OPENCODE_MODEL:-gpt-5.4-mini}
  local opencode_path=${OCC_E2E_OPENCODE_PATH:-}
  local claude_model=${OCC_E2E_CLAUDE_MODEL:-claude-sonnet-4-6}
  local claude_path=${OCC_E2E_CLAUDE_PATH:-}
  local codex_model=${OCC_E2E_CODEX_MODEL:-gpt-5.4-mini}
  local codex_provider=${OCC_E2E_CODEX_PROVIDER:-}
  local codex_path=${OCC_E2E_CODEX_PATH:-}
  local gemini_model=${OCC_E2E_GEMINI_MODEL:-gemini-2.5-flash}
  local gemini_path=${OCC_E2E_GEMINI_PATH:-}

  local opencode_pr_provider=${OCC_E2E_OPENCODE_PR_PROVIDER:-$opencode_provider}
  local opencode_pr_model=${OCC_E2E_OPENCODE_PR_MODEL:-gpt-5.4}
  local opencode_cheap_provider=${OCC_E2E_OPENCODE_CHEAP_PROVIDER:-$opencode_provider}
  local opencode_cheap_model=${OCC_E2E_OPENCODE_CHEAP_MODEL:-gpt-5.4-mini}
  local claude_pr_model=${OCC_E2E_CLAUDE_PR_MODEL:-claude-opus-4-6}
  local claude_cheap_model=${OCC_E2E_CLAUDE_CHEAP_MODEL:-claude-haiku-4-5}
  local codex_pr_provider=${OCC_E2E_CODEX_PR_PROVIDER:-$codex_provider}
  local codex_pr_model=${OCC_E2E_CODEX_PR_MODEL:-gpt-5.4}
  local codex_cheap_provider=${OCC_E2E_CODEX_CHEAP_PROVIDER:-$codex_provider}
  local codex_cheap_model=${OCC_E2E_CODEX_CHEAP_MODEL:-gpt-5.4-mini}
  local gemini_pr_model=${OCC_E2E_GEMINI_PR_MODEL:-gemini-3-flash-preview}
  local gemini_cheap_model=${OCC_E2E_GEMINI_CHEAP_MODEL:-gemini-3.1-flash-lite-preview}
  local commit_timeout=${OCC_E2E_COMMIT_TIMEOUT_SECONDS:-120}
  local pr_timeout=${OCC_E2E_PR_TIMEOUT_SECONDS:-300}

  cat >"$output_path" <<EOF
backend = "${primary_backend}"
backend-order = ${backend_order}
commit-mode = "adaptive"
sparkle-mode = "adaptive"
branch-mode = "conventional"
diff-source = "auto"
pr-base-branch = "main"
active-language = "English"
commit-branch-timeout-seconds = ${commit_timeout}
pr-timeout-seconds = ${pr_timeout}

[sensitive]
enforcement = "warn"

provider = "${opencode_provider}"
model = "${opencode_model}"
cli-path = "${opencode_path}"
claude-path = "${claude_path}"
codex-path = "${codex_path}"
claude-model = "${claude_model}"
codex-model = "${codex_model}"
codex-provider = "${codex_provider}"
gemini-path = "${gemini_path}"
gemini-model = "${gemini_model}"
opencode-pr-provider = "${opencode_pr_provider}"
opencode-pr-model = "${opencode_pr_model}"
opencode-cheap-provider = "${opencode_cheap_provider}"
opencode-cheap-model = "${opencode_cheap_model}"
claude-pr-model = "${claude_pr_model}"
claude-cheap-model = "${claude_cheap_model}"
codex-pr-provider = "${codex_pr_provider}"
codex-pr-model = "${codex_pr_model}"
codex-cheap-provider = "${codex_cheap_provider}"
codex-cheap-model = "${codex_cheap_model}"
gemini-pr-model = "${gemini_pr_model}"
gemini-cheap-model = "${gemini_cheap_model}"

[api.openai]
model = "gpt-5.4-mini"
endpoint = "https://api.openai.com/v1/chat/completions"
key-env = "OPENAI_API_KEY"
pr-model = "gpt-5.4"
cheap-model = "gpt-5.4-mini"

[api.anthropic]
model = "claude-sonnet-4-6"
endpoint = "https://api.anthropic.com/v1/messages"
key-env = "ANTHROPIC_API_KEY"
pr-model = "claude-opus-4-6"
cheap-model = "claude-haiku-4-5"

[api.gemini]
model = "gemini-2.5-flash"
endpoint = "https://generativelanguage.googleapis.com/v1beta"
key-env = "GEMINI_API_KEY"
pr-model = "gemini-3-flash-preview"
cheap-model = "gemini-3.1-flash-lite-preview"

[api.openrouter]
model = "anthropic/claude-sonnet-4"
endpoint = "https://openrouter.ai/api/v1/chat/completions"
key-env = "OPENROUTER_API_KEY"
pr-model = "openai/gpt-5.4"
cheap-model = "openai/gpt-5.4-mini"

[api.opencode]
model = "gpt-5.4-mini"
endpoint = "https://opencode.ai/zen/v1/chat/completions"
key-env = "OPENCODE_API_KEY"
pr-model = "gpt-5.4"
cheap-model = "gpt-5.4-mini"

[api.ollama]
model = "${ollama_model}"
endpoint = "${ollama_base}"
key-env = ""
pr-model = "${ollama_model}"
cheap-model = "${ollama_model}"

[api.lm-studio]
model = "${llama_model}"
endpoint = "${llama_base}"
key-env = ""
pr-model = "${llama_model}"
cheap-model = "${llama_model}"

[api.custom]
model = "${llama_model}"
endpoint = "${llama_base}"
key-env = ""
pr-model = "${llama_model}"
cheap-model = "${llama_model}"
EOF
}

occ_e2e_render_config() {
  local profile=$1
  local output_path=$2
  local repo_root
  repo_root=$(occ_e2e_repo_root)
  local template="$repo_root/test-fixtures/e2e/config.${profile}.toml.in"
  if [ ! -f "$template" ]; then
    echo "[fail] missing config template: $template" >&2
    return 1
  fi

  local llama_base=${OCC_E2E_LLAMA_BASE_URL:-http://127.0.0.1:8080}
  local llama_model=${OCC_E2E_LLAMA_MODEL_ID:-${OCC_E2E_LLAMA_MODEL_REPO:-unsloth/Qwen3.5-2B-GGUF}:${OCC_E2E_LLAMA_MODEL_QUANT:-Q4_K_M}}
  local ollama_base=${OCC_E2E_OLLAMA_BASE_URL:-http://127.0.0.1:11434}
  local ollama_model=${OCC_E2E_OLLAMA_MODEL:-qwen3.5:latest}

  sed \
    -e "s|__LLAMA_BASE_URL__|$llama_base|g" \
    -e "s|__LLAMA_MODEL__|$llama_model|g" \
    -e "s|__OLLAMA_BASE_URL__|$ollama_base|g" \
    -e "s|__OLLAMA_MODEL__|$ollama_model|g" \
    "$template" > "$output_path"
}

occ_e2e_wait_for_openai_server() {
  local base_url=$1
  local attempts=${2:-180}
  local payload='{"model":"healthcheck","messages":[{"role":"user","content":"ok"}],"max_tokens":4,"temperature":0}'

  for _ in $(seq 1 "$attempts"); do
    if curl -sf -H 'Content-Type: application/json' -d "$payload" "$base_url/v1/chat/completions" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done

  echo "[fail] OpenAI-compatible server not ready at $base_url" >&2
  return 1
}

occ_e2e_start_llama_server() {
  occ_e2e_require_cmd "${OCC_E2E_LLAMA_BIN:-llama-server}"
  local repo_root
  repo_root=$(occ_e2e_repo_root)

  export OCC_E2E_LLAMA_BIN=${OCC_E2E_LLAMA_BIN:-llama-server}
  export OCC_E2E_LLAMA_HOST=${OCC_E2E_LLAMA_HOST:-127.0.0.1}
  export OCC_E2E_LLAMA_PORT=${OCC_E2E_LLAMA_PORT:-8080}
  export OCC_E2E_LLAMA_BASE_URL=${OCC_E2E_LLAMA_BASE_URL:-http://${OCC_E2E_LLAMA_HOST}:${OCC_E2E_LLAMA_PORT}}
  export OCC_E2E_LLAMA_MODEL_REPO=${OCC_E2E_LLAMA_MODEL_REPO:-unsloth/Qwen3.5-2B-GGUF}
  export OCC_E2E_LLAMA_MODEL_QUANT=${OCC_E2E_LLAMA_MODEL_QUANT:-Q4_K_M}
  export LLAMA_CACHE=${LLAMA_CACHE:-$repo_root/.cache/llama.cpp}
  export HF_HOME=${HF_HOME:-$repo_root/.cache/huggingface}

  mkdir -p "$LLAMA_CACHE" "$HF_HOME"

  local log_path=${OCC_E2E_LLAMA_LOG:-$(mktemp -t occ-llama-log.XXXXXX)}
  export OCC_E2E_LLAMA_LOG=$log_path

  "${OCC_E2E_LLAMA_BIN}" \
    -hf "${OCC_E2E_LLAMA_MODEL_REPO}:${OCC_E2E_LLAMA_MODEL_QUANT}" \
    --host "$OCC_E2E_LLAMA_HOST" \
    --port "$OCC_E2E_LLAMA_PORT" \
    --ctx-size 4096 \
    --n-predict 128 \
    >"$log_path" 2>&1 &
  export OCC_E2E_LLAMA_PID=$!

  occ_e2e_wait_for_openai_server "$OCC_E2E_LLAMA_BASE_URL" 240 || {
    cat "$log_path" >&2 || true
    return 1
  }
}

occ_e2e_stop_llama_server() {
  if [ -n "${OCC_E2E_LLAMA_PID:-}" ]; then
    kill "$OCC_E2E_LLAMA_PID" >/dev/null 2>&1 || true
    wait "$OCC_E2E_LLAMA_PID" >/dev/null 2>&1 || true
    unset OCC_E2E_LLAMA_PID
  fi
}

occ_e2e_prepare_workspace() {
  local workspace=$1
  rm -rf "$workspace"
  mkdir -p "$workspace"

  git -C "$workspace" init -q
  git -C "$workspace" config user.name "OpenCodeCommit E2E"
  git -C "$workspace" config user.email "e2e@example.com"

  mkdir -p "$workspace/src" "$workspace/docs"
  cat > "$workspace/src/app.ts" <<'APP'
export function add(left: number, right: number): number {
  return left + right
}
APP
  cat > "$workspace/README.md" <<'README'
# OpenCodeCommit E2E Fixture
README

  git -C "$workspace" add README.md src/app.ts
  git -C "$workspace" commit -q -m "chore: seed e2e fixture"
  git -C "$workspace" checkout -q -b feature/e2e-coverage

  cat > "$workspace/src/app.ts" <<'APP'
export function add(left: number, right: number): number {
  return left + right
}

export function subtract(left: number, right: number): number {
  return left - right
}
APP

  cat > "$workspace/docs/notes.md" <<'NOTES'
- add subtract helper
- document the behavior for staging verification
NOTES

  git -C "$workspace" add src/app.ts docs/notes.md

  cat > "$workspace/src/app.ts" <<'APP'
export function add(left: number, right: number): number {
  return left + right
}

export function subtract(left: number, right: number): number {
  return left - right
}

export function multiply(left: number, right: number): number {
  return left * right
}
APP
}

occ_e2e_cleanup_dir() {
  if [ -n "${1:-}" ] && [ -d "$1" ]; then
    rm -rf "$1"
  fi
}
