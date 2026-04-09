# Changelog

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
