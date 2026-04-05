# OpenCodeCommit

AI-powered commit messages for VS Code and VS Codium — delegates to CLI tools you already have installed.

**Built-in security scanning** — diffs are scanned locally for API keys, credentials, and secrets before anything leaves your machine. [Details below.](#sensitive-content-detection)

Pick your CLI, pick your model, click the sparkle button. That's it.

[VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=Nevaberry.opencodecommit) · [npm](https://www.npmjs.com/package/@nevaberry/opencodecommit) · [GitHub](https://github.com/Nevaberry/opencodecommit)

## Why This Exists

Most AI commit extensions bundle their own API client, force you onto one provider, or don't work on Linux / VS Codium. OpenCodeCommit delegates to terminal AI agents — so it works with any LLM provider those tools support, inherits their auth, and stays out of your way.

## Features

- **Security scanning** — local detection of API keys, credentials, and secrets before sending diffs to any AI backend
- **Four CLI backends** — [Codex CLI](https://github.com/openai/codex), [OpenCode](https://github.com/nicepkg/opencode), [Gemini CLI](https://github.com/google-gemini/gemini-cli), [Claude Code](https://docs.anthropic.com/en/docs/claude-code)
- **Configurable fallback order** — tries each backend in sequence until one succeeds
- **Four commit modes** — adaptive, adaptive-oneliner, conventional, conventional-oneliner
- **Adaptive mode** — matches the style of your recent commits automatically
- **Multi-language** — English and Finnish built-in, add any language with custom instructions
- **Configurable prompts** — every prompt module is editable in VS Code settings
- **Refine** — iterate on the generated message with natural language feedback
- **Smart context** — sends file contents alongside the diff with intelligent truncation
- **Customizable** — templates, emoji maps, custom prompts, type/message rule overrides
- **Codex CLI provider support** — use OpenRouter, Ollama, or any provider via `-c model_provider`
- **Flatpak support** — works in sandboxed VS Codium via `flatpak-spawn --host`
- **Zero runtime dependencies** — just the extension and your CLI of choice

## Prerequisites

Install at least one of the supported CLI tools:

| Backend | Install | Providers |
|---------|---------|-----------|
| [Codex CLI](https://github.com/openai/codex) | `npm i -g @openai/codex` | OpenAI, OpenRouter, Ollama |
| [OpenCode](https://github.com/nicepkg/opencode) | `npm i -g opencode` | OpenAI, Anthropic, Google, Groq, Mistral, xAI, OpenRouter, Copilot, AWS Bedrock, Azure, Vertex AI, ... |
| [Gemini CLI](https://github.com/google-gemini/gemini-cli) | `npm i -g @google/gemini-cli` | Google (Gemini API, Vertex AI, OAuth) |
| [Claude Code](https://docs.anthropic.com/en/docs/claude-code) | `npm i -g @anthropic-ai/claude-code` | Anthropic |

## Installation

Search for **"OpenCodeCommit"** in the VS Code / VS Codium extension marketplace, or:

```
ext install Nevaberry.opencodecommit
```

## Usage

1. Stage your changes (or leave unstaged — the extension auto-detects)
2. Click the **sparkle button** in the Source Control title bar
3. The commit message appears in the input box

Use the **dropdown menu** next to the sparkle button for:
- Mode-specific generation (adaptive, conventional, oneliner variants)
- Refine — tweak the current message with feedback
- Switch language
- Diagnose — debug the pipeline
- Open settings

## Configuration

All settings are prefixed with `opencodecommit.` in your `settings.json`.

### Language

| Setting | Default | Description |
|---------|---------|-------------|
| `languages` | English, Suomi, Custom (example) | Array of language objects with prompt modules |
| `activeLanguage` | `English` | Currently active language (dropdown in settings) |
| `showLanguageSelector` | `true` | Show "Switch Language" in the SCM menu |

Each language entry contains an `instruction` field plus optional prompt modules (`baseModule`, `adaptiveFormat`, `conventionalFormat`, `multilineLength`, `onelinerLength`, `sensitiveContentNote`). See [Multi-Language Example](#multi-language-example) below.

### Backend Order

| Setting | Default | Description |
|---------|---------|-------------|
| `backendOrder` | `["codex","opencode","claude","gemini"]` | Backend CLI fallback order. First entry is the primary default. |

### Commit Style

| Setting | Default | Description |
|---------|---------|-------------|
| `commitMode` | `adaptive` | Default commit mode |
| `sparkleMode` | `adaptive` | Mode used by the sparkle button |
| `diffSource` | `auto` | Diff source: `auto`, `staged`, or `all` |
| `maxDiffLength` | `10000` | Max diff characters sent to the AI backend |
| `useEmojis` | `false` | Include emojis in commit messages |
| `useLowerCase` | `true` | Lowercase first letter of subject |
| `commitTemplate` | `{{type}}: {{message}}` | Template with `{{type}}`, `{{emoji}}`, `{{message}}` placeholders |
| `custom.emojis` | `{}` | Override default emojis per commit type |
| `refine.defaultFeedback` | `make it shorter` | Default text for the refine input box |

### CLI Backends

Each backend has provider, model, and path settings (provider and path are optional). Paths are auto-detected if empty.

| Setting | Default | Description |
|---------|---------|-------------|
| `codexCLIProvider` | | Model provider (e.g. openrouter, ollama). Passed as `-c model_provider`. |
| `codexCLIModel` | `gpt-5.4-mini` | Codex CLI model |
| `codexCLIPath` | | Codex CLI path |
| `opencodeCLIProvider` | `openai` | OpenCode provider (e.g. openai, anthropic, openrouter) |
| `opencodeCLIModel` | `gpt-5.4-mini` | OpenCode model (combined as provider/model) |
| `opencodeCLIPath` | | OpenCode CLI path |
| `claudeCodeCLIModel` | `claude-sonnet-4-6` | Claude Code model |
| `claudeCodeCLIPath` | | Claude Code CLI path |
| `geminiCLIModel` | | Gemini model (e.g. gemini-2.5-flash). If empty, uses default. |
| `geminiCLIPath` | | Gemini CLI path |

## Multi-Language Example

Each language entry has an `instruction` field (required) and optional prompt module overrides. If a prompt module is omitted, the value from the first language in the list is used as fallback.

```json
{
  "opencodecommit.languages": [
    {
      "label": "English",
      "instruction": "Write the commit message in English.",
      "baseModule": "You are an expert at writing git commit messages. ...",
      "adaptiveFormat": "Match the style of the recent commits. ..."
    },
    {
      "label": "Deutsch",
      "instruction": "Schreibe die Commit-Nachricht auf Deutsch. Verwende den Imperativ."
    }
  ]
}
```

The full default language entries (with all prompt modules) are visible in VS Code Settings under `opencodecommit.languages`. Use the **"Reset Settings to Defaults"** command to restore them if you've customized them.

## Sensitive Content Detection

Before sending the diff to an AI backend, OpenCodeCommit scans for potential secrets and credentials. If anything is detected, a warning dialog lets you cancel or continue.

### What triggers the warning

**Sensitive file names** — any changed file matching:

- `.env`, `.env.local`, `.env.production`, etc.
- `credentials.json`
- `secret.json`, `secrets.yaml`, etc.
- `.netrc`
- `service-account*.json`

**Sensitive patterns in added lines** (only `+` lines in the diff, not removals):

| Category | Patterns |
|----------|----------|
| Generic secrets | `API_KEY`, `SECRET_KEY`, `ACCESS_TOKEN`, `AUTH_TOKEN`, `PRIVATE_KEY`, `PASSWORD`, `PASSWD`, `DB_PASSWORD`, `DATABASE_URL`, `CLIENT_SECRET`, `CREDENTIALS` |
| Service-specific | `AWS_SECRET`, `GH_TOKEN`, `NPM_TOKEN`, `SLACK_TOKEN`, `STRIPE_SECRET`, `STRIPE_KEY`, `SENDGRID_KEY`, `TWILIO_AUTH`, `TWILIO_SID` |
| Bearer tokens | `Bearer` followed by 20+ alphanumeric characters |
| Provider key prefixes | `sk-` (OpenAI), `ghp_` (GitHub), `AKIA` (AWS access key ID) — each requiring 20+ characters |

The detection is pattern-based and local — nothing is sent to any server until you confirm. Diff header lines (`+++`, `---`) and removed lines (`-`) are excluded to avoid false positives from old code being deleted.

## CLI Tool (`occ`)

OpenCodeCommit also ships as a standalone Rust CLI for terminal use.

### Install

```bash
# via npm
npm i -g @nevaberry/opencodecommit

# via cargo
cargo install opencodecommit
```

### Usage

```bash
# Generate commit message
opencodecommit commit

# Generate branch name from diff
opencodecommit branch

# Generate PR title and body
opencodecommit pr

# Generate changelog entry
opencodecommit changelog

# Headless mode (for AI agents and scripts)
opencodecommit commit --headless --stdin < diff.txt
```

### Headless Mode

The CLI supports headless operation for AI-to-AI workflows and CI/CD:

```bash
opencodecommit commit --headless --backend codex
# Output: {"status":"success","message":"feat: add login page","type":"feat",...}
```

Exit codes: 0=success, 1=no changes, 2=provider error, 3=config error

## Future

- **TUI** — Ratatui-based terminal UI for Neovim/Emacs users
- **Neovim plugin** — Lua plugin calling the CLI
- **Emacs package** — Magit integration
- **More providers** — as CLI tools add provider support, OpenCodeCommit inherits it

## License

[MIT](LICENSE)
