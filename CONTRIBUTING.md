# Contributing

## Prerequisites

- [Rust](https://rustup.rs/) 1.94+
- [Bun](https://bun.sh/)
- VS Code or VSCodium for extension testing

## Setup

```sh
bun install                      # root deps (biome, test tools)
cd extension && bun install      # extension deps
```

## Build

```sh
cargo build                      # debug CLI build
cargo build --release            # release CLI build
bun run build                    # compile extension TypeScript
bun run watch                    # extension watch mode
```

## Test

```sh
cargo test --workspace
bunx tsc -p extension/tsconfig.json --noEmit
bun test extension/src/test --path-ignore-patterns='**/wdio/**'
```

OpenCodeCommit also keeps live backend E2E coverage because AI CLIs and hosted
models change frequently. Maintainers should run the relevant live suite before
release when credentials and local services are available:

```sh
scripts/test-live-backends.sh
scripts/test-e2e.sh --target cli --suite artifacts
scripts/test-e2e.sh --target tui --suite artifacts
scripts/test-e2e.sh --target extension --suite artifacts
```

The `Live AI smoke` GitHub workflow runs hosted API artifact tests on a
schedule and by manual dispatch when provider secrets are configured.

## Lint

```sh
cargo fmt && cargo clippy        # Rust
bun run lint                     # TypeScript (biome)
bun run lint:fix                 # TypeScript autofix
```

## Run locally

```sh
cargo run --bin occ -- tui       # run TUI from source
cargo run --bin occ -- commit    # run CLI commit from source
scripts/dev-cli.sh               # worktree-aware CLI runner
scripts/dev-extension.sh         # build + launch in VSCodium
scripts/dev-install.sh           # quick install into VSCodium Flatpak
```

## Version sync

```sh
scripts/sync-version.sh 1.7.0   # set version across all manifests
```

## Publish (maintainer)

```sh
scripts/publish.sh --all         # extension + npm + crates.io
```

Requires `.ovsx-pat` and `.vsce-pat` token files in repo root.
