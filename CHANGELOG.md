# Changelog

## 1.10

- Moves local and CI Rust builds to the rolling stable toolchain while declaring
  Rust 1.97 as the minimum supported compiler.
- Moves GitHub Actions, package tooling, and examples to Node.js 24 LTS.
- Refreshes the Rust, TypeScript, test, packaging, and GitHub Actions dependency
  stacks to their latest stable releases.
- Automates dependency update PRs with a 48-hour release cooldown and enforces
  the same package-age policy in Bun, CI, and production releases.
- Migrates the default Claude model from Opus 4.8 to Opus 5 across the CLI,
  extension, and Assisted-by catalog.

## 1.9

- Adds optional `occ evidence` mode with `samd` and `defence` profiles for
  sidecar audit trails, sensitive-term controls, and `Assisted-by` attribution.
- Moves the Codex and Claude `Assisted-by` quick actions into the main Source
  Control menu.
- Introduces the `development` -> `main` -> `production` promotion flow, with
  marketplace deployment triggered by every push to `production`.

## 1.8

- Replaced hook/global guard flows with a repo-local `occ guard install` workflow
  that controls raw `git commit`, preserves explicit manual commits once, and
  updates the TUI and VS Code extension.

## 1.7

- Made Codex commit and branch generation faster and more reliable with
  prompt-only execution, minimal `CODEX_HOME`, native binary detection, and
  structured output fallback.

## 1.6

- Added the occ-managed minimal Codex home, disabled MCP loading for managed
  prompt tasks, tightened scoped conventional prompts, and expanded release QA.

## 1.4

- Made `config.toml` the shared CLI/TUI/extension source of truth with
  `OPENCODECOMMIT_CONFIG` path overrides.

## 1.3

- Added more languages, backend selection, sensitive guard profiles, and a more
  capable TUI file/diff workflow.

## 1.2

- Added two-stage PR generation, `occ update` / `occ upgrade`, and backend
  fallback reporting.

## 1.1

- Added the Ratatui TUI, the first commit guard, structured sensitive reporting,
  and the single-screen commit view.

## 1.0

- Shipped the `occ` CLI rename, publishing pipeline, and configurable language
  prompt modules.

## 0.9

- Added secret scanning for commit generation and configurable branch naming.

## 0.8

- Rebranded to `opencodecommit` and added editable language prompts, Gemini,
  backend order, language selection, and Diagnose.
