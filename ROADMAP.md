# Roadmap

## Future Features

### Changelog Conflict Resolution

When `CHANGELOG.md` already contains the requested version, do not stop at a duplicate-version error.
Use the PR timeout and the stronger, more expensive model, send both the existing changelog entry and the newly generated entry, and ask the model to return one merged Keep a Changelog block for that version.
If that merge still fails, fall back to manual editing.

### AI-Assisted Sensitive Content Gating

In `config.toml`, a feature (on by default) where lines flagged as sensitive by the regex are sent to a strong/expensive model for secondary analysis. The model responds with a simple **pass** or **block** verdict.

- **Pass example:** `OPENAI_API_KEY="placeholder"` — the regex triggers, but the AI recognizes it as a placeholder and lets it through automatically.
- **Block example:** `OPENAI_API_KEY="sk-real-key-here"` — the AI detects a real credential and blocks the commit, requiring human confirmation (same flow as the current warning prompt).

This sits between the regex detection and the user prompt: regex fires → AI verdict → if pass, no prompt; if block, show the existing confirmation dialog.

### Sensitive Content Scanning in Metadata Messages

Run the sensitive-content regex against commit messages, branch names, PR titles/bodies, and changelog entries — not just file diffs. Catches cases where secrets or sensitive data leak into metadata rather than code.

Optionally, flagged metadata can also be sent to the strong AI model for pass/block analysis, including an explanation of why the regex triggered.

### Faster Default Models for Basic Commits

Switch the default LLM models used in the standard commit flow to faster, cheaper alternatives. The current models are more capable than necessary for straightforward commit-message generation and changelog updates. By defaulting to lighter models, the basic commit path becomes noticeably quicker while keeping the stronger, more expensive models available for tasks that need them (e.g., conflict resolution, sensitive-content analysis).
