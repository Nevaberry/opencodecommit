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

## CLI Usage

```bash
occ commit                         # generate message + commit
occ commit --dry-run               # preview only, don't commit
occ commit --language Suomi        # generate in Finnish
occ branch                         # generate branch name + checkout
occ branch --mode adaptive         # match existing branch naming style
occ pr                             # generate PR title + body
occ changelog                      # generate changelog entry

# JSON output (default), or --text for human readable plain text
occ commit --text
occ commit --allow-sensitive       # skip secret scanning
```

`occ` is the short form. `opencodecommit` also works if `occ` clashes with something on your system.

Exit codes: 0 success, 1 no changes, 2 backend error, 3 config error, 5 sensitive content detected

## Sensitive Content Detection

Diffs are scanned locally before being sent to any AI backend. The CLI blocks (exit 5) and the extension shows a warning dialog.

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
| Generic secrets | `API_KEY`, `SECRET_KEY`, `ACCESS_TOKEN`, `AUTH_TOKEN`, `PRIVATE_KEY`, `PASSWORD`, `DB_PASSWORD`, `DATABASE_URL`, `CLIENT_SECRET`, `CREDENTIALS` |
| Service-specific | `AWS_SECRET`, `GH_TOKEN`, `NPM_TOKEN`, `SLACK_TOKEN`, `STRIPE_SECRET`, `SENDGRID_KEY`, `TWILIO_AUTH` |
| Token patterns | `Bearer <20+ chars>`, `sk-<20+ chars>`, `ghp_<20+ chars>`, `AKIA<12+ chars>` |

## Configuration

All VSCodium / VS Code settings are prefixed with `opencodecommit.`. Key settings:

| Setting | Default | Description |
|---------|---------|-------------|
| `backendOrder` | `["codex","opencode","claude","gemini"]` | Backend fallback order |
| `commitMode` | `adaptive` | `adaptive`, `adaptive-oneliner`, `conventional`, `conventional-oneliner` |
| `branchMode` | `conventional` | `conventional` or `adaptive` (matches existing branch names) |
| `diffSource` | `auto` | `auto`, `staged`, or `all` |
| `languages` | English, Suomi | Array of language configs with custom prompt modules |
| `commitTemplate` | `{{type}}: {{message}}` | Supports `{{type}}`, `{{emoji}}`, `{{message}}` |

CLI config: `~/.config/opencodecommit/config.toml` (TOML with the same fields in kebab-case).

## Languages

Built-in: **English** (default), **Suomi** (Finnish), **Custom (example)** (template for your own).

Each language defines full prompt modules (base, adaptive, conventional, length, sensitive note). Missing modules fall back to English. CLI: `--language Suomi`. Extension: dropdown menu or `opencodecommit.activeLanguage` setting.

Add custom languages in config — only `label` and `instruction` are required:

```toml
[[languages]]
label = "Deutsch"
instruction = "Schreibe die Commit-Nachricht auf Deutsch."
```

## License

[MIT](LICENSE)
