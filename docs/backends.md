# Backends

OpenCodeCommit can use terminal AI CLIs and direct API providers from the same fallback chain.

## CLI Backends

Install only the tools you want to use:

```bash
npm i -g @openai/codex
npm i -g @opencode/cli # OpenCode 2 or newer
npm i -g @anthropic-ai/claude-code
# Antigravity
curl -fsSL https://antigravity.google/cli/install.sh | bash
# Grok Build
curl -fsSL https://x.ai/cli/install.sh | bash
```

Supported CLI backends:
- Codex CLI
- OpenCode CLI 2.x
- Claude Code CLI
- Antigravity CLI (`agy`)
- Grok Build CLI

The default backend order is:

```toml
backend-order = ["codex", "opencode", "claude", "agy", "grok"]
```

OpenCodeCommit tries each backend in order until one succeeds.

### OpenCode 2

The OpenCode CLI backend requires OpenCode 2 or newer and was verified with
2.2.7. Install it with `npm i -g @opencode/cli`, or use the official
[`https://opencode.ai/v2/install` installer](https://opencode.ai/docs/).
The old `opencode` npm package and the 1.x CLI are not supported.

Requests use `opencode run --standalone` so each invocation inherits the
current environment through its own private server. Fast generation selects
`#none` for GPT-6 Luna and GPT-6.1 Terra, and `#low` for Astra and GPT-6.1 Sol.
An explicit model variant such as `gpt-6.1-sol#high` takes precedence. PR stages
keep the configured model's default reasoning effort. OpenCode 2 removed the
separate `--variant` flag and the positional provider filter for `models`.

Sources: [OpenCode CLI commands](https://opencode.ai/docs/cli/commands/),
[model variants](https://opencode.ai/docs/models/#variants), and
[V1 migration guide](https://opencode.ai/docs/migrate-v1/).

## Direct API Backends

Supported direct API backends:
- OpenAI
- Anthropic
- Google Gemini
- OpenRouter
- OpenCode Zen
- Ollama
- LM Studio
- Custom OpenAI-compatible endpoints

Hosted API backends read API keys from environment variables. Ollama and LM Studio can auto-detect the lexicographically first available local model when their `model` field is empty.

Model defaults checked on November 11, 2026:

| Backend | Commit | PR | Summarization |
| --- | --- | --- | --- |
| Codex, OpenCode, OpenAI API, OpenCode API | `gpt-6-luna` | `gpt-6-luna` | `gpt-6-luna` |
| Claude / Anthropic API | `claude-sonnet-5-5` | `claude-opus-5-5` | `claude-haiku-5-5` |
| Gemini API | `gemini-3.8-flash` | `gemini-3.8-flash` | `gemini-3.5-flash-lite` |
| OpenRouter | `anthropic/claude-sonnet-5.5` | `openai/gpt-6-luna` | `openai/gpt-6-luna` |
| Antigravity | `Gemini 3.8 Flash (Low)` | `Gemini 4 Argon (High)` | `Gemini 3.8 Flash (Low)` |

The Assisted-by picker also includes GPT-6.1 Sol and Terra, Claude Fable 5.2,
Gemini 4 Carbon and Argon, Kimi K3.1 Pro, Grok 4.7, and DeepSeek Flash.
Luna remains the default OpenAI model for fast, inexpensive generation.
Saved model choices remain explicit overrides; update your config to adopt new
defaults if it already contains older IDs. Antigravity model access depends on
your plan; use `agy models` to see the available display names.

Sources: [OpenAI models](https://developers.openai.com/api/docs/models),
[Claude models](https://platform.claude.com/docs/en/models/overview),
[Gemini models](https://ai.google.dev/gemini-api/docs/latest-model),
[Antigravity models](https://antigravity.google/docs/models/),
[OpenRouter catalog](https://openrouter.ai/api/v1/models),
[OpenCode endpoints](https://opencode.ai/docs/zen/),
[Kimi models](https://platform.moonshot.ai/docs/models),
[Grok models](https://docs.x.ai/developers/models), and
[DeepSeek models](https://api-docs.deepseek.com/quick_start/pricing).

Example:

```toml
backend = "openai-api"
backend-order = ["codex", "openai-api", "ollama-api"]

[api.openai]
model = "gpt-6-luna"
endpoint = "https://api.openai.com/v1/chat/completions"
key-env = "OPENAI_API_KEY"
pr-model = "gpt-6-luna"
cheap-model = "gpt-6-luna"

[api.ollama]
model = ""
endpoint = "http://localhost:11434"
key-env = ""
```

## Task-Specific Models

Commit, refine, branch, and changelog generation use the primary backend model.

PR generation can use a stronger model for final writing and a cheaper model for summarization. Configure `pr-model` and `cheap-model` in the relevant backend section.

Codex one-shot tasks use a fast, prompt-only profile. The default `gpt-6-luna` commit model runs with reasoning effort `none`; GPT-6 Astra and GPT-6.1 Sol use `low`, their lowest supported effort. PR generation keeps the more conservative quality profile. See [OpenAI migration guidance](https://developers.openai.com/api/docs/guides/latest-model#limitations).

Grok Build uses its `grok-build` model key in headless single-prompt mode. OpenCodeCommit disables auto-updates, tools, memory, subagents, planning, and web search for these runs because all required diff context is already in the prompt.

Antigravity uses `agy --print` in plan mode with its sandbox enabled. Configure its model display names with `agy-model`, `agy-pr-model`, and `agy-cheap-model`.

## API Compatibility

OpenAI Chat Completions requests use `max_completion_tokens`. Other compatible
Chat Completions providers retain `max_tokens`. Versioned base URLs such as
`https://api.openai.com/v1` and full endpoint URLs are both supported.

Endpoints ending in `/responses` use the Responses request and output formats,
with storage disabled. OpenCode's GPT defaults use
`https://opencode.ai/inference/openai/v1/responses`; replace the old
`https://opencode.ai/zen/v1/chat/completions` endpoint in saved configurations.
For other OpenCode model families, use the provider endpoint listed in the
[OpenCode documentation](https://opencode.ai/docs/zen/).

## Backend Checks

Run `cargo test --workspace` and `bun run test:ts` for deterministic CLI, TUI,
configuration, and API contract checks. API checks use local HTTP fixtures or
mocked fetch responses and do not require credentials.

Run `bash scripts/test-live-backends.sh` to generate messages from a synthetic
diff using installed, authenticated CLIs. This uses the shipped model defaults
in an isolated config and leaves your saved configuration untouched. Set
`OCC_SMOKE_CONFIG` to test a specific config. To check a particular model:

```bash
bash scripts/test-live-backends.sh codex:gpt-6.1-sol claude:claude-sonnet-5-5
```

Missing CLI binaries are reported as skipped. Authentication, model access,
network, and generation failures are reported as failures; fallback cannot hide
a failed backend because each invocation selects exactly one backend.

`scripts/check_model_updates.sh` compares installed CLI catalogs with the
defaults. OpenAI updates stay within the configured model tier, so a newer Sol
release does not suggest replacing an intentional Luna default.
