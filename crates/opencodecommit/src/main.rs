mod actions;
mod guard;
mod tui;
mod update;

use std::path::Path;
use std::process;

use clap::{Parser, Subcommand, ValueEnum};
use opencodecommit::config::{CliBackend, CommitMode, Config, DiffSource};

use crate::actions::{ActionError, CommitRequest, HookOperation};

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
        #[arg(long, value_enum, default_value_t = CliBackendArg::Opencode)]
        backend: CliBackendArg,

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

        /// Language label (e.g. "English", "Suomi", "Custom (example)")
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

        #[arg(long, value_enum, default_value_t = CliBackendArg::Opencode)]
        backend: CliBackendArg,

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
        #[arg(long, value_enum, default_value_t = CliBackendArg::Opencode)]
        backend: CliBackendArg,

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
        backend: Option<CliBackendArg>,

        /// Path to config file
        #[arg(long)]
        config: Option<String>,
    },

    /// Update occ to the latest version
    #[command(alias = "upgrade")]
    Update,

    /// Generate a changelog entry
    Changelog {
        #[arg(long, value_enum, default_value_t = CliBackendArg::Opencode)]
        backend: CliBackendArg,

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

    #[command(hide = true)]
    Internal {
        #[command(subcommand)]
        action: InternalAction,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum CliBackendArg {
    Opencode,
    Claude,
    Codex,
    Gemini,
}

impl CliBackendArg {
    fn to_config(&self) -> CliBackend {
        match self {
            CliBackendArg::Opencode => CliBackend::Opencode,
            CliBackendArg::Claude => CliBackend::Claude,
            CliBackendArg::Codex => CliBackend::Codex,
            CliBackendArg::Gemini => CliBackend::Gemini,
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
    backend: &CliBackendArg,
    provider: &Option<String>,
    model: &Option<String>,
    cli_path: &Option<String>,
) {
    config.backend = backend.to_config();
    config.backend_order = vec![config.backend];
    if let Some(provider) = provider {
        config.provider = provider.clone();
    }
    if let Some(model) = model {
        match config.backend {
            CliBackend::Opencode => config.model = model.clone(),
            CliBackend::Claude => config.claude_model = model.clone(),
            CliBackend::Codex => config.codex_model = model.clone(),
            CliBackend::Gemini => config.gemini_model = model.clone(),
        }
    }
    if let Some(path) = cli_path {
        match config.backend {
            CliBackend::Opencode => config.cli_path = path.clone(),
            CliBackend::Claude => config.claude_path = path.clone(),
            CliBackend::Codex => config.codex_path = path.clone(),
            CliBackend::Gemini => config.gemini_path = path.clone(),
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
    backend: &CliBackendArg,
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

    let preview = match actions::generate_commit_preview_with_fallback(config, &request, |_| {}) {
        Ok(preview) => preview,
        Err(err) => {
            if text {
                eprintln!("error: {err}");
                process::exit(1);
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
    let preview = match actions::generate_branch_preview(config, description.as_deref(), mode) {
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
    let preview = match actions::generate_pr_preview(config, base) {
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
    let preview = match actions::generate_changelog_preview_with_fallback(config, |_| {}) {
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

fn handle_tui(config: Option<String>, backend: Option<CliBackendArg>) {
    let mut cfg = load_config_or_exit_plain(config.as_deref());
    if let Some(backend) = backend {
        cfg.backend = backend.to_config();
    }
    if let Err(err) = tui::run(cfg) {
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
                assert_eq!(backend, Some(CliBackendArg::Codex));
                assert_eq!(config, None);
            }
            _ => panic!("expected tui command"),
        }
    }

    #[test]
    fn backend_override_locks_backend_order() {
        let mut config = Config::default();
        config.backend_order = vec![CliBackend::Codex, CliBackend::Opencode];

        apply_backend_overrides(&mut config, &CliBackendArg::Claude, &None, &None, &None);

        assert_eq!(config.backend, CliBackend::Claude);
        assert_eq!(config.backend_order, vec![CliBackend::Claude]);
    }
}
