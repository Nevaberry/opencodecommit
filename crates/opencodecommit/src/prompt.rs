use crate::config::{BranchMode, CommitMode, Config};
use crate::context::CommitContext;
use crate::languages;

/// Build the prompt for commit message generation.
pub fn build_prompt(context: &CommitContext, config: &Config, mode: Option<CommitMode>) -> String {
    // Custom prompt override
    if !config.custom.prompt.is_empty() {
        return config.custom.prompt.replace("{diff}", &context.diff);
    }

    let active_mode = mode.unwrap_or(config.commit_mode);
    let mods = config.active_prompt_modules();
    let mut parts: Vec<String> = Vec::new();

    // Base (always)
    parts.push(mods.base_module);

    // Format module
    match active_mode {
        CommitMode::Adaptive | CommitMode::AdaptiveOneliner => {
            let recent_text = if context.recent_commits.is_empty() {
                "(no recent commits)".to_owned()
            } else {
                context.recent_commits.join("\n")
            };
            parts.push(
                mods.adaptive_format
                    .replace("{recentCommits}", &recent_text),
            );
        }
        CommitMode::Conventional | CommitMode::ConventionalOneliner => {
            parts.push(mods.conventional_format);
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
            parts.push(mods.oneliner_length);
        }
        _ => {
            parts.push(mods.multiline_length);
        }
    }

    // Sensitive content note
    if context.has_sensitive_content {
        parts.push(mods.sensitive_content_note);
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
    languages::REFINE_TEMPLATE
        .replace("{currentMessage}", current_message)
        .replace("{feedback}", feedback)
        .replace("{maxDiffLength}", &config.max_diff_length.to_string())
        .replace("{diff}", diff)
        .replace(
            "{languageInstruction}",
            &config.active_language_instruction(),
        )
}

/// Build the prompt for branch name generation.
pub fn build_branch_prompt(
    context_or_description: &str,
    diff: Option<&str>,
    config: &Config,
    mode: BranchMode,
    existing_branches: &[String],
) -> String {
    let mut parts = vec![languages::BRANCH_EXPERT.to_owned()];

    match mode {
        BranchMode::Conventional => {
            parts.push(languages::BRANCH_CONVENTIONAL.to_owned());
        }
        BranchMode::Adaptive => {
            if existing_branches.is_empty() {
                parts.push(languages::BRANCH_CONVENTIONAL.to_owned());
            } else {
                let branch_text = existing_branches.join("\n");
                parts.push(
                    languages::BRANCH_ADAPTIVE_FORMAT.replace("{existingBranches}", &branch_text),
                );
            }
        }
    }

    parts.push(languages::BRANCH_RESPOND_ONLY.to_owned());
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
    let mut parts = vec![languages::PR_EXPERT.to_owned()];

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

/// Build stage-1 summarization prompt for the two-stage PR pipeline.
pub fn build_pr_summary_prompt(diff: &str, commits: &[String], config: &Config) -> String {
    let commit_text = if commits.is_empty() {
        "(no commit messages available)".to_owned()
    } else {
        commits.join("\n---\n")
    };

    let mut prompt = languages::PR_SUMMARIZER
        .replace("{commits}", &commit_text)
        .replace("{diff}", diff);

    prompt.push_str("\n\n");
    prompt.push_str(&config.active_language_instruction());

    prompt
}

/// Build stage-2 PR generation prompt from a stage-1 summary.
pub fn build_pr_final_prompt(
    summary: &str,
    branch: &str,
    commit_onelines: &[String],
    config: &Config,
) -> String {
    let mut parts = vec![languages::PR_EXPERT.to_owned()];

    parts.push(config.active_language_instruction());

    if !commit_onelines.is_empty() {
        parts.push("Commits in this branch:".to_owned());
        parts.push(commit_onelines.join("\n"));
    }

    parts.push(format!("Branch: {branch}"));
    parts.push("--- Change Summary (from code review) ---".to_owned());
    parts.push(summary.to_owned());

    parts.join("\n\n")
}

/// Build the prompt for changelog entry generation.
pub fn build_changelog_prompt(context: &CommitContext, config: &Config) -> String {
    let mut parts = vec![languages::CHANGELOG_EXPERT.to_owned()];

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
            sensitive_report: crate::sensitive::SensitiveReport::from_findings(vec![]),
            sensitive_findings: vec![],
            has_sensitive_content: false,
        };
        f(&mut ctx);
        ctx
    }

    // --- buildPrompt tests ---

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
            c.active_language = "Finnish".to_owned();
        });
        let context = make_context(|_| {});
        let prompt = build_prompt(&context, &config, None);
        assert!(prompt.contains("Kirjoita commit-viesti suomeksi"));
    }

    #[test]
    fn finnish_uses_finnish_prompt_modules() {
        let config = make_config(|c| {
            c.active_language = "Finnish".to_owned();
        });
        let context = make_context(|_| {});
        let prompt = build_prompt(&context, &config, None);
        assert!(prompt.contains("Olet asiantuntija git-commit-viestien kirjoittamisessa"));
        assert!(!prompt.contains("You are an expert at writing git commit messages"));
    }

    #[test]
    fn custom_language_falls_back_to_english_modules() {
        let config = make_config(|c| {
            c.active_language = "Custom (example)".to_owned();
        });
        let context = make_context(|_| {});
        let prompt = build_prompt(&context, &config, None);
        // Should have English base module (fallback) but Custom instruction
        assert!(prompt.contains("You are an expert at writing git commit messages"));
        assert!(prompt.contains("your preferred language and style"));
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

    #[test]
    fn finnish_sensitive_note() {
        let config = make_config(|c| {
            c.active_language = "Finnish".to_owned();
        });
        let context = make_context(|c| c.has_sensitive_content = true);
        let prompt = build_prompt(&context, &config, None);
        assert!(prompt.contains("arkaluonteista sisältöä"));
    }

    #[test]
    fn finnish_conventional_mode() {
        let config = make_config(|c| {
            c.active_language = "Finnish".to_owned();
        });
        let context = make_context(|_| {});
        let prompt = build_prompt(&context, &config, Some(CommitMode::Conventional));
        assert!(prompt.contains("conventional commit -muotoa"));
    }

    #[test]
    fn finnish_adaptive_mode() {
        let config = make_config(|c| {
            c.active_language = "Finnish".to_owned();
        });
        let context = make_context(|c| {
            c.recent_commits = vec!["abc feat: test".to_owned()];
        });
        let prompt = build_prompt(&context, &config, Some(CommitMode::Adaptive));
        assert!(prompt.contains("Noudata alla näkyvien"));
    }

    // --- buildBranchPrompt ---

    #[test]
    fn branch_prompt_conventional_contains_type_slug() {
        let config = Config::default();
        let prompt = build_branch_prompt(
            "add login feature",
            None,
            &config,
            BranchMode::Conventional,
            &[],
        );
        assert!(prompt.contains("type/short-description-slug"));
        assert!(prompt.contains("feat, fix, docs"));
        assert!(prompt.contains("add login feature"));
    }

    #[test]
    fn branch_prompt_adaptive_includes_branches() {
        let config = Config::default();
        let branches = vec!["feat/add-login".to_owned(), "fix/auth-bug".to_owned()];
        let prompt = build_branch_prompt(
            "",
            Some("diff here"),
            &config,
            BranchMode::Adaptive,
            &branches,
        );
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
        let prompt =
            build_refine_prompt("feat: add login", "make it shorter", "diff here", &config);
        assert!(prompt.contains("feat: add login"));
        assert!(prompt.contains("make it shorter"));
        assert!(prompt.contains("diff here"));
    }

    #[test]
    fn pr_summary_prompt_includes_commits_and_diff() {
        let config = Config::default();
        let commits = vec![
            "feat: add login page".to_owned(),
            "fix: handle edge case".to_owned(),
        ];
        let prompt = build_pr_summary_prompt("diff content", &commits, &config);
        assert!(prompt.contains("feat: add login page"));
        assert!(prompt.contains("fix: handle edge case"));
        assert!(prompt.contains("diff content"));
        assert!(prompt.contains("expert code reviewer"));
    }

    #[test]
    fn pr_summary_prompt_empty_commits() {
        let config = Config::default();
        let prompt = build_pr_summary_prompt("diff", &[], &config);
        assert!(prompt.contains("no commit messages available"));
    }

    #[test]
    fn pr_final_prompt_includes_summary_and_branch() {
        let config = Config::default();
        let onelines = vec!["abc123 feat: add login".to_owned()];
        let prompt = build_pr_final_prompt("summary text", "feature/login", &onelines, &config);
        assert!(prompt.contains("summary text"));
        assert!(prompt.contains("feature/login"));
        assert!(prompt.contains("abc123 feat: add login"));
        assert!(prompt.contains("Change Summary"));
    }
}
