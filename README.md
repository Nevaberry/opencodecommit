# OpenCodeCommit

AI commit, branch, PR, and changelog generation through terminal AI CLIs.

OpenCodeCommit works as:
- a Rust / npm CLI (`occ`)
- a terminal TUI
- a VS Code / VSCodium extension

Before any prompt leaves your machine, OpenCodeCommit scans the diff locally for secrets, credential files, source maps, private keys, and other sensitive artifacts.

[Open VSX](https://open-vsx.org/extension/Nevaberry/opencodecommit) · [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=Nevaberry.opencodecommit) · [npm](https://www.npmjs.com/package/opencodecommit) · [scoped npm alias](https://www.npmjs.com/package/@nevaberry/opencodecommit) · [crates.io](https://crates.io/crates/opencodecommit) · [GitHub](https://github.com/Nevaberry/opencodecommit)

## Install

Extension:
- Search for `OpenCodeCommit` in VS Code or VSCodium
- Or run `ext install Nevaberry.opencodecommit`

CLI:
- `cargo install opencodecommit`
- `npm i -g opencodecommit`

Backends:
- `npm i -g @openai/codex`
- `npm i -g opencode`
- `npm i -g @anthropic-ai/claude-code`
- `npm i -g @google/gemini-cli`

## Highlights

- Backend fallback across Codex, OpenCode, Claude Code, and Gemini, plus one-shot backend picks in the TUI and extension.
- Commit, PR, branch, and changelog generation from the same config surface.
- Built-in languages: English, Finnish, Japanese, Chinese, Spanish, Portuguese, French, Korean, Russian, Vietnamese, and German.
- Terminal TUI with a file sidebar that merges staged, unstaged, and untracked files and lets you stage or unstage the selected file with `Space`.
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
occ commit --backend gemini --dry-run --text
occ commit --language Japanese
occ branch --dry-run
occ pr --text
occ changelog --text
occ guard install --global
occ update
```

## Security Scanner

The local scanner now checks for:
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

Use `occ guard profile human` for warnings-first local use, or `occ guard profile strict-agent` when you want non-bypassable blocking behavior for autonomous tooling.

## Config

VS Code / VSCodium settings live under `opencodecommit.*`.

CLI config lives at `~/.config/opencodecommit/config.toml`.
On first CLI use, OpenCodeCommit writes the full default config there so the available settings are visible in one file.

Useful settings:
- `backendOrder`
- `commitMode`
- `branchMode`
- `diffSource`
- `activeLanguage`
- `sensitive.enforcement`
- `sensitive.allowlist`

## License

[MIT](LICENSE)
