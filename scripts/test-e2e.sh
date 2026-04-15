#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
# shellcheck source=scripts/e2e-common.sh
source "$REPO_ROOT/scripts/e2e-common.sh"

usage() {
  cat <<'USAGE'
Usage: scripts/test-e2e.sh [options]

Run targeted OpenCodeCommit e2e suites with backend filtering and plain-text logs.

Options:
  -b, --backends CSV|all   Backends to enable, for example codex or codex,claude
  -t, --target TARGET      extension, cli, tui, both, or all
  -s, --suite SUITE        full or artifacts
  -l, --log-file PATH      Log file path. Defaults to .logs/e2e-<timestamp>.log
  -h, --help               Show this help

Examples:
  scripts/test-e2e.sh --backends codex --target all --suite artifacts
  scripts/test-e2e.sh -b all -t extension -l .logs/e2e-extension.log
USAGE
}

log() {
  printf '[%s] %s\n' "$(date '+%Y-%m-%d %H:%M:%S')" "$*"
}

fail() {
  log "[fail] $*"
  exit 1
}

run_step() {
  local label=$1
  shift
  local start
  start=$(date +%s)
  log "[step] $label"
  "$@"
  log "[pass] $label ($(($(date +%s) - start))s)"
}

prepare_vscodium_wrapper() {
  if [ -n "${OCC_E2E_VSCODE_EXECUTABLE:-}" ]; then
    return 0
  fi

  if command -v flatpak >/dev/null 2>&1 && flatpak info com.vscodium.codium >/dev/null 2>&1; then
    local wrapper_path=$RUN_ROOT/vscodium-electron.sh
    cat >"$wrapper_path" <<'WRAPPER'
#!/usr/bin/env bash
exec flatpak run --command=/app/bin/com.vscodium.codium-wrapper com.vscodium.codium "$@"
WRAPPER
    chmod +x "$wrapper_path"
    export OCC_E2E_VSCODE_EXECUTABLE=$wrapper_path
    log "[info] using VSCodium Flatpak wrapper: $OCC_E2E_VSCODE_EXECUTABLE"
    return 0
  fi

  fail "extension target requires OCC_E2E_VSCODE_EXECUTABLE or VSCodium Flatpak com.vscodium.codium"
}

prepare_backend_requirements() {
  local backends_csv=$1
  local backend
  local needs_llama=0
  local items=()

  occ_e2e_require_cmd git
  occ_e2e_require_cmd curl

  IFS=',' read -r -a items <<<"$backends_csv"
  for backend in "${items[@]}"; do
    case "$backend" in
      codex)
        occ_e2e_require_cmd codex
        export OCC_E2E_CODEX_PATH=${OCC_E2E_CODEX_PATH:-$(command -v codex)}
        ;;
      opencode)
        occ_e2e_require_cmd opencode
        export OCC_E2E_OPENCODE_PATH=${OCC_E2E_OPENCODE_PATH:-$(command -v opencode)}
        ;;
      claude)
        occ_e2e_require_cmd claude
        export OCC_E2E_CLAUDE_PATH=${OCC_E2E_CLAUDE_PATH:-$(command -v claude)}
        ;;
      gemini)
        occ_e2e_require_cmd gemini
        export OCC_E2E_GEMINI_PATH=${OCC_E2E_GEMINI_PATH:-$(command -v gemini)}
        ;;
      openai-api)
        occ_e2e_require_env OPENAI_API_KEY
        ;;
      anthropic-api)
        occ_e2e_require_env ANTHROPIC_API_KEY
        ;;
      gemini-api)
        occ_e2e_require_env GEMINI_API_KEY
        ;;
      openrouter-api)
        occ_e2e_require_env OPENROUTER_API_KEY
        ;;
      opencode-api)
        occ_e2e_require_env OPENCODE_API_KEY
        ;;
      ollama-api)
        occ_e2e_require_cmd ollama
        if ! curl -sf "${OCC_E2E_OLLAMA_BASE_URL:-http://127.0.0.1:11434}/api/tags" >/dev/null 2>&1; then
          fail "ollama daemon is not responding at ${OCC_E2E_OLLAMA_BASE_URL:-http://127.0.0.1:11434}"
        fi
        ;;
      lm-studio-api|custom-api)
        occ_e2e_require_cmd "${OCC_E2E_LLAMA_BIN:-llama-server}"
        export OCC_E2E_LLAMA_BIN=${OCC_E2E_LLAMA_BIN:-$(command -v "${OCC_E2E_LLAMA_BIN:-llama-server}")}
        needs_llama=1
        ;;
    esac
  done

  export OCC_E2E_NEEDS_LLAMA=$needs_llama
}

prepare_target_requirements() {
  case "$TARGET" in
    extension)
      occ_e2e_require_cmd bun
      occ_e2e_require_cmd node
      prepare_vscodium_wrapper
      ;;
    cli|tui)
      occ_e2e_require_cmd cargo
      ;;
    both|all)
      occ_e2e_require_cmd cargo
      occ_e2e_require_cmd bun
      occ_e2e_require_cmd node
      prepare_vscodium_wrapper
      ;;
  esac
}

cleanup() {
  local status=$?
  occ_e2e_stop_llama_server >/dev/null 2>&1 || true

  if [ -n "${OCC_E2E_RESPONSE_LOG:-}" ] && [ -s "$OCC_E2E_RESPONSE_LOG" ]; then
    printf '\n===== AI RESPONSES =====\n'
    cat "$OCC_E2E_RESPONSE_LOG"
  fi

  if [ $status -eq 0 ]; then
    if [ -n "${RUN_ROOT:-}" ] && [ -d "$RUN_ROOT" ]; then
      occ_e2e_cleanup_dir "$RUN_ROOT"
    fi
    log "[done] e2e run completed successfully"
  else
    if [ -n "${RUN_ROOT:-}" ] && [ -d "$RUN_ROOT" ]; then
      log "[info] preserved temp files at $RUN_ROOT"
    fi
    log "[fail] e2e run exited with status $status"
  fi
  if [ -n "${LOG_FILE:-}" ]; then
    log "[info] log file: $LOG_FILE"
  fi
}

BACKENDS=all
TARGET=both
SUITE=full
LOG_FILE=

while [ $# -gt 0 ]; do
  case "$1" in
    -b|--backends)
      [ $# -ge 2 ] || fail "--backends requires a value"
      BACKENDS=$2
      shift 2
      ;;
    --backends=*)
      BACKENDS=${1#*=}
      shift
      ;;
    -t|--target)
      [ $# -ge 2 ] || fail "--target requires a value"
      TARGET=${2,,}
      shift 2
      ;;
    --target=*)
      TARGET=${1#*=}
      TARGET=${TARGET,,}
      shift
      ;;
    -s|--suite)
      [ $# -ge 2 ] || fail "--suite requires a value"
      SUITE=${2,,}
      shift 2
      ;;
    --suite=*)
      SUITE=${1#*=}
      SUITE=${SUITE,,}
      shift
      ;;
    -l|--log-file)
      [ $# -ge 2 ] || fail "--log-file requires a value"
      LOG_FILE=$2
      shift 2
      ;;
    --log-file=*)
      LOG_FILE=${1#*=}
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      usage >&2
      fail "unknown option: $1"
      ;;
  esac
done

case "$TARGET" in
  extension|cli|tui|both|all)
    ;;
  *)
    fail "unknown target: $TARGET"
    ;;
esac

case "$SUITE" in
  full|artifacts)
    ;;
  *)
    fail "unknown suite: $SUITE"
    ;;
esac

BACKENDS=$(occ_e2e_normalize_backends "$BACKENDS")
MODE=$(occ_e2e_mode_for_backends "$BACKENDS")
STAMP=$(date '+%Y%m%d-%H%M%S')
LOG_FILE=${LOG_FILE:-$REPO_ROOT/.logs/e2e-$STAMP.log}
mkdir -p "$(dirname -- "$LOG_FILE")"
LOG_FILE=$(cd -- "$(dirname -- "$LOG_FILE")" && pwd)/$(basename -- "$LOG_FILE")
: > "$LOG_FILE"

mkdir -p "$REPO_ROOT/.tmp"
RUN_ROOT=$(mktemp -d "$REPO_ROOT/.tmp/occ-e2e.XXXXXX")
trap cleanup EXIT

exec > >(tee -a "$LOG_FILE") 2>&1

log "[info] repo root: $REPO_ROOT"
log "[info] target: $TARGET"
log "[info] suite: $SUITE"
log "[info] backends: $BACKENDS"
log "[info] mode: $MODE"

prepare_backend_requirements "$BACKENDS"
prepare_target_requirements

export OCC_E2E_MODE=$MODE
export OCC_E2E_ACTIVE_BACKENDS=$BACKENDS
export OCC_E2E_SUITE=$SUITE
export OCC_E2E_RESPONSE_LOG=$RUN_ROOT/ai-responses.log
export RUST_TEST_THREADS=${RUST_TEST_THREADS:-1}

if [ "${OCC_E2E_NEEDS_LLAMA:-0}" = "1" ]; then
  export OCC_E2E_LLAMA_LOG=$RUN_ROOT/llama-server.log
  run_step "start llama.cpp test server" occ_e2e_start_llama_server
fi

run_cli_suite() {
  local config_path=$RUN_ROOT/cli-config.toml
  export OCC_E2E_CONFIG_PATH=$config_path
  export OPENCODECOMMIT_CONFIG=$config_path
  occ_e2e_render_config_for_backends "$BACKENDS" "$config_path"

  if [ "$SUITE" = "artifacts" ]; then
    run_step "run cli artifact e2e" cargo test --test e2e_cli artifacts_ -- --nocapture
  else
    run_step "run cli e2e" cargo test --test e2e_cli -- --nocapture
  fi
}

run_extension_suite() {
  local config_path=$RUN_ROOT/extension-config.toml
  export OCC_E2E_CONFIG_PATH=$config_path
  export OCC_E2E_WORK_ROOT=$RUN_ROOT/extension-host
  export OPENCODECOMMIT_CONFIG=$config_path
  occ_e2e_render_config_for_backends "$BACKENDS" "$config_path"
  run_step "build extension test bundle" bun run build
  if [ "$SUITE" = "artifacts" ]; then
    run_step "run extension artifact e2e via VSCodium" node extension/out/test/e2e/runTest.js
  else
    run_step "run extension e2e via VSCodium" node extension/out/test/e2e/runTest.js
  fi
}

run_tui_suite() {
  local config_path=$RUN_ROOT/tui-config.toml
  export OCC_E2E_CONFIG_PATH=$config_path
  export OPENCODECOMMIT_CONFIG=$config_path
  occ_e2e_render_config_for_backends "$BACKENDS" "$config_path"

  if [ "$MODE" != "staging" ] && [ "${BACKENDS#*,}" = "$BACKENDS" ]; then
    export OCC_E2E_TUI_BACKEND_OVERRIDE=$BACKENDS
  else
    unset OCC_E2E_TUI_BACKEND_OVERRIDE || true
  fi

  if [ "$SUITE" = "artifacts" ]; then
    export OCC_E2E_TEST_CASE=tui_artifacts
    run_step "run tui artifact e2e" cargo test --test e2e_tui artifacts_ -- --nocapture
  elif [ "$MODE" != "staging" ] && [ "${BACKENDS#*,}" = "$BACKENDS" ]; then
    export OCC_E2E_TEST_CASE=tui_targeted_single_backend_smoke
    run_step \
      "run tui e2e smoke" \
      cargo test --test e2e_tui tui_targeted_single_backend_smoke -- --nocapture
  else
    unset OCC_E2E_TEST_CASE || true
    run_step "run tui e2e" cargo test --test e2e_tui -- --nocapture
  fi

  unset OCC_E2E_TEST_CASE || true
}

case "$TARGET" in
  extension)
    run_extension_suite
    ;;
  cli)
    run_cli_suite
    ;;
  tui)
    run_tui_suite
    ;;
  both)
    run_tui_suite
    run_extension_suite
    ;;
  all)
    run_cli_suite
    run_tui_suite
    run_extension_suite
    ;;
esac
