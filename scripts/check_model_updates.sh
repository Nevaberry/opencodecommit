#!/usr/bin/env bash
# check_model_updates.sh — detect when installed CLIs know about newer models
# than our hardcoded defaults.
#
# Exit codes:
#   0 — defaults are current (no output)
#   1 — updates available (outputs change details)
#   2 — error (missing dependency, etc.)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEFAULTS_FILE="$SCRIPT_DIR/../crates/opencodecommit/model-catalog.json"

if [[ ! -f "$DEFAULTS_FILE" ]]; then
    echo "ERROR: model-catalog.json not found at $DEFAULTS_FILE" >&2
    exit 2
fi

if ! command -v jq &>/dev/null; then
    echo "ERROR: jq is required but not installed" >&2
    exit 2
fi

# Read a default value from model-catalog.json
get_default() {
    local backend="$1" tier="$2"
    jq -r --arg b "$backend" --arg t "$tier" '.backendDefaults[$b][$t] // ""' "$DEFAULTS_FILE"
}

changes=()
cli_versions=()

# ── Codex CLI ──────────────────────────────────────────────────────────────────

detect_codex_models() {
    local cache="$HOME/.codex/models_cache.json"

    if ! command -v codex &>/dev/null; then
        echo "SKIP: codex CLI not found in PATH" >&2
        return
    fi

    local version
    version=$(codex --version 2>/dev/null | grep -oP '\d+\.\d+\.\d+' | head -1 || echo "unknown")
    cli_versions+=("codex=$version")

    if [[ ! -f "$cache" ]]; then
        echo "SKIP: codex models cache not found at $cache" >&2
        return
    fi

    # PR model: lowest priority number among visible models
    local detected_pr
    detected_pr=$(jq -r '
        .models
        | map(select(.visibility == "list"))
        | sort_by(.priority)
        | .[0].slug // ""
    ' "$cache")

    # Commit/Cheap model: lowest priority visible -mini model
    local detected_commit
    detected_commit=$(jq -r '
        .models
        | map(select(.visibility == "list" and (.slug | endswith("-mini"))))
        | sort_by(.priority)
        | .[0].slug // ""
    ' "$cache")

    local cur_pr cur_commit cur_cheap
    cur_pr=$(get_default codex pr_model)
    cur_commit=$(get_default codex commit_model)
    cur_cheap=$(get_default codex cheap_model)

    if [[ -n "$detected_pr" && "$detected_pr" != "$cur_pr" ]]; then
        changes+=("CODEX_PR_MODEL: $cur_pr -> $detected_pr")
    fi
    if [[ -n "$detected_commit" && "$detected_commit" != "$cur_commit" ]]; then
        changes+=("CODEX_COMMIT_MODEL: $cur_commit -> $detected_commit")
    fi
    if [[ -n "$detected_commit" && "$detected_commit" != "$cur_cheap" ]]; then
        changes+=("CODEX_CHEAP_MODEL: $cur_cheap -> $detected_commit")
    fi

    # OpenCode mirrors Codex defaults
    local oc_pr oc_commit oc_cheap
    oc_pr=$(get_default opencode pr_model)
    oc_commit=$(get_default opencode commit_model)
    oc_cheap=$(get_default opencode cheap_model)

    if [[ -n "$detected_pr" && "$detected_pr" != "$oc_pr" ]]; then
        changes+=("OPENCODE_PR_MODEL: $oc_pr -> $detected_pr")
    fi
    if [[ -n "$detected_commit" && "$detected_commit" != "$oc_commit" ]]; then
        changes+=("OPENCODE_COMMIT_MODEL: $oc_commit -> $detected_commit")
    fi
    if [[ -n "$detected_commit" && "$detected_commit" != "$oc_cheap" ]]; then
        changes+=("OPENCODE_CHEAP_MODEL: $oc_cheap -> $detected_commit")
    fi
}

# ── Claude Code ────────────────────────────────────────────────────────────────

detect_claude_models() {
    if ! command -v claude &>/dev/null; then
        echo "SKIP: claude CLI not found in PATH" >&2
        return
    fi

    local version
    version=$(claude --version 2>/dev/null | grep -oP '\d+\.\d+\.\d+' | head -1 || echo "unknown")
    cli_versions+=("claude=$version")

    local claude_bin
    claude_bin=$(which claude | xargs readlink -f 2>/dev/null || which claude)

    if ! command -v strings &>/dev/null; then
        echo "SKIP: strings command not available (install binutils)" >&2
        return
    fi

    # Extract all claude model identifiers from the binary
    local all_models
    all_models=$(strings "$claude_bin" | grep -oP 'claude-(sonnet|opus|haiku)-\d+(-\d+)*' | sort -u || true)

    if [[ -z "$all_models" ]]; then
        echo "SKIP: no claude model strings found in binary" >&2
        return
    fi

    # Pick the latest model for a given family.
    # Prefer short aliases (e.g. claude-opus-4-6) over dated ones (claude-opus-4-6-20260101).
    # Sort by version numbers descending.
    pick_latest() {
        local family="$1"
        local family_models
        family_models=$(echo "$all_models" | grep "^claude-${family}-" || true)
        if [[ -z "$family_models" ]]; then
            return
        fi

        # Separate short aliases (2-4 version segments) from dated ones (8-digit suffix)
        local short dated
        short=$(echo "$family_models" | grep -vP '\d{8}' || true)
        dated=$(echo "$family_models" | grep -P '\d{8}' || true)

        local candidates
        if [[ -n "$short" ]]; then
            candidates="$short"
        else
            candidates="$dated"
        fi

        # Sort by version segments descending and take the first
        echo "$candidates" | sort -t'-' -k3,3nr -k4,4nr -k5,5nr | head -1
    }

    local detected_pr detected_commit detected_cheap
    detected_pr=$(pick_latest opus)
    detected_commit=$(pick_latest sonnet)
    detected_cheap=$(pick_latest haiku)

    local cur_pr cur_commit cur_cheap
    cur_pr=$(get_default claude pr_model)
    cur_commit=$(get_default claude commit_model)
    cur_cheap=$(get_default claude cheap_model)

    if [[ -n "$detected_pr" && "$detected_pr" != "$cur_pr" ]]; then
        changes+=("CLAUDE_PR_MODEL: $cur_pr -> $detected_pr")
    fi
    if [[ -n "$detected_commit" && "$detected_commit" != "$cur_commit" ]]; then
        changes+=("CLAUDE_COMMIT_MODEL: $cur_commit -> $detected_commit")
    fi
    if [[ -n "$detected_cheap" && "$detected_cheap" != "$cur_cheap" ]]; then
        changes+=("CLAUDE_CHEAP_MODEL: $cur_cheap -> $detected_cheap")
    fi
}

# ── Antigravity CLI ────────────────────────────────────────────────────────────

detect_agy_models() {
    if ! command -v agy &>/dev/null; then
        echo "SKIP: Antigravity CLI (agy) not found in PATH" >&2
        return
    fi

    local version
    version=$(agy --version 2>/dev/null | grep -oP '\d+\.\d+\.\d+' | head -1 || echo "unknown")
    cli_versions+=("agy=$version")

    local all_models
    all_models=$(agy models 2>/dev/null || true)
    if [[ -z "$all_models" ]]; then
        echo "SKIP: agy returned no available models" >&2
        return
    fi

    local detected_pr detected_commit detected_cheap
    detected_pr=$(echo "$all_models" | grep -E 'Pro.*\(High\)$' | head -1 || true)
    detected_commit=$(echo "$all_models" | grep -E 'Flash.*\(Low\)$' | head -1 || true)
    detected_cheap=$detected_commit

    local cur_pr cur_commit cur_cheap
    cur_pr=$(get_default agy pr_model)
    cur_commit=$(get_default agy commit_model)
    cur_cheap=$(get_default agy cheap_model)

    if [[ -n "$detected_pr" && "$detected_pr" != "$cur_pr" ]]; then
        changes+=("AGY_PR_MODEL: $cur_pr -> $detected_pr")
    fi
    if [[ -n "$detected_commit" && "$detected_commit" != "$cur_commit" ]]; then
        changes+=("AGY_COMMIT_MODEL: $cur_commit -> $detected_commit")
    fi
    if [[ -n "$detected_cheap" && "$detected_cheap" != "$cur_cheap" ]]; then
        changes+=("AGY_CHEAP_MODEL: $cur_cheap -> $detected_cheap")
    fi
}

# ── Main ───────────────────────────────────────────────────────────────────────

detect_codex_models
detect_claude_models
detect_agy_models

if [[ ${#changes[@]} -eq 0 ]]; then
    exit 0
fi

echo "MODEL_UPDATES_AVAILABLE=true"
for change in "${changes[@]}"; do
    echo "$change"
done
if [[ ${#cli_versions[@]} -gt 0 ]]; then
    echo "CLI_VERSIONS: $(IFS=' '; echo "${cli_versions[*]}")"
fi
exit 1
