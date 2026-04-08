# Contributing

## Prerequisites

- [Bun](https://bun.sh/) (package manager + runtime)
- VS Code (`code`) or VS Codium (Flatpak: `com.vscodium.codium`)

## Setup

```sh
bun install                # root deps (biome, test tools, etc.)
cd extension && bun install  # extension deps
```

## Build & test

```sh
bun run build                            # compile TypeScript
bun test extension/src/test/inline.test.ts  # unit tests
bun run lint                             # biome check
```

## Package & install locally

```sh
cd extension && bunx @vscode/vsce package   # creates opencode-commit-<version>.vsix
```

### VS Code (RPM/native)

```sh
code --install-extension extension/opencode-commit-*.vsix
```

### VSCodium (Flatpak)

```sh
./scripts/dev-install.sh
```


```sh
flatpak run com.vscodium.codium --install-extension extension/opencode-commit-*.vsix
```

### VSCodium Insiders (Flatpak)

```sh
flatpak run com.vscodium.codium-insiders --install-extension extension/opencode-commit-*.vsix
```

## Publish (maintainer)

```sh
scripts/publish.sh   # builds, packages, publishes to Open VSX + VS Code Marketplace
```

Requires `.ovsx-pat` and `.vsce-pat` files in the repo root.
