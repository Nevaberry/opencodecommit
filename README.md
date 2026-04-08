# OpenCodeCommit

AI commit messages via terminal AI agents. VSCodium / VS Code extension + standalone Rust / npm CLI.

**Security scanning built in** — diffs are scanned locally for secrets, source maps, and private keys before anything leaves your machine.

[VSCodium Open VSX registry](https://open-vsx.org/extension/Nevaberry/opencodecommit)<br>
[VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=Nevaberry.opencodecommit) · [npm](https://www.npmjs.com/package/opencodecommit) · [scoped npm](https://www.npmjs.com/package/@nevaberry/opencodecommit) · [crates.io](https://crates.io/crates/opencodecommit) · [GitHub](https://github.com/Nevaberry/opencodecommit)

## Install

**Extension:** Search "OpenCodeCommit" in VSCodium / VS Code, or `ext install Nevaberry.opencodecommit`

**CLI:** `cargo install opencodecommit` or `npm i -g opencodecommit` (official unscoped alias: `@nevaberry/opencodecommit`)

**Prerequisite:** At least one CLI backend:

| Backend | Install |
|---------|---------|
| [Codex CLI](https://github.com/openai/codex) | `npm i -g @openai/codex` |
| [OpenCode](https://github.com/nicepkg/opencode) | `npm i -g opencode` |
| [Gemini CLI](https://github.com/google-gemini/gemini-cli) | `npm i -g @google/gemini-cli` |
| [Claude Code](https://docs.anthropic.com/en/docs/claude-code) | `npm i -g @anthropic-ai/claude-code` |

## VSCodium / VS Code Usage

1. Stage changes (or leave unstaged — auto-detected)
2. Click the **sparkle button** in Source Control
3. Commit message appears in the input box

Dropdown menu: mode-specific generation, refine, branch name generation, switch language, diagnose.
Single-backend testing is available from dedicated SCM submenus for adaptive commit generation and PR generation; the main generate actions still follow fallback order.

## CLI Usage

```bash
occ tui                            # launch the minimal interactive TUI
occ commit                         # generate message + commit
occ commit --dry-run               # preview only, don't commit
occ commit --backend gemini --dry-run --text
occ commit --language Finnish      # generate in Finnish
occ commit --language Spanish      # generate in Spanish
occ commit --language Korean       # generate in Korean
occ guard profile human            # set human-friendly warning mode
occ guard profile strict-agent     # set strict agent-safe mode
occ branch                         # generate branch name + checkout
occ branch --mode adaptive         # match existing branch naming style
occ pr                             # generate PR title + body
occ pr --backend gemini --text
occ changelog                      # generate changelog entry

# JSON output (default), or --text for human readable plain text
occ commit --text
occ commit --allow-sensitive       # bypass blocking findings in non-strict modes
```

`occ tui` is a small launcher over the existing commands, not a full git dashboard. It lets you generate, shorten, and commit messages, preview branch / PR / changelog output, install the safety hook, switch between human / strict-agent sensitive profiles, and run one-shot `Commit Backend` / `PR Backend` actions without changing the default backend.

`occ` is the short form. `opencodecommit` also works if `occ` clashes with something on your system.

Exit codes: 0 success, 1 no changes, 2 backend error, 3 config error, 5 sensitive content detected

## Transparent Git Guard

Use OpenCodeCommit as a background safety layer for normal `git commit` usage:

```bash
occ guard install --global         # install a machine-wide commit guard
occ guard uninstall --global       # remove the machine-wide guard
occ guard profile human            # warn by default, tuned for humans
occ guard profile strict-agent     # strict-all, no bypass
```

This installs a managed global hooks directory via `core.hooksPath`. `pre-commit` scans the staged diff for sensitive content, and other hook names are chained through so existing repo hooks still run.

## Sensitive Content Detection

Diffs are scanned locally before being sent to any AI backend. Findings are classified as:

- `confirmed-secret`: real provider tokens, private keys, credential-bearing URLs, webhook secrets
- `sensitive-artifact`: `.env`, kubeconfig, Terraform state, credential stores, key containers
- `suspicious`: generic assignments, local connection strings, public IPv4s, source maps, docs/examples with weaker evidence

Enforcement modes:

- `warn`: default. Show the report, but continue after acknowledgement.
- `block-high`: block only high-confidence findings, allow a one-shot bypass.
- `block-all`: block all findings, allow a one-shot bypass.
- `strict-high`: block high-confidence findings, ignore bypass flags.
- `strict-all`: block all findings, ignore bypass flags.

`occ commit` exits with code `5` for blocking findings. In `warn` mode, text-mode `occ commit` prints the report and continues automatically. The global guard warns and returns success in `warn` mode, blocks in `block-*`, and blocks without bypass in `strict-*`.

Reports include the file, line number when available, rule, tier, and a redacted snippet preview. If a non-strict guard block is an intentional false positive, bypass only OpenCodeCommit for that one command:

```bash
OCC_ALLOW_SENSITIVE=1 git commit ...
```

Strict modes ignore `OCC_ALLOW_SENSITIVE=1` and `--allow-sensitive`.

**Flagged file names:**

| Category | Patterns |
|----------|----------|
| Environment / secrets | `.env*`, `credentials.json`, `secret.*`, `secrets.*`, `.netrc`, `service-account*.json` |
| Source maps | `*.js.map`, `*.css.map`, `*.map` — [can expose full source code](https://arstechnica.com/ai/2026/03/entire-claude-code-cli-source-code-leaks-thanks-to-exposed-map-file/) |
| Private keys / certs | `*.pem`, `*.key`, `*.p12`, `*.pfx`, `*.keystore`, `*.jks` |
| SSH keys | `id_rsa`, `id_ed25519`, `id_ecdsa`, `id_dsa`, `.ssh/*` |
| Auth files | `.htpasswd` |

| Category | Patterns |
|----------|----------|
| Generic secrets | assignment-based heuristics for `PASSWORD`, `SECRET_KEY`, `ACCESS_TOKEN`, `DB_PASSWORD`, `DATABASE_URL`, `CLIENT_SECRET`, `CREDENTIALS` |
| Service-specific | OpenAI, GitHub, AWS, Slack, Stripe, SendGrid, npm, PyPI, Docker, Vault, Discord, Teams |
| Structural patterns | `Bearer <token>`, JWTs, private key PEM headers, Docker auth blobs, kube auth fields, credential-bearing connection strings |

## Configuration

All VSCodium / VS Code settings are prefixed with `opencodecommit.`. Key settings:

| Setting | Default | Description |
|---------|---------|-------------|
| `backendOrder` | `["codex","opencode","claude","gemini"]` | Backend fallback order |
| `commitMode` | `adaptive` | `adaptive`, `adaptive-oneliner`, `conventional`, `conventional-oneliner` |
| `branchMode` | `conventional` | `conventional` or `adaptive` (matches existing branch names) |
| `diffSource` | `auto` | `auto`, `staged`, or `all` |
| `languages` | English, Finnish, Japanese, Chinese, Spanish, Portuguese, French, Korean, Russian, Vietnamese, German, Custom (example) | Array of language configs with custom prompt modules |
| `commitTemplate` | `{{type}}: {{message}}` | Supports `{{type}}`, `{{emoji}}`, `{{message}}` |
| `sensitive.enforcement` | `warn` | `warn`, `block-high`, `block-all`, `strict-high`, or `strict-all` |
| `sensitive.allowlist` | `[]` | Suppress findings by `pathRegex`, `rule`, and/or `valueRegex` with AND semantics |

CLI config: `~/.config/opencodecommit/config.toml` (TOML with the same fields in kebab-case).

Example:

```toml
[sensitive]
enforcement = "block-high"

[[sensitive.allowlist]]
path-regex = "\\.env\\.example$"
rule = "openai-project-key"
value-regex = "^sk-proj-"
```

## Languages

Built-in: **English** (default), **Finnish**, **Japanese**, **Chinese**, **Spanish**, **Portuguese**, **French**, **Korean**, **Russian**, **Vietnamese**, **German**, **Custom (example)** (template for your own).

Each language defines full prompt modules (base, adaptive, conventional, length, sensitive note). Missing modules fall back to English. CLI: `--language <built-in label>`. Extension: dropdown menu or `opencodecommit.activeLanguage` setting.

Add custom languages in config — only `label` and `instruction` are required:

```toml
[[languages]]
label = "Deutsch"
instruction = "Schreibe die Commit-Nachricht auf Deutsch."
```

## License

[MIT](LICENSE)
