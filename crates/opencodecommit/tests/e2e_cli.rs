mod common;

use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

#[cfg(unix)]
use common::repo_root;
use common::{
    FixtureRepo, append_response_log, assert_branch_shape, assert_changelog_shape,
    assert_commit_shape, assert_pr_shape, load_env, run_occ, stderr, stdout,
};
#[cfg(unix)]
use opencodecommit::config::Config;
#[cfg(unix)]
use opencodecommit::sensitive::SensitiveEnforcement;

fn config_arg(config_path: &Path) -> [&str; 2] {
    ["--config", config_path.to_str().expect("utf8 config path")]
}

#[cfg(unix)]
fn fake_opencode(repo: &FixtureRepo, message: &str) -> PathBuf {
    let path = repo.path.join("fake-opencode");
    fs::write(&path, format!("#!/bin/sh\necho '{}'\n", message)).expect("write fake opencode");
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod fake opencode");
    path
}

#[cfg(windows)]
fn fake_opencode(repo: &FixtureRepo, message: &str) -> PathBuf {
    let path = repo.path.join("fake-opencode.cmd");
    fs::write(
        &path,
        format!("@echo off\r\necho {message}\r\nexit /b 0\r\n"),
    )
    .expect("write fake opencode");
    path
}

fn fake_config(repo: &FixtureRepo, fake_cli: &Path) -> PathBuf {
    let path = repo.path.join("guard-config.toml");
    let cli_path = toml::Value::String(fake_cli.to_string_lossy().into_owned()).to_string();
    fs::write(
        &path,
        format!("backend = \"opencode\"\nbackend-order = [\"opencode\"]\ncli-path = {cli_path}\n"),
    )
    .expect("write fake config");
    path
}

fn run_git_commit(repo: &FixtureRepo, args: &[&str], config_path: &Path) -> std::process::Output {
    Command::new("git")
        .args(["commit"])
        .args(args)
        .current_dir(&repo.path)
        .env("OPENCODECOMMIT_CONFIG", config_path)
        .env("GIT_AUTHOR_NAME", "OpenCodeCommit E2E")
        .env("GIT_AUTHOR_EMAIL", "e2e@example.com")
        .env("GIT_COMMITTER_NAME", "OpenCodeCommit E2E")
        .env("GIT_COMMITTER_EMAIL", "e2e@example.com")
        .output()
        .expect("run git commit")
}

fn last_commit_message(repo: &FixtureRepo) -> String {
    let output = Command::new("git")
        .args(["log", "-1", "--pretty=%B"])
        .current_dir(&repo.path)
        .output()
        .expect("read git log");
    assert!(output.status.success(), "git log failed");
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

#[cfg(unix)]
#[test]
fn targeted_e2e_renderer_keeps_cli_settings_at_toml_root() {
    let repo = FixtureRepo::new("e2e-render-config");
    let config_path = repo.path.join("rendered-config.toml");
    let common_script = repo_root().join("scripts/e2e-common.sh");

    let output = Command::new("bash")
        .arg("-c")
        .arg(r#"source "$1"; occ_e2e_render_config_for_backends codex "$2""#)
        .arg("occ-e2e-render-config")
        .arg(&common_script)
        .arg(&config_path)
        .env("OCC_E2E_OPENCODE_PROVIDER", "renderer-provider")
        .env("OCC_E2E_OPENCODE_MODEL", "renderer-model")
        .env("OCC_E2E_OPENCODE_PATH", "/renderer/opencode")
        .env("OCC_E2E_CODEX_PATH", "/renderer/codex")
        .env("OCC_E2E_CODEX_MODEL", "renderer-codex-model")
        .env("OCC_E2E_CODEX_PROVIDER", "renderer-codex-provider")
        .env("OCC_E2E_CODEX_PR_MODEL", "renderer-codex-pr-model")
        .env("OCC_E2E_CODEX_CHEAP_MODEL", "renderer-codex-cheap-model")
        .env("OCC_E2E_COMMIT_TIMEOUT_SECONDS", "17")
        .env("OCC_E2E_PR_TIMEOUT_SECONDS", "29")
        .output()
        .expect("render targeted e2e config");

    assert!(
        output.status.success(),
        "config renderer failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let config = Config::load(&config_path).expect("load rendered config");
    assert_eq!(config.provider, "renderer-provider");
    assert_eq!(config.model, "renderer-model");
    assert_eq!(config.cli_path, "/renderer/opencode");
    assert_eq!(config.codex_path, "/renderer/codex");
    assert_eq!(config.codex_model, "renderer-codex-model");
    assert_eq!(config.codex_provider, "renderer-codex-provider");
    assert_eq!(config.codex_pr_model, "renderer-codex-pr-model");
    assert_eq!(config.codex_cheap_model, "renderer-codex-cheap-model");
    assert_eq!(config.commit_branch_timeout_seconds, 17);
    assert_eq!(config.pr_timeout_seconds, 29);
    assert_eq!(config.sensitive.enforcement, SensitiveEnforcement::Warn);
}

#[test]
fn artifacts_commit_dry_run_generates_valid_output_across_backends() {
    let Some(env) = load_env() else { return };
    let repo = FixtureRepo::new("e2e-cli-commit");
    let diff = repo.staged_diff();
    let config = config_arg(&env.config_path);

    for backend in &env.active_backends {
        for mode in ["adaptive", "conventional"] {
            let output = run_occ(
                &repo.path,
                &[
                    "commit",
                    "--stdin",
                    "--dry-run",
                    "--text",
                    "--backend",
                    backend,
                    "--mode",
                    mode,
                    config[0],
                    config[1],
                ],
                Some(&diff),
            );
            assert!(
                output.status.success(),
                "commit failed for backend={backend} mode={mode}: {}",
                stderr(&output)
            );
            let message = stdout(&output);
            assert_commit_shape(&message, mode == "conventional");
            append_response_log(
                "occ",
                &format!("artifacts_commit_{mode}"),
                "commit",
                backend,
                &message,
            );
        }
    }
}

#[test]
fn refine_generates_valid_conventional_output_across_backends() {
    let Some(env) = load_env() else { return };
    let repo = FixtureRepo::new("e2e-cli-refine");
    let diff = repo.staged_diff();
    let config = config_arg(&env.config_path);
    let seed = "feat: update helper";

    for backend in &env.active_backends {
        let output = run_occ(
            &repo.path,
            &[
                "commit",
                "--stdin",
                "--dry-run",
                "--text",
                "--backend",
                backend,
                "--mode",
                "conventional",
                "--refine",
                seed,
                "--feedback",
                "make it shorter and mention subtraction",
                config[0],
                config[1],
            ],
            Some(&diff),
        );
        assert!(
            output.status.success(),
            "refine failed for backend={backend}: {}",
            stderr(&output)
        );
        let message = stdout(&output);
        assert_ne!(message, seed, "refine should change the message");
        assert_commit_shape(&message, true);
    }
}

#[test]
fn artifacts_branch_dry_run_generates_slug_across_backends() {
    let Some(env) = load_env() else { return };
    let repo = FixtureRepo::new("e2e-cli-branch");
    let config = config_arg(&env.config_path);

    for backend in &env.active_backends {
        let output = run_occ(
            &repo.path,
            &[
                "branch",
                "--dry-run",
                "--text",
                "--backend",
                backend,
                "--mode",
                "conventional",
                config[0],
                config[1],
            ],
            None,
        );
        assert!(
            output.status.success(),
            "branch failed for backend={backend}: {}",
            stderr(&output)
        );
        let branch_name = stdout(&output);
        assert_branch_shape(&branch_name);
        append_response_log("occ", "artifacts_branch", "branch", backend, &branch_name);
    }
}

#[test]
fn artifacts_pr_generation_produces_structured_title_and_body_across_backends() {
    let Some(env) = load_env() else { return };
    let repo = FixtureRepo::new("e2e-cli-pr");
    let config = config_arg(&env.config_path);

    for backend in &env.active_backends {
        let output = run_occ(
            &repo.path,
            &["pr", "--text", "--backend", backend, config[0], config[1]],
            None,
        );
        assert!(
            output.status.success(),
            "pr failed for backend={backend}: {}",
            stderr(&output)
        );
        let draft = stdout(&output);
        assert_pr_shape(&draft);
        append_response_log("occ", "artifacts_pr", "pr", backend, &draft);
    }
}

#[test]
fn artifacts_changelog_generation_produces_sections_across_backends() {
    let Some(env) = load_env() else { return };
    let repo = FixtureRepo::new("e2e-cli-changelog");
    let config = config_arg(&env.config_path);

    for backend in &env.active_backends {
        let output = run_occ(
            &repo.path,
            &[
                "changelog",
                "--text",
                "--backend",
                backend,
                config[0],
                config[1],
            ],
            None,
        );
        assert!(
            output.status.success(),
            "changelog failed for backend={backend}: {}",
            stderr(&output)
        );
        let entry = stdout(&output);
        assert_changelog_shape(&entry);
        append_response_log("occ", "artifacts_changelog", "changelog", backend, &entry);
    }
}

#[test]
fn guard_install_status_and_uninstall_manage_repo_hooks_path() {
    let Some(_env) = load_env() else { return };
    let repo = FixtureRepo::new("e2e-cli-guard");

    let install = run_occ(&repo.path, &["guard", "install"], None);
    assert!(
        install.status.success(),
        "guard install failed: {}",
        stderr(&install)
    );
    assert!(
        repo.path.join(".git/occ/hooks/prepare-commit-msg").exists(),
        "guard hook wrapper should exist after install"
    );

    let status = run_occ(&repo.path, &["guard", "status"], None);
    assert!(
        status.status.success(),
        "guard status failed: {}",
        stderr(&status)
    );
    assert!(stdout(&status).contains("OpenCodeCommit guard: installed"));

    let uninstall = run_occ(&repo.path, &["guard", "uninstall"], None);
    assert!(
        uninstall.status.success(),
        "guard uninstall failed: {}",
        stderr(&uninstall)
    );
    assert!(
        !repo.path.join(".git/occ").exists(),
        "guard state should be removed after uninstall"
    );
}

#[test]
fn guard_rewrites_raw_git_commit_message_and_no_verify() {
    let repo = FixtureRepo::new("e2e-cli-guard-rewrite");
    let fake = fake_opencode(&repo, "feat: guard generated message");
    let config = fake_config(&repo, &fake);

    let install = run_occ(&repo.path, &["guard", "install"], None);
    assert!(
        install.status.success(),
        "guard install failed: {}",
        stderr(&install)
    );

    let commit = run_git_commit(&repo, &["-m", "agent message"], &config);
    assert!(
        commit.status.success(),
        "raw git commit failed: {}",
        String::from_utf8_lossy(&commit.stderr)
    );
    assert_eq!(last_commit_message(&repo), "feat: guard generated message");

    fs::write(repo.path.join("docs/no-verify.md"), "no verify\n").expect("write file");
    Command::new("git")
        .args(["add", "docs/no-verify.md"])
        .current_dir(&repo.path)
        .output()
        .expect("stage file");

    let no_verify = run_git_commit(&repo, &["--no-verify", "-m", "agent no verify"], &config);
    assert!(
        no_verify.status.success(),
        "raw git commit --no-verify failed: {}",
        String::from_utf8_lossy(&no_verify.stderr)
    );
    assert_eq!(last_commit_message(&repo), "feat: guard generated message");
}

#[test]
fn guard_allow_next_preserves_manual_git_commit_message() {
    let repo = FixtureRepo::new("e2e-cli-guard-allow-next");
    let fake = fake_opencode(&repo, "fix: hook should not rewrite");
    let config = fake_config(&repo, &fake);

    let install = run_occ(&repo.path, &["guard", "install"], None);
    assert!(
        install.status.success(),
        "guard install failed: {}",
        stderr(&install)
    );

    let allow = run_occ(&repo.path, &["guard", "allow-next", "--manual"], None);
    assert!(
        allow.status.success(),
        "guard allow-next failed: {}",
        stderr(&allow)
    );
    let commit = run_git_commit(&repo, &["-m", "human manual message"], &config);
    assert!(
        commit.status.success(),
        "manual git commit failed: {}",
        String::from_utf8_lossy(&commit.stderr)
    );
    assert_eq!(last_commit_message(&repo), "human manual message");
}

#[test]
fn scan_detects_blocking_secret_from_stdin() {
    let Some(_env) = load_env() else { return };
    let repo = FixtureRepo::new("e2e-cli-scan");
    let diff = "diff --git a/.env b/.env\n--- a/.env\n+++ b/.env\n@@ -0,0 +1 @@\n+AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE\n";

    let output = run_occ(
        &repo.path,
        &[
            "scan",
            "--stdin",
            "--format",
            "text",
            "--enforcement",
            "block-high",
        ],
        Some(diff),
    );
    assert_eq!(
        output.status.code(),
        Some(2),
        "scan should block the secret"
    );
    assert!(stdout(&output).contains("AKIAIOSFODNN7EXAMPLE"));
}

#[test]
fn unreachable_custom_endpoint_fails_cleanly() {
    let Some(env) = load_env() else { return };
    if !env
        .active_backends
        .iter()
        .any(|backend| backend == "custom-api")
    {
        return;
    }

    let repo = FixtureRepo::new("e2e-cli-unreachable");
    let diff = repo.staged_diff();
    let broken_config = repo.path.join("broken.toml");
    std::fs::write(
        &broken_config,
        "backend = \"custom-api\"\nbackend-order = [\"custom-api\"]\n[api.custom]\nmodel = \"test-model\"\nendpoint = \"http://127.0.0.1:1\"\nkey-env = \"\"\n",
    )
    .expect("write broken config");

    let output = run_occ(
        &repo.path,
        &[
            "commit",
            "--stdin",
            "--dry-run",
            "--text",
            "--backend",
            "custom-api",
            "--config",
            broken_config.to_str().expect("utf8 config path"),
        ],
        Some(&diff),
    );
    assert!(!output.status.success(), "broken endpoint should fail");
    assert!(
        stderr(&output).contains("request failed") || stderr(&output).contains("backend error"),
        "unexpected stderr: {}",
        stderr(&output)
    );
}
