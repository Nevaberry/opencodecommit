mod common;

use std::process::Command;
use std::time::Duration;

use common::{FixtureRepo, TUI_BACKENDS, load_env, occ_bin};
use expectrl::session::OsSession;
use expectrl::{ControlCode, Eof, Expect, Regex, Session};

fn backend_label(backend: &str) -> &'static str {
    match backend {
        "opencode" => "OpenCode CLI",
        "claude" => "Claude Code CLI",
        "codex" => "Codex CLI",
        "gemini" => "Gemini CLI",
        "openai-api" => "OpenAI API",
        "anthropic-api" => "Anthropic API",
        "gemini-api" => "Gemini API",
        "openrouter-api" => "OpenRouter API",
        "opencode-api" => "OpenCode Zen API",
        "ollama-api" => "Ollama API",
        "lm-studio-api" => "LM Studio API",
        "custom-api" => "Custom API",
        other => panic!("unknown backend: {other}"),
    }
}

fn tui_expect_timeout(mode: &str) -> Duration {
    if mode == "staging" {
        Duration::from_secs(300)
    } else {
        Duration::from_secs(180)
    }
}

fn spawn_tui(
    repo: &FixtureRepo,
    config_path: &std::path::Path,
    expect_timeout: Duration,
) -> OsSession {
    let mut command = Command::new(occ_bin());
    command
        .current_dir(&repo.path)
        .arg("tui")
        .arg("--config")
        .arg(config_path)
        .env("NO_COLOR", "1")
        .env_remove("CLICOLOR")
        .env_remove("CLICOLOR_FORCE");

    if let Ok(backend) = std::env::var("OCC_E2E_TUI_BACKEND_OVERRIDE") {
        let backend = backend.trim();
        if !backend.is_empty() {
            command.arg("--backend").arg(backend);
        }
    }

    let mut session = Session::spawn(command).expect("spawn tui session");
    session
        .get_process_mut()
        .set_window_size(120, 30)
        .expect("resize tui session");
    session.set_expect_timeout(Some(expect_timeout));
    session
}

fn menu_backends(mode: &str, active_backends: &[String]) -> Vec<(&'static str, char)> {
    if mode == "staging" {
        return TUI_BACKENDS.to_vec();
    }

    TUI_BACKENDS
        .iter()
        .copied()
        .filter(|(backend, _)| active_backends.iter().any(|value| value == backend))
        .collect()
}

fn targeted_single_backend(mode: &str, active_backends: &[String]) -> bool {
    mode != "staging" && active_backends.len() == 1
}

#[test]
fn tui_core_buttons_and_sidebar_work_in_a_real_pty() {
    let Some(env) = load_env() else { return };
    let repo = FixtureRepo::new("e2e-tui-core");
    let mut session = spawn_tui(&repo, &env.config_path, tui_expect_timeout(&env.mode));
    let single_backend = targeted_single_backend(&env.mode, &env.active_backends);

    session.expect("OpenCodeCommit").unwrap();
    session.expect("1 Commit").unwrap();
    session.expect("7 PR").unwrap();

    for _ in 0..8 {
        session.send(ControlCode::HT).unwrap();
    }
    session.send("j").unwrap();
    session.send(" ").unwrap();
    session.expect(Regex("(Staged|Unstaged) .*\\.")).unwrap();

    session.send("1").unwrap();
    session.expect("Generated commit message").unwrap();
    if !single_backend {
        session.send("s").unwrap();
        session.expect("Shortened commit message").unwrap();
    }
    session.send("c").unwrap();
    session.expect("Committed:").unwrap();

    session.send("2").unwrap();
    session.expect("BRANCH NAME PREVIEW").unwrap();
    session.send("c").unwrap();
    session.expect("Switched to new branch").unwrap();

    session.send("3").unwrap();
    session.expect("Generated PR preview").unwrap();
    if !single_backend {
        session.send("p").unwrap();
        session
            .expect(Regex("PR copied to clipboard\\.|Clipboard copy failed"))
            .unwrap();
        session.send("r").unwrap();
        session.expect("Generated PR preview").unwrap();
    }
    session.send(ControlCode::ESC).unwrap();

    session.send("4").unwrap();
    session.expect("SAFETY SETTINGS").unwrap();
    session.send("i").unwrap();
    session.expect("[y Yes]").unwrap();
    session.send("y").unwrap();
    session.expect("installed prepare-commit-msg hook").unwrap();

    session.send("4").unwrap();
    session.expect("SAFETY SETTINGS").unwrap();
    session.send("u").unwrap();
    session.expect("[y Yes]").unwrap();
    session.send("y").unwrap();
    session
        .expect("uninstalled prepare-commit-msg hook")
        .unwrap();

    session.send("4").unwrap();
    session.expect("SAFETY SETTINGS").unwrap();
    session.send("h").unwrap();
    session.expect("Applied human sensitive profile").unwrap();

    session.send("4").unwrap();
    session.expect("SAFETY SETTINGS").unwrap();
    session.send("a").unwrap();
    session
        .expect("Applied strict-agent sensitive profile")
        .unwrap();

    session.send("q").unwrap();
    session.expect(Eof).unwrap();
}

#[test]
fn tui_backend_selector_covers_the_expected_entries() {
    let Some(env) = load_env() else { return };
    let repo = FixtureRepo::new("e2e-tui-backend-selector");
    let mut session = spawn_tui(&repo, &env.config_path, tui_expect_timeout(&env.mode));

    session.expect("OpenCodeCommit").unwrap();
    let backends = menu_backends(&env.mode, &env.active_backends);

    for (index, (backend, key)) in backends.iter().enumerate() {
        session.send("5").unwrap();
        session.expect("BACKEND SELECTOR").unwrap();
        session.send(key.to_string()).unwrap();
        session
            .expect(format!("Backend set to {}.", backend_label(backend)))
            .unwrap();
        if index + 1 == backends.len() {
            break;
        }
    }

    session.send("q").unwrap();
    session.expect(Eof).unwrap();
}

#[test]
fn tui_one_shot_backend_menus_run_every_enabled_backend() {
    let Some(env) = load_env() else { return };
    let repo = FixtureRepo::new("e2e-tui-backend-menus");
    let mut session = spawn_tui(&repo, &env.config_path, tui_expect_timeout(&env.mode));

    session.expect("OpenCodeCommit").unwrap();
    let backends = menu_backends(&env.mode, &env.active_backends);

    if targeted_single_backend(&env.mode, &env.active_backends) {
        session.send("6").unwrap();
        session.expect("COMMIT BACKEND SELECTOR").unwrap();
        session.send(ControlCode::ESC).unwrap();

        session.send("7").unwrap();
        session.expect("PR BACKEND SELECTOR").unwrap();
        session.send(ControlCode::ESC).unwrap();

        session.send("q").unwrap();
        session.expect(Eof).unwrap();
        return;
    }

    for (_, key) in &backends {
        session.send("6").unwrap();
        session.expect("COMMIT BACKEND SELECTOR").unwrap();
        session.send(key.to_string()).unwrap();
        session.expect("Generated commit message").unwrap();
        session.send(ControlCode::ESC).unwrap();
    }

    for (_, key) in &backends {
        session.send("7").unwrap();
        session.expect("PR BACKEND SELECTOR").unwrap();
        session.send(key.to_string()).unwrap();
        session.expect("Generated PR preview").unwrap();
        session.send(ControlCode::ESC).unwrap();
    }

    session.send("q").unwrap();
    session.expect(Eof).unwrap();
}

#[test]
fn artifacts_tui_generation_smoke() {
    let Some(env) = load_env() else { return };
    let repo = FixtureRepo::new("e2e-tui-artifacts");
    let mut session = spawn_tui(&repo, &env.config_path, tui_expect_timeout(&env.mode));

    session.expect("OpenCodeCommit").unwrap();

    session.send("1").unwrap();
    session.expect("Generated commit message").unwrap();

    session.send("2").unwrap();
    session.expect("BRANCH NAME PREVIEW").unwrap();

    session.send("3").unwrap();
    session.expect("PR PREVIEW").unwrap();

    session.send("q").unwrap();
    session.expect(Eof).unwrap();
}

#[test]
fn tui_targeted_single_backend_smoke() {
    let Some(env) = load_env() else { return };
    if !targeted_single_backend(&env.mode, &env.active_backends) {
        return;
    }

    let repo = FixtureRepo::new("e2e-tui-targeted-single");
    let mut session = spawn_tui(&repo, &env.config_path, tui_expect_timeout(&env.mode));

    session.expect("OpenCodeCommit").unwrap();
    session.expect("1 Commit").unwrap();
    session.expect("2 Branch").unwrap();
    session.expect("3 PR").unwrap();
    session.expect("4 Safety Hook").unwrap();
    session.expect("5 Backend").unwrap();
    session.expect("6 Commit").unwrap();
    session.expect("7 PR").unwrap();

    session.send("1").unwrap();
    session.expect("Generated commit message").unwrap();

    session.send("2").unwrap();
    session.expect("BRANCH NAME PREVIEW").unwrap();

    session.send("3").unwrap();
    session.expect("PR PREVIEW").unwrap();

    session.send("4").unwrap();
    session.expect("SAFETY SETTINGS").unwrap();
    session.send(ControlCode::ESC).unwrap();

    session.send("5").unwrap();
    session.expect("BACKEND SELECTOR").unwrap();
    session.send(ControlCode::ESC).unwrap();

    session.send("q").unwrap();
    session.expect(Eof).unwrap();

    let repo = FixtureRepo::new("e2e-tui-targeted-single-commit-menu");
    let mut session = spawn_tui(&repo, &env.config_path, tui_expect_timeout(&env.mode));
    session.expect("OpenCodeCommit").unwrap();
    session.send("6").unwrap();
    session.expect("COMMIT BACKEND SELECTOR").unwrap();
    session.send(ControlCode::ESC).unwrap();
    session.send("q").unwrap();
    session.expect(Eof).unwrap();

    let repo = FixtureRepo::new("e2e-tui-targeted-single-pr-menu");
    let mut session = spawn_tui(&repo, &env.config_path, tui_expect_timeout(&env.mode));
    session.expect("OpenCodeCommit").unwrap();
    session.send("7").unwrap();
    session.expect("PR BACKEND SELECTOR").unwrap();
    session.send(ControlCode::ESC).unwrap();
    session.send("q").unwrap();
    session.expect(Eof).unwrap();
}
