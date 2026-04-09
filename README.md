# OpenCodeCommit

AI commit, branch, PR, and changelog generation through terminal AI CLIs and direct provider APIs.

OpenCodeCommit works as:
- a VS Code / VSCodium extension
- a Rust / npm CLI (`occ`)
- a terminal TUI (`occ tui`)
- a standalone CI/CD scanner in cloud (`occ scan`)

Before any prompt leaves your machine, OpenCodeCommit scans the diff locally for secrets, credential files, source maps, private keys, and other sensitive artifacts.

- <a href="https://open-vsx.org/extension/Nevaberry/opencodecommit"><img src="https://raw.githubusercontent.com/Nevaberry/opencodecommit/HEAD/.github/icons/openvsx.png" width="14"> Open VSX</a>
- <a href="https://marketplace.visualstudio.com/items?itemName=Nevaberry.opencodecommit"><img src="https://raw.githubusercontent.com/Nevaberry/opencodecommit/HEAD/.github/icons/vscode.png" width="14"> VS Code Marketplace</a>
- <a href="https://www.npmjs.com/package/opencodecommit"><img src="https://raw.githubusercontent.com/Nevaberry/opencodecommit/HEAD/.github/icons/npm.png" width="14"> npm</a>
- <a href="https://crates.io/crates/opencodecommit"><img src="https://raw.githubusercontent.com/Nevaberry/opencodecommit/HEAD/.github/icons/crates.png" width="14"> crates.io</a>
- <a href="https://github.com/Nevaberry/opencodecommit"><img src="https://raw.githubusercontent.com/Nevaberry/opencodecommit/HEAD/.github/icons/github.png" width="14"> GitHub</a>

## Install

Extension:
- Search for `OpenCodeCommit` in VS Code or VSCodium marketplace

CLI:
- `cargo install opencodecommit`
- `npm i -g opencodecommit`

Optional CLI backends:
- `npm i -g @openai/codex`
- `npm i -g opencode`
- `npm i -g @anthropic-ai/claude-code`
- `npm i -g @google/gemini-cli`

Direct API backends:
- OpenAI
- Anthropic
- Google Gemini
- OpenRouter
- OpenCode Zen
- Ollama
- LM Studio
- Custom OpenAI-compatible endpoints

Hosted API backends use API keys from environment variables. Ollama and LM Studio can auto-detect the lexicographically first available model when their `model` field is left empty.

## Highlights

- Mixed fallback chains across CLI and API backends from the same `backend` / `backend-order` config.
- Commit, PR, branch, and changelog generation from the CLI, TUI, and extension with the same config surface.
- `occ scan` for CI/CD with `text`, `json`, `sarif`, and `github-annotations` output modes.
- Built-in languages: English, Finnish, Japanese, Chinese, Spanish, Portuguese, French, Korean, Russian, Vietnamese, and German.
- Terminal TUI with one-shot backend picks and a file sidebar that stages or unstages the selected file with `Space`.
- Transparent git guard for normal `git commit` flows.

## Quick Start

Extension:
1. Open Source Control.
2. Click the sparkle action.
3. Use the dropdown for refine, branch, PR, language, backend, or diagnose actions.

CLI:

```bash
occ tui
occ commit
occ commit --backend openai-api --dry-run --text
occ commit --backend gemini --dry-run --text
occ branch --dry-run
occ pr --backend openrouter-api --text
occ changelog --text
occ scan --format text
occ scan --format sarif --output occ-scan.sarif
occ guard install --global
occ update
```

## Security Scanner

The local scanner checks for:
- provider tokens and webhook URLs for OpenAI, Anthropic, GitHub, GitLab, AWS, Slack, Stripe, SendGrid, npm, PyPI, Docker, Vault, Discord, Teams, and more
- bearer tokens, JWTs, Docker auth blobs, kube auth fields, and credential-bearing connection strings
- `.env*`, `.npmrc`, `.git-credentials`, `.kube/config`, Terraform state and vars, service-account JSON, key stores, SSH keys, and private key material
- exposed source maps such as `*.js.map` and `*.css.map`

Enforcement modes:
- `warn`
- `block-high`
- `block-all`
- `strict-high`
- `strict-all`

`occ scan` reuses the same scanner outside the AI flow. It accepts git diff input, `--stdin`, or `--diff-file`, returns `0` when the selected enforcement allows the diff, and returns `2` when blocking findings remain.

Use `occ guard profile human` for warnings-first local use, or `occ guard profile strict-agent` when you want non-bypassable blocking behavior for autonomous tooling.

See [SENSITIVE.md](SENSITIVE.md) for the full scanning flow and [PROCESS.md](PROCESS.md) for how it fits into generation and CI/CD.

## Config

`~/.config/opencodecommit/config.toml` is the single source of truth for both CLI and extension.
On first use, OpenCodeCommit writes the full default config there so every setting is visible in one file.
VS Code / VSCodium settings under `opencodecommit.*` are synced bidirectionally with the file.

Override the path with the `OPENCODECOMMIT_CONFIG` environment variable.

Useful settings:
- `backend`
- `backend-order`
- `commit-mode`
- `branch-mode`
- `diff-source`
- `active-language`
- `commit-template`
- `sensitive.enforcement`
- `sensitive.allowlist`
- `api.openai`
- `api.anthropic`
- `api.gemini`
- `api.openrouter`
- `api.opencode`
- `api.ollama`
- `api.lm-studio`
- `api.custom`

Example:

```toml
backend = "openai-api"
backend-order = ["claude", "openai-api", "ollama-api"]

[api.openai]
model = "gpt-5.4-mini"
endpoint = "https://api.openai.com/v1/chat/completions"
key-env = "OPENAI_API_KEY"
pr-model = "gpt-5.4"
cheap-model = "gpt-5.4-mini"

[api.ollama]
model = ""
endpoint = "http://localhost:11434"
key-env = ""
```

## CI/CD

- GitHub Action: [`action.yml`](action.yml)
- Examples: [`examples/ci/github-actions.yml`](examples/ci/github-actions.yml), [`examples/ci/azure-pipelines.yml`](examples/ci/azure-pipelines.yml), [`examples/ci/gitlab-ci.yml`](examples/ci/gitlab-ci.yml)

The composite action installs the published `opencodecommit` package, runs `occ scan`, can upload SARIF to GitHub code scanning, emits GitHub annotations, and supports a workflow-level manual override that preserves reports without hiding findings.

## License

[MIT](LICENSE)
