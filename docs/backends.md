# Backends

OpenCodeCommit can use terminal AI CLIs and direct API providers from the same fallback chain.

## CLI Backends

Install only the tools you want to use:

```bash
npm i -g @openai/codex
npm i -g opencode
npm i -g @anthropic-ai/claude-code
# Antigravity
curl -fsSL https://antigravity.google/cli/install.sh | bash
# Grok Build
curl -fsSL https://x.ai/cli/install.sh | bash
```

Supported CLI backends:
- Codex CLI
- OpenCode CLI
- Claude Code CLI
- Antigravity CLI (`agy`)
- Grok Build CLI

The default backend order is:

```toml
backend-order = ["codex", "opencode", "claude", "agy", "grok"]
```

OpenCodeCommit tries each backend in order until one succeeds.

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

Example:

```toml
backend = "openai-api"
backend-order = ["codex", "openai-api", "ollama-api"]

[api.openai]
model = "gpt-5.6-terra"
endpoint = "https://api.openai.com/v1/chat/completions"
key-env = "OPENAI_API_KEY"
pr-model = "gpt-5.4"
cheap-model = "gpt-5.6-terra"

[api.ollama]
model = ""
endpoint = "http://localhost:11434"
key-env = ""
```

## Task-Specific Models

Commit, refine, branch, and changelog generation use the primary backend model.

PR generation can use a stronger model for final writing and a cheaper model for summarization. Configure `pr-model` and `cheap-model` in the relevant backend section.

Codex one-shot tasks use a fast, prompt-only profile. The default `gpt-5.6-terra` commit model runs with reasoning effort `none`; PR generation keeps the more conservative quality profile.

Grok Build uses its `grok-build` model key in headless single-prompt mode. OpenCodeCommit disables auto-updates, tools, memory, subagents, planning, and web search for these runs because all required diff context is already in the prompt.

Antigravity uses `agy --print` in plan mode with its sandbox enabled. Configure its model display names with `agy-model`, `agy-pr-model`, and `agy-cheap-model`.
