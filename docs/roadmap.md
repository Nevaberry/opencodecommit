# Roadmap

OpenCodeCommit is intentionally local-first: the core product should keep making everyday git work faster without hiding where code context goes.

## Near Term

- More real-world E2E coverage against live AI backends, especially artifact flows for commit, branch, PR, and changelog generation. The hosted API smoke workflow is the first scheduled step.
- Better public onboarding with screenshots, short recordings, and clearer extension marketplace assets.
- More scanner fixtures from real CI incidents, with tight allowlist behavior and low false-positive rates.
- Cleaner extension diagnostics for backend timeouts, sandbox issues, and PATH/auth problems.

## Product Direction

- Keep the default workflow simple: generate, review, refine, commit.
- Keep CLI, TUI, extension, and CI behavior aligned through shared fixtures and mirrored Rust/TypeScript implementations.
- Prefer observable reliability work over new toggles.
- Treat model defaults as release decisions backed by live E2E results, not static assumptions.

## Possible Future Work

- Changelog conflict resolution that merges an existing version entry with a newly generated entry instead of failing immediately.
- Sensitive-content scanning for generated metadata such as commit messages, branch names, PR titles, PR bodies, and changelog entries.
- More focused CI helpers for teams that want scheduled live backend checks.
