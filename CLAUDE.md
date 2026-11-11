# CLAUDE.md

## What is this

OpenCodeCommit (`occ`) -- AI-powered git commit, branch, PR, and changelog generator. Ships as a **Rust CLI/TUI** and a **VS Code extension (TypeScript)**.

## CRITICAL: Dual implementation -- keep Rust and TypeScript in sync

The extension (`extension/src/inline/`) mirrors the Rust library. **Every change to prompts, config schema, security patterns, language templates, or core logic must be applied to both Rust and TypeScript.** Also update both `package.json` files (root and `extension/`). Forgetting one side is the #1 source of bugs in this repo.

Mapping: `languages.rs` <-> `package.json` language defaults, `config.rs` <-> `config-schema.ts`, `sensitive.rs` <-> `sensitive.ts`, `prompt.rs` <-> `generator.ts`, `context.rs` <-> `context.ts`, `backend.rs` <-> `backends.ts`, `api/` <-> `api.ts`.

## Build & test

```sh
# Rust
cargo build && cargo test --workspace        # build + all tests
cargo fmt && cargo clippy                     # format + lint
cargo run --bin occ -- tui                    # run TUI
cargo run --bin occ -- commit --dry-run --text

# TypeScript (extension)
bun install && cd extension && bun install    # first-time
bun run build                                 # compile
bunx tsc -p extension/tsconfig.json --noEmit  # type check
bun test extension/src/test/                  # unit tests
bun run lint                                  # biome check

# E2E
scripts/test-e2e.sh --target cli|tui|extension

# CI equivalent
cargo test --workspace
bunx tsc -p extension/tsconfig.json --noEmit
bun test extension/src/test/inline.test.ts extension/src/test/config.test.ts extension/src/test/api.test.ts
```


## Architecture

```
Rust — crates/opencodecommit/src/
  main.rs (CLI), tui.rs (TUI), lib.rs
  core: config, languages, prompt, context, sensitive, scan,
        dispatch, backend, api/, response, git, actions,
        guard, update, codex_home

TypeScript — extension/src/
  extension.ts (VS Code entry)
  inline/  mirrors Rust core (see mapping above)
  other:   cli.ts, pr.ts, changelog.ts, host-io.ts, types.ts
```

## Key rules

- Security scanning always runs before sending diffs to AI (sensitive.rs / sensitive.ts, 60+ regex patterns, not skippable)
- Backend fallback chains: `backend-order` in config.toml, CLI backends + direct API backends
- Config bidirectionally synced between `config.toml` and VS Code `settings.json` (`opencodecommit.*` namespace)

# Claude orchestrates, Codex implements

Claude's budget is scarce, Codex's is not. Hand Codex every implementation, debugging, root-cause, review and second-opinion task; do it yourself only for a one-line edit or when the answer is already in context. Then verify: `git status`, read the diff, run the tests, and report what they show rather than what Codex claimed.

## Delegating

```bash
CJ=$(ls -d ~/.claude/plugins/cache/openai-codex/codex/*/scripts/codex-companion.mjs | sort -V | tail -1)
node "$CJ" task --cwd <abs-repo-root> [--write] [--background] [--effort E] [--model M] [--resume-last] "<prompt>" 2>/dev/null
```

Codex sees none of this conversation. Send absolute paths, goal, the context you gathered, constraints, done criteria, the commands that prove it, and real error text rather than a description of it. When a specific tool or source is required, say what to do if it fails ("stop and report"); otherwise Codex may quietly fall back and still report success.

- `${CLAUDE_PLUGIN_ROOT}` is substituted only inside plugin files, and every Bash call is a fresh shell, so resolve `$CJ` like that each time.
- **stderr is progress, stdout is the answer** — hence the redirect, on every call. Fatal wrapper errors go to stderr too, so empty stdout means failure; rerun that one without it.
- **Unknown flags become prompt text**, never an error: a misspelled `--write` silently runs read-only.
- **`--cwd <abs path>`** pins the workspace to that path's git root, or the path itself outside a repo. Jobs are filed per workspace, so give `status`, `result` and `cancel` the same `--cwd`; run from another directory, they report "No job found".
- **`--write`** = `workspace-write`, approvals `never`: unattended edits anywhere under that root. Omit for review, diagnosis and research, and say "read-only, do not modify" in the prompt too.
- **`--prompt-file <path>`** for long or quote-heavy prompts; stdin also works.
- Omit `--effort`/`--model` to inherit `~/.codex/config.toml`, or `.codex/config.toml` in a trusted repo; drop to `--effort low` for cheap mechanical work. Models reject `none`/`minimal`, and the stock companion (1.0.6) rejects `max`/`ultra`, so set those in the config.
- Same `$CJ`: `status [id|--all]`, `result [id]`, `cancel [id]`, `review`, `adversarial-review`, `transfer`, `setup`. Both reviews ignore `--background` and block, so run them in a background Bash call.

Do not relay through the `codex:codex-rescue` subagent — ~12k Sonnet tokens to forward a prompt you can send for a few hundred — except when the user types `/codex:rescue`, which requires it. Never `Skill(codex:rescue)`: it re-enters the command and hangs the session. Apart from `rescue` and `setup`, every `/codex:*` command is `disable-model-invocation`, so run `$CJ` yourself or ask the user to type the command.

## Codex's sandbox

- **Reads are unrestricted** across the whole filesystem. **Writes** reach only the workspace root, `/tmp` and `$TMPDIR`; `$HOME` and `/var/tmp` are denied.
- `.git`, `.codex` and `.agents` stay read-only _inside_ the workspace, so Codex edits files but cannot stage or commit — `git add` dies on `.git/index.lock: Read-only file system`. **Claude does the committing.** `rm -f` is refused too ("use a safer approach"); Codex deletes through `apply_patch`.
- Shell network is **off** by default, so curl and Firecrawl fail; hosted `web__run` still works. `network_access = true` under `[sandbox_workspace_write]` in `~/.codex/config.toml` turns it on for `--write` tasks only, and `writable_roots` there widens the write set.

## Codex's own tools

- It has native web search (`web__run`), a node REPL, `view_image`, its configured MCP servers and app connectors, and can spawn its own subagents.
- Codex skills are callable by name: say "use the `knowledge-patch:using-knowledge-patch` skill" instead of pasting paths or documentation; that gateway routes to the matching technology patch.
- `image_gen__imagegen` takes `prompt`, `transparent_background`, and for edits either `referenced_image_paths` (local files) or `num_last_images_to_include` (≤5). There is no size or quality parameter. Output is ~1.57 MP with the aspect ratio inferred from the prompt: "16:9 banner" gives 1672x941, "9:16 poster" 941x1672, unstated 1254x1254. Files land in `~/.codex/generated_images/<thread>/`, outside Codex's write roots, so it can copy them out but can neither move nor delete them. Generation takes minutes, so always `--background`.

## Background jobs

Foreground blocks the Bash call and long tasks hit its timeout, so use `--background` for anything open-ended. It prints the job ID (`task-…`) and returns, and nothing pushes completion to you. Poll exactly that ID, never one guessed from `status --all`, in **one** background Bash call — it waits without burning model turns and wakes Claude with the result:

```bash
CJ=$(ls -d ~/.claude/plugins/cache/openai-codex/codex/*/scripts/codex-companion.mjs | sort -V | tail -1)
while
  s=$(node "$CJ" status <job-id> --cwd <abs-repo-root> --wait --timeout-ms 240000 --json | jq -r '.job.status')
  [[ "$s" == queued || "$s" == running ]]
do :; done
node "$CJ" result <job-id> --cwd <abs-repo-root> 2>/dev/null
```

`status --wait` gives up at its timeout even if the job is still running — hence the loop.

## Resuming, the cheapest path

`--resume-last` continues the previous thread with its memory intact: send context once, then iterate in short follow-ups. It resolves to the newest _finished_ `task` thread for this workspace **in the current Claude session**, not to _your_ thread. A task you or a subagent started in the same repo steals it, and a running one makes it fail, so keep resumable work to one thread at a time. Without the flag, a task starts fresh. The companion cannot reach an earlier session's thread.

A resumed thread keeps the sandbox it started with: `--write` does not upgrade a read-only thread, which then edits nothing, still exits 0 and reports `sandbox_mode=read-only`. When an audit should lead to edits, start it with `--write` and say "read-only, do not modify" in the prompt, or hand the plan to a fresh `--write` task.

## Gotchas

- Ending the Claude session kills its running Codex jobs and wipes their `status`/`result` records, so finish background work first. It also shuts down the repo's shared Codex broker, which can take down another Claude session's jobs in the same repo. The Codex session ID those two print is the only durable handle; `codex resume <id>` reopens that run in the Codex TUI.
- User-typed `/codex:transfer` copies this whole conversation into a resumable Codex thread.
- `node "$CJ" setup --json` diagnoses a missing or logged-out Codex; logging in needs the user: `!codex login`.
- The review gate (`/codex:setup --enable-review-gate`) runs Codex on every Stop and can loop. Leave it off unless the user is watching.
