#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
# shellcheck source=scripts/e2e-common.sh
source "$REPO_ROOT/scripts/e2e-common.sh"

TARGET=${1:-all}
export OCC_E2E_MODE=dev-local
export OCC_E2E_ACTIVE_BACKENDS=${OCC_E2E_ACTIVE_BACKENDS:-$(occ_e2e_default_backends_for_profile dev-local)}

"$REPO_ROOT/scripts/e2e-preflight.sh" dev-local

occ_e2e_start_llama_server
trap 'occ_e2e_stop_llama_server' EXIT

TMP_DIR=$(mktemp -d -t occ-e2e-dev.XXXXXX)
trap 'occ_e2e_stop_llama_server; occ_e2e_cleanup_dir "$TMP_DIR"' EXIT

export OCC_E2E_CONFIG_PATH="$TMP_DIR/config.toml"
export OPENCODECOMMIT_CONFIG="$OCC_E2E_CONFIG_PATH"
occ_e2e_render_config dev-local "$OCC_E2E_CONFIG_PATH"

case "$TARGET" in
  all)
    bun run build
    cargo test --test e2e_cli -- --nocapture
    cargo test --test e2e_tui -- --nocapture
    xvfb-run -a bun run test:e2e:extension
    ;;
  cli)
    cargo test --test e2e_cli -- --nocapture
    ;;
  tui)
    cargo test --test e2e_tui -- --nocapture
    ;;
  extension)
    bun run build
    xvfb-run -a bun run test:e2e:extension
    ;;
  *)
    echo "unknown target: $TARGET" >&2
    exit 1
    ;;
 esac
