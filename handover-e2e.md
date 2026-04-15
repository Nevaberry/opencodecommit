# E2E Handover

Date: 2026-04-15
Worktree: `/home/antti/code/opencodecommit/.worktrees/e2e-tests`

## Current goal

The next task after reboot is to add a small terminal helper script that follows the live e2e log while `scripts/test-e2e.sh` is running.

Desired outcome:
- start an e2e run
- watch the log live in the terminal
- see the merged `AI RESPONSES` blocks as they are appended
- avoid having to open the log manually in a second terminal every time

Likely shape:
- script name idea: `scripts/watch-e2e.sh`
- input: log file path, or optional backend/target/suite args that derive the same log path as `scripts/test-e2e.sh`
- behavior: `tail -f` or `tail -F` with a small header and maybe a convenience mode that starts the test in the background first

## What was implemented in this session

### Runner and logging

`scripts/test-e2e.sh` now supports:
- `--backends`
- `--target extension|cli|tui|both|all`
- `--suite full|artifacts`
- `--log-file`

It now also:
- writes a per-run log file and truncates it at the start of the run
- exports `OCC_E2E_RESPONSE_LOG`
- prints a merged `===== AI RESPONSES =====` section at the end of the run

### New response logging

Structured response logging was added for:
- `occ`
- `occ tui`
- extension e2e harness

Log block format is:
- `platform`
- `test`
- `operation`
- `backend`
- `response`

### Artifact-focused suites

Artifact suite means just the four AI-generated outputs:
- commit
- branch
- PR
- changelog

Implemented as:
- CLI artifact tests in `crates/opencodecommit/tests/e2e_cli.rs`
- TUI artifact smoke in `crates/opencodecommit/tests/e2e_tui.rs`
- extension artifact filtering in `extension/src/test/e2e/commands.e2e.ts`

### Extension harness changes

Relevant extension e2e changes:
- artifact suite runs only the four artifact commands
- config-lifecycle suite is skipped during `--suite artifacts`
- longer artifact wait budget
- command tests now wait on observable side effects instead of awaiting command completion directly
- calmer VSCodium launch args were added in `extension/src/test/e2e/runTest.ts`

## Current status

### Passing

These passed against `codex`:

1. CLI artifacts
Command:
```bash
scripts/test-e2e.sh --backends codex --target cli --suite artifacts --log-file .logs/e2e-codex-artifacts-cli.log
```
Log:
- `.logs/e2e-codex-artifacts-cli.log`

2. TUI artifacts
Command:
```bash
scripts/test-e2e.sh --backends codex --target tui --suite artifacts --log-file .logs/e2e-codex-artifacts-tui.log
```
Log:
- `.logs/e2e-codex-artifacts-tui.log`

### Failing

Extension artifacts are still not stable in this environment.

Command:
```bash
scripts/test-e2e.sh --backends codex --target extension --suite artifacts --log-file .logs/e2e-codex-artifacts-extension.log
```

Observed failure mode:
- VSCodium Flatpak test host dies with `SIGKILL`
- recent retries often die before the test host fully starts

Current extension log:
- `.logs/e2e-codex-artifacts-extension.log`

## Important environment finding

`journalctl` showed real memory pressure and OOM activity around the earlier long VSCodium runs.

Useful command used in this session:
```bash
journalctl --since '10 minutes ago' | rg 'codium|SIGKILL|Killed process|oom|Out of memory'
```

Key signal seen:
- multiple `app-flatpak-com.vscodium.codium` scopes had large memory peaks
- earlier runs hit OOM pressure around `2026-04-15 14:44`
- later quick `SIGKILL` runs may be fallout from the same environment pressure

Conclusion:
- CLI/TUI logging work is real and working
- extension instability currently looks environmental, not purely a test-logic failure

## Files changed in the worktree

Current modified/untracked files:
- `crates/opencodecommit/src/actions.rs`
- `crates/opencodecommit/src/main.rs`
- `crates/opencodecommit/src/tui.rs`
- `crates/opencodecommit/tests/common/mod.rs`
- `crates/opencodecommit/tests/e2e_cli.rs`
- `crates/opencodecommit/tests/e2e_tui.rs`
- `extension/src/test/e2e/commands.e2e.ts`
- `extension/src/test/e2e/config-lifecycle.e2e.ts`
- `extension/src/test/e2e/runTest.ts`
- `extension/src/test/e2e/shared.ts`
- `scripts/test-e2e.sh`
- `crates/opencodecommit/src/e2e_trace.rs`

## Logs and temp dirs worth keeping in mind

Useful logs:
- `.logs/e2e-codex-artifacts-cli.log`
- `.logs/e2e-codex-artifacts-tui.log`
- `.logs/e2e-codex-artifacts-extension.log`
- `.logs/e2e-codex-artifacts.log`

Preserved temp dirs from failed extension runs:
- `.tmp/occ-e2e.8cqg0s`
- `.tmp/occ-e2e.ik85dD`

These were preserved by the runner on failure. They can be inspected or removed later.

## Resume checklist after reboot

From the worktree root:

1. Confirm the worktree state
```bash
git status --short
```

2. Reconfirm the runner still parses
```bash
bash -n scripts/test-e2e.sh
```

3. Reconfirm extension TS compiles
```bash
bunx tsc -p extension/ --noEmit
```

4. Reconfirm Rust e2e targets compile
```bash
cargo test --test e2e_cli artifacts_ --no-run
cargo test --test e2e_tui artifacts_ --no-run
```

5. Re-run known-good artifact paths if needed
```bash
scripts/test-e2e.sh --backends codex --target cli --suite artifacts --log-file .logs/e2e-codex-artifacts-cli.log
scripts/test-e2e.sh --backends codex --target tui --suite artifacts --log-file .logs/e2e-codex-artifacts-tui.log
```

6. If retrying extension, check OOM pressure first
```bash
journalctl --since '10 minutes ago' | rg 'codium|SIGKILL|Killed process|oom|Out of memory'
```

## Recommended next task

Implement the live-tail helper.

Minimal version:
- `scripts/watch-e2e.sh <log-file>`
- just does `tail -F "$log_file"`
- prints a short banner with the absolute path

Better version:
- `scripts/watch-e2e.sh --backends codex --target cli --suite artifacts`
- derives the same default log path pattern or accepts `--log-file`
- optional `--run` flag that starts `scripts/test-e2e.sh` in background and then follows the log

## Notes for the next session

- `scripts/test-e2e.sh` now truncates the log file at the start of each run.
- The existing `.logs/e2e-codex-artifacts-extension.log` still contains historical failed runs from before that fix.
- The extension artifact suite was intentionally narrowed to reduce memory/watcher load.
- Do not assume the extension failures are fixed until a fresh VSCodium run passes in this environment.
