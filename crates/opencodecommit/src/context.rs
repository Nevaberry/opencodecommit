use std::path::Path;
use std::sync::LazyLock;

use crate::Result;
use crate::config::DiffSource;
use crate::git;
use crate::sensitive::{SensitiveFinding, scan_diff_for_sensitive_content};

/// Truncation strategy applied to a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TruncationMode {
    Full,
    Sections,
    Outline,
    Skipped,
}

impl std::fmt::Display for TruncationMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TruncationMode::Full => write!(f, "full"),
            TruncationMode::Sections => write!(f, "sections"),
            TruncationMode::Outline => write!(f, "outline"),
            TruncationMode::Skipped => write!(f, "skipped"),
        }
    }
}

/// File content with truncation metadata.
#[derive(Debug, Clone)]
pub struct FileContext {
    pub path: String,
    pub content: String,
    pub truncation_mode: TruncationMode,
}

/// Full context for commit message generation.
#[derive(Debug, Clone)]
pub struct CommitContext {
    pub diff: String,
    pub recent_commits: Vec<String>,
    pub branch: String,
    pub file_contents: Vec<FileContext>,
    pub changed_files: Vec<String>,
    pub sensitive_findings: Vec<SensitiveFinding>,
    pub has_sensitive_content: bool,
}

// --- Skip patterns ---

static SKIP_PATTERNS: LazyLock<Vec<regex::Regex>> = LazyLock::new(|| {
    [
        r"\.lock$",
        r"package-lock\.json$",
        r"yarn\.lock$",
        r"pnpm-lock\.yaml$",
        r"bun\.lockb$",
        r"Cargo\.lock$",
        r"Gemfile\.lock$",
        r"poetry\.lock$",
        r"composer\.lock$",
        r"go\.sum$",
        r"\.min\.js$",
        r"\.min\.css$",
        r"\.map$",
        r"\.bundle\.js$",
        r"\.png$",
        r"\.jpg$",
        r"\.jpeg$",
        r"\.gif$",
        r"\.ico$",
        r"\.woff2?$",
        r"\.ttf$",
        r"\.eot$",
        r"(?:^|/)dist/",
        r"(?:^|/)build/",
        r"(?:^|/)node_modules/",
        r"(?:^|/)\.next/",
        r"(?:^|/)__pycache__/",
    ]
    .iter()
    .map(|p| regex::Regex::new(p).unwrap())
    .collect()
});

/// Detect if the diff or changed files contain sensitive content.
pub fn detect_sensitive_content(diff: &str, changed_files: &[String]) -> bool {
    !detect_sensitive_findings(diff, changed_files).is_empty()
}

/// Return structured findings for sensitive content matches.
pub fn detect_sensitive_findings(diff: &str, changed_files: &[String]) -> Vec<SensitiveFinding> {
    scan_diff_for_sensitive_content(diff, changed_files).findings
}

/// Check if a file should be skipped for context reading.
pub fn should_skip(file_path: &str) -> bool {
    SKIP_PATTERNS.iter().any(|p| p.is_match(file_path))
}

// --- Signature pattern for outline mode ---

static SIGNATURE_PATTERN: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r"^(?:export\s+)?(?:default\s+)?(?:async\s+)?(?:function|class|interface|type|const|let|var|enum|abstract\s+class|public|private|protected|def |fn )\b",
    )
    .unwrap()
});

/// Extract changed file paths from a unified diff.
pub fn extract_changed_file_paths(diff: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let re = regex::Regex::new(r"^diff --git a/.+ b/(.+)$").unwrap();
    for line in diff.lines() {
        if let Some(caps) = re.captures(line) {
            paths.push(caps[1].to_owned());
        }
    }
    paths
}

/// Extract hunk start line numbers for a specific file from a diff.
fn get_hunk_line_numbers(diff: &str, file_path: &str) -> Vec<usize> {
    let mut lines = Vec::new();
    let mut in_file = false;
    let hunk_re = regex::Regex::new(r"^@@ -\d+(?:,\d+)? \+(\d+)").unwrap();

    for line in diff.lines() {
        if line.starts_with("diff --git") {
            in_file = line.contains(&format!("b/{file_path}"));
            continue;
        }
        if in_file
            && let Some(caps) = hunk_re.captures(line)
            && let Ok(n) = caps[1].parse::<usize>()
        {
            lines.push(n);
        }
    }
    lines
}

/// Read a file with smart truncation.
fn read_file_content(file_path: &str, repo_root: &Path, diff: &str) -> FileContext {
    let full_path = repo_root.join(file_path);

    // Guard against path traversal
    if let (Ok(resolved), Ok(resolved_root)) = (full_path.canonicalize(), repo_root.canonicalize())
    {
        if !resolved.starts_with(&resolved_root) {
            return FileContext {
                path: file_path.to_owned(),
                content: String::new(),
                truncation_mode: TruncationMode::Skipped,
            };
        }
    }

    let content = match std::fs::read_to_string(&full_path) {
        Ok(c) => c,
        Err(_) => {
            return FileContext {
                path: file_path.to_owned(),
                content: String::new(),
                truncation_mode: TruncationMode::Skipped,
            };
        }
    };

    let file_lines: Vec<&str> = content.lines().collect();
    let line_count = file_lines.len();

    // Full mode: ≤500 lines
    if line_count <= 500 {
        return FileContext {
            path: file_path.to_owned(),
            content,
            truncation_mode: TruncationMode::Full,
        };
    }

    let hunk_lines = get_hunk_line_numbers(diff, file_path);

    // Sections mode: ≤2000 lines — header + context windows around hunks
    if line_count <= 2000 {
        let mut parts = Vec::new();
        let header_end = 30.min(file_lines.len());
        parts.push(file_lines[..header_end].join("\n"));

        for &hunk_line in &hunk_lines {
            let start = hunk_line.saturating_sub(25);
            let end = (hunk_line + 25).min(file_lines.len());
            parts.push(format!("\n... (line {}) ...\n", start + 1));
            parts.push(file_lines[start..end].join("\n"));
        }

        return FileContext {
            path: file_path.to_owned(),
            content: parts.join("\n"),
            truncation_mode: TruncationMode::Sections,
        };
    }

    // Outline mode: >2000 lines — signatures + hunk windows
    let mut parts: Vec<String> = Vec::new();
    for line in &file_lines {
        if SIGNATURE_PATTERN.is_match(line.trim()) {
            parts.push(line.to_string());
        }
    }

    for &hunk_line in &hunk_lines {
        let start = hunk_line.saturating_sub(10);
        let end = (hunk_line + 10).min(file_lines.len());
        parts.push(format!("\n... (line {}) ...\n", start + 1));
        parts.push(file_lines[start..end].join("\n"));
    }

    FileContext {
        path: file_path.to_owned(),
        content: parts.join("\n"),
        truncation_mode: TruncationMode::Outline,
    }
}

/// Read file contents for changed files with a total character budget.
pub fn get_file_contents(
    changed_files: &[String],
    repo_root: &Path,
    diff: &str,
) -> Vec<FileContext> {
    const TOTAL_BUDGET: usize = 30_000;
    let mut results = Vec::new();
    let mut total_chars = 0;

    // Filter skipped files and sort by file size (smallest first)
    let mut files_with_size: Vec<_> = changed_files
        .iter()
        .filter(|f| !should_skip(f))
        .map(|f| {
            let size = repo_root
                .join(f)
                .metadata()
                .map(|m| m.len() as usize)
                .unwrap_or(0);
            (f.as_str(), size)
        })
        .collect();
    files_with_size.sort_by_key(|&(_, size)| size);

    for (file, _) in files_with_size {
        if total_chars >= TOTAL_BUDGET {
            break;
        }

        let mut fc = read_file_content(file, repo_root, diff);
        if fc.truncation_mode == TruncationMode::Skipped || fc.content.is_empty() {
            continue;
        }

        // Trim to fit budget
        let remaining = TOTAL_BUDGET - total_chars;
        if fc.content.len() > remaining {
            fc.content = format!(
                "{}\n... (truncated to fit context budget)",
                &fc.content[..remaining]
            );
        }

        total_chars += fc.content.len();
        results.push(fc);
    }

    results
}

/// Gather full context for commit message generation.
pub fn gather_context(repo_root: &Path, diff_source: DiffSource) -> Result<CommitContext> {
    let diff = git::get_diff(diff_source, repo_root)?;
    let recent_commits = git::get_recent_commits(repo_root, 10).unwrap_or_default();
    let branch = git::get_branch_name(repo_root).unwrap_or_else(|_| "unknown".to_owned());
    let changed_files = extract_changed_file_paths(&diff);
    let sensitive_findings = detect_sensitive_findings(&diff, &changed_files);
    let has_sensitive_content = !sensitive_findings.is_empty();
    let file_contents = get_file_contents(&changed_files, repo_root, &diff);

    Ok(CommitContext {
        diff,
        recent_commits,
        branch,
        file_contents,
        changed_files,
        sensitive_findings,
        has_sensitive_content,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- detectSensitiveContent tests (ported from TS) ---

    #[test]
    fn detects_env_file() {
        assert!(detect_sensitive_content("some diff", &[".env".to_owned()]));
    }

    #[test]
    fn detects_env_production() {
        assert!(detect_sensitive_content(
            "some diff",
            &[".env.production".to_owned()]
        ));
    }

    #[test]
    fn detects_nested_env_file() {
        assert!(detect_sensitive_content(
            "some diff",
            &["config/.env.local".to_owned()]
        ));
    }

    #[test]
    fn detects_credentials_json() {
        assert!(detect_sensitive_content(
            "some diff",
            &["credentials.json".to_owned()]
        ));
    }

    #[test]
    fn detects_api_key_in_added_lines() {
        let diff = "diff --git a/config.ts b/config.ts\n+const API_KEY = \"sk-abc123\"";
        assert!(detect_sensitive_content(diff, &["config.ts".to_owned()]));
    }

    #[test]
    fn detects_secret_key_in_added_lines() {
        let diff = "+  SECRET_KEY: \"my-secret\"";
        assert!(detect_sensitive_content(diff, &["config.ts".to_owned()]));
    }

    #[test]
    fn detects_access_token_in_added_lines() {
        let diff = "+export const ACCESS_TOKEN = process.env.TOKEN";
        assert!(detect_sensitive_content(diff, &["auth.ts".to_owned()]));
    }

    #[test]
    fn detects_password_in_added_lines() {
        let diff = "+  DB_PASSWORD=hunter2";
        assert!(detect_sensitive_content(diff, &["config.ts".to_owned()]));
    }

    #[test]
    fn detects_sk_prefixed_keys() {
        let diff = "+  key: \"sk-abcdefghijklmnopqrstuvwxyz\"";
        assert!(detect_sensitive_content(diff, &["config.ts".to_owned()]));
    }

    #[test]
    fn detects_ghp_tokens() {
        let diff = "+  GITHUB_TOKEN=ghp_abcdefghijklmnopqrstuvwxyz1234";
        assert!(detect_sensitive_content(diff, &["ci.yml".to_owned()]));
    }

    #[test]
    fn detects_aws_access_keys() {
        let diff = "+  aws_key = \"AKIAIOSFODNN7EXAMPLE\"";
        assert!(detect_sensitive_content(diff, &["config.ts".to_owned()]));
    }

    #[test]
    fn ignores_removed_lines() {
        let diff = "-  API_KEY = \"old-key\"";
        assert!(!detect_sensitive_content(diff, &["config.ts".to_owned()]));
    }

    #[test]
    fn ignores_diff_header_lines() {
        let diff = "+++ b/API_KEY_handler.ts";
        assert!(!detect_sensitive_content(
            diff,
            &["API_KEY_handler.ts".to_owned()]
        ));
    }

    #[test]
    fn returns_false_for_normal_code() {
        let diff = "+  const result = await fetchData()";
        assert!(!detect_sensitive_content(diff, &["app.ts".to_owned()]));
    }

    #[test]
    fn detects_source_map_files() {
        assert!(detect_sensitive_content(
            "diff",
            &["bundle.js.map".to_owned()]
        ));
        assert!(detect_sensitive_content(
            "diff",
            &["styles.css.map".to_owned()]
        ));
        assert!(detect_sensitive_content(
            "diff",
            &["dist/app.map".to_owned()]
        ));
    }

    #[test]
    fn detects_private_key_files() {
        assert!(detect_sensitive_content("diff", &["server.pem".to_owned()]));
        assert!(detect_sensitive_content("diff", &["cert.p12".to_owned()]));
        assert!(detect_sensitive_content("diff", &["ssl.key".to_owned()]));
        assert!(detect_sensitive_content(
            "diff",
            &["app.keystore".to_owned()]
        ));
    }

    #[test]
    fn detects_ssh_private_keys() {
        assert!(detect_sensitive_content("diff", &["id_rsa".to_owned()]));
        assert!(detect_sensitive_content("diff", &["id_ed25519".to_owned()]));
        assert!(detect_sensitive_content(
            "diff",
            &[".ssh/config".to_owned()]
        ));
    }

    #[test]
    fn detects_htpasswd() {
        assert!(detect_sensitive_content("diff", &[".htpasswd".to_owned()]));
    }

    // --- skip patterns ---

    #[test]
    fn skips_lock_files() {
        assert!(should_skip("package-lock.json"));
        assert!(should_skip("yarn.lock"));
        assert!(should_skip("Cargo.lock"));
        assert!(should_skip("bun.lockb"));
    }

    #[test]
    fn skips_minified_files() {
        assert!(should_skip("bundle.min.js"));
        assert!(should_skip("styles.min.css"));
    }

    #[test]
    fn skips_images_and_fonts() {
        assert!(should_skip("logo.png"));
        assert!(should_skip("icon.jpg"));
        assert!(should_skip("font.woff2"));
        assert!(should_skip("font.ttf"));
    }

    #[test]
    fn skips_dist_and_build() {
        assert!(should_skip("dist/bundle.js"));
        assert!(should_skip("build/output.js"));
        assert!(should_skip("node_modules/pkg/index.js"));
    }

    #[test]
    fn does_not_skip_source_files() {
        assert!(!should_skip("src/app.ts"));
        assert!(!should_skip("lib/utils.rs"));
        assert!(!should_skip("README.md"));
    }

    // --- extract_changed_file_paths ---

    #[test]
    fn extracts_file_paths_from_diff() {
        let diff = "diff --git a/src/app.ts b/src/app.ts\nindex abc..def 100644\n--- a/src/app.ts\n+++ b/src/app.ts\n@@ -1,3 +1,4 @@\n+import something\ndiff --git a/lib/utils.ts b/lib/utils.ts\n";
        let paths = extract_changed_file_paths(diff);
        assert_eq!(paths, vec!["src/app.ts", "lib/utils.ts"]);
    }
}
