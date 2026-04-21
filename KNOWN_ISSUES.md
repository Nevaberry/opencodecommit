# Known Issues

Last updated: 2026-04-21

## `cargo test --workspace` can fail on machines with Codex CLI installed

Status: open

### Symptom

Running `cargo test --workspace` may fail in the `occ` binary tests with:

- `actions::tests::repo_summary_reports_backend_error`
- Follow-on failures in `actions`, `guard`, and `tui` tests caused by
  `PoisonError`

### Root Cause

`Config::default()` now defaults to `Backend::Codex`, but
`repo_summary_reports_backend_error` only overrides `cli_path`, which applies
to the OpenCode CLI path. On machines where `codex` is installed, backend
detection succeeds for Codex, so the test's `backend_path.is_none()` assertion
is no longer valid.

After that first panic, later tests that share `TEST_CWD_LOCK` can fail because
the mutex becomes poisoned.

Relevant code:

- `crates/opencodecommit/src/config.rs`
- `crates/opencodecommit/src/actions.rs`
- `crates/opencodecommit/src/backend.rs`

### Workaround

- Treat the Codex-only local E2E wrapper as the reliable backend gate for now:
  `bash scripts/test-backend-local.sh -a`
- If you need to inspect the Rust failures, rerun the first failing test in
  isolation:
  `cargo test actions::tests::repo_summary_reports_backend_error --bin occ -- --exact --nocapture`

### Intended Fix

Update the stale unit test so it explicitly matches the backend under test, for
example by pinning `backend = Backend::Opencode` or by setting the invalid path
field for the active backend instead of relying on the current default.
