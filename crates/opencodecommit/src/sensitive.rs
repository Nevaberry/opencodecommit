use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SensitiveTier {
    ConfirmedSecret,
    SensitiveArtifact,
    Suspicious,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SensitiveSeverity {
    Block,
    Warn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum SensitiveEnforcement {
    #[default]
    Warn,
    BlockHigh,
    BlockAll,
    StrictHigh,
    StrictAll,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct SensitiveAllowlistEntry {
    #[serde(default)]
    pub path_regex: Option<String>,
    #[serde(default)]
    pub rule: Option<String>,
    #[serde(default)]
    pub value_regex: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensitiveFinding {
    pub category: &'static str,
    pub rule: &'static str,
    pub file_path: String,
    pub line_number: Option<usize>,
    pub preview: String,
    pub tier: SensitiveTier,
    pub severity: SensitiveSeverity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensitiveReport {
    pub findings: Vec<SensitiveFinding>,
    pub enforcement: SensitiveEnforcement,
    pub warning_count: usize,
    pub blocking_count: usize,
    pub has_findings: bool,
    pub has_blocking_findings: bool,
}

impl SensitiveReport {
    pub fn from_findings(findings: Vec<SensitiveFinding>) -> Self {
        Self::from_findings_with_enforcement(findings, SensitiveEnforcement::Warn)
    }

    pub fn from_findings_with_enforcement(
        findings: Vec<SensitiveFinding>,
        enforcement: SensitiveEnforcement,
    ) -> Self {
        let mut warning_count = 0;
        let mut blocking_count = 0;

        for finding in &findings {
            if is_blocking_finding(finding, enforcement) {
                blocking_count += 1;
            } else {
                warning_count += 1;
            }
        }

        Self {
            has_findings: !findings.is_empty(),
            has_blocking_findings: blocking_count > 0,
            findings,
            enforcement,
            warning_count,
            blocking_count,
        }
    }

    pub fn has_findings(&self) -> bool {
        self.has_findings
    }

    pub fn has_blocking_findings(&self) -> bool {
        self.has_blocking_findings
    }

    pub fn format_occ_commit_message(&self) -> String {
        if self.has_blocking_findings {
            let footer = if allows_sensitive_bypass(self.enforcement) {
                "Sensitive content detected in diff. Use --allow-sensitive to skip this check."
            } else {
                "Sensitive content detected in diff. Strict sensitive mode is active; change the config to continue."
            };
            self.format_message(footer)
        } else {
            self.format_message("Sensitive findings are warnings only.")
        }
    }

    pub fn format_git_hook_message(&self) -> String {
        if self.has_blocking_findings {
            let footer = if allows_sensitive_bypass(self.enforcement) {
                "Commit blocked by OpenCodeCommit.\nBypass only OCC for this command with: OCC_ALLOW_SENSITIVE=1 git commit ..."
            } else {
                "Commit blocked by OpenCodeCommit.\nStrict sensitive mode is active; change the config to continue."
            };
            self.format_message(footer)
        } else {
            self.format_message("OpenCodeCommit warning: sensitive findings detected.")
        }
    }

    fn format_message(&self, footer: &str) -> String {
        if self.findings.is_empty() {
            return footer.to_owned();
        }

        let mut lines = vec!["Sensitive findings:".to_owned()];
        for finding in &self.findings {
            let location = match finding.line_number {
                Some(line) => format!("{}:{}", finding.file_path, line),
                None => finding.file_path.clone(),
            };
            let action = if is_blocking_finding(finding, self.enforcement) {
                "BLOCK"
            } else {
                "WARN"
            };
            lines.push(format!(
                "- {} {} [{:?} / {}] {}",
                action, location, finding.tier, finding.rule, finding.preview
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
struct DiffFileEntry {
    path: String,
    deleted: bool,
}

#[derive(Debug, Clone)]
struct PathContext {
    normalized_path: String,
    lower_path: String,
    skip_content: bool,
    low_confidence: bool,
    env_template: bool,
    env_file: bool,
    docker_config: bool,
    npmrc: bool,
    kube_config: bool,
}

#[derive(Debug, Clone)]
struct ProviderRule {
    pattern: regex::Regex,
    category: &'static str,
    rule: &'static str,
    tier: SensitiveTier,
    severity: SensitiveSeverity,
}

#[derive(Debug, Clone)]
struct LineCandidate {
    category: &'static str,
    rule: &'static str,
    file_path: String,
    line_number: Option<usize>,
    preview: String,
    raw_value: Option<String>,
    tier: SensitiveTier,
    severity: SensitiveSeverity,
}

static DIFF_FILE_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^diff --git a/.+ b/(.+)$").unwrap());

static DIFF_HUNK_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@").unwrap());

static COMMENT_ONLY_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^\s*(?:#|//|/\*|\*|--|%|rem\b|')").unwrap());

static IPV4_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b").unwrap());

static PRIVATE_KEY_HEADER_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"-----BEGIN (?:(?:RSA|DSA|EC|OPENSSH|PGP) )?PRIVATE KEY(?: BLOCK)?-----")
        .unwrap()
});

static ENCRYPTED_PRIVATE_KEY_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"-----BEGIN ENCRYPTED PRIVATE KEY-----").unwrap());

static CONNECTION_STRING_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r#"\b((?:postgres(?:ql)?|mysql|mongodb(?:\+srv)?|redis|rediss|amqp|amqps|mssql|sqlserver)://)([^/\s:@]+):([^@\s]+)@([^\s'"]+)"#,
    )
    .unwrap()
});

static BEARER_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r#"(?i)\b(?:authorization|bearer)\b\s*[:=]\s*['"]?bearer\s+([A-Za-z0-9._~+/\-]{20,})"#,
    )
    .unwrap()
});

static JWT_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_.+/=-]{10,}\b")
        .unwrap()
});

static DOCKER_AUTH_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r#""auth"\s*:\s*"([^"]+)""#).unwrap());

static KUBECONFIG_AUTH_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"(?:^|\b)(token|client-key-data)\b\s*:\s*("?[^"\s]+"?)"#).unwrap()
});

static NPM_LITERAL_AUTH_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r#"(?i)(?::|^)_(?:authToken|auth|password)\s*=\s*([^\s#]+)|//[^\s]+:_authToken\s*=\s*([^\s#]+)"#,
    )
    .unwrap()
});

static GENERIC_ASSIGNMENT_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        "(?i)\\b([A-Za-z0-9_.-]{0,40}(?:password|passwd|pwd|secret|token|api[_-]?key|apikey|auth[_-]?token|access[_-]?token|private[_-]?key|client[_-]?secret|credentials?|database[_-]?url|db[_-]?password|webhook[_-]?secret|signing[_-]?key|encryption[_-]?key)[A-Za-z0-9_.-]{0,20})\\b[\"']?\\s*[:=]\\s*(\"[^\"]*\"|'[^']*'|[^\\s,#;]+)",
    )
    .unwrap()
});

static TEMPLATE_ENV_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r"(?:^|/)(?:\.env\.(?:example|sample|template|defaults|schema|spec|test|ci)|[^/]*\.(?:example|sample|template)\.env)$",
    )
    .unwrap()
});

static REAL_ENV_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?:^|/)\.env(?:\.[^/]+)?$|(?:^|/)\.envrc$|(?:^|/)\.direnv/").unwrap()
});

static LOW_CONFIDENCE_PATH_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r"(?:^|/)(?:test|tests|__tests__|spec|__spec__|docs|documentation|example|examples|sample|samples|fixture|fixtures|__fixtures__|testdata|test-data|mock|mocks|__mocks__|stubs?)(?:/|$)",
    )
    .unwrap()
});

static LOW_CONFIDENCE_EXT_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\.(?:md|rst|adoc|txt|d\.ts|schema\.json|schema\.ya?ml)$").unwrap()
});

static SKIP_CONTENT_PATH_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r"(?i)(?:^|/)(?:vendor|node_modules|third_party|\.git)(?:/|$)|(?:^|/)(?:package-lock\.json|yarn\.lock|pnpm-lock\.yaml|Gemfile\.lock|Cargo\.lock|poetry\.lock|composer\.lock|go\.sum|Pipfile\.lock)$|\.(?:png|jpe?g|gif|bmp|ico|svg|tiff|webp|mp[34]|avi|mov|wav|flac|ogg|woff2?|eot|otf|ttf|exe|dll|so|dylib|bin|o|a|class|pyc|pyo|wasm|zip|tar|gz|bz2|xz|rar|7z|jar|war|ear)$",
    )
    .unwrap()
});

static PROVIDER_RULES: LazyLock<Vec<ProviderRule>> = LazyLock::new(|| {
    [
        (
            r"github_pat_[A-Za-z0-9]{22}_[A-Za-z0-9]{59}",
            "token",
            "github-fine-grained-token",
            SensitiveTier::ConfirmedSecret,
            SensitiveSeverity::Block,
        ),
        (
            r"gh[pousr]_[A-Za-z0-9]{36,76}",
            "token",
            "github-token",
            SensitiveTier::ConfirmedSecret,
            SensitiveSeverity::Block,
        ),
        (
            r"(?:AKIA|ASIA)[A-Z0-9]{16}",
            "token",
            "aws-access-key",
            SensitiveTier::ConfirmedSecret,
            SensitiveSeverity::Block,
        ),
        (
            r"gl(?:pat|dt|ptt|rt)-[0-9A-Za-z_-]{20,}",
            "token",
            "gitlab-token",
            SensitiveTier::ConfirmedSecret,
            SensitiveSeverity::Block,
        ),
        (
            r"xoxb-[0-9]+-[0-9A-Za-z]+-[A-Za-z0-9]+",
            "token",
            "slack-bot-token",
            SensitiveTier::ConfirmedSecret,
            SensitiveSeverity::Block,
        ),
        (
            r"(?i)xoxp-[0-9]+-[0-9]+-[0-9]+-[a-f0-9]+",
            "token",
            "slack-user-token",
            SensitiveTier::ConfirmedSecret,
            SensitiveSeverity::Block,
        ),
        (
            r"xapp-1-[A-Z0-9]+-[0-9]+-[A-Za-z0-9]+",
            "token",
            "slack-app-token",
            SensitiveTier::ConfirmedSecret,
            SensitiveSeverity::Block,
        ),
        (
            r"https://hooks\.slack\.com/services/T[a-zA-Z0-9_]+/B[a-zA-Z0-9_]+/[a-zA-Z0-9_]+",
            "webhook",
            "slack-webhook",
            SensitiveTier::ConfirmedSecret,
            SensitiveSeverity::Block,
        ),
        (
            r"sk_live_[0-9A-Za-z]{24,}",
            "token",
            "stripe-live-secret-key",
            SensitiveTier::ConfirmedSecret,
            SensitiveSeverity::Block,
        ),
        (
            r"rk_live_[0-9A-Za-z]{24,}",
            "token",
            "stripe-live-restricted-key",
            SensitiveTier::ConfirmedSecret,
            SensitiveSeverity::Block,
        ),
        (
            r"sk_test_[0-9A-Za-z]{24,}",
            "token",
            "stripe-test-secret-key",
            SensitiveTier::Suspicious,
            SensitiveSeverity::Warn,
        ),
        (
            r"rk_test_[0-9A-Za-z]{24,}",
            "token",
            "stripe-test-restricted-key",
            SensitiveTier::Suspicious,
            SensitiveSeverity::Warn,
        ),
        (
            r"SG\.[A-Za-z0-9_-]{22}\.[A-Za-z0-9_-]{43}",
            "token",
            "sendgrid-api-key",
            SensitiveTier::ConfirmedSecret,
            SensitiveSeverity::Block,
        ),
        (
            r"sk-proj-[A-Za-z0-9_-]{20,}",
            "token",
            "openai-project-key",
            SensitiveTier::ConfirmedSecret,
            SensitiveSeverity::Block,
        ),
        (
            r"sk-svcacct-[A-Za-z0-9_-]{20,}",
            "token",
            "openai-service-account-key",
            SensitiveTier::ConfirmedSecret,
            SensitiveSeverity::Block,
        ),
        (
            r"\bsk-[A-Za-z0-9]{32,}\b",
            "token",
            "openai-legacy-key",
            SensitiveTier::ConfirmedSecret,
            SensitiveSeverity::Block,
        ),
        (
            r"sk-ant-(?:api03|admin01)-[A-Za-z0-9_-]{80,}",
            "token",
            "anthropic-key",
            SensitiveTier::ConfirmedSecret,
            SensitiveSeverity::Block,
        ),
        (
            r"AIza[0-9A-Za-z_-]{35}",
            "token",
            "gcp-api-key",
            SensitiveTier::ConfirmedSecret,
            SensitiveSeverity::Block,
        ),
        (
            r"GOCSPX-[A-Za-z0-9_-]{28}",
            "token",
            "gcp-oauth-secret",
            SensitiveTier::ConfirmedSecret,
            SensitiveSeverity::Block,
        ),
        (
            r"npm_[A-Za-z0-9]{36}",
            "token",
            "npm-token",
            SensitiveTier::ConfirmedSecret,
            SensitiveSeverity::Block,
        ),
        (
            r"pypi-[A-Za-z0-9_-]{50,}",
            "token",
            "pypi-token",
            SensitiveTier::ConfirmedSecret,
            SensitiveSeverity::Block,
        ),
        (
            r"dckr_pat_[A-Za-z0-9_-]{20,}",
            "token",
            "docker-token",
            SensitiveTier::ConfirmedSecret,
            SensitiveSeverity::Block,
        ),
        (
            r"sntrys_[A-Za-z0-9+/=_-]{20,}",
            "token",
            "sentry-token",
            SensitiveTier::ConfirmedSecret,
            SensitiveSeverity::Block,
        ),
        (
            r"(?i)key-[0-9a-f]{32}",
            "token",
            "mailgun-key",
            SensitiveTier::ConfirmedSecret,
            SensitiveSeverity::Block,
        ),
        (
            r"hvs\.[A-Za-z0-9_-]{24,}",
            "token",
            "vault-token",
            SensitiveTier::ConfirmedSecret,
            SensitiveSeverity::Block,
        ),
        (
            r"https://discord(?:app)?\.com/api/webhooks/[0-9]+/[A-Za-z0-9_-]+",
            "webhook",
            "discord-webhook",
            SensitiveTier::ConfirmedSecret,
            SensitiveSeverity::Block,
        ),
        (
            "(?i)https://[a-z0-9.-]+\\.webhook\\.office\\.com/[^\\s'\"`]+",
            "webhook",
            "teams-webhook",
            SensitiveTier::ConfirmedSecret,
            SensitiveSeverity::Block,
        ),
        (
            r"AGE-SECRET-KEY-1[qpzry9x8gf2tvdw0s3jn54khce6mua7l]{58}",
            "key",
            "age-secret-key",
            SensitiveTier::ConfirmedSecret,
            SensitiveSeverity::Block,
        ),
    ]
    .into_iter()
    .map(|(pattern, category, rule, tier, severity)| ProviderRule {
        pattern: regex::Regex::new(pattern).unwrap(),
        category,
        rule,
        tier,
        severity,
    })
    .collect()
});

pub fn allows_sensitive_bypass(enforcement: SensitiveEnforcement) -> bool {
    matches!(
        enforcement,
        SensitiveEnforcement::Warn
            | SensitiveEnforcement::BlockHigh
            | SensitiveEnforcement::BlockAll
    )
}

pub fn is_blocking_finding(finding: &SensitiveFinding, enforcement: SensitiveEnforcement) -> bool {
    match enforcement {
        SensitiveEnforcement::Warn => false,
        SensitiveEnforcement::BlockHigh | SensitiveEnforcement::StrictHigh => {
            finding.severity == SensitiveSeverity::Block
        }
        SensitiveEnforcement::BlockAll | SensitiveEnforcement::StrictAll => true,
    }
}

pub fn scan_diff_for_sensitive_content(diff: &str, changed_files: &[String]) -> SensitiveReport {
    scan_diff_for_sensitive_content_with_options(
        diff,
        changed_files,
        SensitiveEnforcement::Warn,
        &[],
    )
}

pub fn scan_diff_for_sensitive_content_with_options(
    diff: &str,
    changed_files: &[String],
    enforcement: SensitiveEnforcement,
    allowlist: &[SensitiveAllowlistEntry],
) -> SensitiveReport {
    let deletion_state: HashMap<String, bool> = parse_diff_file_entries(diff)
        .into_iter()
        .map(|entry| (entry.path, entry.deleted))
        .collect();

    let mut findings = Vec::new();
    for file in changed_files {
        let info = classify_path(file);
        if deletion_state
            .get(info.normalized_path.as_str())
            .copied()
            .unwrap_or(false)
        {
            continue;
        }

        findings.extend(scan_file_path(&info, allowlist));
    }

    let fallback_file = changed_files
        .first()
        .filter(|_| changed_files.len() == 1)
        .map(|file| normalize_path(file));

    let mut current_file = fallback_file;
    let mut current_info = current_file.as_deref().map(classify_path_from_normalized);
    let mut current_line: Option<usize> = None;

    for line in diff.lines() {
        if let Some(captures) = DIFF_FILE_RE.captures(line) {
            current_file = Some(normalize_path(&captures[1]));
            current_info = current_file.as_deref().map(classify_path_from_normalized);
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
            let info = current_info
                .clone()
                .unwrap_or_else(|| classify_path_from_normalized(file_path.as_str()));
            if !info.skip_content {
                findings.extend(scan_added_line(
                    &file_path,
                    &info,
                    added_line,
                    current_line,
                    allowlist,
                ));
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

    SensitiveReport::from_findings_with_enforcement(dedupe_findings(findings), enforcement)
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
                path: normalize_path(&captures[1]),
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

fn normalize_path(file_path: &str) -> String {
    file_path.replace('\\', "/")
}

fn classify_path(file_path: &str) -> PathContext {
    classify_path_from_normalized(&normalize_path(file_path))
}

fn classify_path_from_normalized(file_path: &str) -> PathContext {
    let normalized_path = file_path.to_owned();
    let lower_path = normalized_path.to_lowercase();
    let env_template = TEMPLATE_ENV_RE.is_match(lower_path.as_str());
    let env_file = REAL_ENV_RE.is_match(lower_path.as_str()) && !env_template;

    PathContext {
        normalized_path,
        lower_path: lower_path.clone(),
        skip_content: SKIP_CONTENT_PATH_RE.is_match(lower_path.as_str()),
        low_confidence: LOW_CONFIDENCE_PATH_RE.is_match(lower_path.as_str())
            || LOW_CONFIDENCE_EXT_RE.is_match(lower_path.as_str()),
        env_template,
        env_file,
        docker_config: lower_path.ends_with("/.docker/config.json")
            || lower_path == ".docker/config.json"
            || lower_path.ends_with("/.dockercfg")
            || lower_path == ".dockercfg",
        npmrc: lower_path.ends_with("/.npmrc") || lower_path == ".npmrc",
        kube_config: lower_path.ends_with("/kubeconfig")
            || lower_path == "kubeconfig"
            || lower_path.ends_with("/.kube/config")
            || lower_path == ".kube/config",
    }
}

fn scan_file_path(
    info: &PathContext,
    allowlist: &[SensitiveAllowlistEntry],
) -> Vec<SensitiveFinding> {
    let mut findings = Vec::new();

    let mut push = |category: &'static str,
                    rule: &'static str,
                    tier: SensitiveTier,
                    severity: SensitiveSeverity| {
        push_candidate(
            &mut findings,
            allowlist,
            LineCandidate {
                category,
                rule,
                file_path: info.normalized_path.clone(),
                line_number: None,
                preview: info.normalized_path.clone(),
                raw_value: None,
                tier,
                severity,
            },
        );
    };

    if info.env_file {
        push(
            "artifact",
            "env-file",
            SensitiveTier::SensitiveArtifact,
            SensitiveSeverity::Block,
        );
    } else if info.lower_path.ends_with("/.netrc")
        || info.lower_path == ".netrc"
        || info.lower_path.ends_with("/.git-credentials")
        || info.lower_path == ".git-credentials"
    {
        push(
            "artifact",
            "credential-store-file",
            SensitiveTier::SensitiveArtifact,
            SensitiveSeverity::Block,
        );
    } else if info.docker_config {
        push(
            "artifact",
            "docker-config-file",
            SensitiveTier::Suspicious,
            SensitiveSeverity::Warn,
        );
    } else if info.npmrc {
        push(
            "artifact",
            "npmrc-file",
            SensitiveTier::Suspicious,
            SensitiveSeverity::Warn,
        );
    } else if info.lower_path.ends_with("/.pypirc")
        || info.lower_path == ".pypirc"
        || info.lower_path.ends_with("/.gem/credentials")
        || info.lower_path == ".gem/credentials"
        || regex::Regex::new(r"(?:^|/)\.cargo/credentials(?:\.toml)?$")
            .unwrap()
            .is_match(info.lower_path.as_str())
    {
        push(
            "artifact",
            "package-manager-credential-file",
            SensitiveTier::SensitiveArtifact,
            SensitiveSeverity::Block,
        );
    } else if regex::Regex::new(r"terraform\.tfstate(?:\.backup)?$")
        .unwrap()
        .is_match(info.lower_path.as_str())
        || info.lower_path.contains("/.terraform/")
    {
        push(
            "artifact",
            "terraform-state-file",
            SensitiveTier::SensitiveArtifact,
            SensitiveSeverity::Block,
        );
    } else if info.lower_path.ends_with(".tfvars") || info.lower_path.ends_with(".auto.tfvars") {
        push(
            "artifact",
            "terraform-vars-file",
            SensitiveTier::Suspicious,
            SensitiveSeverity::Warn,
        );
    } else if info.kube_config {
        push(
            "artifact",
            "kubeconfig-file",
            SensitiveTier::SensitiveArtifact,
            SensitiveSeverity::Block,
        );
    } else if regex::Regex::new(r"(?:^|/)credentials\.json$")
        .unwrap()
        .is_match(info.lower_path.as_str())
        || regex::Regex::new(r"(?:^|/)service[-_]?account.*\.json$")
            .unwrap()
            .is_match(info.lower_path.as_str())
    {
        push(
            "artifact",
            "service-account-file",
            SensitiveTier::SensitiveArtifact,
            SensitiveSeverity::Block,
        );
    } else if regex::Regex::new(r"(?:^|/)id_(?:rsa|ed25519|ecdsa|dsa)$")
        .unwrap()
        .is_match(info.lower_path.as_str())
        || regex::Regex::new(r"(?:^|/)\.ssh/")
            .unwrap()
            .is_match(info.lower_path.as_str())
    {
        push(
            "artifact",
            "ssh-private-key-file",
            SensitiveTier::SensitiveArtifact,
            SensitiveSeverity::Block,
        );
    } else if info.lower_path.ends_with(".pem") {
        push(
            "artifact",
            "pem-file",
            SensitiveTier::Suspicious,
            SensitiveSeverity::Warn,
        );
    } else if regex::Regex::new(r"\.(?:p12|pfx|keystore|jks|pepk|ppk|key)$")
        .unwrap()
        .is_match(info.lower_path.as_str())
        || info.lower_path.ends_with("/key.properties")
        || info.lower_path == "key.properties"
    {
        push(
            "artifact",
            "key-material-file",
            SensitiveTier::SensitiveArtifact,
            SensitiveSeverity::Block,
        );
    } else if info.lower_path.ends_with(".har") {
        push(
            "artifact",
            "http-archive-file",
            SensitiveTier::SensitiveArtifact,
            SensitiveSeverity::Block,
        );
    } else if regex::Regex::new(r"\.(?:hprof|core|dmp|mdmp|pcap|pcapng)$")
        .unwrap()
        .is_match(info.lower_path.as_str())
        || regex::Regex::new(r"core\.\d+$")
            .unwrap()
            .is_match(info.lower_path.as_str())
    {
        push(
            "artifact",
            "dump-file",
            SensitiveTier::SensitiveArtifact,
            SensitiveSeverity::Block,
        );
    } else if info.lower_path.ends_with(".mobileprovision") {
        push(
            "artifact",
            "mobileprovision-file",
            SensitiveTier::Suspicious,
            SensitiveSeverity::Warn,
        );
    } else if regex::Regex::new(r"\.(?:sqlite|sqlite3|db|sql)$")
        .unwrap()
        .is_match(info.lower_path.as_str())
    {
        push(
            "artifact",
            "database-artifact-file",
            SensitiveTier::Suspicious,
            SensitiveSeverity::Warn,
        );
    } else if info.lower_path.ends_with(".map") {
        push(
            "artifact",
            "source-map-file",
            SensitiveTier::Suspicious,
            SensitiveSeverity::Warn,
        );
    } else if info.lower_path.ends_with("/.htpasswd") || info.lower_path == ".htpasswd" {
        push(
            "artifact",
            "auth-file",
            SensitiveTier::SensitiveArtifact,
            SensitiveSeverity::Block,
        );
    }

    findings
}

fn scan_added_line(
    file_path: &str,
    info: &PathContext,
    line: &str,
    line_number: Option<usize>,
    allowlist: &[SensitiveAllowlistEntry],
) -> Vec<SensitiveFinding> {
    let provider_matched = has_provider_match(line);
    let structural_matched = has_structural_match(info, line);
    let providers = scan_provider_line(file_path, line, line_number, allowlist);
    let structural = scan_structural_line(file_path, info, line, line_number, allowlist);
    if provider_matched || structural_matched {
        return dedupe_findings([providers, structural].concat());
    }

    if COMMENT_ONLY_RE.is_match(line) {
        return vec![];
    }

    let generic = scan_generic_assignments(file_path, info, line, line_number, allowlist);
    let network = scan_ip_line(file_path, line, line_number, allowlist);
    dedupe_findings([generic, network].concat())
}

fn scan_provider_line(
    file_path: &str,
    line: &str,
    line_number: Option<usize>,
    allowlist: &[SensitiveAllowlistEntry],
) -> Vec<SensitiveFinding> {
    let mut findings = Vec::new();

    for rule in PROVIDER_RULES.iter() {
        for matched in rule.pattern.find_iter(line) {
            let value = matched.as_str();
            if is_placeholder_value(value) {
                continue;
            }

            push_candidate(
                &mut findings,
                allowlist,
                LineCandidate {
                    category: rule.category,
                    rule: rule.rule,
                    file_path: file_path.to_owned(),
                    line_number,
                    preview: format_line_preview(line),
                    raw_value: Some(value.to_owned()),
                    tier: rule.tier,
                    severity: rule.severity,
                },
            );
        }
    }

    dedupe_findings(findings)
}

fn has_provider_match(line: &str) -> bool {
    PROVIDER_RULES.iter().any(|rule| {
        rule.pattern
            .find_iter(line)
            .any(|matched| !is_placeholder_value(matched.as_str()))
    })
}

fn scan_structural_line(
    file_path: &str,
    info: &PathContext,
    line: &str,
    line_number: Option<usize>,
    allowlist: &[SensitiveAllowlistEntry],
) -> Vec<SensitiveFinding> {
    let mut findings = Vec::new();

    if PRIVATE_KEY_HEADER_RE.is_match(line) {
        push_candidate(
            &mut findings,
            allowlist,
            LineCandidate {
                category: "key",
                rule: "private-key-block",
                file_path: file_path.to_owned(),
                line_number,
                preview: format_line_preview(line),
                raw_value: Some(line.trim().to_owned()),
                tier: SensitiveTier::ConfirmedSecret,
                severity: SensitiveSeverity::Block,
            },
        );
    } else if ENCRYPTED_PRIVATE_KEY_RE.is_match(line) {
        push_candidate(
            &mut findings,
            allowlist,
            LineCandidate {
                category: "key",
                rule: "encrypted-private-key-block",
                file_path: file_path.to_owned(),
                line_number,
                preview: format_line_preview(line),
                raw_value: Some(line.trim().to_owned()),
                tier: SensitiveTier::Suspicious,
                severity: SensitiveSeverity::Warn,
            },
        );
    }

    for captures in CONNECTION_STRING_RE.captures_iter(line) {
        let password = clean_value(captures.get(3).map(|m| m.as_str()).unwrap_or_default());
        let host = captures.get(4).map(|m| m.as_str()).unwrap_or_default();
        if is_placeholder_value(password.as_str()) {
            continue;
        }

        let severity = if is_local_host(host) {
            SensitiveSeverity::Warn
        } else {
            SensitiveSeverity::Block
        };
        let tier = if severity == SensitiveSeverity::Block {
            SensitiveTier::ConfirmedSecret
        } else {
            SensitiveTier::Suspicious
        };

        push_candidate(
            &mut findings,
            allowlist,
            LineCandidate {
                category: "connection",
                rule: "credential-connection-string",
                file_path: file_path.to_owned(),
                line_number,
                preview: format_line_preview(line),
                raw_value: Some(password),
                tier,
                severity,
            },
        );
    }

    for captures in BEARER_RE.captures_iter(line) {
        let token = clean_value(captures.get(1).map(|m| m.as_str()).unwrap_or_default());
        if is_placeholder_value(token.as_str()) {
            continue;
        }

        push_candidate(
            &mut findings,
            allowlist,
            LineCandidate {
                category: "token",
                rule: "bearer-token",
                file_path: file_path.to_owned(),
                line_number,
                preview: format_line_preview(line),
                raw_value: Some(token),
                tier: SensitiveTier::ConfirmedSecret,
                severity: SensitiveSeverity::Block,
            },
        );
    }

    for matched in JWT_RE.find_iter(line) {
        let token = matched.as_str();
        if is_placeholder_value(token) {
            continue;
        }

        push_candidate(
            &mut findings,
            allowlist,
            LineCandidate {
                category: "token",
                rule: "jwt-token",
                file_path: file_path.to_owned(),
                line_number,
                preview: format_line_preview(line),
                raw_value: Some(token.to_owned()),
                tier: SensitiveTier::Suspicious,
                severity: SensitiveSeverity::Warn,
            },
        );
    }

    if info.docker_config {
        for captures in DOCKER_AUTH_RE.captures_iter(line) {
            let value = clean_value(captures.get(1).map(|m| m.as_str()).unwrap_or_default());
            if is_placeholder_value(value.as_str()) {
                continue;
            }

            push_candidate(
                &mut findings,
                allowlist,
                LineCandidate {
                    category: "credential",
                    rule: "docker-config-auth",
                    file_path: file_path.to_owned(),
                    line_number,
                    preview: format_line_preview(line),
                    raw_value: Some(value),
                    tier: SensitiveTier::ConfirmedSecret,
                    severity: SensitiveSeverity::Block,
                },
            );
        }
    }

    if info.kube_config {
        for captures in KUBECONFIG_AUTH_RE.captures_iter(line) {
            let value = clean_value(captures.get(2).map(|m| m.as_str()).unwrap_or_default());
            if is_placeholder_value(value.as_str()) {
                continue;
            }

            push_candidate(
                &mut findings,
                allowlist,
                LineCandidate {
                    category: "credential",
                    rule: "kubeconfig-auth",
                    file_path: file_path.to_owned(),
                    line_number,
                    preview: format_line_preview(line),
                    raw_value: Some(value),
                    tier: SensitiveTier::ConfirmedSecret,
                    severity: SensitiveSeverity::Block,
                },
            );
        }
    }

    if info.npmrc {
        for captures in NPM_LITERAL_AUTH_RE.captures_iter(line) {
            let value = clean_value(
                captures
                    .get(1)
                    .or_else(|| captures.get(2))
                    .map(|m| m.as_str())
                    .unwrap_or_default(),
            );
            if value.is_empty() || is_placeholder_value(value.as_str()) {
                continue;
            }

            push_candidate(
                &mut findings,
                allowlist,
                LineCandidate {
                    category: "credential",
                    rule: "npm-auth",
                    file_path: file_path.to_owned(),
                    line_number,
                    preview: format_line_preview(line),
                    raw_value: Some(value),
                    tier: SensitiveTier::ConfirmedSecret,
                    severity: SensitiveSeverity::Block,
                },
            );
        }
    }

    dedupe_findings(findings)
}

fn has_structural_match(info: &PathContext, line: &str) -> bool {
    if PRIVATE_KEY_HEADER_RE.is_match(line) || ENCRYPTED_PRIVATE_KEY_RE.is_match(line) {
        return true;
    }

    if CONNECTION_STRING_RE.captures_iter(line).any(|captures| {
        let password = clean_value(captures.get(3).map(|m| m.as_str()).unwrap_or_default());
        !is_placeholder_value(password.as_str())
    }) || BEARER_RE.captures_iter(line).any(|captures| {
        let token = clean_value(captures.get(1).map(|m| m.as_str()).unwrap_or_default());
        !is_placeholder_value(token.as_str())
    }) || JWT_RE
        .find_iter(line)
        .any(|matched| !is_placeholder_value(matched.as_str()))
    {
        return true;
    }

    if info.docker_config
        && DOCKER_AUTH_RE.captures_iter(line).any(|captures| {
            let value = clean_value(captures.get(1).map(|m| m.as_str()).unwrap_or_default());
            !is_placeholder_value(value.as_str())
        })
    {
        return true;
    }

    if info.kube_config
        && KUBECONFIG_AUTH_RE.captures_iter(line).any(|captures| {
            let value = clean_value(captures.get(2).map(|m| m.as_str()).unwrap_or_default());
            !is_placeholder_value(value.as_str())
        })
    {
        return true;
    }

    info.npmrc
        && NPM_LITERAL_AUTH_RE.captures_iter(line).any(|captures| {
            let value = clean_value(
                captures
                    .get(1)
                    .or_else(|| captures.get(2))
                    .map(|m| m.as_str())
                    .unwrap_or_default(),
            );
            !value.is_empty() && !is_placeholder_value(value.as_str())
        })
}

fn scan_generic_assignments(
    file_path: &str,
    info: &PathContext,
    line: &str,
    line_number: Option<usize>,
    allowlist: &[SensitiveAllowlistEntry],
) -> Vec<SensitiveFinding> {
    let mut findings = Vec::new();

    for captures in GENERIC_ASSIGNMENT_RE.captures_iter(line) {
        let value = clean_value(captures.get(2).map(|m| m.as_str()).unwrap_or_default());
        if value.is_empty()
            || is_placeholder_value(value.as_str())
            || is_reference_value(value.as_str())
            || !passes_generic_secret_heuristics(value.as_str())
        {
            continue;
        }

        let _downgraded = info.low_confidence || info.env_template;
        push_candidate(
            &mut findings,
            allowlist,
            LineCandidate {
                category: "credential",
                rule: "generic-secret-assignment",
                file_path: file_path.to_owned(),
                line_number,
                preview: format_line_preview(line),
                raw_value: Some(value),
                tier: SensitiveTier::Suspicious,
                severity: SensitiveSeverity::Warn,
            },
        );
    }

    dedupe_findings(findings)
}

fn scan_ip_line(
    file_path: &str,
    line: &str,
    line_number: Option<usize>,
    allowlist: &[SensitiveAllowlistEntry],
) -> Vec<SensitiveFinding> {
    let mut findings = Vec::new();

    for matched in IPV4_RE.find_iter(line) {
        let ip = matched.as_str();
        let Some(parsed) = parse_ipv4(ip) else {
            continue;
        };
        if !is_public_ipv4(parsed) {
            continue;
        }

        push_candidate(
            &mut findings,
            allowlist,
            LineCandidate {
                category: "network",
                rule: "public-ipv4",
                file_path: file_path.to_owned(),
                line_number,
                preview: format_line_preview(line),
                raw_value: Some(ip.to_owned()),
                tier: SensitiveTier::Suspicious,
                severity: SensitiveSeverity::Warn,
            },
        );
    }

    dedupe_findings(findings)
}

fn push_candidate(
    findings: &mut Vec<SensitiveFinding>,
    allowlist: &[SensitiveAllowlistEntry],
    candidate: LineCandidate,
) {
    if matches_allowlist(&candidate, allowlist) {
        return;
    }

    findings.push(SensitiveFinding {
        category: candidate.category,
        rule: candidate.rule,
        file_path: candidate.file_path,
        line_number: candidate.line_number,
        preview: candidate.preview,
        tier: candidate.tier,
        severity: candidate.severity,
    });
}

fn matches_allowlist(candidate: &LineCandidate, allowlist: &[SensitiveAllowlistEntry]) -> bool {
    allowlist.iter().any(|entry| {
        let path_ok = entry
            .path_regex
            .as_deref()
            .map(|pattern| {
                regex::Regex::new(pattern)
                    .unwrap()
                    .is_match(&candidate.file_path)
            })
            .unwrap_or(true);
        let rule_ok = entry
            .rule
            .as_deref()
            .map(|rule| rule == candidate.rule)
            .unwrap_or(true);
        let value_target = candidate.raw_value.as_deref().unwrap_or(&candidate.preview);
        let value_ok = entry
            .value_regex
            .as_deref()
            .map(|pattern| regex::Regex::new(pattern).unwrap().is_match(value_target))
            .unwrap_or(true);
        path_ok && rule_ok && value_ok
    })
}

fn dedupe_findings(findings: Vec<SensitiveFinding>) -> Vec<SensitiveFinding> {
    let mut seen = HashSet::new();
    findings
        .into_iter()
        .filter(|finding| {
            let key = format!(
                "{}::{}::{}::{}",
                finding.rule,
                finding.file_path,
                finding.line_number.unwrap_or_default(),
                finding.preview
            );
            seen.insert(key)
        })
        .collect()
}

fn clean_value(value: &str) -> String {
    value
        .trim()
        .trim_start_matches(|ch| matches!(ch, '"' | '\'' | '`'))
        .trim_end_matches(|ch| matches!(ch, '"' | '\'' | '`' | ';' | ','))
        .to_owned()
}

fn format_line_preview(line: impl Into<String>) -> String {
    let mut preview = line.into().trim().to_owned();
    if preview.len() > 160 {
        preview.truncate(157);
        preview.push_str("...");
    }
    preview
}

fn is_placeholder_value(value: &str) -> bool {
    let trimmed = clean_value(value);
    let lower = trimmed.to_lowercase();

    if trimmed.is_empty() || trimmed.len() < 8 {
        return true;
    }
    if is_reference_value(trimmed.as_str()) {
        return true;
    }

    let exact_placeholders = [
        "example",
        "sample",
        "demo",
        "test",
        "dummy",
        "fake",
        "placeholder",
        "mock",
        "fixme",
        "todo",
        "temp",
        "tmp",
        "none",
        "null",
        "undefined",
        "empty",
        "default",
        "redacted",
        "removed",
        "censored",
        "changeme",
        "replace_me",
        "password",
        "qwerty",
        "letmein",
        "123456",
        "000000",
        "111111",
        "user:pass",
        "username:password",
    ];
    if exact_placeholders.contains(&lower.as_str()) {
        return true;
    }

    if regex::Regex::new(
        r"(?i)your[_-]?(?:api[_-]?key|token|secret|password|key)[_-]?here|(?:replace|change|insert|fill|update|put|add)[_-]?(?:me|your)",
    )
    .unwrap()
    .is_match(trimmed.as_str())
    {
        return true;
    }

    if regex::Regex::new(r"(?i)^(?:x{4,}|\*{4,}|0{6,}|1{6,}|#{4,}|\.{4,})$")
        .unwrap()
        .is_match(trimmed.as_str())
    {
        return true;
    }

    trimmed.contains("...")
}

fn is_reference_value(value: &str) -> bool {
    regex::Regex::new(
        r#"(?ix)
        ^\$\{.+\}$|
        ^\$\(.+\)$|
        ^%[A-Z_][A-Z0-9_]*%$|
        ^\{\{.+\}\}$|
        ^<[A-Za-z0-9_-]+>$|
        ^\$[A-Z_][A-Z0-9_]*$|
        \bprocess\.env\.|
        \bos\.environ\[|
        \bos\.getenv\(|
        \bSystem\.getenv\(|
        \bENV\[|
        \$ENV\{|
        \benv\(['"][A-Za-z0-9_]+['"]\)
        "#,
    )
    .unwrap()
    .is_match(value)
        || value.contains("${")
        || value.contains("{{")
        || value.contains("$(")
}

fn passes_generic_secret_heuristics(value: &str) -> bool {
    if value.len() < 8 {
        return false;
    }
    if value.chars().filter(|ch| ch.is_ascii_digit()).count() < 2 {
        return false;
    }

    let unique_chars = value.chars().collect::<HashSet<_>>().len();
    if unique_chars < 6 {
        return false;
    }

    let hex_like = regex::Regex::new(r"^[0-9a-f]+$").unwrap().is_match(value);
    let entropy = shannon_entropy(value);
    if hex_like {
        entropy >= 3.0
    } else {
        entropy >= 3.0
    }
}

fn shannon_entropy(value: &str) -> f64 {
    let mut counts = HashMap::new();
    for ch in value.chars() {
        *counts.entry(ch).or_insert(0_usize) += 1;
    }

    let len = value.len() as f64;
    counts
        .values()
        .map(|count| {
            let p = *count as f64 / len;
            -p * p.log2()
        })
        .sum()
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

fn is_public_ipv4(ip: [u8; 4]) -> bool {
    let [a, b, c, _] = ip;
    if a == 10 || a == 127 || a == 0 {
        return false;
    }
    if (a, b) == (169, 254) {
        return false;
    }
    if a == 172 && (16..=31).contains(&b) {
        return false;
    }
    if (a, b) == (192, 168) {
        return false;
    }
    !matches!((a, b, c), (192, 0, 2) | (198, 51, 100) | (203, 0, 113))
}

fn is_local_host(host: &str) -> bool {
    let value = host
        .to_lowercase()
        .split([':', '/'])
        .next()
        .unwrap_or_default()
        .to_owned();

    if matches!(
        value.as_str(),
        "localhost" | "127.0.0.1" | "0.0.0.0" | "::1"
    ) || value.ends_with(".local")
        || value.ends_with(".internal")
        || value.ends_with(".example")
        || value.ends_with(".test")
    {
        return true;
    }

    parse_ipv4(value.as_str())
        .map(|ip| !is_public_ipv4(ip))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ScenarioFinding {
        category: String,
        rule: String,
        file_path: String,
        line_number: Option<usize>,
        preview: String,
        tier: String,
        severity: String,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Scenario {
        name: String,
        diff: String,
        changed_files: Vec<String>,
        expected_findings: Vec<ScenarioFinding>,
    }

    fn load_shared_scenarios() -> Vec<Scenario> {
        let path = format!(
            "{}/../../test-fixtures/sensitive-scenarios.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let content = std::fs::read_to_string(path).unwrap();
        serde_json::from_str(&content).unwrap()
    }

    fn tier_name(tier: SensitiveTier) -> &'static str {
        match tier {
            SensitiveTier::ConfirmedSecret => "confirmed-secret",
            SensitiveTier::SensitiveArtifact => "sensitive-artifact",
            SensitiveTier::Suspicious => "suspicious",
        }
    }

    fn severity_name(severity: SensitiveSeverity) -> &'static str {
        match severity {
            SensitiveSeverity::Block => "block",
            SensitiveSeverity::Warn => "warn",
        }
    }

    #[test]
    fn shared_scenarios_match_rust_detector() {
        for scenario in load_shared_scenarios() {
            let report = scan_diff_for_sensitive_content(&scenario.diff, &scenario.changed_files);
            assert_eq!(
                report.findings.len(),
                scenario.expected_findings.len(),
                "scenario {} finding count mismatch",
                scenario.name
            );

            for (finding, expected) in report.findings.iter().zip(&scenario.expected_findings) {
                assert_eq!(finding.category, expected.category, "{}", scenario.name);
                assert_eq!(finding.rule, expected.rule, "{}", scenario.name);
                assert_eq!(finding.file_path, expected.file_path, "{}", scenario.name);
                assert_eq!(
                    finding.line_number, expected.line_number,
                    "{}",
                    scenario.name
                );
                assert_eq!(finding.preview, expected.preview, "{}", scenario.name);
                assert_eq!(tier_name(finding.tier), expected.tier, "{}", scenario.name);
                assert_eq!(
                    severity_name(finding.severity),
                    expected.severity,
                    "{}",
                    scenario.name
                );
            }
        }
    }

    #[test]
    fn allowlist_matches_path_rule_and_value() {
        let diff = "\
diff --git a/.env.example b/.env.example
--- a/.env.example
+++ b/.env.example
@@ -0,0 +1 @@
+OPENAI_API_KEY=sk-proj-abcdefghijklmnopqrstuvwxyz1234567890
";
        let report = scan_diff_for_sensitive_content_with_options(
            diff,
            &[".env.example".to_owned()],
            SensitiveEnforcement::Warn,
            &[SensitiveAllowlistEntry {
                path_regex: Some(r"\.env\.example$".to_owned()),
                rule: Some("openai-project-key".to_owned()),
                value_regex: Some(r"^sk-proj-".to_owned()),
            }],
        );
        assert!(!report.has_findings());
    }

    #[test]
    fn allowlist_matches_path_only_artifact_findings() {
        let report = scan_diff_for_sensitive_content_with_options(
            "diff",
            &[".env".to_owned()],
            SensitiveEnforcement::Warn,
            &[SensitiveAllowlistEntry {
                path_regex: Some(r"\.env$".to_owned()),
                rule: Some("env-file".to_owned()),
                value_regex: None,
            }],
        );
        assert!(!report.has_findings());
    }

    #[test]
    fn warn_mode_never_marks_findings_blocking() {
        let report = SensitiveReport::from_findings_with_enforcement(
            vec![SensitiveFinding {
                category: "artifact",
                rule: "env-file",
                file_path: ".env".to_owned(),
                line_number: None,
                preview: ".env".to_owned(),
                tier: SensitiveTier::SensitiveArtifact,
                severity: SensitiveSeverity::Block,
            }],
            SensitiveEnforcement::Warn,
        );
        assert_eq!(report.blocking_count, 0);
        assert_eq!(report.warning_count, 1);
    }

    #[test]
    fn strict_all_blocks_warnings_and_disables_bypass() {
        let report = SensitiveReport::from_findings_with_enforcement(
            vec![SensitiveFinding {
                category: "credential",
                rule: "generic-secret-assignment",
                file_path: "src/auth.ts".to_owned(),
                line_number: Some(1),
                preview: r#"const PASSWORD = "Alpha9981Zeta""#.to_owned(),
                tier: SensitiveTier::Suspicious,
                severity: SensitiveSeverity::Warn,
            }],
            SensitiveEnforcement::StrictAll,
        );
        assert!(report.has_blocking_findings());
        assert!(!allows_sensitive_bypass(report.enforcement));
    }

    #[test]
    fn formats_git_hook_message_with_strict_footer() {
        let report = SensitiveReport::from_findings_with_enforcement(
            vec![SensitiveFinding {
                category: "credential",
                rule: "generic-secret-assignment",
                file_path: "src/auth.ts".to_owned(),
                line_number: Some(18),
                preview: r#"const PASSWORD = "Alpha9981Zeta""#.to_owned(),
                tier: SensitiveTier::Suspicious,
                severity: SensitiveSeverity::Warn,
            }],
            SensitiveEnforcement::StrictAll,
        );

        let message = report.format_git_hook_message();
        assert!(message.contains("Strict sensitive mode is active"));
    }
}
