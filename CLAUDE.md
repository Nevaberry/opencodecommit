# CLAUDE.md

## What is this

OpenCodeCommit (`occ`) is an AI-powered git commit message, branch name, PR, and changelog generator. It ships as three interfaces: a Rust CLI/TUI and a VS Code extension (TypeScript). All three share a unified config (`~/.config/opencodecommit/config.toml`) and security scanner.

## Build & Test Commands

### Rust

```sh
cargo build                          # debug build
cargo test --workspace               # all Rust tests (unit + integration)
cargo test -p opencodecommit <name>  # single test by name
cargo fmt && cargo clippy            # format + lint
cargo run --bin occ -- tui           # run TUI from source
cargo run --bin occ -- commit --dry-run --text  # test CLI without committing
```

Rust edition 2024, MSRV 1.94.

### TypeScript (extension)

```sh
bun install && cd extension && bun install  # first-time setup
bun run build                               # compile extension
bun run watch                               # watch mode
bunx tsc -p extension/tsconfig.json --noEmit  # type check only
bun test extension/src/test/                 # unit tests
bun run lint                                 # biome check
bun run lint:fix                             # biome autofix
```

### Full CI equivalent

```sh
cargo test --workspace
bunx tsc -p extension/tsconfig.json --noEmit
bun test extension/src/test/inline.test.ts extension/src/test/config.test.ts extension/src/test/api.test.ts
```

### E2E

```sh
scripts/test-e2e.sh --target cli      # CLI E2E (uses expectrl)
scripts/test-e2e.sh --target tui      # TUI E2E
scripts/test-e2e.sh --target extension # extension E2E (WebdriverIO)
scripts/dev-cli.sh tui                 # run TUI from worktree
scripts/dev-cli.sh -w dev commit --dry-run --text  # run CLI from specific worktree
```

### Version & Publish

```sh
scripts/sync-version.sh 1.7.0   # set version across all manifests
scripts/publish.sh --all         # extension + npm + crates.io
```

## Architecture

```
Entry points: CLI (main.rs) | TUI (tui.rs) | VS Code Extension (extension.ts)
                              ↓
              Core library (crates/opencodecommit/src/lib.rs)
              ├── context.rs    — gather diff + commits + branch + file list
              ├── sensitive.rs  — 60+ regex security patterns, runs before any diff leaves the machine
              ├── prompt.rs     — build AI prompts from context
              ├── dispatch.rs   — route to chosen backend
              ├── backend.rs    — detect & invoke CLI backends (opencode, claude, codex, gemini)
              ├── api/          — direct API calls (OpenAI, Anthropic, Google, Ollama, OpenRouter, custom)
              ├── response.rs   — parse AI output into structured commit/branch/PR
              ├── config.rs     — config.toml loading, defaults, schema
              ├── languages.rs  — 11 language prompt templates
              ├── git.rs        — git operations (diff, log, commit, branch)
              └── scan.rs       — standalone `occ scan` for CI
```

The extension (`extension/src/inline/`) mirrors the Rust library modules: `config.ts`, `sensitive.ts`, `context.ts`, `generator.ts`, `cli.ts`, `api.ts`, `backends.ts`. Changes to core logic (prompts, security patterns, config schema) typically need updates in both Rust and TypeScript.

## Key Design Decisions

- **Security scanning always runs before sending diffs to AI.** The `sensitive.rs` / `sensitive.ts` modules contain 60+ regex patterns for secrets. Enforcement is configurable (warn/block/strict) but the scan itself is not skippable in normal flows.
- **Backend fallback chains.** Users configure `backend-order` in config.toml; if the first backend fails, the next is tried. Both CLI backends (spawning `opencode`, `claude`, etc.) and API backends (direct HTTP) are supported.
- **Dual implementation.** The extension runs its own TypeScript implementation for inline mode (direct API calls) but can also shell out to the Rust CLI. Keep both implementations in sync.
- **Config is bidirectionally synced** between `config.toml` and VS Code `settings.json` under the `opencodecommit.*` namespace.

## Style

- **Rust:** standard `cargo fmt` style
- **TypeScript:** Biome — 2-space indent, no semicolons, organized imports
- Commit messages follow Conventional Commits (`feat(scope)`, `fix(scope)`, `refactor(scope)`, etc.)
