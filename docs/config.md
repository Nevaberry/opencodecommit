# Configuration

OpenCodeCommit uses one config file for the CLI, TUI, and VS Code / VSCodium extension:

```text
~/.config/opencodecommit/config.toml
```

On first use, OpenCodeCommit writes the default config so every setting is visible in one place. Override the path with:

```bash
export OPENCODECOMMIT_CONFIG=/path/to/config.toml
```

## Common Settings

```toml
backend = "codex"
backend-order = ["codex", "opencode", "claude", "gemini"]
commit-mode = "adaptive"
branch-mode = "conventional"
diff-source = "auto"
active-language = "English"
commit-template = "{{type}}({{scope}}): {{message}}"
max-diff-length = 10000
commit-branch-timeout-seconds = 70
pr-timeout-seconds = 180
```

Useful sections:
- `[sensitive]`
- `[api.openai]`
- `[api.anthropic]`
- `[api.gemini]`
- `[api.openrouter]`
- `[api.opencode]`
- `[api.ollama]`
- `[api.lm-studio]`
- `[api.custom]`

## Sensitive Enforcement

```toml
[sensitive]
enforcement = "warn"
allowlist = []
```

Enforcement modes:
- `warn`
- `block-high`
- `block-all`
- `strict-high`
- `strict-all`

Strict modes disable bypass actions.

## Language And Formatting

Built-in language templates cover English, Finnish, Japanese, Chinese, Spanish, Portuguese, French, Korean, Russian, Vietnamese, and German.

The default commit template preserves scopes when the model returns `type(scope): message`:

```toml
commit-template = "{{type}}({{scope}}): {{message}}"
```

Custom prompt modules and custom language entries can be edited directly in `config.toml`.
