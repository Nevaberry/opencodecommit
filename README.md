# OpenCodeCommit

AI commit messages should not require copy-pasting diffs into chat windows, leaking secrets by accident, or fighting a different tool for every backend.

OpenCodeCommit gives you one local workflow for commit messages, branch names, pull request drafts, changelog entries, and CI secret scanning. It runs in VS Code / VSCodium, as the `occ` CLI, as a terminal TUI, and as a GitHub Action.

Before a prompt is sent to any AI backend, OpenCodeCommit scans the diff locally for secrets, credential files, private keys, source maps, and other sensitive artifacts.

- <a href="https://open-vsx.org/extension/Nevaberry/opencodecommit"><img src="https://raw.githubusercontent.com/Nevaberry/opencodecommit/HEAD/.github/icons/openvsx.png" width="14"> Open VSX</a>
- <a href="https://marketplace.visualstudio.com/items?itemName=Nevaberry.opencodecommit"><img src="https://raw.githubusercontent.com/Nevaberry/opencodecommit/HEAD/.github/icons/vscode.png" width="14"> VS Code Marketplace</a>
- <a href="https://www.npmjs.com/package/opencodecommit"><img src="https://raw.githubusercontent.com/Nevaberry/opencodecommit/HEAD/.github/icons/npm.png" width="14"> npm</a>
- <a href="https://crates.io/crates/opencodecommit"><img src="https://raw.githubusercontent.com/Nevaberry/opencodecommit/HEAD/.github/icons/crates.png" width="14"> crates.io</a>
- <a href="https://github.com/Nevaberry/opencodecommit"><img src="https://raw.githubusercontent.com/Nevaberry/opencodecommit/HEAD/.github/icons/github.png" width="14"> GitHub</a>

## Why It Exists

Good commit history is useful only if writing it is cheap enough to do every time.

OpenCodeCommit is for teams and solo developers who want:
- specific commit messages that match the repository's recent style
- PR drafts and changelog entries without another browser round trip
- local-first safety checks before any diff reaches an AI provider
- one config shared by the extension, CLI, TUI, and CI scanner
- fallback across Codex, OpenCode, Claude, Gemini, hosted APIs, and local OpenAI-compatible endpoints

## Install

Extension:
- Search for `OpenCodeCommit` in VS Code or VSCodium

CLI:

```bash
cargo install opencodecommit
# or
npm i -g opencodecommit
```

Optional CLI backends:

```bash
npm i -g @openai/codex
npm i -g opencode
npm i -g @anthropic-ai/claude-code
npm i -g @google/gemini-cli
```

Direct API backends are also supported for OpenAI, Anthropic, Google Gemini, OpenRouter, OpenCode Zen, Ollama, LM Studio, and custom OpenAI-compatible endpoints.

## Use It

VS Code / VSCodium:
1. Open Source Control.
2. Click the sparkle action.
3. Use the `occ` menu for refine, branch, PR, language, backend, config, and diagnose actions.

Terminal:

```bash
occ tui
occ commit
occ commit --backend codex --dry-run --text
occ branch --dry-run
occ pr --backend openrouter-api --text
occ changelog --text
```

CI and local scanning:

```bash
occ scan --format text
occ scan --format sarif --output occ-scan.sarif
occ guard install --global
```

## What You Get

- Commit generation that can adapt to recent commit style or force conventional commits.
- Branch names, PR drafts, and changelog entries from the same context pipeline.
- A terminal TUI with backend picks, diff view, output panels, and file staging.
- Local sensitive-content scanning with `warn`, `block-*`, and `strict-*` enforcement modes.
- CI output as text, JSON, SARIF, or GitHub annotations.
- Built-in language templates for English, Finnish, Japanese, Chinese, Spanish, Portuguese, French, Korean, Russian, Vietnamese, and German.

## Privacy And Security

OpenCodeCommit has no hosted service and no telemetry. Diffs and file context are processed locally first, then sent only to the backend you configure.

The scanner can block provider tokens, webhooks, credential-bearing connection strings, `.env*` files, key stores, private keys, source maps, and other high-risk artifacts before generation runs.

See [SECURITY.md](SECURITY.md) for vulnerability reporting and data-flow details.

## Configuration

`~/.config/opencodecommit/config.toml` is the shared config for the CLI, TUI, and extension. The extension syncs VS Code / VSCodium settings with that file.

Override the path with `OPENCODECOMMIT_CONFIG`.

Start here:
- [Backends](docs/backends.md)
- [Configuration](docs/config.md)
- [CI scanning](docs/ci-scan.md)
- [VS Code and VSCodium](docs/vscode-vscodium.md)
- [Sensitive scanning flow](docs/sensitive-scanning.md)
- [Process flow](docs/process-flow.md)
- [Architecture](docs/architecture.md)
- [Roadmap](docs/roadmap.md)

## CI/CD

Use the bundled GitHub Action:

```yaml
- uses: Nevaberry/opencodecommit@v1
  with:
    enforcement: block-high
    upload-sarif: true
```

Examples are available for [GitHub Actions](examples/ci/github-actions.yml), [Azure Pipelines](examples/ci/azure-pipelines.yml), and [GitLab CI](examples/ci/gitlab-ci.yml).

## Contributing

OpenCodeCommit intentionally tests against both deterministic unit paths and real AI backends. See [CONTRIBUTING.md](CONTRIBUTING.md) for setup, verification, and live E2E commands.

## License

[MIT](LICENSE)
