use crate::config::{BranchMode, CommitMode, Config};
use crate::context::CommitContext;

// --- Prompt module constants (verbatim from TS generator.ts) ---

const BASE_MODULE: &str = "You are an expert at writing git commit messages.
Analyze the code changes and generate a specific, descriptive commit message.

Be specific about WHAT changed. Describe the actual functionality, file, or behavior affected.
Never write vague messages like \"update code\", \"make changes\", or \"update files\".

Respond with ONLY the commit message. No explanations, no code blocks, no markdown.";

const ADAPTIVE_FORMAT: &str = "Match the style of the recent commits shown below. Adapt to whatever conventions
the project uses — the recent commits are your primary guide.

If the recent commits use conventional commits (type: description), follow that format.
If they use custom prefixes (e.g. developer initials, dates, version numbers, or
non-standard categories like private, public, dev, production), match that style.
If no clear style exists, fall back to: type: description

Common conventional types for reference (use these as defaults when no other style is apparent):
feat, fix, docs, style, refactor, test, perf, security, revert, chore

Be specific about what changed — do not write vague messages like \"update code\".

Recent commits:
{recentCommits}";

const CONVENTIONAL_FORMAT: &str = "Use conventional commit format: type(scope): description

Choose the type that best matches the actual changes:
- feat: new features or capabilities
- fix: bug fixes, error corrections
- docs: documentation, README, markdown, comments, JSDoc/rustdoc changes
- style: formatting, whitespace, semicolons (no logic change)
- refactor: code restructuring without behavior change
- test: adding or modifying tests
- perf: performance improvements
- security: security fixes, vulnerability patches, auth hardening
- revert: reverting previous changes
- chore: build process, dependencies, tooling (only if nothing else fits)
Scope: derive from the primary area affected (optional, omit if unclear).
Use imperative mood. No period at end. Lowercase after colon.";

const MULTILINE_LENGTH: &str = "If the change is simple, use a single line under 72 characters.
If the change is complex with multiple aspects, add a body after a blank line
with bullet points (prefix each with \"- \"). Wrap at 72 characters.";

const ONELINER_LENGTH: &str = "Write exactly one line, no body. Maximum 72 characters.";

const SENSITIVE_CONTENT_NOTE: &str = "The diff contains sensitive content (API keys, credentials, or env variables).
Mention this naturally in the first line of the commit message, e.g. \"add API keys for payment service\"
or \"configure production env variables\". Just acknowledge what is being committed — no warnings or caveats.";

// --- Prompt assembly ---

/// Build the prompt for commit message generation.
pub fn build_prompt(context: &CommitContext, config: &Config, mode: Option<CommitMode>) -> String {
    // Custom prompt override
    if !config.custom.prompt.is_empty() {
        return config.custom.prompt.replace("{diff}", &context.diff);
    }

    let active_mode = mode.unwrap_or(config.commit_mode);
    let mut parts: Vec<String> = Vec::new();

    // Base (always)
    parts.push(BASE_MODULE.to_owned());

    // Format module
    match active_mode {
        CommitMode::Adaptive | CommitMode::AdaptiveOneliner => {
            let recent_text = if context.recent_commits.is_empty() {
                "(no recent commits)".to_owned()
            } else {
                context.recent_commits.join("\n")
            };
            parts.push(ADAPTIVE_FORMAT.replace("{recentCommits}", &recent_text));
        }
        CommitMode::Conventional | CommitMode::ConventionalOneliner => {
            parts.push(CONVENTIONAL_FORMAT.to_owned());
            if !config.custom.type_rules.is_empty() {
                parts.push(config.custom.type_rules.clone());
            }
            if !config.custom.commit_message_rules.is_empty() {
                parts.push(config.custom.commit_message_rules.clone());
            }
        }
    }

    // Length module
    match active_mode {
        CommitMode::AdaptiveOneliner | CommitMode::ConventionalOneliner => {
            parts.push(ONELINER_LENGTH.to_owned());
        }
        _ => {
            parts.push(MULTILINE_LENGTH.to_owned());
        }
    }

    // Sensitive content note
    if context.has_sensitive_content {
        parts.push(SENSITIVE_CONTENT_NOTE.to_owned());
    }

    // Language instruction
    parts.push(config.active_language_instruction());

    // Context section
    parts.push(format!("Branch: {}", context.branch));

    if !context.file_contents.is_empty() {
        parts.push("Original files (for understanding context):".to_owned());
        for fc in &context.file_contents {
            parts.push(format!("--- {} ({}) ---", fc.path, fc.truncation_mode));
            parts.push(fc.content.clone());
        }
    }

    parts.push("--- Git Diff ---".to_owned());
    parts.push(context.diff.clone());

    parts.join("\n\n")
}

/// Build the prompt for refining an existing commit message.
pub fn build_refine_prompt(
    current_message: &str,
    feedback: &str,
    diff: &str,
    config: &Config,
) -> String {
    format!(
        "The following commit message was generated for a git diff:

Current message:
{current_message}

User feedback: {feedback}

Original diff (first {} characters):
{diff}

Generate an improved commit message based on the feedback.
Keep the same type prefix unless the feedback suggests otherwise.
{}

Respond with ONLY the improved commit message. No markdown, no code blocks, no explanations.",
        config.max_diff_length,
        config.active_language_instruction()
    )
}

const ADAPTIVE_BRANCH_FORMAT: &str = "Match the naming style of the existing branches shown below.
Adapt to whatever conventions the project uses — the existing branches are your primary guide.

If they use type/description (e.g. feat/add-login, fix/auth-bug), follow that format.
If they use other patterns (e.g. username/description, JIRA-123/description, dates), match that style.
If no clear pattern exists, fall back to: type/short-description-slug

Be specific about what the branch is for — do not write vague names.

Existing branches:
{existingBranches}";

/// Build the prompt for branch name generation.
pub fn build_branch_prompt(
    context_or_description: &str,
    diff: Option<&str>,
    config: &Config,
    mode: BranchMode,
    existing_branches: &[String],
) -> String {
    let mut parts = vec![
        "You are an expert at naming git branches.".to_owned(),
    ];

    match mode {
        BranchMode::Conventional => {
            parts.push("Generate a branch name in the format: type/short-description-slug".to_owned());
            parts.push("Types: feat, fix, docs, refactor, test, chore".to_owned());
            parts.push("Use lowercase, hyphens between words, max 50 characters total.".to_owned());
        }
        BranchMode::Adaptive => {
            if existing_branches.is_empty() {
                parts.push("Generate a branch name in the format: type/short-description-slug".to_owned());
                parts.push("Types: feat, fix, docs, refactor, test, chore".to_owned());
                parts.push("Use lowercase, hyphens between words, max 50 characters total.".to_owned());
            } else {
                let branch_text = existing_branches.join("\n");
                parts.push(ADAPTIVE_BRANCH_FORMAT.replace("{existingBranches}", &branch_text));
            }
        }
    }

    parts.push("Respond with ONLY the branch name. No explanations.".to_owned());
    parts.push(config.active_language_instruction());

    if let Some(diff) = diff {
        parts.push("--- Git Diff ---".to_owned());
        parts.push(diff.to_owned());
    }

    if !context_or_description.is_empty() {
        parts.push(format!("Description: {context_or_description}"));
    }

    parts.join("\n\n")
}

/// Build the prompt for PR title and body generation.
pub fn build_pr_prompt(context: &CommitContext, config: &Config) -> String {
    let mut parts = vec![
        "You are an expert at writing pull request descriptions.".to_owned(),
        "Generate a PR title and body from the changes below.".to_owned(),
        "Format:".to_owned(),
        "TITLE: <concise title under 70 chars>".to_owned(),
        "BODY:".to_owned(),
        "## Summary".to_owned(),
        "<1-3 bullet points describing the changes>".to_owned(),
        "".to_owned(),
        "## Test plan".to_owned(),
        "<bullet points for testing>".to_owned(),
        "".to_owned(),
        "Respond with ONLY the title and body in the format above.".to_owned(),
    ];

    parts.push(config.active_language_instruction());

    if !context.recent_commits.is_empty() {
        parts.push("Commits in this branch:".to_owned());
        parts.push(context.recent_commits.join("\n"));
    }

    parts.push(format!("Branch: {}", context.branch));
    parts.push("--- Git Diff ---".to_owned());
    parts.push(context.diff.clone());

    parts.join("\n\n")
}

/// Build the prompt for changelog entry generation.
pub fn build_changelog_prompt(context: &CommitContext, config: &Config) -> String {
    let mut parts = vec![
        "You are an expert at writing changelog entries.".to_owned(),
        "Generate a changelog entry from the commits and diff below.".to_owned(),
        "Use Keep a Changelog format with sections: Added, Changed, Fixed, Removed.".to_owned(),
        "Only include sections that apply. Use bullet points.".to_owned(),
        "Respond with ONLY the changelog entry. No explanations.".to_owned(),
    ];

    parts.push(config.active_language_instruction());

    if !context.recent_commits.is_empty() {
        parts.push("Recent commits:".to_owned());
        parts.push(context.recent_commits.join("\n"));
    }

    parts.push("--- Git Diff ---".to_owned());
    parts.push(context.diff.clone());

    parts.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BranchMode;
    use crate::context::{FileContext, TruncationMode};

    fn make_config(f: impl FnOnce(&mut Config)) -> Config {
        let mut cfg = Config::default();
        f(&mut cfg);
        cfg
    }

    fn make_context(f: impl FnOnce(&mut CommitContext)) -> CommitContext {
        let mut ctx = CommitContext {
            diff: "diff content here".to_owned(),
            recent_commits: vec![
                "abc1234 feat: add login page".to_owned(),
                "def5678 fix: resolve auth bug".to_owned(),
            ],
            branch: "feature/my-branch".to_owned(),
            file_contents: vec![],
            changed_files: vec!["src/app.ts".to_owned()],
            has_sensitive_content: false,
        };
        f(&mut ctx);
        ctx
    }

    // --- buildPrompt tests (ported from TS) ---

    #[test]
    fn includes_diff_in_prompt() {
        let config = Config::default();
        let context = make_context(|c| c.diff = "diff content here".to_owned());
        let prompt = build_prompt(&context, &config, None);
        assert!(prompt.contains("diff content here"));
    }

    #[test]
    fn includes_language_instruction() {
        let config = make_config(|c| {
            c.languages = vec![crate::config::LanguageConfig {
                label: "Finnish".to_owned(),
                instruction: "Write in Finnish.".to_owned(),
            }];
            c.active_language = "Finnish".to_owned();
        });
        let context = make_context(|_| {});
        let prompt = build_prompt(&context, &config, None);
        assert!(prompt.contains("Write in Finnish."));
    }

    #[test]
    fn uses_custom_prompt_when_set() {
        let config = make_config(|c| {
            c.custom.prompt = "Custom prompt with {diff} here".to_owned();
        });
        let context = make_context(|c| c.diff = "my diff".to_owned());
        let prompt = build_prompt(&context, &config, None);
        assert_eq!(prompt, "Custom prompt with my diff here");
    }

    #[test]
    fn adaptive_mode_includes_recent_commits() {
        let config = Config::default();
        let context = make_context(|c| {
            c.recent_commits = vec![
                "abc1234 feat: add login".to_owned(),
                "def5678 fix: auth bug".to_owned(),
            ];
        });
        let prompt = build_prompt(&context, &config, Some(CommitMode::Adaptive));
        assert!(prompt.contains("abc1234 feat: add login"));
        assert!(prompt.contains("def5678 fix: auth bug"));
        assert!(prompt.contains("Match the style"));
    }

    #[test]
    fn adaptive_mode_no_recent_commits_placeholder() {
        let config = Config::default();
        let context = make_context(|c| c.recent_commits = vec![]);
        let prompt = build_prompt(&context, &config, Some(CommitMode::Adaptive));
        assert!(prompt.contains("(no recent commits)"));
    }

    #[test]
    fn conventional_mode_includes_type_rules() {
        let config = Config::default();
        let context = make_context(|_| {});
        let prompt = build_prompt(&context, &config, Some(CommitMode::Conventional));
        assert!(prompt.contains("conventional commit format"));
        assert!(prompt.contains("- feat: new features"));
    }

    #[test]
    fn conventional_mode_custom_type_rules() {
        let config = make_config(|c| {
            c.custom.type_rules = "Custom type rules here".to_owned();
        });
        let context = make_context(|_| {});
        let prompt = build_prompt(&context, &config, Some(CommitMode::Conventional));
        assert!(prompt.contains("Custom type rules here"));
    }

    #[test]
    fn conventional_mode_custom_message_rules() {
        let config = make_config(|c| {
            c.custom.commit_message_rules = "Custom message rules".to_owned();
        });
        let context = make_context(|_| {});
        let prompt = build_prompt(&context, &config, Some(CommitMode::Conventional));
        assert!(prompt.contains("Custom message rules"));
    }

    #[test]
    fn oneliner_mode_instruction() {
        let config = Config::default();
        let context = make_context(|_| {});
        let prompt = build_prompt(&context, &config, Some(CommitMode::AdaptiveOneliner));
        assert!(prompt.contains("exactly one line"));
        assert!(prompt.contains("Maximum 72 characters"));
    }

    #[test]
    fn multiline_mode_instruction() {
        let config = Config::default();
        let context = make_context(|_| {});
        let prompt = build_prompt(&context, &config, Some(CommitMode::Adaptive));
        assert!(prompt.contains("bullet points"));
        assert!(!prompt.contains("exactly one line"));
    }

    #[test]
    fn includes_branch_name() {
        let config = Config::default();
        let context = make_context(|c| c.branch = "feature/auth".to_owned());
        let prompt = build_prompt(&context, &config, None);
        assert!(prompt.contains("Branch: feature/auth"));
    }

    #[test]
    fn includes_file_contents() {
        let config = Config::default();
        let context = make_context(|c| {
            c.file_contents = vec![FileContext {
                path: "src/app.ts".to_owned(),
                content: "const x = 1".to_owned(),
                truncation_mode: TruncationMode::Full,
            }];
        });
        let prompt = build_prompt(&context, &config, None);
        assert!(prompt.contains("--- src/app.ts (full) ---"));
        assert!(prompt.contains("const x = 1"));
    }

    #[test]
    fn includes_sensitive_content_note() {
        let config = Config::default();
        let context = make_context(|c| c.has_sensitive_content = true);
        let prompt = build_prompt(&context, &config, None);
        assert!(prompt.contains("sensitive content"));
        assert!(prompt.contains("first line"));
    }

    #[test]
    fn omits_sensitive_content_note() {
        let config = Config::default();
        let context = make_context(|c| c.has_sensitive_content = false);
        let prompt = build_prompt(&context, &config, None);
        assert!(!prompt.contains("sensitive content"));
    }

    // --- buildBranchPrompt ---

    #[test]
    fn branch_prompt_conventional_contains_type_slug() {
        let config = Config::default();
        let prompt = build_branch_prompt("add login feature", None, &config, BranchMode::Conventional, &[]);
        assert!(prompt.contains("type/short-description-slug"));
        assert!(prompt.contains("feat, fix, docs"));
        assert!(prompt.contains("add login feature"));
    }

    #[test]
    fn branch_prompt_adaptive_includes_branches() {
        let config = Config::default();
        let branches = vec!["feat/add-login".to_owned(), "fix/auth-bug".to_owned()];
        let prompt = build_branch_prompt("", Some("diff here"), &config, BranchMode::Adaptive, &branches);
        assert!(prompt.contains("feat/add-login"));
        assert!(prompt.contains("fix/auth-bug"));
        assert!(prompt.contains("Match the naming style"));
    }

    #[test]
    fn branch_prompt_adaptive_no_branches_falls_back() {
        let config = Config::default();
        let prompt = build_branch_prompt("desc", None, &config, BranchMode::Adaptive, &[]);
        assert!(prompt.contains("type/short-description-slug"));
    }

    // --- buildRefinePrompt ---

    #[test]
    fn refine_prompt_includes_all_fields() {
        let config = Config::default();
        let prompt = build_refine_prompt("feat: add login", "make it shorter", "diff here", &config);
        assert!(prompt.contains("feat: add login"));
        assert!(prompt.contains("make it shorter"));
        assert!(prompt.contains("diff here"));
    }
}
