# VS Code And VSCodium

OpenCodeCommit contributes actions to the Source Control title bar.

Common actions:
- generate a commit message
- refine the current message
- generate a branch name
- generate a PR draft
- create a changelog entry
- switch language
- pick a backend for a one-shot run
- open or reveal the shared config file
- run Diagnose

The extension uses the same `config.toml` as the CLI and TUI. Settings under `opencodecommit.*` are synchronized with that file.

## Flatpak Notes

VSCodium Flatpak installations can have a sparse PATH. OpenCodeCommit checks common CLI locations and shell profile PATH repair paths when auto-detecting backend CLIs.

If auto-detection cannot find a backend, set the explicit path in the config or extension settings.

## Diagnose

Use `occ: Diagnose` from the command palette or Source Control menu when a backend fails. Diagnose logs:
- config path and source
- sandbox mode
- backend order and resolved CLI paths
- diff and prompt sizes
- prompt input breakdown
- command invocation
- backend response or timeout
