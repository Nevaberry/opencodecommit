#![allow(dead_code)]
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use regex::Regex;

#[derive(Debug, Clone)]
pub struct E2eEnv {
    pub mode: String,
    pub config_path: PathBuf,
    pub active_backends: Vec<String>,
}

pub fn load_env() -> Option<E2eEnv> {
    let mode = env::var("OCC_E2E_MODE").ok()?;
    let config_path = PathBuf::from(env::var("OCC_E2E_CONFIG_PATH").ok()?);
    let active_backends = env::var("OCC_E2E_ACTIVE_BACKENDS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    Some(E2eEnv {
        mode,
        config_path,
        active_backends,
    })
}

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

pub fn occ_bin() -> PathBuf {
    env::var_os("CARGO_BIN_EXE_occ")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_root().join("target/debug/occ"))
}

fn unique_path(prefix: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    env::temp_dir().join(format!("occ-{prefix}-{}-{ts}", std::process::id()))
}

fn run_git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(repo)
        .env("GIT_AUTHOR_NAME", "OpenCodeCommit E2E")
        .env("GIT_AUTHOR_EMAIL", "e2e@example.com")
        .env("GIT_COMMITTER_NAME", "OpenCodeCommit E2E")
        .env("GIT_COMMITTER_EMAIL", "e2e@example.com")
        .status()
        .expect("git command to run");
    assert!(status.success(), "git {:?} failed", args);
}

fn git_stdout(repo: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .expect("git command to run");
    assert!(output.status.success(), "git {:?} failed", args);
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

#[derive(Debug)]
pub struct FixtureRepo {
    pub path: PathBuf,
}

impl FixtureRepo {
    pub fn new(name: &str) -> Self {
        let path = unique_path(name);
        fs::create_dir_all(path.join("src")).expect("fixture src dir");
        fs::create_dir_all(path.join("docs")).expect("fixture docs dir");

        run_git(&path, &["init", "-q"]);
        run_git(&path, &["config", "user.name", "OpenCodeCommit E2E"]);
        run_git(&path, &["config", "user.email", "e2e@example.com"]);

        fs::write(
            path.join("src/app.ts"),
            "export function add(left: number, right: number): number {\n  return left + right\n}\n",
        )
        .expect("seed app.ts");
        fs::write(path.join("README.md"), "# OpenCodeCommit E2E Fixture\n")
            .expect("seed README");

        run_git(&path, &["add", "README.md", "src/app.ts"]);
        run_git(&path, &["commit", "-q", "-m", "chore: seed e2e fixture"]);
        run_git(&path, &["checkout", "-q", "-b", "feature/e2e-coverage"]);

        fs::write(
            path.join("src/app.ts"),
            "export function add(left: number, right: number): number {\n  return left + right\n}\n\nexport function subtract(left: number, right: number): number {\n  return left - right\n}\n",
        )
        .expect("staged app.ts");
        fs::write(
            path.join("docs/notes.md"),
            "- add subtract helper\n- document staging verification\n",
        )
        .expect("staged notes.md");
        run_git(&path, &["add", "src/app.ts", "docs/notes.md"]);

        fs::write(
            path.join("src/app.ts"),
            "export function add(left: number, right: number): number {\n  return left + right\n}\n\nexport function subtract(left: number, right: number): number {\n  return left - right\n}\n\nexport function multiply(left: number, right: number): number {\n  return left * right\n}\n",
        )
        .expect("unstaged app.ts");

        Self { path }
    }

    pub fn staged_diff(&self) -> String {
        git_stdout(&self.path, &["diff", "--cached"])
    }

    pub fn hook_path(&self) -> PathBuf {
        self.path.join(".git/hooks/prepare-commit-msg")
    }
}

impl Drop for FixtureRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub fn run_occ(repo: &Path, args: &[&str], stdin: Option<&str>) -> Output {
    let mut command = Command::new(occ_bin());
    command
        .current_dir(repo)
        .args(args)
        .env("NO_COLOR", "1")
        .env_remove("CLICOLOR")
        .env_remove("CLICOLOR_FORCE")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if stdin.is_some() {
        command.stdin(Stdio::piped());
    }

    let mut child = command.spawn().expect("spawn occ");
    if let Some(input) = stdin {
        use std::io::Write as _;
        child
            .stdin
            .as_mut()
            .expect("stdin pipe")
            .write_all(input.as_bytes())
            .expect("write stdin");
    }

    child.wait_with_output().expect("wait for occ")
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).trim().to_owned()
}

pub fn assert_commit_shape(message: &str, conventional: bool) {
    let trimmed = message.trim();
    assert!(!trimmed.is_empty(), "commit output was empty");
    let first_line = trimmed.lines().next().unwrap_or_default();
    assert!(first_line.len() <= 72, "subject too long: {first_line}");
    assert!(
        !first_line.ends_with('.'),
        "subject should not end with a period: {first_line}"
    );
    if conventional {
        let re = Regex::new(
            r"^(feat|fix|docs|style|refactor|test|chore|perf|security|revert)(\([^)]+\))?!?: .+",
        )
        .unwrap();
        assert!(re.is_match(first_line), "invalid conventional commit: {first_line}");
    }
}

pub fn assert_branch_shape(name: &str) {
    let re = Regex::new(r"^[a-z0-9][a-z0-9/_-]{2,79}$").unwrap();
    assert!(re.is_match(name.trim()), "invalid branch name: {name}");
}

pub fn assert_pr_shape(output: &str) {
    let mut sections = output.trim().splitn(2, "\n\n");
    let title = sections.next().unwrap_or_default().trim();
    let body = sections.next().unwrap_or_default().trim();
    assert!(!title.is_empty(), "PR title was empty");
    assert!(title.len() <= 80, "PR title too long: {title}");
    assert!(body.len() >= 20, "PR body too short: {body}");
    assert!(body.contains("## "), "PR body missing markdown heading: {body}");
}

pub fn assert_changelog_shape(output: &str) {
    let re = Regex::new(r"(?m)^(?:##\s+)?(Added|Changed|Fixed|Removed)\b").unwrap();
    assert!(re.is_match(output.trim()), "changelog missing section heading: {output}");
}

pub const TUI_BACKENDS: [(&str, char); 12] = [
    ("opencode", '1'),
    ("claude", '2'),
    ("codex", '3'),
    ("gemini", '4'),
    ("openai-api", '5'),
    ("anthropic-api", '6'),
    ("gemini-api", '7'),
    ("openrouter-api", '8'),
    ("opencode-api", '9'),
    ("ollama-api", 'a'),
    ("lm-studio-api", 'b'),
    ("custom-api", 'c'),
];
