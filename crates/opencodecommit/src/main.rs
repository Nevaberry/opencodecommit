use std::path::Path;
use std::process;

use clap::{Parser, Subcommand, ValueEnum};
use opencodecommit::backend::{build_invocation, detect_cli, exec_cli};
use opencodecommit::config::{CliBackend, CommitMode, Config, DiffSource, LanguageConfig};
use opencodecommit::context::gather_context;
use opencodecommit::git;
use opencodecommit::prompt::{
    build_branch_prompt, build_changelog_prompt, build_pr_prompt, build_prompt, build_refine_prompt,
};
use opencodecommit::response::{
    format_adaptive_message, format_branch_name, format_commit_message, parse_pr_response,
    parse_response,
};

#[derive(Parser)]
#[command(name = "opencodecommit", version, about = "AI-powered git commit message generator")]
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

        /// Language instruction (e.g. "Write the commit message in English.")
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
}

#[derive(Clone, ValueEnum)]
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

#[derive(Subcommand)]
enum HookAction {
    /// Install prepare-commit-msg hook
    Install,
    /// Uninstall prepare-commit-msg hook
    Uninstall,
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

/// Apply CLI args onto a Config, overriding file/default values.
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
) {
    config.backend = backend.to_config();
    config.commit_mode = mode.to_config();
    config.diff_source = diff_source.to_config();

    if let Some(p) = provider {
        config.provider = p.clone();
    }
    if let Some(m) = model {
        match config.backend {
            CliBackend::Opencode => config.model = m.clone(),
            CliBackend::Claude => config.claude_model = m.clone(),
            CliBackend::Codex => config.codex_model = m.clone(),
            CliBackend::Gemini => config.gemini_model = m.clone(),
        }
    }
    if let Some(n) = max_diff_length {
        config.max_diff_length = n;
    }
    if let Some(lang) = language {
        config.languages = vec![LanguageConfig {
            label: "CLI".to_owned(),
            instruction: lang.clone(),
        }];
        config.active_language = "CLI".to_owned();
    }
    if emoji {
        config.use_emojis = true;
    }
    if no_lowercase {
        config.use_lower_case = false;
    }
    if let Some(t) = template {
        config.commit_template = t.clone();
    }
    if let Some(p) = cli_path {
        match config.backend {
            CliBackend::Opencode => config.cli_path = p.clone(),
            CliBackend::Claude => config.claude_path = p.clone(),
            CliBackend::Codex => config.codex_path = p.clone(),
            CliBackend::Gemini => config.gemini_path = p.clone(),
        }
    }
    if let Some(p) = custom_prompt {
        config.custom.prompt = p.clone();
    }
    if let Some(r) = custom_type_rules {
        config.custom.type_rules = r.clone();
    }
    if let Some(r) = custom_message_rules {
        config.custom.commit_message_rules = r.clone();
    }
}

fn run_commit(
    config: &Config,
    refine: &Option<String>,
    feedback: &Option<String>,
    text: bool,
    use_stdin: bool,
    allow_sensitive: bool,
) {
    // Exit code contract (default JSON mode):
    // 0 = success, 1 = no changes, 2 = provider error, 3 = config error,
    // 4 = stdin error, 5 = sensitive content

    let repo_root = match git::get_repo_root() {
        Ok(r) => r,
        Err(e) => {
            if text {
                eprintln!("error: {e}");
                process::exit(1);
            }
            let output = serde_json::json!({ "status": "error", "error": format!("{e}") });
            println!("{}", serde_json::to_string(&output).unwrap());
            process::exit(3);
        }
    };

    // If --stdin, read diff from stdin instead of git
    let mut context = if use_stdin {
        use std::io::Read;
        let mut diff = String::new();
        if let Err(e) = std::io::stdin().read_to_string(&mut diff) {
            if text {
                eprintln!("error: failed to read stdin: {e}");
                process::exit(1);
            }
            let output = serde_json::json!({ "status": "error", "error": format!("stdin: {e}") });
            println!("{}", serde_json::to_string(&output).unwrap());
            process::exit(4);
        }
        if diff.trim().is_empty() {
            if text {
                eprintln!("error: empty stdin");
                process::exit(1);
            }
            let output = serde_json::json!({ "status": "error", "error": "empty stdin" });
            println!("{}", serde_json::to_string(&output).unwrap());
            process::exit(4);
        }
        let changed_files = opencodecommit::context::extract_changed_file_paths(&diff);
        let has_sensitive = opencodecommit::context::detect_sensitive_content(&diff, &changed_files);
        let branch = git::get_branch_name(&repo_root).unwrap_or_else(|_| "unknown".to_owned());
        let recent = git::get_recent_commits(&repo_root, 10).unwrap_or_default();
        opencodecommit::context::CommitContext {
            diff,
            recent_commits: recent,
            branch,
            file_contents: vec![],
            changed_files,
            has_sensitive_content: has_sensitive,
        }
    } else {
        // Gather context from git
        match gather_context(&repo_root, config.diff_source) {
            Ok(c) => c,
            Err(e) => {
                if text {
                    eprintln!("error: {e}");
                    process::exit(1);
                }
                let code = if matches!(e, opencodecommit::Error::NoChanges) { 1 } else { 3 };
                let output = serde_json::json!({ "status": "error", "error": format!("{e}") });
                println!("{}", serde_json::to_string(&output).unwrap());
                process::exit(code);
            }
        }
    };

    // Block on sensitive content unless --allow-sensitive
    if context.has_sensitive_content && !allow_sensitive {
        if text {
            eprintln!("error: sensitive content detected in diff (API keys, credentials, or tokens)");
            eprintln!("The diff appears to contain secrets that would be sent to an AI backend.");
            eprintln!("Use --allow-sensitive to skip this check.");
            process::exit(1);
        }
        let output = serde_json::json!({
            "status": "error",
            "error": "sensitive content detected in diff (API keys, credentials, or tokens). Use --allow-sensitive to skip this check."
        });
        println!("{}", serde_json::to_string(&output).unwrap());
        process::exit(5);
    }

    // Truncate diff if needed
    if context.diff.len() > config.max_diff_length {
        context.diff = format!(
            "{}\n... (truncated)",
            &context.diff[..config.max_diff_length]
        );
    }

    // Build prompt
    let prompt = if let Some(current_message) = refine {
        let fb = feedback
            .as_deref()
            .unwrap_or(&config.refine.default_feedback);
        build_refine_prompt(current_message, fb, &context.diff, config)
    } else {
        build_prompt(&context, config, Some(config.commit_mode))
    };

    // Detect backend and execute
    let cli_path = match detect_cli(config.backend, config.backend_cli_path()) {
        Ok(p) => p,
        Err(e) => {
            if text {
                eprintln!("error: {e}");
                process::exit(1);
            }
            let output = serde_json::json!({ "status": "error", "error": format!("{e}") });
            println!("{}", serde_json::to_string(&output).unwrap());
            process::exit(2);
        }
    };

    let invocation = build_invocation(&cli_path, &prompt, config);
    let start = std::time::Instant::now();
    let response = match exec_cli(&invocation) {
        Ok(r) => r,
        Err(e) => {
            if text {
                eprintln!("error: {e}");
                process::exit(1);
            }
            let output = serde_json::json!({ "status": "error", "error": format!("{e}") });
            println!("{}", serde_json::to_string(&output).unwrap());
            process::exit(2);
        }
    };
    let duration_ms = start.elapsed().as_millis();

    // Process response
    let message = match config.commit_mode {
        CommitMode::Adaptive | CommitMode::AdaptiveOneliner => format_adaptive_message(&response),
        CommitMode::Conventional | CommitMode::ConventionalOneliner => {
            let parsed = parse_response(&response);
            format_commit_message(&parsed, config)
        }
    };

    // Output
    if text {
        println!("{message}");
    } else {
        let parsed = parse_response(&response);
        let backend_name = format!("{:?}", config.backend).to_lowercase();
        let output = serde_json::json!({
            "status": "success",
            "message": message,
            "type": parsed.type_name,
            "description": parsed.description,
            "provider": backend_name,
            "files_analyzed": context.changed_files.len(),
            "duration_ms": duration_ms,
        });
        println!("{}", serde_json::to_string(&output).unwrap());
    }
}

fn run_branch(config: &Config, description: Option<&str>, text: bool) {
    let repo_root = match git::get_repo_root() {
        Ok(r) => r,
        Err(e) => {
            if text { eprintln!("error: {e}"); process::exit(1); }
            let output = serde_json::json!({ "status": "error", "error": format!("{e}") });
            println!("{}", serde_json::to_string(&output).unwrap());
            process::exit(3);
        }
    };

    let diff = if description.is_none() {
        match git::get_diff(config.diff_source, &repo_root) {
            Ok(d) => Some(d),
            Err(e) => {
                if text { eprintln!("error: {e}"); process::exit(1); }
                let output = serde_json::json!({ "status": "error", "error": format!("{e}") });
                println!("{}", serde_json::to_string(&output).unwrap());
                process::exit(1);
            }
        }
    } else {
        None
    };

    let desc = description.unwrap_or("");
    let prompt = build_branch_prompt(desc, diff.as_deref(), config);

    let cli_path = match detect_cli(config.backend, config.backend_cli_path()) {
        Ok(p) => p,
        Err(e) => {
            if text { eprintln!("error: {e}"); process::exit(1); }
            let output = serde_json::json!({ "status": "error", "error": format!("{e}") });
            println!("{}", serde_json::to_string(&output).unwrap());
            process::exit(2);
        }
    };
    let invocation = build_invocation(&cli_path, &prompt, config);
    let response = match exec_cli(&invocation) {
        Ok(r) => r,
        Err(e) => {
            if text { eprintln!("error: {e}"); process::exit(1); }
            let output = serde_json::json!({ "status": "error", "error": format!("{e}") });
            println!("{}", serde_json::to_string(&output).unwrap());
            process::exit(2);
        }
    };

    let name = format_branch_name(&response);

    if text {
        println!("{name}");
    } else {
        let output = serde_json::json!({ "status": "success", "name": name });
        println!("{}", serde_json::to_string(&output).unwrap());
    }
}

fn run_pr(config: &Config, _base: &str, text: bool) {
    let repo_root = match git::get_repo_root() {
        Ok(r) => r,
        Err(e) => {
            if text { eprintln!("error: {e}"); process::exit(1); }
            let output = serde_json::json!({ "status": "error", "error": format!("{e}") });
            println!("{}", serde_json::to_string(&output).unwrap());
            process::exit(3);
        }
    };

    let mut context = match gather_context(&repo_root, config.diff_source) {
        Ok(c) => c,
        Err(e) => {
            if text { eprintln!("error: {e}"); process::exit(1); }
            let code = if matches!(e, opencodecommit::Error::NoChanges) { 1 } else { 3 };
            let output = serde_json::json!({ "status": "error", "error": format!("{e}") });
            println!("{}", serde_json::to_string(&output).unwrap());
            process::exit(code);
        }
    };

    if context.diff.len() > config.max_diff_length {
        context.diff = format!("{}\n... (truncated)", &context.diff[..config.max_diff_length]);
    }

    let prompt = build_pr_prompt(&context, config);
    let cli_path = match detect_cli(config.backend, config.backend_cli_path()) {
        Ok(p) => p,
        Err(e) => {
            if text { eprintln!("error: {e}"); process::exit(1); }
            let output = serde_json::json!({ "status": "error", "error": format!("{e}") });
            println!("{}", serde_json::to_string(&output).unwrap());
            process::exit(2);
        }
    };
    let invocation = build_invocation(&cli_path, &prompt, config);
    let response = match exec_cli(&invocation) {
        Ok(r) => r,
        Err(e) => {
            if text { eprintln!("error: {e}"); process::exit(1); }
            let output = serde_json::json!({ "status": "error", "error": format!("{e}") });
            println!("{}", serde_json::to_string(&output).unwrap());
            process::exit(2);
        }
    };

    let parsed = parse_pr_response(&response);

    if text {
        println!("{}\n\n{}", parsed.title, parsed.body);
    } else {
        let output = serde_json::json!({
            "status": "success",
            "title": parsed.title,
            "body": parsed.body,
        });
        println!("{}", serde_json::to_string(&output).unwrap());
    }
}

fn run_changelog(config: &Config, text: bool) {
    let repo_root = match git::get_repo_root() {
        Ok(r) => r,
        Err(e) => {
            if text { eprintln!("error: {e}"); process::exit(1); }
            let output = serde_json::json!({ "status": "error", "error": format!("{e}") });
            println!("{}", serde_json::to_string(&output).unwrap());
            process::exit(3);
        }
    };

    let mut context = match gather_context(&repo_root, config.diff_source) {
        Ok(c) => c,
        Err(e) => {
            if text { eprintln!("error: {e}"); process::exit(1); }
            let code = if matches!(e, opencodecommit::Error::NoChanges) { 1 } else { 3 };
            let output = serde_json::json!({ "status": "error", "error": format!("{e}") });
            println!("{}", serde_json::to_string(&output).unwrap());
            process::exit(code);
        }
    };

    if context.diff.len() > config.max_diff_length {
        context.diff = format!("{}\n... (truncated)", &context.diff[..config.max_diff_length]);
    }

    let prompt = build_changelog_prompt(&context, config);
    let cli_path = match detect_cli(config.backend, config.backend_cli_path()) {
        Ok(p) => p,
        Err(e) => {
            if text { eprintln!("error: {e}"); process::exit(1); }
            let output = serde_json::json!({ "status": "error", "error": format!("{e}") });
            println!("{}", serde_json::to_string(&output).unwrap());
            process::exit(2);
        }
    };
    let invocation = build_invocation(&cli_path, &prompt, config);
    let response = match exec_cli(&invocation) {
        Ok(r) => r,
        Err(e) => {
            if text { eprintln!("error: {e}"); process::exit(1); }
            let output = serde_json::json!({ "status": "error", "error": format!("{e}") });
            println!("{}", serde_json::to_string(&output).unwrap());
            process::exit(2);
        }
    };

    let entry = opencodecommit::response::sanitize_response(&response);

    if text {
        println!("{entry}");
    } else {
        let output = serde_json::json!({ "status": "success", "entry": entry });
        println!("{}", serde_json::to_string(&output).unwrap());
    }
}

fn run_hook(action: HookAction) {
    let repo_root = match git::get_repo_root() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(1);
        }
    };

    let hooks_dir = repo_root.join(".git").join("hooks");
    let hook_path = hooks_dir.join("prepare-commit-msg");

    match action {
        HookAction::Install => {
            if let Err(e) = std::fs::create_dir_all(&hooks_dir) {
                eprintln!("error: failed to create hooks dir: {e}");
                process::exit(1);
            }

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
            if let Err(e) = std::fs::write(&hook_path, hook_script) {
                eprintln!("error: failed to write hook: {e}");
                process::exit(1);
            }

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&hook_path, std::fs::Permissions::from_mode(0o755));
            }

            println!("installed prepare-commit-msg hook at {}", hook_path.display());
        }
        HookAction::Uninstall => {
            if hook_path.exists() {
                // Only remove if it's ours
                let content = std::fs::read_to_string(&hook_path).unwrap_or_default();
                if content.contains("opencodecommit") {
                    if let Err(e) = std::fs::remove_file(&hook_path) {
                        eprintln!("error: failed to remove hook: {e}");
                        process::exit(1);
                    }
                    println!("uninstalled prepare-commit-msg hook");
                } else {
                    eprintln!("prepare-commit-msg hook exists but was not installed by opencodecommit");
                    process::exit(1);
                }
            } else {
                println!("no prepare-commit-msg hook found");
            }
        }
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
            stdin: use_stdin,
        } => {
            let config_path = config.as_deref().map(Path::new);
            let mut cfg = match Config::load_or_default(config_path) {
                Ok(c) => c,
                Err(e) => {
                    if text {
                        eprintln!("error: {e}");
                        process::exit(1);
                    }
                    let output = serde_json::json!({ "status": "error", "error": format!("{e}") });
                    println!("{}", serde_json::to_string(&output).unwrap());
                    process::exit(3);
                }
            };

            apply_commit_args(
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
            );

            run_commit(&cfg, &refine, &feedback, text, use_stdin, allow_sensitive);
        }
        Commands::Branch {
            description,
            backend,
            provider,
            model,
            cli_path,
            config,
            text,
        } => {
            let config_path = config.as_deref().map(Path::new);
            let mut cfg = match Config::load_or_default(config_path) {
                Ok(c) => c,
                Err(e) => {
                    if text { eprintln!("error: {e}"); process::exit(1); }
                    let output = serde_json::json!({ "status": "error", "error": format!("{e}") });
                    println!("{}", serde_json::to_string(&output).unwrap());
                    process::exit(3);
                }
            };
            cfg.backend = backend.to_config();
            if let Some(p) = provider { cfg.provider = p; }
            if let Some(m) = model {
                match cfg.backend {
                    CliBackend::Opencode => cfg.model = m,
                    CliBackend::Claude => cfg.claude_model = m,
                    CliBackend::Codex => cfg.codex_model = m,
                    CliBackend::Gemini => cfg.gemini_model = m,
                }
            }
            if let Some(p) = cli_path {
                match cfg.backend {
                    CliBackend::Opencode => cfg.cli_path = p,
                    CliBackend::Claude => cfg.claude_path = p,
                    CliBackend::Codex => cfg.codex_path = p,
                    CliBackend::Gemini => cfg.gemini_path = p,
                }
            }
            run_branch(&cfg, description.as_deref(), text);
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
            let config_path = config.as_deref().map(Path::new);
            let mut cfg = match Config::load_or_default(config_path) {
                Ok(c) => c,
                Err(e) => {
                    if text { eprintln!("error: {e}"); process::exit(1); }
                    let output = serde_json::json!({ "status": "error", "error": format!("{e}") });
                    println!("{}", serde_json::to_string(&output).unwrap());
                    process::exit(3);
                }
            };
            cfg.backend = backend.to_config();
            if let Some(p) = provider { cfg.provider = p; }
            if let Some(m) = model {
                match cfg.backend {
                    CliBackend::Opencode => cfg.model = m,
                    CliBackend::Claude => cfg.claude_model = m,
                    CliBackend::Codex => cfg.codex_model = m,
                    CliBackend::Gemini => cfg.gemini_model = m,
                }
            }
            if let Some(p) = cli_path {
                match cfg.backend {
                    CliBackend::Opencode => cfg.cli_path = p,
                    CliBackend::Claude => cfg.claude_path = p,
                    CliBackend::Codex => cfg.codex_path = p,
                    CliBackend::Gemini => cfg.gemini_path = p,
                }
            }
            run_pr(&cfg, &base, text);
        }
        Commands::Changelog {
            backend,
            provider,
            model,
            cli_path,
            config,
            text,
        } => {
            let config_path = config.as_deref().map(Path::new);
            let mut cfg = match Config::load_or_default(config_path) {
                Ok(c) => c,
                Err(e) => {
                    if text { eprintln!("error: {e}"); process::exit(1); }
                    let output = serde_json::json!({ "status": "error", "error": format!("{e}") });
                    println!("{}", serde_json::to_string(&output).unwrap());
                    process::exit(3);
                }
            };
            cfg.backend = backend.to_config();
            if let Some(p) = provider { cfg.provider = p; }
            if let Some(m) = model {
                match cfg.backend {
                    CliBackend::Opencode => cfg.model = m,
                    CliBackend::Claude => cfg.claude_model = m,
                    CliBackend::Codex => cfg.codex_model = m,
                    CliBackend::Gemini => cfg.gemini_model = m,
                }
            }
            if let Some(p) = cli_path {
                match cfg.backend {
                    CliBackend::Opencode => cfg.cli_path = p,
                    CliBackend::Claude => cfg.claude_path = p,
                    CliBackend::Codex => cfg.codex_path = p,
                    CliBackend::Gemini => cfg.gemini_path = p,
                }
            }
            run_changelog(&cfg, text);
        }
        Commands::Hook { action } => {
            run_hook(action);
        }
    }
}
