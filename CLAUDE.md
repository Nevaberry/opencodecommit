# CLAUDE.md

## What is this

OpenCodeCommit (`occ`) -- AI-powered git commit, branch, PR, and changelog generator. Ships as a **Rust CLI/TUI** and a **VS Code extension (TypeScript)**.

## CRITICAL: Dual implementation -- keep Rust and TypeScript in sync

The extension (`extension/src/inline/`) mirrors the Rust library. **Every change to prompts, config schema, security patterns, language templates, or core logic must be applied to both Rust and TypeScript.** Also update both `package.json` files (root and `extension/`). Forgetting one side is the #1 source of bugs in this repo.

Mapping: `languages.rs` <-> `package.json` language defaults, `config.rs` <-> `config-schema.ts`, `sensitive.rs` <-> `sensitive.ts`, `prompt.rs` <-> `generator.ts`, `context.rs` <-> `context.ts`, `backend.rs` <-> `backends.ts`, `api/` <-> `api.ts`.

## Build & test

```sh
# Rust
cargo build && cargo test --workspace        # build + all tests
cargo fmt && cargo clippy                     # format + lint
cargo run --bin occ -- tui                    # run TUI
cargo run --bin occ -- commit --dry-run --text

# TypeScript (extension)
bun install && cd extension && bun install    # first-time
bun run build                                 # compile
bunx tsc -p extension/tsconfig.json --noEmit  # type check
bun test extension/src/test/                  # unit tests
bun run lint                                  # biome check

# E2E
scripts/test-e2e.sh --target cli|tui|extension

# CI equivalent
cargo test --workspace
bunx tsc -p extension/tsconfig.json --noEmit
bun test extension/src/test/inline.test.ts extension/src/test/config.test.ts extension/src/test/api.test.ts
```


## Architecture

```
Rust — crates/opencodecommit/src/
  main.rs (CLI), tui.rs (TUI), lib.rs
  core: config, languages, prompt, context, sensitive, scan,
        dispatch, backend, api/, response, git, actions,
        guard, update, codex_home

TypeScript — extension/src/
  extension.ts (VS Code entry)
  inline/  mirrors Rust core (see mapping above)
  other:   cli.ts, pr.ts, changelog.ts, host-io.ts, types.ts
```

## Key rules

- Security scanning always runs before sending diffs to AI (sensitive.rs / sensitive.ts, 60+ regex patterns, not skippable)
- Backend fallback chains: `backend-order` in config.toml, CLI backends + direct API backends
- Config bidirectionally synced between `config.toml` and VS Code `settings.json` (`opencodecommit.*` namespace)
