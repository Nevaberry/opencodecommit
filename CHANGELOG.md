# Changelog

## 1.4.0

### Changed

- **config.toml as single source of truth** -- configuration is now driven entirely by `config.toml` for both the CLI and the VS Code extension, replacing the split between VS Code settings and CLI config
- **Canonical config path** -- `OPENCODECOMMIT_CONFIG` environment variable honored for config file location

---

## 1.3.2

Version bump and documentation refresh.

---

## 1.3.1

### Fixed

- **TUI: merge staged, unstaged, and untracked file states** -- the change set shape now correctly reflects the actual scope so commit messages match what is being committed
- **TUI: keep output action footers visible while scrolling** -- panel buttons stay pinned in commit, sensitive, branch, PR, backend, and hook views
- **TUI: keep output actions visible while paging the diff** -- cap output panel height to leave room for the diff; PageUp/PageDown scroll the diff even when the output panel is open
- **Backend fallback timeout** increased from 45 s to 60 s

### Added

- **`-w` alias** for worktree selection in dev scripts

---

## 1.3.0

### Added

- **Nine new built-in languages** -- Japanese, Chinese, Spanish, Portuguese, French, Korean, Russian, Vietnamese, and German, each with full prompt modules (base, adaptive, conventional, length, oneliner, sensitive note)
- **Dedicated backend selection** -- TUI menus and `occ tui --backend` flag let the user pick a single backend for commit and PR generation; the VS Code extension adds per-backend adaptive commit commands (Codex, OpenCode, Claude, Gemini)
- **Gemini CLI invocation fix** -- prompt is now passed as a command-line argument instead of stdin, and `gemini-2.5-flash` is the default model
- **Sensitive guard profiles** -- human and strict-agent profiles classify findings into confirmed-secret, sensitive-artifact, and suspicious tiers with enforcement modes and allowlists
- **Continue / Cancel actions for sensitive warnings** -- replaces the old Bypass Once button; `c` to continue, `x` to dismiss
- **Worktree-aware dev scripts** for CLI and VSCodium extension development
- **Backend fallback and timeout handling for PR previews** -- shared PR preview helper with exec timeouts and per-backend failure reporting

### Changed

- Finnish language label renamed from "Suomi" to "Finnish" for consistency with other language labels

---

## 1.2.1

### Added

- **Backend fallback with progress tracking** -- `exec_with_fallback` tries backends in order with a 45 s timeout, surfaces `BackendProgress` and `BackendFailure` details in all preview structs
- **CLI backend fallback progress** surfaced in preview commands
- **Backend selection controls** for TUI and extension -- carry backend failure details through commit, branch, PR, and changelog previews

---

## 1.2.0

### Added

- **Two-stage PR generation** from committed branch changes -- when no working-tree diff exists, detect the base branch and generate PRs from commits ahead; file sidebar with commit-grouped browser, three-panel layout
- **Auto-update check** and `occ update` / `occ upgrade` commands -- detect installation source (npm or cargo) and update from the appropriate registry; background version check on TUI launch shows a non-intrusive header notice
- **Automatic LLM model update detection script** -- compares hardcoded model defaults against models available in locally installed CLIs (Codex, Claude Code, Gemini) for cron-based staleness detection

### Changed

- Release workflow now delegates version tag creation to CI

---

## 1.1.3

### Added

- **Compact sensitive-content summary and redacted report** -- replace the modal's full warning block with a short summary; add Inspect Report to open the redacted findings in a plaintext tab
- **Full sensitive line previews** shown in the sensitive report
- **PR preview clipboard copy** with top/bottom borders

### Changed

- **npm restructure** -- `opencodecommit` is now the primary unscoped package with binaries and postinstall; `@nevaberry/opencodecommit` is a scoped redirect

### Fixed

- Extension and CLI build checks resolved

---

## 1.1.2

### Added

- **TUI redesign as single-screen commit view** -- header with repo path, branch, and staged/unstaged counts; syntax-colored diff viewer; adaptive output panel for commit messages, sensitive warnings, branch previews, PR markdown, and hook confirmations; compact numbered button bar with Tab/arrow navigation and number-key shortcuts
- **Unified Tab focus** across panel and bar buttons, with Safety Hook submenu (merged Install/Uninstall into a single button)
- **Output panel actions** -- Shorten, Commit, and Regenerate moved into the output panel with key hints; all panel types (Branch, PR, Sensitive, Hook) gained inline key hints
- **PR preview clipboard copy** with top/bottom borders

### Fixed

- Button renumbering: 1=Commit 2=Branch 3=PR 4=Safety Hook 0=Quit

---

## 1.1.1

### Added

- **Structured sensitive reporting** for env files, credentials, tokens, private keys, and IPv4 addresses
- Redacted report shown in the VS Code warning modal and logs
- `--allow-sensitive` hint added to the Rust warning formatter

---

## 1.1.0

### Added

- **Ratatui TUI launcher** -- terminal UI for interactive commit generation
- **Global commit guard** for sensitive content -- `occ guard install|uninstall --global` manages `core.hooksPath` wrappers; blocks sensitive staged diffs in `pre-commit` while chaining existing repo and global hooks; exposes structured sensitive findings with redacted previews; `OCC_ALLOW_SENSITIVE` bypass documented

### Changed

- Added unscoped npm package alias

---

## 1.0.4

### Changed

- **CI cross-compile and auto-publish pipeline** -- `release.yml` cross-compiles `occ` for all 5 platforms via GitHub Actions, then publishes to npm, crates.io, VS Code Marketplace, and Open VSX on every push to master
- `ci.yml` slimmed to tests only (cross-compile moved to release workflow)
- `sync-version.sh` now creates a git tag after bumping manifests

---

## 1.0.3

Version bump for extension, CLI, npm package, and linux-x64 binary.

---

## 1.0.2

Version bump for extension, CLI, and npm package.

---

## 1.0.1

Version bump for extension, CLI, and npm package.

---

## 1.0.0

### Added

- **Ship the `occ` CLI** -- renamed the Rust and npm binaries from `opencodecommit` to `occ`
- **Configurable language prompt modules** with fallback resolution

### Changed

- CI artifacts and README updated for the new CLI name
- README updated for VSCodium and npm CLI support

---

## 0.9.0

### Added

- **Secret scanning** -- CLI blocks secret-containing diffs unless `--allow-sensitive` is passed
- **Dry-run branch creation** and sensitive-content prompts in the CLI
- **Branch mode config** and CLI flag for conventional vs adaptive branch names
- Detection of **source maps, keys, and htpasswd** as sensitive content
- Sensitive file and line detection in the extension context

---

## 0.8.3

### Changed

- Consolidated to a **single npm package** (removed per-platform packages); binaries shipped under `npm/opencodecommit/platforms` with postinstall linking
- npm published last in the release pipeline

---

## 0.8.2

### Added

- Documentation for local API key, credential, and secret scanning in README
- npm wrapper binary fallback for unsupported platforms

### Changed

- Default diff limit lowered to 10 000 characters
- Platform packages renamed to the `@nevaberry/opencodecommit-*` scope
- `private/` and `temp/` directories ignored in the repository

---

## 0.8.0

Rebranded from `opencode-commit` to `opencodecommit`. Published as a new extension on Open VSX.

### Added

- **Language-embedded prompt templates** -- all prompt modules (base, adaptive, conventional, oneliner, multiline, sensitive content note) are now part of each language entry in settings, making them fully editable without touching files
- **Custom (example) language** -- ships as a third default language entry alongside English and Suomi, serving as a starting point for new languages
- **Reset Settings to Defaults** command -- clears all user overrides so updated defaults take effect (SCM menu and command palette)
- **Gemini CLI** backend support with configurable model and path
- **Backend Order** setting with human-readable labels (Codex CLI, OpenCode CLI, Claude Code CLI, Gemini CLI) and Gemini-last default
- **Active Language dropdown** -- language selector is now a proper dropdown instead of a free-text field
- **Diagnose** command for troubleshooting CLI detection, prompts, and diffs

### Changed

- Settings page reorganised: language settings at top, backend CLI settings at bottom, with provider/model/path ordering within each backend group
- Removed 9 redundant settings from the UI (`prompt.*`, `custom.prompt`, `custom.typeRules`, `custom.commitMessageRules`) -- underlying features still work via language configs
- Extension settings keys renamed for clearer VS Code labels

### Fixed

- Prompt template file URIs now resolve correctly in installed VSIX (was pointing to `src/` instead of `out/`)
