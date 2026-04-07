use std::collections::HashMap;
use std::fmt;
use std::sync::LazyLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensitiveFinding {
    pub category: &'static str,
    pub rule: &'static str,
    pub file_path: String,
    pub line_number: Option<usize>,
    pub preview: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SensitiveReport {
    pub findings: Vec<SensitiveFinding>,
}

impl SensitiveReport {
    pub fn from_findings(findings: Vec<SensitiveFinding>) -> Self {
        Self { findings }
    }

    pub fn has_findings(&self) -> bool {
        !self.findings.is_empty()
    }

    pub fn format_occ_commit_message(&self) -> String {
        self.format_block_message(
            "Sensitive content detected in diff. Use --allow-sensitive to skip this check.",
        )
    }

    pub fn format_git_hook_message(&self) -> String {
        self.format_block_message(
            "Commit blocked by OpenCodeCommit.\nBypass only OCC for this command with: OCC_ALLOW_SENSITIVE=1 git commit ...",
        )
    }

    fn format_block_message(&self, footer: &str) -> String {
        if self.findings.is_empty() {
            return footer.to_owned();
        }

        let mut lines = vec!["Sensitive findings:".to_owned()];
        for finding in &self.findings {
            let location = match finding.line_number {
                Some(line) => format!("{}:{}", finding.file_path, line),
                None => finding.file_path.clone(),
            };
            lines.push(format!(
                "- {} [{} / {}] {}",
                location, finding.category, finding.rule, finding.preview
            ));
        }
        lines.push(footer.to_owned());
        lines.join("\n")
    }
}

impl fmt::Display for SensitiveReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_occ_commit_message())
    }
}

#[derive(Debug, Clone)]
struct FileRule {
    pattern: regex::Regex,
    category: &'static str,
    rule: &'static str,
}

#[derive(Debug, Clone)]
struct LineRule {
    pattern: regex::Regex,
    category: &'static str,
    rule: &'static str,
}

#[derive(Debug, Clone)]
struct DiffFileEntry {
    path: String,
    deleted: bool,
}

static DIFF_FILE_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^diff --git a/.+ b/(.+)$").unwrap());

static DIFF_HUNK_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@").unwrap());

static IPV4_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b").unwrap());

static FILE_RULES: LazyLock<Vec<FileRule>> = LazyLock::new(|| {
    [
        (r"(?:^|/)\.env(?:\.\w+)?$", "filename", "env-file"),
        (r"(?:^|/)credentials\.json$", "filename", "credentials-json"),
        (r"(?:^|/)secrets?\.\w+$", "filename", "secret-file"),
        (r"(?:^|/)\.netrc$", "filename", "netrc"),
        (
            r"(?:^|/)service[-_]?account.*\.json$",
            "filename",
            "service-account",
        ),
        (r"\.(?:js|css)\.map$", "filename", "source-map"),
        (r"(?:^|/)[^/]+\.map$", "filename", "source-map"),
        (r"\.pem$", "filename", "private-key"),
        (r"\.p12$", "filename", "private-key"),
        (r"\.pfx$", "filename", "private-key"),
        (r"\.key$", "filename", "private-key"),
        (r"\.keystore$", "filename", "private-key"),
        (r"\.jks$", "filename", "private-key"),
        (
            r"(?:^|/)id_(?:rsa|ed25519|ecdsa|dsa)$",
            "filename",
            "ssh-private-key",
        ),
        (r"(?:^|/)\.ssh/", "filename", "ssh-config"),
        (r"(?:^|/)\.htpasswd$", "filename", "auth-file"),
    ]
    .into_iter()
    .map(|(pattern, category, rule)| FileRule {
        pattern: regex::Regex::new(pattern).unwrap(),
        category,
        rule,
    })
    .collect()
});

static LINE_RULES: LazyLock<Vec<LineRule>> = LazyLock::new(|| {
    [
        (r"\bsk-[A-Za-z0-9]{20,}", "token", "openai-key"),
        (r"\bghp_[A-Za-z0-9]{20,}", "token", "github-token"),
        (r"\bAKIA[A-Z0-9]{12,}", "token", "aws-access-key"),
        (
            r"(?i)\bBEARER\s+[A-Za-z0-9_.~+/\-]{20,}",
            "token",
            "bearer-token",
        ),
        (r"(?i)\bAPI[_-]?KEY\b", "credential", "api-key-marker"),
        (r"(?i)\bSECRET[_-]?KEY\b", "credential", "secret-key-marker"),
        (
            r"(?i)\bACCESS[_-]?TOKEN\b",
            "credential",
            "access-token-marker",
        ),
        (r"(?i)\bAUTH[_-]?TOKEN\b", "credential", "auth-token-marker"),
        (
            r"(?i)\bPRIVATE[_-]?KEY\b",
            "credential",
            "private-key-marker",
        ),
        (r"(?i)\bPASSWORD\b", "credential", "password-marker"),
        (r"(?i)\bPASSWD\b", "credential", "passwd-marker"),
        (
            r"(?i)\bDB[_-]?PASSWORD\b",
            "credential",
            "db-password-marker",
        ),
        (
            r"(?i)\bDATABASE[_-]?URL\b",
            "credential",
            "database-url-marker",
        ),
        (
            r"(?i)\bCLIENT[_-]?SECRET\b",
            "credential",
            "client-secret-marker",
        ),
        (r"(?i)\bAWS[_-]?SECRET\b", "credential", "aws-secret-marker"),
        (r"(?i)\bGH[_-]?TOKEN\b", "credential", "gh-token-marker"),
        (r"(?i)\bNPM[_-]?TOKEN\b", "credential", "npm-token-marker"),
        (
            r"(?i)\bSLACK[_-]?TOKEN\b",
            "credential",
            "slack-token-marker",
        ),
        (
            r"(?i)\bSTRIPE[_-]?(?:SECRET|KEY)\b",
            "credential",
            "stripe-secret-marker",
        ),
        (
            r"(?i)\bSENDGRID[_-]?(?:API)?[_-]?KEY\b",
            "credential",
            "sendgrid-key-marker",
        ),
        (
            r"(?i)\bTWILIO[_-]?(?:AUTH|SID)\b",
            "credential",
            "twilio-secret-marker",
        ),
        (r"(?i)\bCREDENTIALS?\b", "credential", "credentials-marker"),
    ]
    .into_iter()
    .map(|(pattern, category, rule)| LineRule {
        pattern: regex::Regex::new(pattern).unwrap(),
        category,
        rule,
    })
    .collect()
});

static SECRET_ASSIGNMENT_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r#"(?i)\b([A-Z0-9_.-]*(?:KEY|TOKEN|PASSWORD|PASSWD|SECRET|URL|CREDENTIALS?|SID)\b\s*[:=]\s*)(?:"[^"]*"|'[^']*'|[^\s,;]+)"#,
    )
    .unwrap()
});

static LONG_SECRET_REPLACERS: LazyLock<Vec<regex::Regex>> = LazyLock::new(|| {
    [
        r"\bsk-[A-Za-z0-9]{20,}",
        r"\bghp_[A-Za-z0-9]{20,}",
        r"\bAKIA[A-Z0-9]{12,}",
        r"(?i)\bBEARER\s+[A-Za-z0-9_.~+/\-]{20,}",
    ]
    .into_iter()
    .map(|pattern| regex::Regex::new(pattern).unwrap())
    .collect()
});

pub fn scan_diff_for_sensitive_content(diff: &str, changed_files: &[String]) -> SensitiveReport {
    let deletion_state: HashMap<String, bool> = parse_diff_file_entries(diff)
        .into_iter()
        .map(|entry| (entry.path, entry.deleted))
        .collect();

    let mut findings = Vec::new();
    for file in changed_files {
        if deletion_state.get(file).copied().unwrap_or(false) {
            continue;
        }

        if let Some(finding) = scan_file_path(file) {
            findings.push(finding);
        }
    }

    let fallback_file = changed_files
        .first()
        .filter(|_| changed_files.len() == 1)
        .cloned();

    let mut current_file = fallback_file;
    let mut current_line: Option<usize> = None;

    for line in diff.lines() {
        if let Some(captures) = DIFF_FILE_RE.captures(line) {
            current_file = Some(captures[1].to_owned());
            current_line = None;
            continue;
        }

        if let Some(captures) = DIFF_HUNK_RE.captures(line) {
            current_line = captures[1].parse::<usize>().ok();
            continue;
        }

        if line.starts_with("+++") {
            continue;
        }

        if let Some(added_line) = line.strip_prefix('+') {
            let file_path = current_file.clone().unwrap_or_else(|| "unknown".to_owned());
            let line_number = current_line;
            if let Some(finding) = scan_added_line(&file_path, added_line, line_number) {
                findings.push(finding);
            }
            if let Some(line_no) = current_line.as_mut() {
                *line_no += 1;
            }
            continue;
        }

        if line.starts_with(' ')
            && let Some(line_no) = current_line.as_mut()
        {
            *line_no += 1;
        }
    }

    SensitiveReport::from_findings(findings)
}

fn parse_diff_file_entries(diff: &str) -> Vec<DiffFileEntry> {
    let mut entries = Vec::new();
    let mut current: Option<DiffFileEntry> = None;

    for line in diff.lines() {
        if let Some(captures) = DIFF_FILE_RE.captures(line) {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            current = Some(DiffFileEntry {
                path: captures[1].to_owned(),
                deleted: false,
            });
            continue;
        }

        if (line == "deleted file mode 100644"
            || line == "deleted file mode 100755"
            || line == "+++ /dev/null")
            && let Some(entry) = current.as_mut()
        {
            entry.deleted = true;
        }
    }

    if let Some(entry) = current {
        entries.push(entry);
    }

    entries
}

fn scan_file_path(file_path: &str) -> Option<SensitiveFinding> {
    FILE_RULES.iter().find_map(|rule| {
        rule.pattern.is_match(file_path).then(|| SensitiveFinding {
            category: rule.category,
            rule: rule.rule,
            file_path: file_path.to_owned(),
            line_number: None,
            preview: file_path.to_owned(),
        })
    })
}

fn scan_added_line(
    file_path: &str,
    line: &str,
    line_number: Option<usize>,
) -> Option<SensitiveFinding> {
    if let Some(rule) = LINE_RULES.iter().find(|rule| rule.pattern.is_match(line)) {
        return Some(SensitiveFinding {
            category: rule.category,
            rule: rule.rule,
            file_path: file_path.to_owned(),
            line_number,
            preview: redact_line_preview(line),
        });
    }

    if first_sensitive_ipv4(line).is_some() {
        return Some(SensitiveFinding {
            category: "network",
            rule: "ipv4-address",
            file_path: file_path.to_owned(),
            line_number,
            preview: redact_line_preview(line),
        });
    }

    None
}

fn first_sensitive_ipv4(line: &str) -> Option<&str> {
    IPV4_RE.find_iter(line).find_map(|matched| {
        let ip = matched.as_str();
        parse_ipv4(ip)
            .filter(|parsed| !is_example_ipv4(*parsed))
            .map(|_| ip)
    })
}

fn parse_ipv4(value: &str) -> Option<[u8; 4]> {
    let mut octets = [0_u8; 4];
    let mut count = 0;

    for (index, part) in value.split('.').enumerate() {
        if index >= 4 {
            return None;
        }
        octets[index] = part.parse::<u8>().ok()?;
        count += 1;
    }

    (count == 4).then_some(octets)
}

fn is_example_ipv4(ip: [u8; 4]) -> bool {
    matches!(
        (ip[0], ip[1], ip[2]),
        (192, 0, 2) | (198, 51, 100) | (203, 0, 113)
    )
}

fn redact_line_preview(line: &str) -> String {
    let mut preview = line.trim().to_owned();
    preview = SECRET_ASSIGNMENT_RE
        .replace_all(&preview, "${1}<redacted>")
        .to_string();

    for regex in LONG_SECRET_REPLACERS.iter() {
        preview = regex.replace_all(&preview, "<redacted>").to_string();
    }

    preview = IPV4_RE
        .replace_all(&preview, |captures: &regex::Captures<'_>| {
            if first_sensitive_ipv4(captures.get(0).unwrap().as_str()).is_some() {
                "<redacted-ip>".to_owned()
            } else {
                captures.get(0).unwrap().as_str().to_owned()
            }
        })
        .to_string();

    if preview.len() > 160 {
        preview.truncate(157);
        preview.push_str("...");
    }

    preview
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_sensitive_filename() {
        let report = scan_diff_for_sensitive_content("diff", &[".env.production".to_owned()]);
        assert!(report.has_findings());
        assert_eq!(report.findings[0].rule, "env-file");
        assert_eq!(report.findings[0].preview, ".env.production");
    }

    #[test]
    fn ignores_deleted_sensitive_filename() {
        let diff = "\
diff --git a/.env b/.env
deleted file mode 100644
index 1234567..0000000
--- a/.env
+++ /dev/null
@@ -1 +0,0 @@
-API_KEY=secret
";
        let report = scan_diff_for_sensitive_content(diff, &[".env".to_owned()]);
        assert!(!report.has_findings());
    }

    #[test]
    fn detects_line_rule_with_line_number() {
        let diff = "\
diff --git a/src/config.ts b/src/config.ts
index 1234567..89abcde 100644
--- a/src/config.ts
+++ b/src/config.ts
@@ -10,0 +11,2 @@
+const API_KEY = \"sk-abcdefghijklmnopqrstuvwxyz\";
+const safe = true;
";
        let report = scan_diff_for_sensitive_content(diff, &["src/config.ts".to_owned()]);
        assert!(report.has_findings());
        assert_eq!(report.findings[0].file_path, "src/config.ts");
        assert_eq!(report.findings[0].line_number, Some(11));
        assert!(report.findings[0].preview.contains("<redacted>"));
    }

    #[test]
    fn detects_non_example_ipv4_literals() {
        let diff = "\
diff --git a/src/app.ts b/src/app.ts
--- a/src/app.ts
+++ b/src/app.ts
@@ -1 +1,2 @@
+const host = \"10.24.8.12\";
";
        let report = scan_diff_for_sensitive_content(diff, &["src/app.ts".to_owned()]);
        assert!(report.has_findings());
        assert_eq!(report.findings[0].rule, "ipv4-address");
        assert!(report.findings[0].preview.contains("<redacted-ip>"));
    }

    #[test]
    fn allows_documentation_example_ipv4_literals() {
        let diff = "\
diff --git a/README.md b/README.md
--- a/README.md
+++ b/README.md
@@ -1 +1,2 @@
+Example server: 203.0.113.10
";
        let report = scan_diff_for_sensitive_content(diff, &["README.md".to_owned()]);
        assert!(!report.has_findings());
    }

    #[test]
    fn formats_git_hook_message_with_bypass_instruction() {
        let report = SensitiveReport::from_findings(vec![SensitiveFinding {
            category: "credential",
            rule: "api-key-marker",
            file_path: "src/config.ts".to_owned(),
            line_number: Some(18),
            preview: "const API_KEY = <redacted>".to_owned(),
        }]);

        let message = report.format_git_hook_message();
        assert!(message.contains("Commit blocked by OpenCodeCommit"));
        assert!(message.contains("src/config.ts:18"));
        assert!(message.contains("OCC_ALLOW_SENSITIVE=1 git commit"));
    }

    #[test]
    fn formats_occ_commit_message_with_allow_sensitive_instruction() {
        let report = SensitiveReport::from_findings(vec![SensitiveFinding {
            category: "credential",
            rule: "api-key-marker",
            file_path: "src/config.ts".to_owned(),
            line_number: Some(18),
            preview: "const API_KEY = <redacted>".to_owned(),
        }]);

        let message = report.format_occ_commit_message();
        assert!(message.contains("Sensitive findings:"));
        assert!(message.contains("src/config.ts:18"));
        assert!(message.contains("--allow-sensitive"));
    }
}
