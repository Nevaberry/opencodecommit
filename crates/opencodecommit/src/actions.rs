use std::fmt;
use std::path::PathBuf;
use std::time::Instant;

use opencodecommit::backend::{build_invocation, detect_cli, exec_cli};
use opencodecommit::config::{BranchMode, CommitMode, Config, DiffSource};
use opencodecommit::context::{self, CommitContext};
use opencodecommit::git;
use opencodecommit::prompt::{
    build_branch_prompt, build_changelog_prompt, build_pr_prompt, build_prompt, build_refine_prompt,
};
use opencodecommit::response::{
    self, ParsedCommit, ParsedPr, format_adaptive_message, format_branch_name,
    format_commit_message, parse_pr_response, parse_response,
};
use opencodecommit::sensitive::SensitiveReport;

#[derive(Debug)]
pub enum ActionError {
    Occ(opencodecommit::Error),
    SensitiveContent(SensitiveReport),
    InvalidInput(String),
    Hook(String),
    NonTty(String),
}

impl fmt::Display for ActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActionError::Occ(err) => write!(f, "{err}"),
            ActionError::SensitiveContent(report) => write!(f, "{report}"),
            ActionError::InvalidInput(msg) => write!(f, "{msg}"),
            ActionError::Hook(msg) => write!(f, "{msg}"),
            ActionError::NonTty(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for ActionError {}

impl From<opencodecommit::Error> for ActionError {
    fn from(err: opencodecommit::Error) -> Self {
        Self::Occ(err)
    }
}

pub type Result<T> = std::result::Result<T, ActionError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffOrigin {
    Staged,
    WorkingTree,
    Stdin,
}

impl fmt::Display for DiffOrigin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiffOrigin::Staged => write!(f, "staged"),
            DiffOrigin::WorkingTree => write!(f, "working tree"),
            DiffOrigin::Stdin => write!(f, "stdin"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommitPreview {
    pub message: String,
    pub parsed: ParsedCommit,
    pub provider: String,
    pub files_analyzed: usize,
    pub duration_ms: u128,
    pub changed_files: Vec<String>,
    pub branch: String,
    pub diff_origin: DiffOrigin,
}

#[derive(Debug, Clone)]
pub struct CommitRequest {
    pub refine: Option<String>,
    pub feedback: Option<String>,
    pub stdin_diff: Option<String>,
    pub allow_sensitive: bool,
}

#[derive(Debug, Clone)]
pub struct CommitResult {
    pub git_output: String,
    pub staged_all: bool,
}

#[derive(Debug, Clone)]
pub struct BranchPreview {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct BranchResult {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct PrPreview {
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct ChangelogPreview {
    pub entry: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookOperation {
    Install,
    Uninstall,
}

#[derive(Debug, Clone)]
pub struct RepoSummary {
    pub repo_name: String,
    pub repo_root: PathBuf,
    pub branch: String,
    pub staged_files: usize,
    pub unstaged_files: usize,
    pub active_language: String,
    pub backend_label: &'static str,
    pub backend_path: Option<PathBuf>,
    pub backend_error: Option<String>,
}

fn backend_label(backend: opencodecommit::config::CliBackend) -> &'static str {
    match backend {
        opencodecommit::config::CliBackend::Opencode => "OpenCode CLI",
        opencodecommit::config::CliBackend::Claude => "Claude Code CLI",
        opencodecommit::config::CliBackend::Codex => "Codex CLI",
        opencodecommit::config::CliBackend::Gemini => "Gemini CLI",
    }
}

fn truncate_diff(context: &mut CommitContext, max_diff_length: usize) {
    if context.diff.len() > max_diff_length {
        context.diff = format!("{}\n... (truncated)", &context.diff[..max_diff_length]);
    }
}

fn infer_diff_origin(source: DiffSource, repo_root: &std::path::Path) -> DiffOrigin {
    match source {
        DiffSource::Staged => DiffOrigin::Staged,
        DiffSource::All => DiffOrigin::WorkingTree,
        DiffSource::Auto => match git::get_diff(DiffSource::Staged, repo_root) {
            Ok(diff) if !diff.is_empty() => DiffOrigin::Staged,
            _ => DiffOrigin::WorkingTree,
        },
    }
}

fn load_commit_context(
    config: &Config,
    stdin_diff: Option<&str>,
) -> Result<(CommitContext, DiffOrigin)> {
    let repo_root = git::get_repo_root()?;

    if let Some(diff) = stdin_diff {
        if diff.trim().is_empty() {
            return Err(ActionError::InvalidInput("empty stdin".to_owned()));
        }

        let changed_files = context::extract_changed_file_paths(diff);
        let sensitive_findings = context::detect_sensitive_findings(diff, &changed_files);
        let has_sensitive = !sensitive_findings.is_empty();
        let branch = git::get_branch_name(&repo_root).unwrap_or_else(|_| "unknown".to_owned());
        let recent = git::get_recent_commits(&repo_root, 10).unwrap_or_default();

        return Ok((
            CommitContext {
                diff: diff.to_owned(),
                recent_commits: recent,
                branch,
                file_contents: vec![],
                changed_files,
                sensitive_findings,
                has_sensitive_content: has_sensitive,
            },
            DiffOrigin::Stdin,
        ));
    }

    let context = opencodecommit::context::gather_context(&repo_root, config.diff_source)?;
    let diff_origin = infer_diff_origin(config.diff_source, &repo_root);
    Ok((context, diff_origin))
}

pub fn generate_commit_preview(config: &Config, request: &CommitRequest) -> Result<CommitPreview> {
    let (mut context, diff_origin) = load_commit_context(config, request.stdin_diff.as_deref())?;

    if context.has_sensitive_content && !request.allow_sensitive {
        return Err(ActionError::SensitiveContent(
            SensitiveReport::from_findings(context.sensitive_findings.clone()),
        ));
    }

    truncate_diff(&mut context, config.max_diff_length);

    let prompt = if let Some(current_message) = request.refine.as_deref() {
        let feedback = request
            .feedback
            .as_deref()
            .unwrap_or(&config.refine.default_feedback);
        build_refine_prompt(current_message, feedback, &context.diff, config)
    } else {
        build_prompt(&context, config, Some(config.commit_mode))
    };

    let cli_path = detect_cli(config.backend, config.backend_cli_path())?;
    let invocation = build_invocation(&cli_path, &prompt, config);
    let start = Instant::now();
    let response = exec_cli(&invocation)?;
    let duration_ms = start.elapsed().as_millis();

    let message = match config.commit_mode {
        CommitMode::Adaptive | CommitMode::AdaptiveOneliner => format_adaptive_message(&response),
        CommitMode::Conventional | CommitMode::ConventionalOneliner => {
            let parsed = parse_response(&response);
            format_commit_message(&parsed, config)
        }
    };
    let parsed = parse_response(&response);

    Ok(CommitPreview {
        message,
        parsed,
        provider: format!("{:?}", config.backend).to_lowercase(),
        files_analyzed: context.changed_files.len(),
        duration_ms,
        changed_files: context.changed_files,
        branch: context.branch,
        diff_origin,
    })
}

pub fn commit_message(message: &str, used_stdin: bool) -> Result<CommitResult> {
    let repo_root = git::get_repo_root()?;
    let mut staged_all = false;

    if !used_stdin {
        let staged = git::get_diff(DiffSource::Staged, &repo_root);
        let had_staged = staged.is_ok() && staged.as_ref().is_ok_and(|diff| !diff.is_empty());
        if !had_staged {
            git::stage_all(&repo_root)?;
            staged_all = true;
        }
    }

    let git_output = git::git_commit(&repo_root, message)?;
    Ok(CommitResult {
        git_output,
        staged_all,
    })
}

pub fn generate_branch_preview(
    config: &Config,
    description: Option<&str>,
    branch_mode: BranchMode,
) -> Result<BranchPreview> {
    let repo_root = git::get_repo_root()?;

    let diff = if description.is_none() {
        Some(git::get_diff(config.diff_source, &repo_root)?)
    } else {
        None
    };

    let existing_branches = if branch_mode == BranchMode::Adaptive {
        git::get_recent_branch_names(&repo_root, 20).unwrap_or_default()
    } else {
        vec![]
    };

    let prompt = build_branch_prompt(
        description.unwrap_or(""),
        diff.as_deref(),
        config,
        branch_mode,
        &existing_branches,
    );
    let cli_path = detect_cli(config.backend, config.backend_cli_path())?;
    let invocation = build_invocation(&cli_path, &prompt, config);
    let response = exec_cli(&invocation)?;

    Ok(BranchPreview {
        name: format_branch_name(&response),
    })
}

pub fn create_branch(name: &str) -> Result<BranchResult> {
    let repo_root = git::get_repo_root()?;
    git::create_and_checkout_branch(&repo_root, name)?;
    Ok(BranchResult {
        name: name.to_owned(),
    })
}

fn build_context_preview(config: &Config) -> Result<CommitContext> {
    let repo_root = git::get_repo_root()?;
    let mut context = opencodecommit::context::gather_context(&repo_root, config.diff_source)?;
    truncate_diff(&mut context, config.max_diff_length);
    Ok(context)
}

pub fn generate_pr_preview(config: &Config) -> Result<PrPreview> {
    let context = build_context_preview(config)?;
    let prompt = build_pr_prompt(&context, config);
    let cli_path = detect_cli(config.backend, config.backend_cli_path())?;
    let invocation = build_invocation(&cli_path, &prompt, config);
    let response = exec_cli(&invocation)?;
    let parsed: ParsedPr = parse_pr_response(&response);
    Ok(PrPreview {
        title: parsed.title,
        body: parsed.body,
    })
}

pub fn generate_changelog_preview(config: &Config) -> Result<ChangelogPreview> {
    let context = build_context_preview(config)?;
    let prompt = build_changelog_prompt(&context, config);
    let cli_path = detect_cli(config.backend, config.backend_cli_path())?;
    let invocation = build_invocation(&cli_path, &prompt, config);
    let response = exec_cli(&invocation)?;
    Ok(ChangelogPreview {
        entry: response::sanitize_response(&response),
    })
}

pub fn run_hook(action: HookOperation) -> Result<String> {
    let repo_root = git::get_repo_root()?;
    let git_dir = git::get_git_dir(&repo_root)?;
    let hooks_dir = git_dir.join("hooks");
    let hook_path = hooks_dir.join("prepare-commit-msg");

    match action {
        HookOperation::Install => {
            std::fs::create_dir_all(&hooks_dir)
                .map_err(|e| ActionError::Hook(format!("failed to create hooks dir: {e}")))?;

            let hook_script = r#"#!/bin/sh
# Generated by opencodecommit
# This hook generates a commit message using AI when none is provided.

COMMIT_MSG_FILE="$1"
COMMIT_SOURCE="$2"

# Only run for regular commits (not merge, squash, etc.)
if [ -z "$COMMIT_SOURCE" ]; then
    # Check if the message file has a non-comment message already
    MSG=$(grep -v '^#' "$COMMIT_MSG_FILE" | grep -v '^$' | head -1)
    if [ -z "$MSG" ]; then
        GENERATED=$(opencodecommit commit 2>/dev/null)
        if [ $? -eq 0 ]; then
            # Extract message from JSON
            MESSAGE=$(echo "$GENERATED" | grep -o '"message":"[^"]*"' | head -1 | sed 's/"message":"//;s/"$//')
            if [ -n "$MESSAGE" ]; then
                echo "$MESSAGE" > "$COMMIT_MSG_FILE"
            fi
        fi
    fi
fi
"#;

            std::fs::write(&hook_path, hook_script)
                .map_err(|e| ActionError::Hook(format!("failed to write hook: {e}")))?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ =
                    std::fs::set_permissions(&hook_path, std::fs::Permissions::from_mode(0o755));
            }

            Ok(format!(
                "installed prepare-commit-msg hook at {}",
                hook_path.display()
            ))
        }
        HookOperation::Uninstall => {
            if hook_path.exists() {
                let content = std::fs::read_to_string(&hook_path).unwrap_or_default();
                if content.contains("opencodecommit") {
                    std::fs::remove_file(&hook_path)
                        .map_err(|e| ActionError::Hook(format!("failed to remove hook: {e}")))?;
                    Ok("uninstalled prepare-commit-msg hook".to_owned())
                } else {
                    Err(ActionError::Hook(
                        "prepare-commit-msg hook exists but was not installed by opencodecommit"
                            .to_owned(),
                    ))
                }
            } else {
                Ok("no prepare-commit-msg hook found".to_owned())
            }
        }
    }
}

pub fn load_repo_summary(config: &Config) -> Result<RepoSummary> {
    let repo_root = git::get_repo_root()?;
    let repo_name = repo_root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repository")
        .to_owned();
    let branch = git::get_branch_name(&repo_root)?;
    let staged_files = git::get_changed_files(DiffSource::Staged, &repo_root)?.len();
    let unstaged_files = git::get_unstaged_files(&repo_root)?.len();
    let backend_label = backend_label(config.backend);
    let (backend_path, backend_error) = match detect_cli(config.backend, config.backend_cli_path())
    {
        Ok(path) => (Some(path), None),
        Err(err) => (None, Some(err.to_string())),
    };

    Ok(RepoSummary {
        repo_name,
        repo_root,
        branch,
        staged_files,
        unstaged_files,
        active_language: config.active_language.clone(),
        backend_label,
        backend_path,
        backend_error,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;

    fn setup_repo(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("occ-actions-test-{}-{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let run = |args: &[&str]| {
            Command::new("git")
                .args(args)
                .current_dir(&dir)
                .env("GIT_AUTHOR_NAME", "Test")
                .env("GIT_AUTHOR_EMAIL", "test@test.com")
                .env("GIT_COMMITTER_NAME", "Test")
                .env("GIT_COMMITTER_EMAIL", "test@test.com")
                .output()
                .unwrap()
        };

        run(&["init"]);
        run(&["config", "user.email", "test@test.com"]);
        run(&["config", "user.name", "Test"]);
        fs::write(dir.join("README.md"), "# Hello").unwrap();
        run(&["add", "README.md"]);
        run(&["commit", "-m", "initial commit"]);

        dir
    }

    fn cleanup(dir: &std::path::Path) {
        let _ = fs::remove_dir_all(dir);
    }

    fn with_repo<T>(repo: &std::path::Path, f: impl FnOnce() -> T) -> T {
        let _lock = opencodecommit::TEST_CWD_LOCK.lock().unwrap();
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(repo).unwrap();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        std::env::set_current_dir(original).unwrap();
        match result {
            Ok(value) => value,
            Err(payload) => std::panic::resume_unwind(payload),
        }
    }

    fn fake_cli(script_name: &str, body: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "occ-fake-cli-{}-{}",
            std::process::id(),
            script_name
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(script_name);
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        }
        path
    }

    #[test]
    fn commit_preview_blocks_sensitive_content() {
        let repo = setup_repo("sensitive");
        let cli = fake_cli("opencode", "echo 'feat: ignored'");

        with_repo(&repo, || {
            let cfg = Config {
                cli_path: cli.to_string_lossy().to_string(),
                ..Config::default()
            };
            let request = CommitRequest {
                refine: None,
                feedback: None,
                stdin_diff: Some(
                    "diff --git a/.env b/.env\nnew file mode 100644\n--- /dev/null\n+++ b/.env\n+API_KEY=secret\n"
                        .to_owned(),
                ),
                allow_sensitive: false,
            };

            let err = generate_commit_preview(&cfg, &request).unwrap_err();
            assert!(matches!(err, ActionError::SensitiveContent(_)));
        });

        cleanup(&repo);
    }

    #[test]
    fn commit_message_auto_stages_when_needed() {
        let repo = setup_repo("commit-stage");
        fs::write(repo.join("file.txt"), "content").unwrap();

        let result = with_repo(&repo, || commit_message("feat: add file", false)).unwrap();
        assert!(result.staged_all);

        with_repo(&repo, || {
            let log = Command::new("git")
                .args(["log", "--oneline", "-n", "1"])
                .current_dir(&repo)
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&log.stdout);
            assert!(stdout.contains("feat: add file"));
        });

        cleanup(&repo);
    }

    #[test]
    fn repo_summary_reports_backend_error() {
        let repo = setup_repo("repo-summary");

        with_repo(&repo, || {
            let cfg = Config {
                cli_path: "/no/such/opencode".to_owned(),
                ..Config::default()
            };
            let summary = load_repo_summary(&cfg).unwrap();
            assert!(summary.repo_name.contains("repo-summary"));
            assert!(!summary.branch.is_empty());
            assert!(summary.backend_path.is_none());
            assert!(summary.backend_error.is_some());
        });

        cleanup(&repo);
    }
}
