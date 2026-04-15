mod common;

use std::path::PathBuf;

use common::{
    FixtureRepo, assert_branch_shape, assert_changelog_shape, assert_commit_shape,
    assert_pr_shape, load_env, run_occ, stderr, stdout,
};

fn config_arg(config_path: &PathBuf) -> [&str; 2] {
    ["--config", config_path.to_str().expect("utf8 config path")]
}

#[test]
fn commit_dry_run_generates_valid_output_across_backends() {
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
            assert_commit_shape(&stdout(&output), mode == "conventional");
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
fn branch_dry_run_generates_slug_across_backends() {
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
        assert_branch_shape(&stdout(&output));
    }
}

#[test]
fn pr_generation_produces_structured_title_and_body_across_backends() {
    let Some(env) = load_env() else { return };
    let repo = FixtureRepo::new("e2e-cli-pr");
    let config = config_arg(&env.config_path);

    for backend in &env.active_backends {
        let output = run_occ(
            &repo.path,
            &[
                "pr",
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
            "pr failed for backend={backend}: {}",
            stderr(&output)
        );
        assert_pr_shape(&stdout(&output));
    }
}

#[test]
fn changelog_generation_produces_sections_across_backends() {
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
        assert_changelog_shape(&stdout(&output));
    }
}

#[test]
fn hook_install_and_uninstall_touch_the_repo_hook() {
    let Some(_env) = load_env() else { return };
    let repo = FixtureRepo::new("e2e-cli-hook");

    let install = run_occ(&repo.path, &["hook", "install"], None);
    assert!(install.status.success(), "hook install failed: {}", stderr(&install));
    assert!(repo.hook_path().exists(), "hook should exist after install");

    let uninstall = run_occ(&repo.path, &["hook", "uninstall"], None);
    assert!(
        uninstall.status.success(),
        "hook uninstall failed: {}",
        stderr(&uninstall)
    );
    assert!(!repo.hook_path().exists(), "hook should be removed after uninstall");
}

#[test]
fn scan_detects_blocking_secret_from_stdin() {
    let Some(_env) = load_env() else { return };
    let repo = FixtureRepo::new("e2e-cli-scan");
    let diff = "diff --git a/.env b/.env\n--- a/.env\n+++ b/.env\n@@ -0,0 +1 @@\n+AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE\n";

    let output = run_occ(
        &repo.path,
        &["scan", "--stdin", "--format", "text", "--enforcement", "block-high"],
        Some(diff),
    );
    assert_eq!(output.status.code(), Some(2), "scan should block the secret");
    assert!(stdout(&output).contains("AKIAIOSFODNN7EXAMPLE"));
}

#[test]
fn unreachable_custom_endpoint_fails_cleanly() {
    let Some(env) = load_env() else { return };
    if !env.active_backends.iter().any(|backend| backend == "custom-api") {
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
