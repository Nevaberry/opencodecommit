# OpenCodeCommit

AI commit messages via terminal AI agents. VS Code extension + standalone Rust CLI.

**Security scanning built in** — diffs are scanned locally for secrets, source maps, and private keys before anything leaves your machine.

[VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=Nevaberry.opencodecommit) · [npm](https://www.npmjs.com/package/@nevaberry/opencodecommit) · [crates.io](https://crates.io/crates/opencodecommit) · [GitHub](https://github.com/Nevaberry/opencodecommit)

## Install

**Extension:** Search "OpenCodeCommit" in VS Code / VS Codium, or `ext install Nevaberry.opencodecommit`

**CLI:** `cargo install opencodecommit` or `npm i -g @nevaberry/opencodecommit`

**Prerequisite:** At least one CLI backend:

| Backend | Install |
|---------|---------|
| [Codex CLI](https://github.com/openai/codex) | `npm i -g @openai/codex` |
| [OpenCode](https://github.com/nicepkg/opencode) | `npm i -g opencode` |
| [Gemini CLI](https://github.com/google-gemini/gemini-cli) | `npm i -g @google/gemini-cli` |
| [Claude Code](https://docs.anthropic.com/en/docs/claude-code) | `npm i -g @anthropic-ai/claude-code` |

## VS Code Usage

1. Stage changes (or leave unstaged — auto-detected)
2. Click the **sparkle button** in Source Control
3. Commit message appears in the input box

Dropdown menu: mode-specific generation, refine, branch name generation, switch language, diagnose.

## CLI Usage

```bash
opencodecommit commit              # generate message + commit
opencodecommit commit --dry-run    # preview only, don't commit
opencodecommit branch              # generate branch name + checkout
opencodecommit branch --mode adaptive  # match existing branch naming style
opencodecommit pr                  # generate PR title + body
opencodecommit changelog           # generate changelog entry

# JSON output (default), or --text for plain text
opencodecommit commit --text
opencodecommit commit --allow-sensitive  # skip secret scanning
```

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

**Flagged patterns in added lines** (`+` lines only, not removals):

| Category | Patterns |
|----------|----------|
| Generic secrets | `API_KEY`, `SECRET_KEY`, `ACCESS_TOKEN`, `AUTH_TOKEN`, `PRIVATE_KEY`, `PASSWORD`, `DB_PASSWORD`, `DATABASE_URL`, `CLIENT_SECRET`, `CREDENTIALS` |
| Service-specific | `AWS_SECRET`, `GH_TOKEN`, `NPM_TOKEN`, `SLACK_TOKEN`, `STRIPE_SECRET`, `SENDGRID_KEY`, `TWILIO_AUTH` |
| Token patterns | `Bearer <20+ chars>`, `sk-<20+ chars>`, `ghp_<20+ chars>`, `AKIA<12+ chars>` |

## Configuration

All VS Code settings are prefixed with `opencodecommit.`. Key settings:

| Setting | Default | Description |
|---------|---------|-------------|
| `backendOrder` | `["codex","opencode","claude","gemini"]` | Backend fallback order |
| `commitMode` | `adaptive` | `adaptive`, `adaptive-oneliner`, `conventional`, `conventional-oneliner` |
| `branchMode` | `conventional` | `conventional` or `adaptive` (matches existing branch names) |
| `diffSource` | `auto` | `auto`, `staged`, or `all` |
| `languages` | English, Suomi | Array of language configs with custom prompt modules |
| `commitTemplate` | `{{type}}: {{message}}` | Supports `{{type}}`, `{{emoji}}`, `{{message}}` |

CLI config: `~/.config/opencodecommit/config.toml` (TOML with the same fields in kebab-case).

## License

[MIT](LICENSE)
