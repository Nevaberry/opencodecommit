mod actions;
mod guard;
mod tui;
mod update;

use std::path::Path;
use std::process;

use clap::{Parser, Subcommand, ValueEnum};
use opencodecommit::config::{Backend, CommitMode, Config, DiffSource, SensitiveProfile};
use opencodecommit::scan::{self, ScanFormat};
use opencodecommit::sensitive::SensitiveEnforcement;

use crate::actions::{ActionError, BackendProgress, CommitRequest, HookOperation};

#[derive(Parser)]
#[command(
    name = "occ",
    version,
    about = "AI-powered git commit message generator"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a commit message from the current diff
    Commit {
        /// AI backend to use
        #[arg(long, value_enum, default_value_t = BackendArg::Opencode)]
        backend: BackendArg,

        /// AI provider (for opencode backend)
        #[arg(long)]
        provider: Option<String>,

        /// Model name
        #[arg(long)]
        model: Option<String>,

        /// Commit message mode
        #[arg(long, value_enum, default_value_t = CommitModeArg::Adaptive)]
        mode: CommitModeArg,

        /// Diff source
        #[arg(long, value_enum, default_value_t = DiffSourceArg::Auto)]
        diff_source: DiffSourceArg,

        /// Maximum diff length in characters
        #[arg(long)]
        max_diff_length: Option<usize>,

        /// Language label (e.g. "English", "Finnish", "Spanish", "Japanese")
        #[arg(long)]
        language: Option<String>,

        /// Include emojis in commit messages
        #[arg(long)]
        emoji: bool,

        /// Don't lowercase the first letter of the subject
        #[arg(long)]
        no_lowercase: bool,

        /// Commit message template (e.g. "{{type}}: {{message}}")
        #[arg(long)]
        template: Option<String>,

        /// Override path to the AI CLI binary
        #[arg(long)]
        cli_path: Option<String>,

        /// Path to config file
        #[arg(long)]
        config: Option<String>,

        /// Existing commit message to refine
        #[arg(long)]
        refine: Option<String>,

        /// Feedback for refining
        #[arg(long)]
        feedback: Option<String>,

        /// Custom prompt (overrides built-in prompts, use {diff} placeholder)
        #[arg(long)]
        custom_prompt: Option<String>,

        /// Custom type rules (conventional mode only)
        #[arg(long)]
        custom_type_rules: Option<String>,

        /// Custom commit message rules (conventional mode only)
        #[arg(long)]
        custom_message_rules: Option<String>,

        /// Plain text output instead of JSON (for human use)
        #[arg(long, short)]
        text: bool,

        /// Skip the sensitive content check
        #[arg(long)]
        allow_sensitive: bool,

        /// Read diff from stdin instead of git
        #[arg(long)]
        stdin: bool,

        /// Preview the message without committing
        #[arg(long)]
        dry_run: bool,
    },

    /// Generate a branch name from diff or description
    Branch {
        /// Optional description to generate branch name from
        description: Option<String>,

        #[arg(long, value_enum, default_value_t = BackendArg::Opencode)]
        backend: BackendArg,

        #[arg(long)]
        provider: Option<String>,

        #[arg(long)]
        model: Option<String>,

        #[arg(long)]
        cli_path: Option<String>,

        #[arg(long)]
        config: Option<String>,

        /// Plain text output instead of JSON
        #[arg(long, short)]
        text: bool,

        /// Preview the branch name without creating it
        #[arg(long)]
        dry_run: bool,

        /// Branch naming mode
        #[arg(long, value_enum, default_value_t = BranchModeArg::Conventional)]
        mode: BranchModeArg,
    },

    /// Generate a PR title and body
    Pr {
        #[arg(long, value_enum, default_value_t = BackendArg::Opencode)]
        backend: BackendArg,

        #[arg(long)]
        provider: Option<String>,

        #[arg(long)]
        model: Option<String>,

        #[arg(long)]
        cli_path: Option<String>,

        #[arg(long)]
        config: Option<String>,

        /// Base branch to diff against
        #[arg(long, default_value = "main")]
        base: String,

        /// Plain text output instead of JSON
        #[arg(long, short)]
        text: bool,
    },

    /// Install or uninstall git hooks
    Hook {
        #[command(subcommand)]
        action: HookAction,
    },

    /// Install or uninstall the transparent git commit guard
    Guard {
        #[command(subcommand)]
        action: GuardAction,
    },

    /// Launch the interactive terminal UI
    Tui {
        /// AI backend to use
        #[arg(long, value_enum)]
        backend: Option<BackendArg>,

        /// Path to config file
        #[arg(long)]
        config: Option<String>,
    },

    /// Update occ to the latest version
    #[command(alias = "upgrade")]
    Update,

    /// Generate a changelog entry
    Changelog {
        #[arg(long, value_enum, default_value_t = BackendArg::Opencode)]
        backend: BackendArg,

        #[arg(long)]
        provider: Option<String>,

        #[arg(long)]
        model: Option<String>,

        #[arg(long)]
        cli_path: Option<String>,

        #[arg(long)]
        config: Option<String>,

        /// Plain text output instead of JSON
        #[arg(long, short)]
        text: bool,
    },

    /// Scan a diff for sensitive content
    Scan {
        /// Read diff from file instead of git
        #[arg(long)]
        diff_file: Option<String>,

        /// Read diff from stdin
        #[arg(long)]
        stdin: bool,

        /// Diff source when using git
        #[arg(long, value_enum)]
        diff_source: Option<DiffSourceArg>,

        /// Enforcement level
        #[arg(long, value_enum, default_value_t = SensitiveEnforcementArg::BlockHigh)]
        enforcement: SensitiveEnforcementArg,

        /// Output format
        #[arg(long, value_enum, default_value_t = ScanFormatArg::Text)]
        format: ScanFormatArg,

        /// Output file instead of stdout
        #[arg(long)]
        output: Option<String>,

        /// Path to config file
        #[arg(long)]
        config: Option<String>,

        /// Additional allowlist TOML file
        #[arg(long)]
        allowlist: Option<String>,
    },

    #[command(hide = true)]
    Internal {
        #[command(subcommand)]
        action: InternalAction,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum BackendArg {
    Opencode,
    Claude,
    Codex,
    Gemini,
    OpenaiApi,
    AnthropicApi,
    GeminiApi,
    OpenrouterApi,
    OpencodeApi,
    OllamaApi,
    LmStudioApi,
    CustomApi,
}

impl BackendArg {
    fn to_config(&self) -> Backend {
        match self {
            BackendArg::Opencode => Backend::Opencode,
            BackendArg::Claude => Backend::Claude,
            BackendArg::Codex => Backend::Codex,
            BackendArg::Gemini => Backend::Gemini,
            BackendArg::OpenaiApi => Backend::OpenaiApi,
            BackendArg::AnthropicApi => Backend::AnthropicApi,
            BackendArg::GeminiApi => Backend::GeminiApi,
            BackendArg::OpenrouterApi => Backend::OpenrouterApi,
            BackendArg::OpencodeApi => Backend::OpencodeApi,
            BackendArg::OllamaApi => Backend::OllamaApi,
            BackendArg::LmStudioApi => Backend::LmStudioApi,
            BackendArg::CustomApi => Backend::CustomApi,
        }
    }
}

#[derive(Clone, ValueEnum)]
enum CommitModeArg {
    Adaptive,
    AdaptiveOneliner,
    Conventional,
    ConventionalOneliner,
}

impl CommitModeArg {
    fn to_config(&self) -> CommitMode {
        match self {
            CommitModeArg::Adaptive => CommitMode::Adaptive,
            CommitModeArg::AdaptiveOneliner => CommitMode::AdaptiveOneliner,
            CommitModeArg::Conventional => CommitMode::Conventional,
            CommitModeArg::ConventionalOneliner => CommitMode::ConventionalOneliner,
        }
    }
}

#[derive(Clone, ValueEnum)]
enum DiffSourceArg {
    Staged,
    All,
    Auto,
}

impl DiffSourceArg {
    fn to_config(&self) -> DiffSource {
        match self {
            DiffSourceArg::Staged => DiffSource::Staged,
            DiffSourceArg::All => DiffSource::All,
            DiffSourceArg::Auto => DiffSource::Auto,
        }
    }
}

#[derive(Clone, ValueEnum)]
enum BranchModeArg {
    Conventional,
    Adaptive,
}

impl BranchModeArg {
    fn to_config(&self) -> opencodecommit::config::BranchMode {
        match self {
            BranchModeArg::Conventional => opencodecommit::config::BranchMode::Conventional,
            BranchModeArg::Adaptive => opencodecommit::config::BranchMode::Adaptive,
        }
    }
}

#[derive(Clone, Copy, Subcommand)]
enum HookAction {
    /// Install prepare-commit-msg hook
    Install,
    /// Uninstall prepare-commit-msg hook
    Uninstall,
}

impl HookAction {
    fn to_operation(self) -> HookOperation {
        match self {
            HookAction::Install => HookOperation::Install,
            HookAction::Uninstall => HookOperation::Uninstall,
        }
    }
}

#[derive(Clone, Subcommand)]
enum GuardAction {
    /// Install the global guard via core.hooksPath
    Install {
        /// Install machine-wide
        #[arg(long)]
        global: bool,
    },
    /// Uninstall the global guard
    Uninstall {
        /// Uninstall machine-wide
        #[arg(long)]
        global: bool,
    },
    /// Apply a named sensitive-content profile to the config file
    Profile {
        #[arg(value_enum)]
        profile: SensitiveProfileArg,

        /// Optional config file path
        #[arg(long)]
        config: Option<String>,
    },
}

#[derive(Clone, Subcommand)]
enum InternalAction {
    #[command(name = "run-managed-hook", hide = true)]
    RunManagedHook {
        hook_name: String,
        #[arg(allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum SensitiveProfileArg {
    Human,
    StrictAgent,
}

impl SensitiveProfileArg {
    fn to_config(self) -> SensitiveProfile {
        match self {
            SensitiveProfileArg::Human => SensitiveProfile::Human,
            SensitiveProfileArg::StrictAgent => SensitiveProfile::StrictAgent,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum SensitiveEnforcementArg {
    Warn,
    BlockHigh,
    BlockAll,
    StrictHigh,
    StrictAll,
}

impl SensitiveEnforcementArg {
    fn to_config(self) -> SensitiveEnforcement {
        match self {
            SensitiveEnforcementArg::Warn => SensitiveEnforcement::Warn,
            SensitiveEnforcementArg::BlockHigh => SensitiveEnforcement::BlockHigh,
            SensitiveEnforcementArg::BlockAll => SensitiveEnforcement::BlockAll,
            SensitiveEnforcementArg::StrictHigh => SensitiveEnforcement::StrictHigh,
            SensitiveEnforcementArg::StrictAll => SensitiveEnforcement::StrictAll,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum ScanFormatArg {
    Text,
    Json,
    Sarif,
    GithubAnnotations,
}

impl ScanFormatArg {
    fn to_config(self) -> ScanFormat {
        match self {
            ScanFormatArg::Text => ScanFormat::Text,
            ScanFormatArg::Json => ScanFormat::Json,
            ScanFormatArg::Sarif => ScanFormat::Sarif,
            ScanFormatArg::GithubAnnotations => ScanFormat::GithubAnnotations,
        }
    }
}

fn json_error(message: impl ToString) {
    let output = serde_json::json!({
        "status": "error",
        "error": message.to_string(),
    });
    println!("{}", serde_json::to_string(&output).unwrap());
}

fn load_config_or_exit(config: Option<&str>, text: bool) -> Config {
    let config_path = config.map(Path::new);
    match Config::load_or_default(config_path) {
        Ok(cfg) => cfg,
        Err(err) => {
            if text {
                eprintln!("error: {err}");
                process::exit(1);
            }
            json_error(err);
            process::exit(3);
        }
    }
}

fn load_config_or_exit_plain(config: Option<&str>) -> Config {
    let config_path = config.map(Path::new);
    match Config::load_or_default(config_path) {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("error: {err}");
            process::exit(3);
        }
    }
}

fn apply_backend_overrides(
    config: &mut Config,
    backend: &BackendArg,
    provider: &Option<String>,
    model: &Option<String>,
    cli_path: &Option<String>,
) {
    config.backend = backend.to_config();
    config.backend_order = vec![config.backend];
    if let Some(provider) = provider {
        match config.backend {
            Backend::Opencode => config.provider = provider.clone(),
            Backend::Codex => config.codex_provider = provider.clone(),
            Backend::OpencodeApi
            | Backend::OpenaiApi
            | Backend::AnthropicApi
            | Backend::GeminiApi
            | Backend::OpenrouterApi
            | Backend::OllamaApi
            | Backend::LmStudioApi
            | Backend::CustomApi
            | Backend::Claude
            | Backend::Gemini => {}
        }
    }
    if let Some(model) = model {
        match config.backend {
            Backend::Opencode => config.model = model.clone(),
            Backend::Claude => config.claude_model = model.clone(),
            Backend::Codex => config.codex_model = model.clone(),
            Backend::Gemini => config.gemini_model = model.clone(),
            Backend::OpenaiApi => config.api.openai.model = model.clone(),
            Backend::AnthropicApi => config.api.anthropic.model = model.clone(),
            Backend::GeminiApi => config.api.gemini.model = model.clone(),
            Backend::OpenrouterApi => config.api.openrouter.model = model.clone(),
            Backend::OpencodeApi => config.api.opencode.model = model.clone(),
            Backend::OllamaApi => config.api.ollama.model = model.clone(),
            Backend::LmStudioApi => config.api.lm_studio.model = model.clone(),
            Backend::CustomApi => config.api.custom.model = model.clone(),
        }
    }
    if let Some(path) = cli_path {
        match config.backend {
            Backend::Opencode => config.cli_path = path.clone(),
            Backend::Claude => config.claude_path = path.clone(),
            Backend::Codex => config.codex_path = path.clone(),
            Backend::Gemini => config.gemini_path = path.clone(),
            Backend::OpenaiApi
            | Backend::AnthropicApi
            | Backend::GeminiApi
            | Backend::OpenrouterApi
            | Backend::OpencodeApi
            | Backend::OllamaApi
            | Backend::LmStudioApi
            | Backend::CustomApi => {}
        }
    }
}

fn set_language(config: &mut Config, language: &Option<String>) -> Result<(), String> {
    if let Some(label) = language {
        if config.languages.iter().any(|lang| lang.label == *label) {
            config.active_language = label.clone();
            return Ok(());
        }
        let available: Vec<&str> = config
            .languages
            .iter()
            .map(|lang| lang.label.as_str())
            .collect();
        return Err(format!(
            "unknown language \"{label}\". Available: {}",
            available.join(", ")
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn apply_commit_args(
    config: &mut Config,
    backend: &BackendArg,
    provider: &Option<String>,
    model: &Option<String>,
    mode: &CommitModeArg,
    diff_source: &DiffSourceArg,
    max_diff_length: Option<usize>,
    language: &Option<String>,
    emoji: bool,
    no_lowercase: bool,
    template: &Option<String>,
    cli_path: &Option<String>,
    custom_prompt: &Option<String>,
    custom_type_rules: &Option<String>,
    custom_message_rules: &Option<String>,
) -> Result<(), String> {
    apply_backend_overrides(config, backend, provider, model, cli_path);
    config.commit_mode = mode.to_config();
    config.diff_source = diff_source.to_config();

    if let Some(limit) = max_diff_length {
        config.max_diff_length = limit;
    }
    set_language(config, language)?;
    if emoji {
        config.use_emojis = true;
    }
    if no_lowercase {
        config.use_lower_case = false;
    }
    if let Some(template) = template {
        config.commit_template = template.clone();
    }
    if let Some(prompt) = custom_prompt {
        config.custom.prompt = prompt.clone();
    }
    if let Some(rules) = custom_type_rules {
        config.custom.type_rules = rules.clone();
    }
    if let Some(rules) = custom_message_rules {
        config.custom.commit_message_rules = rules.clone();
    }

    Ok(())
}

fn action_exit_code(err: &ActionError, stdin_mode: bool) -> i32 {
    match err {
        ActionError::SensitiveContent(_) => 5,
        ActionError::InvalidInput(_) if stdin_mode => 4,
        ActionError::Occ(
            opencodecommit::Error::BackendNotFound(_)
            | opencodecommit::Error::BackendExecution(_)
            | opencodecommit::Error::BackendTimeout(_),
        ) => 2,
        ActionError::Occ(opencodecommit::Error::NoChanges) => 1,
        _ => 3,
    }
}

fn cli_progress(text: bool) -> impl Fn(BackendProgress) {
    use std::io::Write as _;
    move |p| {
        if !text {
            return;
        }
        match p {
            BackendProgress::Trying(b) => {
                eprint!("Trying {b}... ");
                let _ = std::io::stderr().flush();
            }
            BackendProgress::Failed { backend, error } => {
                eprintln!("{backend} failed: {error}");
            }
        }
    }
}

fn handle_commit(
    config: &Config,
    refine: Option<String>,
    feedback: Option<String>,
    text: bool,
    use_stdin: bool,
    allow_sensitive: bool,
    dry_run: bool,
) {
    let stdin_diff = if use_stdin {
        use std::io::Read;

        let mut diff = String::new();
        if let Err(err) = std::io::stdin().read_to_string(&mut diff) {
            if text {
                eprintln!("error: failed to read stdin: {err}");
                process::exit(1);
            }
            json_error(format!("stdin: {err}"));
            process::exit(4);
        }
        Some(diff)
    } else {
        None
    };

    let request = CommitRequest {
        refine,
        feedback,
        stdin_diff,
        allow_sensitive,
    };

    let preview = match actions::generate_commit_preview_with_fallback(
        config,
        &request,
        cli_progress(text),
    ) {
        Ok(preview) => preview,
        Err(ActionError::SensitiveContent(report)) if !report.has_blocking_findings() => {
            eprintln!("{}", report.format_occ_commit_message());
            let mut retry = request.clone();
            retry.allow_sensitive = true;
            match actions::generate_commit_preview_with_fallback(config, &retry, cli_progress(text))
            {
                Ok(preview) => preview,
                Err(err) => {
                    if text {
                        eprintln!("error: {err}");
                        process::exit(action_exit_code(&err, use_stdin));
                    }
                    json_error(&err);
                    process::exit(action_exit_code(&err, use_stdin));
                }
            }
        }
        Err(err) => {
            if text {
                eprintln!("error: {err}");
                process::exit(action_exit_code(&err, use_stdin));
            }
            json_error(&err);
            process::exit(action_exit_code(&err, use_stdin));
        }
    };

    if dry_run {
        if text {
            println!("{}", preview.message);
        } else {
            let output = serde_json::json!({
                "status": "success",
                "message": preview.message,
                "committed": false,
                "type": preview.parsed.type_name,
                "description": preview.parsed.description,
                "provider": preview.provider,
                "files_analyzed": preview.files_analyzed,
                "duration_ms": preview.duration_ms,
                "backend_failures": preview.backend_failures,
            });
            println!("{}", serde_json::to_string(&output).unwrap());
        }
        return;
    }

    let result = match actions::commit_message(&preview.message, use_stdin) {
        Ok(result) => result,
        Err(err) => {
            if text {
                eprintln!("error: commit failed: {err}");
                process::exit(1);
            }
            json_error(format!("commit failed: {err}"));
            process::exit(3);
        }
    };

    if text {
        println!("{}", preview.message);
        eprintln!(
            "Committed: {}",
            result
                .git_output
                .lines()
                .next()
                .unwrap_or(&result.git_output)
        );
    } else {
        let output = serde_json::json!({
            "status": "success",
            "message": preview.message,
            "committed": true,
            "git_output": result.git_output,
            "type": preview.parsed.type_name,
            "description": preview.parsed.description,
            "provider": preview.provider,
            "files_analyzed": preview.files_analyzed,
            "duration_ms": preview.duration_ms,
            "backend_failures": preview.backend_failures,
        });
        println!("{}", serde_json::to_string(&output).unwrap());
    }
}

fn handle_branch(
    config: &Config,
    description: Option<String>,
    text: bool,
    dry_run: bool,
    mode: opencodecommit::config::BranchMode,
) {
    let preview = match actions::generate_branch_preview_with_fallback(
        config,
        description.as_deref(),
        mode,
        cli_progress(text),
    ) {
        Ok(preview) => preview,
        Err(err) => {
            if text {
                eprintln!("error: {err}");
                process::exit(1);
            }
            json_error(&err);
            process::exit(action_exit_code(&err, false));
        }
    };

    if dry_run {
        if text {
            println!("{}", preview.name);
        } else {
            let output = serde_json::json!({
                "status": "success",
                "name": preview.name,
                "created": false,
            });
            println!("{}", serde_json::to_string(&output).unwrap());
        }
        return;
    }

    let created = match actions::create_branch(&preview.name) {
        Ok(created) => created,
        Err(err) => {
            if text {
                eprintln!("error: failed to create branch: {err}");
                process::exit(1);
            }
            json_error(format!("failed to create branch: {err}"));
            process::exit(3);
        }
    };

    if text {
        println!("{}", created.name);
        eprintln!("Switched to new branch '{}'", created.name);
    } else {
        let output = serde_json::json!({
            "status": "success",
            "name": created.name,
            "created": true,
        });
        println!("{}", serde_json::to_string(&output).unwrap());
    }
}

fn handle_pr(config: &Config, text: bool, base: Option<&str>) {
    let preview = match actions::generate_pr_preview_with_fallback(config, base, cli_progress(text))
    {
        Ok(preview) => preview,
        Err(err) => {
            if text {
                eprintln!("error: {err}");
                process::exit(1);
            }
            json_error(&err);
            process::exit(action_exit_code(&err, false));
        }
    };

    if text {
        println!("{}\n\n{}", preview.title, preview.body);
    } else {
        let output = serde_json::json!({
            "status": "success",
            "title": preview.title,
            "body": preview.body,
        });
        println!("{}", serde_json::to_string(&output).unwrap());
    }
}

fn handle_changelog(config: &Config, text: bool) {
    let preview =
        match actions::generate_changelog_preview_with_fallback(config, cli_progress(text)) {
            Ok(preview) => preview,
            Err(err) => {
                if text {
                    eprintln!("error: {err}");
                    process::exit(1);
                }
                json_error(&err);
                process::exit(action_exit_code(&err, false));
            }
        };

    if text {
        println!("{}", preview.entry);
    } else {
        let output = serde_json::json!({
            "status": "success",
            "entry": preview.entry,
            "backend_failures": preview.backend_failures,
        });
        println!("{}", serde_json::to_string(&output).unwrap());
    }
}

fn handle_scan(
    diff_file: Option<String>,
    use_stdin: bool,
    diff_source: Option<DiffSourceArg>,
    enforcement: SensitiveEnforcementArg,
    format: ScanFormatArg,
    output: Option<String>,
    config: Option<String>,
    allowlist: Option<String>,
) {
    let selected_inputs = usize::from(use_stdin) + usize::from(diff_file.is_some());
    if selected_inputs > 1 {
        eprintln!("error: choose only one of --stdin or --diff-file");
        process::exit(1);
    }

    let cfg = load_config_or_exit_plain(config.as_deref());
    let enforcement = enforcement.to_config();
    let format = format.to_config();
    let mut allowlist_entries = cfg.sensitive.allowlist.clone();
    if let Some(path) = allowlist.as_deref() {
        match scan::load_allowlist_file(Path::new(path)) {
            Ok(mut entries) => allowlist_entries.append(&mut entries),
            Err(err) => {
                eprintln!("error: {err}");
                process::exit(3);
            }
        }
    }

    let (diff, changed_files) = if use_stdin {
        use std::io::Read;

        let mut diff = String::new();
        if let Err(err) = std::io::stdin().read_to_string(&mut diff) {
            eprintln!("error: failed to read stdin: {err}");
            process::exit(1);
        }
        if diff.trim().is_empty() {
            eprintln!("error: stdin diff was empty");
            process::exit(1);
        }
        let changed_files = scan::changed_files_from_diff(&diff);
        (diff, changed_files)
    } else if let Some(path) = diff_file.as_deref() {
        match std::fs::read_to_string(path) {
            Ok(diff) => {
                let changed_files = scan::changed_files_from_diff(&diff);
                (diff, changed_files)
            }
            Err(err) => {
                eprintln!("error: failed to read diff file {path}: {err}");
                process::exit(1);
            }
        }
    } else {
        let repo_root = match opencodecommit::git::get_repo_root() {
            Ok(root) => root,
            Err(err) => {
                eprintln!("error: {err}");
                process::exit(1);
            }
        };
        let source = diff_source
            .map(|value| value.to_config())
            .unwrap_or(cfg.diff_source);
        match scan::read_git_diff(&repo_root, source) {
            Ok(result) => result,
            Err(err) => {
                eprintln!("error: {err}");
                process::exit(1);
            }
        }
    };

    let result = scan::run_scan(&diff, &changed_files, enforcement, &allowlist_entries);
    let rendered = match format {
        ScanFormat::Text => scan::format_text(&result.report),
        ScanFormat::Json => serde_json::to_string_pretty(&serde_json::json!({
            "scanned_files": result.scanned_files,
            "report": scan::format_json(&result.report),
        }))
        .unwrap(),
        ScanFormat::Sarif => {
            serde_json::to_string_pretty(&scan::format_sarif(&result.report)).unwrap()
        }
        ScanFormat::GithubAnnotations => scan::format_github_annotations(&result.report),
    };

    if let Some(path) = output.as_deref() {
        if let Err(err) = std::fs::write(path, &rendered) {
            eprintln!("error: failed to write output file {path}: {err}");
            process::exit(1);
        }
    } else if !rendered.is_empty() {
        println!("{rendered}");
    }

    process::exit(if result.report.has_blocking_findings() {
        2
    } else {
        0
    });
}

fn handle_hook(action: HookAction) {
    match actions::run_hook(action.to_operation()) {
        Ok(message) => println!("{message}"),
        Err(err) => {
            eprintln!("error: {err}");
            process::exit(1);
        }
    }
}

fn handle_guard(action: GuardAction) {
    let result = match action {
        GuardAction::Install { global } => {
            if !global {
                Err("only --global is currently supported".to_owned())
            } else {
                guard::install_global().map_err(|err| err.to_string())
            }
        }
        GuardAction::Uninstall { global } => {
            if !global {
                Err("only --global is currently supported".to_owned())
            } else {
                guard::uninstall_global().map_err(|err| err.to_string())
            }
        }
        GuardAction::Profile { profile, config } => {
            let path = config.as_deref().map(Path::new);
            match Config::load_or_default(path) {
                Ok(mut cfg) => {
                    cfg.apply_sensitive_profile(profile.to_config());
                    let saved = if let Some(path) = path {
                        cfg.save_to_path(path)
                            .map(|_| path.to_path_buf())
                            .map_err(|err| err.to_string())
                    } else {
                        cfg.save_default().map_err(|err| err.to_string())
                    };
                    saved.map(|saved_path| {
                        format!(
                            "applied {} sensitive profile to {}",
                            match profile {
                                SensitiveProfileArg::Human => "human",
                                SensitiveProfileArg::StrictAgent => "strict-agent",
                            },
                            saved_path.display()
                        )
                    })
                }
                Err(err) => Err(err.to_string()),
            }
        }
    };

    match result {
        Ok(message) => println!("{message}"),
        Err(err) => {
            eprintln!("error: {err}");
            process::exit(1);
        }
    }
}

fn handle_internal(action: InternalAction) {
    let code = match action {
        InternalAction::RunManagedHook { hook_name, args } => {
            match guard::run_managed_hook(&hook_name, &args) {
                Ok(code) => code,
                Err(err) => {
                    eprintln!("OpenCodeCommit guard error: {err}");
                    1
                }
            }
        }
    };

    process::exit(code);
}

fn handle_tui(config: Option<String>, backend: Option<BackendArg>) {
    let mut cfg = load_config_or_exit_plain(config.as_deref());
    if let Some(backend) = backend {
        cfg.backend = backend.to_config();
        cfg.backend_order = vec![cfg.backend];
    }
    if let Err(err) = tui::run(cfg, config.map(Into::into)) {
        eprintln!("error: {err}");
        process::exit(match err {
            ActionError::Occ(_) => 3,
            ActionError::NonTty(_) => 1,
            _ => 1,
        });
    }
}

fn handle_update() {
    let source = update::detect_install_source();
    eprintln!("Detected installation source: {source}");

    if let Err(err) = update::run_update(source) {
        eprintln!("error: {err}");
        process::exit(1);
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Commit {
            backend,
            provider,
            model,
            mode,
            diff_source,
            max_diff_length,
            language,
            emoji,
            no_lowercase,
            template,
            cli_path,
            config,
            refine,
            feedback,
            custom_prompt,
            custom_type_rules,
            custom_message_rules,
            text,
            allow_sensitive,
            stdin,
            dry_run,
        } => {
            let mut cfg = load_config_or_exit(config.as_deref(), text);
            if let Err(err) = apply_commit_args(
                &mut cfg,
                &backend,
                &provider,
                &model,
                &mode,
                &diff_source,
                max_diff_length,
                &language,
                emoji,
                no_lowercase,
                &template,
                &cli_path,
                &custom_prompt,
                &custom_type_rules,
                &custom_message_rules,
            ) {
                if text {
                    eprintln!("error: {err}");
                    process::exit(1);
                }
                json_error(err);
                process::exit(3);
            }

            handle_commit(
                &cfg,
                refine,
                feedback,
                text,
                stdin,
                allow_sensitive,
                dry_run,
            );
        }
        Commands::Branch {
            description,
            backend,
            provider,
            model,
            cli_path,
            config,
            text,
            dry_run,
            mode,
        } => {
            let mut cfg = load_config_or_exit(config.as_deref(), text);
            apply_backend_overrides(&mut cfg, &backend, &provider, &model, &cli_path);
            handle_branch(&cfg, description, text, dry_run, mode.to_config());
        }
        Commands::Pr {
            backend,
            provider,
            model,
            cli_path,
            config,
            base,
            text,
        } => {
            let mut cfg = load_config_or_exit(config.as_deref(), text);
            apply_backend_overrides(&mut cfg, &backend, &provider, &model, &cli_path);
            let base_ref = if base == "main" {
                None
            } else {
                Some(base.as_str())
            };
            handle_pr(&cfg, text, base_ref);
        }
        Commands::Hook { action } => handle_hook(action),
        Commands::Guard { action } => handle_guard(action),
        Commands::Tui { backend, config } => handle_tui(config, backend),
        Commands::Update => handle_update(),
        Commands::Changelog {
            backend,
            provider,
            model,
            cli_path,
            config,
            text,
        } => {
            let mut cfg = load_config_or_exit(config.as_deref(), text);
            apply_backend_overrides(&mut cfg, &backend, &provider, &model, &cli_path);
            handle_changelog(&cfg, text);
        }
        Commands::Scan {
            diff_file,
            stdin,
            diff_source,
            enforcement,
            format,
            output,
            config,
            allowlist,
        } => handle_scan(
            diff_file,
            stdin,
            diff_source,
            enforcement,
            format,
            output,
            config,
            allowlist,
        ),
        Commands::Internal { action } => handle_internal(action),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tui_accepts_backend_flag() {
        let cli = Cli::try_parse_from(["occ", "tui", "--backend", "codex"]).unwrap();
        match cli.command {
            Commands::Tui { backend, config } => {
                assert_eq!(backend, Some(BackendArg::Codex));
                assert_eq!(config, None);
            }
            _ => panic!("expected tui command"),
        }
    }

    #[test]
    fn backend_override_locks_backend_order() {
        let mut config = Config::default();
        config.backend_order = vec![Backend::Codex, Backend::Opencode];

        apply_backend_overrides(&mut config, &BackendArg::Claude, &None, &None, &None);

        assert_eq!(config.backend, Backend::Claude);
        assert_eq!(config.backend_order, vec![Backend::Claude]);
    }
}
