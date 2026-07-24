# Contributing

## Prerequisites

- [Rust](https://rustup.rs/) stable (minimum supported version: 1.97)
- [Node.js](https://nodejs.org/) 24 LTS
- [Bun](https://bun.sh/)
- VS Code or VSCodium for extension testing

## Setup

```sh
bun install                      # root deps (biome, test tools)
cd extension && bun install      # extension deps
```

Bun filters newly resolved direct and transitive packages until they are at
least 48 hours old. Dependabot proposes eligible Bun, Cargo, and GitHub Actions
updates against `development`; CI repeats the registry-age check for every
lockfile change.

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

## Branches and releases

The permanent branches have separate responsibilities:

- `development` is the integration branch for regular feature and fix work.
- `main` is the default branch and contains tested, release-ready work.
- `production` is the deployment branch and should receive only release promotions from `main`.

Promote the same commit through the branches:

```text
development -> main -> production -> npm / crates.io / VS Code Marketplace / Open VSX / GitHub Releases
```

Any push to `production` starts the production release workflow. This includes a merge commit, fast-forward merge, squash merge, rebase-and-merge, or direct commit. No tag or manual workflow dispatch is required.

Before promoting to `production`, update `CHANGELOG.md` and synchronize a new version across the manifests as described below. Package registries reject attempts to publish a version that already exists.

Repository maintainers must set `main` as the default branch in GitHub and configure branch protection or rulesets for the three permanent branches. The default-branch setting and protection rules are repository settings, so they cannot be declared by these workflow files.

## Version sync

```sh
scripts/sync-version.sh X.Y.Z   # set version across all manifests
```

## Publish (maintainer)

```sh
scripts/publish.sh --all         # extension + npm + crates.io
```

Requires `.ovsx-pat` and `.vsce-pat` token files in repo root.
