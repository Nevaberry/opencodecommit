pub mod backend;
pub mod config;
pub mod context;
pub mod git;
pub mod prompt;
pub mod response;

use std::fmt;

/// Crate-level error type.
#[derive(Debug)]
pub enum Error {
    /// Git command failed or repo not found.
    Git(String),
    /// No changes found to generate a message from.
    NoChanges,
    /// AI backend not found or not executable.
    BackendNotFound(String),
    /// AI backend execution failed.
    BackendExecution(String),
    /// Backend timed out.
    BackendTimeout(u64),
    /// Configuration error.
    Config(String),
    /// IO error.
    Io(std::io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Git(msg) => write!(f, "git error: {msg}"),
            Error::NoChanges => write!(f, "no changes found — stage some changes first"),
            Error::BackendNotFound(backend) => {
                write!(f, "{backend} CLI not found — install it or set the path in config")
            }
            Error::BackendExecution(msg) => write!(f, "backend error: {msg}"),
            Error::BackendTimeout(secs) => write!(f, "backend timed out after {secs} seconds"),
            Error::Config(msg) => write!(f, "config error: {msg}"),
            Error::Io(err) => write!(f, "IO error: {err}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

// --- High-level public API ---

/// Generate a commit message from the current git repo state.
///
/// This is the main entry point for library users. It gathers context from git,
/// builds a prompt, executes the AI backend, and returns the formatted message.
pub fn generate_commit_message(cfg: &config::Config) -> Result<String> {
    let repo_root = git::get_repo_root()?;
    let mut context = context::gather_context(&repo_root, cfg.diff_source)?;

    if context.diff.len() > cfg.max_diff_length {
        context.diff = format!(
            "{}\n... (truncated)",
            &context.diff[..cfg.max_diff_length]
        );
    }

    let prompt = prompt::build_prompt(&context, cfg, Some(cfg.commit_mode));
    let cli_path = backend::detect_cli(cfg.backend, cfg.backend_cli_path())?;
    let invocation = backend::build_invocation(&cli_path, &prompt, cfg);
    let response = backend::exec_cli(&invocation)?;

    let message = match cfg.commit_mode {
        config::CommitMode::Adaptive | config::CommitMode::AdaptiveOneliner => {
            response::format_adaptive_message(&response)
        }
        config::CommitMode::Conventional | config::CommitMode::ConventionalOneliner => {
            let parsed = response::parse_response(&response);
            response::format_commit_message(&parsed, cfg)
        }
    };

    Ok(message)
}

/// Refine an existing commit message based on user feedback.
pub fn refine_commit_message(
    current_message: &str,
    feedback: &str,
    cfg: &config::Config,
) -> Result<String> {
    let repo_root = git::get_repo_root()?;
    let mut context = context::gather_context(&repo_root, cfg.diff_source)?;

    if context.diff.len() > cfg.max_diff_length {
        context.diff = format!(
            "{}\n... (truncated)",
            &context.diff[..cfg.max_diff_length]
        );
    }

    let prompt = prompt::build_refine_prompt(current_message, feedback, &context.diff, cfg);
    let cli_path = backend::detect_cli(cfg.backend, cfg.backend_cli_path())?;
    let invocation = backend::build_invocation(&cli_path, &prompt, cfg);
    let response = backend::exec_cli(&invocation)?;

    let parsed = response::parse_response(&response);
    Ok(response::format_commit_message(&parsed, cfg))
}
