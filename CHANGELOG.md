# Changelog

## 1.6.2

- Conventional commit prompts now require `type(scope): description` across
  all built-in languages, so generated messages stay scoped and consistent.
- Staging validation now runs the extension E2E suite against a packaged VSIX,
  which catches packaging regressions before publish.
- Added local QA helpers for backend-local E2E and visible TUI walkthroughs to
  tighten the final release gate before publishing.

## 1.6.0

- **Codex backend ~80 % faster via isolated `CODEX_HOME`.** Every `codex exec`
  invocation now runs against an occ-managed minimal codex home at
  `$XDG_CACHE_HOME/opencodecommit/codex-home` (fallback
  `$HOME/.cache/opencodecommit/codex-home`) so codex no longer parses the
  user's MCP registry, plugin tree, sqlite caches, and session history on
  every call. Paired bench (10 interleaved pairs, real network, real
  provider, same commit fixture): installed 1.5.2 median **8560 ms** →
  candidate 1.6.0 median **1644 ms**, 10/10 pairs favor the new path.
- Defense in depth: `-c mcp_servers={}` is now always passed to `codex exec`
  so MCP servers are never spawned, even if the `CODEX_HOME` setup falls
  back to the user's real home for any reason.
- No action required to pick up the speedup — it activates on the first
  `occ commit --backend codex` call after upgrade. If you want a clean
  1.6.0 baseline for other defaults, run the existing "OpenCodeCommit:
  Reset Settings" command in the VS Code extension.
- The auth link is a symlink to `~/.codex/auth.json`, so `codex login`
  refreshes are seen transparently. Windows users whose environment
  can't create unprivileged symlinks automatically fall back to the
  pre-1.6 codex path (no regression).

## 1.4.0

- **config.toml as single source of truth** for both CLI and VS Code extension
- `OPENCODECOMMIT_CONFIG` env var honored for config file location

## 1.3.0

- Nine new built-in languages (ja, zh, es, pt, fr, ko, ru, vi, de)
- Dedicated backend selection in TUI and extension
- Sensitive guard profiles (human / strict-agent) with tiered enforcement
- TUI: merged staged/unstaged/untracked file states, pinned action footers

## 1.2.0

- Two-stage PR generation from committed branch changes
- `occ update` / `occ upgrade` with auto-update check
- Backend fallback with progress tracking and per-backend failure reporting

## 1.1.0

- Ratatui TUI for interactive commit generation
- Global commit guard (`occ guard install --global`) blocks sensitive diffs in pre-commit
- TUI redesign: single-screen commit view, syntax-colored diff, output panels
- Structured sensitive reporting with redacted previews

## 1.0.0

- Ship the `occ` CLI (renamed from `opencodecommit`)
- CI cross-compile and auto-publish to npm, crates.io, VS Code Marketplace, Open VSX
- Configurable language prompt modules with fallback resolution

## 0.9.0

- Secret scanning -- CLI blocks secret-containing diffs unless `--allow-sensitive`
- Branch mode config (conventional vs adaptive)

## 0.8.0

Rebranded from `opencode-commit` to `opencodecommit`.

- Language-embedded prompt templates (fully editable per language)
- Gemini CLI backend support
- Backend order setting, active language dropdown, diagnose command
