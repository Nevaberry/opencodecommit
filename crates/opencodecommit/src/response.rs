use std::sync::LazyLock;

use crate::config::{Config, DEFAULT_EMOJIS};

/// Parsed conventional commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCommit {
    pub type_name: String,
    pub message: String,
    pub description: Option<String>,
}

static TYPE_PATTERN: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"^(feat|fix|docs|style|refactor|test|chore|perf|security|revert)(\(.*?\))?:\s*(.+)")
        .unwrap()
});

static ANSI_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap());

static PREAMBLE_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r"(?i)^(?:(?:Here(?:'s| is)|Sure[,.].*?(?:here|is)|I(?:'ll| will).*?:?)\s*(?:your |the |a )?(?:commit )?(?:message|response)?[:\s]*\n+)",
    )
    .unwrap()
});

static CODE_BLOCK_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?s)^```(?:\w*)\n(.*?)\n```$").unwrap());

/// Sanitize an AI response: strip ANSI codes, preamble, code blocks, formatting.
pub fn sanitize_response(response: &str) -> String {
    let mut result = response.trim().to_owned();

    // Strip ANSI codes
    result = ANSI_RE.replace_all(&result, "").to_string();

    // Strip preamble
    result = PREAMBLE_RE.replace(&result, "").trim().to_owned();

    // Strip code blocks: ```\n...\n``` or ```lang\n...\n```
    result = CODE_BLOCK_RE.replace(&result, "$1").trim().to_owned();

    // Strip inline backticks wrapping entire response
    if result.starts_with('`') && result.ends_with('`') && !result.contains('\n') {
        result = result[1..result.len() - 1].trim().to_owned();
    }

    // Strip wrapping quotes
    if (result.starts_with('"') && result.ends_with('"'))
        || (result.starts_with('\'') && result.ends_with('\''))
    {
        result = result[1..result.len() - 1].trim().to_owned();
    }

    // Strip markdown bold/italic wrapping entire response
    if result.starts_with("**") && result.ends_with("**") {
        result = result[2..result.len() - 2].trim().to_owned();
    } else if result.starts_with('*') && result.ends_with('*') && !result.starts_with("**") {
        result = result[1..result.len() - 1].trim().to_owned();
    }

    result
}

/// Parse a response into a conventional commit structure.
pub fn parse_response(response: &str) -> ParsedCommit {
    let sanitized = sanitize_response(response);
    let lines: Vec<&str> = sanitized.lines().collect();
    let first_line = lines.first().map(|l| l.trim()).unwrap_or("");

    if let Some(caps) = TYPE_PATTERN.captures(first_line) {
        let type_name = caps.get(1).unwrap().as_str().to_owned();
        let message = caps.get(3).unwrap().as_str().to_owned();
        let remaining: Vec<&str> = lines[1..].iter().filter(|l| !l.trim().is_empty()).copied().collect();
        let description = if remaining.is_empty() {
            None
        } else {
            Some(remaining.join("\n"))
        };
        return ParsedCommit {
            type_name,
            message,
            description,
        };
    }

    ParsedCommit {
        type_name: infer_type(first_line),
        message: if first_line.is_empty() {
            "update code".to_owned()
        } else {
            first_line.to_owned()
        },
        description: None,
    }
}

/// Infer a conventional commit type from an unstructured message.
fn infer_type(message: &str) -> String {
    let lower = message.to_lowercase();
    static DOCS_RE: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"\b(readme|docs?|documentation|changelog|comment|jsdoc|rustdoc)\b").unwrap());
    static FIX_RE: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"\b(fix|bug|patch|resolve|issue|error|crash|repair)\b").unwrap());
    static FEAT_RE: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"\b(add|implement|feature|new|introduce|support|create)\b").unwrap());
    static REFACTOR_RE: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"\b(refactor|restructure|reorganize|rename|move|extract|simplify)\b").unwrap());
    static TEST_RE: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"\b(tests?|spec|assert|coverage)\b").unwrap());
    static STYLE_RE: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"\b(style|format|whitespace|indent|lint|prettier|biome)\b").unwrap());
    static PERF_RE: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"\b(perf|performance|optimiz|speed|faster|cache)\b").unwrap());
    static REVERT_RE: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"\b(revert|undo|rollback)\b").unwrap());

    if DOCS_RE.is_match(&lower) { return "docs".to_owned(); }
    if FIX_RE.is_match(&lower) { return "fix".to_owned(); }
    if FEAT_RE.is_match(&lower) { return "feat".to_owned(); }
    if REFACTOR_RE.is_match(&lower) { return "refactor".to_owned(); }
    if TEST_RE.is_match(&lower) { return "test".to_owned(); }
    if STYLE_RE.is_match(&lower) { return "style".to_owned(); }
    if PERF_RE.is_match(&lower) { return "perf".to_owned(); }
    static SECURITY_RE: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"\b(security|vulnerab|auth|cve|xss|csrf|injection|sanitiz)\b").unwrap());
    if REVERT_RE.is_match(&lower) { return "revert".to_owned(); }
    if SECURITY_RE.is_match(&lower) { return "security".to_owned(); }
    "chore".to_owned()
}

/// Format a parsed commit using the config template, emojis, lowercase.
pub fn format_commit_message(parsed: &ParsedCommit, config: &Config) -> String {
    let mut message = parsed.message.clone();

    // Apply lowercase
    if config.use_lower_case && !message.is_empty() {
        let mut chars = message.chars();
        if let Some(first) = chars.next() {
            message = first.to_lowercase().to_string() + chars.as_str();
        }
    }

    // Resolve emoji
    let mut emoji = String::new();
    if config.use_emojis {
        // Check custom emojis first
        if let Some(e) = config.custom.emojis.get(&parsed.type_name) {
            emoji = e.clone();
        }
        // Fallback to defaults
        if emoji.is_empty() {
            for &(t, e) in DEFAULT_EMOJIS {
                if t == parsed.type_name {
                    emoji = e.to_owned();
                    break;
                }
            }
        }
    }

    // Apply template
    let mut result = config
        .commit_template
        .replace("{{type}}", &parsed.type_name)
        .replace("{{emoji}}", &emoji)
        .replace("{{message}}", &message);

    // Clean up: collapse multiple spaces, remove space before colon
    static MULTI_SPACE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"\s+").unwrap());
    static SPACE_COLON: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"\s+:").unwrap());

    result = MULTI_SPACE.replace_all(&result, " ").to_string();
    result = SPACE_COLON.replace_all(&result, ":").to_string();
    result = result.trim().to_owned();

    // Append description
    if let Some(ref desc) = parsed.description {
        result.push_str(&format!("\n\n{desc}"));
    }

    result
}

/// Format an adaptive-mode response (sanitize only, no type parsing).
pub fn format_adaptive_message(response: &str) -> String {
    let sanitized = sanitize_response(response);
    if sanitized.is_empty() {
        "update code".to_owned()
    } else {
        sanitized
    }
}

/// Format a branch name from AI response: sanitize, slugify.
pub fn format_branch_name(response: &str) -> String {
    let sanitized = sanitize_response(response);
    let name = sanitized.lines().next().unwrap_or("").trim();
    if name.is_empty() {
        return "chore/update".to_owned();
    }
    // Already in type/slug format? Return as-is if it looks right.
    if name.contains('/') && !name.contains(' ') && name.len() <= 60 {
        return name.to_lowercase();
    }
    // Slugify: lowercase, replace non-alphanumeric with hyphens, collapse
    let slug: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '/' || c == '-' { c } else { '-' })
        .collect();
    static MULTI_HYPHEN: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"-{2,}").unwrap());
    let slug = MULTI_HYPHEN.replace_all(&slug, "-");
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "chore/update".to_owned()
    } else {
        slug.to_owned()
    }
}

/// Parsed PR output.
#[derive(Debug, Clone)]
pub struct ParsedPr {
    pub title: String,
    pub body: String,
}

/// Parse a PR response into title and body.
pub fn parse_pr_response(response: &str) -> ParsedPr {
    let sanitized = sanitize_response(response);
    let lines: Vec<&str> = sanitized.lines().collect();

    let mut title = String::new();
    let mut body_start = 0;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if let Some(t) = trimmed.strip_prefix("TITLE:") {
            title = t.trim().to_owned();
            body_start = i + 1;
            break;
        }
    }

    // Skip "BODY:" line if present
    if body_start < lines.len() && lines[body_start].trim().starts_with("BODY:") {
        body_start += 1;
    }

    let body = if body_start < lines.len() {
        lines[body_start..].join("\n").trim().to_owned()
    } else {
        String::new()
    };

    if title.is_empty() {
        // Fallback: first line is title, rest is body
        ParsedPr {
            title: lines.first().unwrap_or(&"Update").to_string(),
            body: if lines.len() > 1 {
                lines[1..].join("\n").trim().to_owned()
            } else {
                String::new()
            },
        }
    } else {
        ParsedPr { title, body }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_config(overrides: impl FnOnce(&mut Config)) -> Config {
        let mut cfg = Config::default();
        overrides(&mut cfg);
        cfg
    }

    // --- sanitizeResponse tests (ported from TS) ---

    #[test]
    fn sanitize_strips_code_block() {
        assert_eq!(
            sanitize_response("```\nfeat: add login\n```"),
            "feat: add login"
        );
    }

    #[test]
    fn sanitize_strips_code_block_with_language() {
        assert_eq!(
            sanitize_response("```text\nfeat: add login\n```"),
            "feat: add login"
        );
    }

    #[test]
    fn sanitize_strips_inline_backticks() {
        assert_eq!(sanitize_response("`feat: add login`"), "feat: add login");
    }

    #[test]
    fn sanitize_strips_double_quotes() {
        assert_eq!(
            sanitize_response("\"feat: add login\""),
            "feat: add login"
        );
    }

    #[test]
    fn sanitize_strips_single_quotes() {
        assert_eq!(
            sanitize_response("'feat: add login'"),
            "feat: add login"
        );
    }

    #[test]
    fn sanitize_strips_markdown_bold() {
        assert_eq!(
            sanitize_response("**feat: add login**"),
            "feat: add login"
        );
    }

    #[test]
    fn sanitize_strips_markdown_italic() {
        assert_eq!(
            sanitize_response("*feat: add login*"),
            "feat: add login"
        );
    }

    #[test]
    fn sanitize_trims_whitespace() {
        assert_eq!(
            sanitize_response("  feat: add login  "),
            "feat: add login"
        );
    }

    #[test]
    fn sanitize_handles_clean_input() {
        assert_eq!(sanitize_response("feat: add login"), "feat: add login");
    }

    #[test]
    fn sanitize_strips_ansi() {
        assert_eq!(
            sanitize_response("\x1b[32mfeat: add login\x1b[0m"),
            "feat: add login"
        );
    }

    #[test]
    fn sanitize_strips_preamble() {
        assert_eq!(
            sanitize_response("Here's your commit message:\nfeat: add login"),
            "feat: add login"
        );
    }

    #[test]
    fn sanitize_strips_sure_preamble() {
        assert_eq!(
            sanitize_response("Sure, here is the commit message:\nfeat: add login"),
            "feat: add login"
        );
    }

    // --- parseResponse tests (ported from TS) ---

    #[test]
    fn parse_conventional_commit() {
        let result = parse_response("feat: add login page");
        assert_eq!(result.type_name, "feat");
        assert_eq!(result.message, "add login page");
        assert!(result.description.is_none());
    }

    #[test]
    fn parse_commit_with_scope() {
        let result = parse_response("fix(auth): resolve token expiry");
        assert_eq!(result.type_name, "fix");
        assert_eq!(result.message, "resolve token expiry");
    }

    #[test]
    fn parse_multiline_response() {
        let result =
            parse_response("feat: update authentication\n\n- add JWT tokens\n- remove session cookies");
        assert_eq!(result.type_name, "feat");
        assert_eq!(result.message, "update authentication");
        let desc = result.description.unwrap();
        assert!(desc.contains("add JWT tokens"));
        assert!(desc.contains("remove session cookies"));
    }

    #[test]
    fn parse_malformed_fallback() {
        let result = parse_response("just some random text");
        assert_eq!(result.type_name, "chore");
        assert_eq!(result.message, "just some random text");
    }

    #[test]
    fn parse_infers_docs_type() {
        let result = parse_response("update README with installation instructions");
        assert_eq!(result.type_name, "docs");
        assert_eq!(result.message, "update README with installation instructions");
    }

    #[test]
    fn parse_infers_feat_type() {
        let result = parse_response("add new login page");
        assert_eq!(result.type_name, "feat");
    }

    #[test]
    fn parse_infers_fix_type() {
        let result = parse_response("fix crash on startup");
        assert_eq!(result.type_name, "fix");
    }

    #[test]
    fn parse_empty_response() {
        let result = parse_response("");
        assert_eq!(result.type_name, "chore");
        assert_eq!(result.message, "update code");
    }

    #[test]
    fn parse_code_block_wrapped() {
        let result = parse_response("```\nfeat: add login\n```");
        assert_eq!(result.type_name, "feat");
        assert_eq!(result.message, "add login");
    }

    #[test]
    fn parse_all_valid_types() {
        let types = [
            "feat", "fix", "docs", "style", "refactor", "test", "chore", "perf", "revert",
        ];
        for t in types {
            let result = parse_response(&format!("{t}: some message"));
            assert_eq!(result.type_name, t, "failed for type: {t}");
        }
    }

    // --- formatCommitMessage tests (ported from TS) ---

    #[test]
    fn format_default_template() {
        let config = Config::default();
        let result = format_commit_message(
            &ParsedCommit {
                type_name: "feat".to_owned(),
                message: "Add login".to_owned(),
                description: None,
            },
            &config,
        );
        assert_eq!(result, "feat: add login");
    }

    #[test]
    fn format_applies_lowercase() {
        let config = make_config(|c| c.use_lower_case = true);
        let result = format_commit_message(
            &ParsedCommit {
                type_name: "feat".to_owned(),
                message: "Add login".to_owned(),
                description: None,
            },
            &config,
        );
        assert_eq!(result, "feat: add login");
    }

    #[test]
    fn format_preserves_case() {
        let config = make_config(|c| c.use_lower_case = false);
        let result = format_commit_message(
            &ParsedCommit {
                type_name: "feat".to_owned(),
                message: "Add login".to_owned(),
                description: None,
            },
            &config,
        );
        assert_eq!(result, "feat: Add login");
    }

    #[test]
    fn format_emoji_without_template_placeholder() {
        let config = make_config(|c| c.use_emojis = true);
        let result = format_commit_message(
            &ParsedCommit {
                type_name: "feat".to_owned(),
                message: "add login".to_owned(),
                description: None,
            },
            &config,
        );
        // Default template has no {{emoji}} placeholder
        assert_eq!(result, "feat: add login");
    }

    #[test]
    fn format_emoji_with_template() {
        let config = make_config(|c| {
            c.use_emojis = true;
            c.commit_template = "{{emoji}} {{type}}: {{message}}".to_owned();
        });
        let result = format_commit_message(
            &ParsedCommit {
                type_name: "feat".to_owned(),
                message: "add login".to_owned(),
                description: None,
            },
            &config,
        );
        assert_eq!(result, "\u{2728} feat: add login");
    }

    #[test]
    fn format_custom_emoji_override() {
        let config = make_config(|c| {
            c.use_emojis = true;
            c.commit_template = "{{emoji}} {{type}}: {{message}}".to_owned();
            c.custom.emojis = HashMap::from([("feat".to_owned(), "\u{1f680}".to_owned())]);
        });
        let result = format_commit_message(
            &ParsedCommit {
                type_name: "feat".to_owned(),
                message: "add login".to_owned(),
                description: None,
            },
            &config,
        );
        assert_eq!(result, "\u{1f680} feat: add login");
    }

    #[test]
    fn format_appends_description() {
        let config = Config::default();
        let result = format_commit_message(
            &ParsedCommit {
                type_name: "feat".to_owned(),
                message: "Update auth".to_owned(),
                description: Some("- add JWT\n- remove cookies".to_owned()),
            },
            &config,
        );
        assert!(result.starts_with("feat: update auth"));
        assert!(result.contains("- add JWT"));
        assert!(result.contains("- remove cookies"));
    }

    #[test]
    fn format_collapses_multiple_spaces() {
        let config = make_config(|c| {
            c.commit_template = "{{type}}:  {{message}}".to_owned();
        });
        let result = format_commit_message(
            &ParsedCommit {
                type_name: "feat".to_owned(),
                message: "add login".to_owned(),
                description: None,
            },
            &config,
        );
        assert!(!result.contains("  "));
    }

    // --- format_adaptive_message ---

    #[test]
    fn adaptive_returns_sanitized() {
        assert_eq!(
            format_adaptive_message("```\nfeat: add login\n```"),
            "feat: add login"
        );
    }

    #[test]
    fn adaptive_empty_fallback() {
        assert_eq!(format_adaptive_message(""), "update code");
    }
}
