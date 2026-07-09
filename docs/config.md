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
backend-order = ["codex", "opencode", "claude", "agy", "grok"]
commit-mode = "adaptive"
branch-mode = "conventional"
diff-source = "auto"
active-language = "English"
commit-template = "{{type}}({{scope}}): {{message}}"
max-diff-length = 10000
commit-branch-timeout-seconds = 70
pr-timeout-seconds = 180
agy-path = ""
agy-model = "Gemini 3.5 Flash (Low)"
agy-pr-model = "Gemini 3.1 Pro (High)"
agy-cheap-model = "Gemini 3.5 Flash (Low)"
grok-path = ""
grok-model = "grok-build"
grok-pr-model = "grok-build"
grok-cheap-model = "grok-build"
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

## Repo-Local Guard

Install the raw Git commit guard per repository:

```bash
occ guard install
occ guard status
occ guard uninstall
```

The guard stores managed hooks and one-shot preserve tokens under `.git/occ/`. It saves and restores any existing local `core.hooksPath`.
