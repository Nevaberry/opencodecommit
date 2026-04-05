# Changelog

## 0.8.0 — 2026-04-05

Rebranded from `opencode-commit` to `opencodecommit`. Published as a new extension on Open VSX.

### Added

- **Language-embedded prompt templates** — all prompt modules (base, adaptive, conventional, oneliner, multiline, sensitive content note) are now part of each language entry in settings, making them fully editable without touching files
- **Custom (example) language** — ships as a third default language entry alongside English and Suomi, serving as a starting point for new languages
- **Reset Settings to Defaults** command — clears all user overrides so updated defaults take effect (SCM menu and command palette)
- **Gemini CLI** backend support with configurable model and path
- **Backend Order** setting with human-readable labels (Codex CLI, OpenCode CLI, Claude Code CLI, Gemini CLI) and Gemini-last default
- **Active Language dropdown** — language selector is now a proper dropdown instead of a free-text field
- **Diagnose** command for troubleshooting CLI detection, prompts, and diffs

### Changed

- Settings page reorganised: language settings at top, backend CLI settings at bottom, with provider/model/path ordering within each backend group
- Removed 9 redundant settings from the UI (`prompt.*`, `custom.prompt`, `custom.typeRules`, `custom.commitMessageRules`) — underlying features still work via language configs
- Extension settings keys renamed for clearer VS Code labels

### Fixed

- Prompt template file URIs now resolve correctly in installed VSIX (was pointing to `src/` instead of `out/`)
