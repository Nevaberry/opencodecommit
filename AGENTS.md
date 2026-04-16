# AGENTS.md

Instructions for all AI coding agents working on this repo.

## CRITICAL: Dual implementation -- keep Rust and TypeScript in sync

This project ships as a Rust CLI/TUI AND a VS Code extension (TypeScript). They share logic but have separate implementations. **Every change to prompts, config schema, security patterns, language templates, or core logic must be applied to both Rust and TypeScript.** Also update both `package.json` files (root and `extension/`).

File mapping:
- `languages.rs` <-> `package.json` language defaults (root + extension/)
- `config.rs` <-> `extension/src/inline/config-schema.ts`
- `sensitive.rs` <-> `extension/src/inline/sensitive.ts`
- `prompt.rs` <-> `extension/src/inline/generator.ts`
- `context.rs` <-> `extension/src/inline/context.ts`
- `backend.rs` <-> `extension/src/inline/backends.ts`
- `api/` <-> `extension/src/inline/api.ts`

## Verification

After any change, run:
```sh
cargo test --workspace
bunx tsc -p extension/tsconfig.json --noEmit
```

## Style

- Rust: `cargo fmt` style, edition 2024
- TypeScript: Biome (2-space indent, no semicolons)
- Commits: Conventional Commits with scope (`feat(scope):`, `fix(scope):`)
