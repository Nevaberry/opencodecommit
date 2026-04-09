use std::path::Path;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::context::extract_changed_file_paths;
use crate::git;
use crate::sensitive::{
    SensitiveAllowlistEntry, SensitiveEnforcement, SensitiveFinding, SensitiveReport,
    is_blocking_finding, scan_diff_for_sensitive_content_with_options,
};
use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanFormat {
    Text,
    Json,
    Sarif,
    GithubAnnotations,
}

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub report: SensitiveReport,
    pub scanned_files: usize,
}

#[derive(Debug, Deserialize)]
struct AllowlistWrapper {
    #[serde(default)]
    allowlist: Vec<SensitiveAllowlistEntry>,
    #[serde(default)]
    sensitive: AllowlistSensitiveWrapper,
}

#[derive(Debug, Default, Deserialize)]
struct AllowlistSensitiveWrapper {
    #[serde(default)]
    allowlist: Vec<SensitiveAllowlistEntry>,
}

pub fn run_scan(
    diff: &str,
    changed_files: &[String],
    enforcement: SensitiveEnforcement,
    allowlist: &[SensitiveAllowlistEntry],
) -> ScanResult {
    let report =
        scan_diff_for_sensitive_content_with_options(diff, changed_files, enforcement, allowlist);
    ScanResult {
        report,
        scanned_files: changed_files.len(),
    }
}

pub fn load_allowlist_file(path: &Path) -> Result<Vec<SensitiveAllowlistEntry>> {
    let content = std::fs::read_to_string(path).map_err(|err| {
        Error::Config(format!(
            "failed to read allowlist file {}: {err}",
            path.display()
        ))
    })?;
    let wrapper: AllowlistWrapper = toml::from_str(&content)
        .map_err(|err| Error::Config(format!("failed to parse allowlist file: {err}")))?;
    if !wrapper.allowlist.is_empty() {
        Ok(wrapper.allowlist)
    } else {
        Ok(wrapper.sensitive.allowlist)
    }
}

pub fn format_json(report: &SensitiveReport) -> Value {
    json!({
        "findings": report.findings,
        "enforcement": report.enforcement,
        "warning_count": report.warning_count,
        "blocking_count": report.blocking_count,
        "has_findings": report.has_findings,
        "has_blocking_findings": report.has_blocking_findings,
    })
}

pub fn format_sarif(report: &SensitiveReport) -> Value {
    let results = report
        .findings
        .iter()
        .map(|finding| sarif_result(finding, report.enforcement))
        .collect::<Vec<_>>();
    json!({
        "version": "2.1.0",
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "OpenCodeCommit",
                    "informationUri": "https://github.com/Nevaberry/opencodecommit",
                }
            },
            "results": results,
        }],
    })
}

pub fn format_github_annotations(report: &SensitiveReport) -> String {
    report
        .findings
        .iter()
        .map(|finding| {
            let level = if is_blocking_finding(finding, report.enforcement) {
                "error"
            } else {
                "warning"
            };
            let file = finding.file_path.replace(',', "%2C");
            let message = format!(
                "{} [{} / {}] {}",
                finding.category,
                format!("{:?}", finding.tier),
                finding.rule,
                finding.preview
            )
            .replace('\n', " ");
            match finding.line_number {
                Some(line) => format!("::{level} file={file},line={line}::{message}"),
                None => format!("::{level} file={file}::{message}"),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn format_text(report: &SensitiveReport) -> String {
    report.format_occ_commit_message()
}

pub fn changed_files_from_diff(diff: &str) -> Vec<String> {
    extract_changed_file_paths(diff)
}

pub fn read_git_diff(
    repo_root: &Path,
    source: crate::config::DiffSource,
) -> Result<(String, Vec<String>)> {
    let diff = git::get_diff(source, repo_root)?;
    let changed_files = changed_files_from_diff(&diff);
    Ok((diff, changed_files))
}

fn sarif_result(finding: &SensitiveFinding, enforcement: SensitiveEnforcement) -> Value {
    let level = if is_blocking_finding(finding, enforcement) {
        "error"
    } else {
        "warning"
    };
    let mut result = json!({
        "ruleId": finding.rule,
        "level": level,
        "message": {
            "text": format!("{} [{}] {}", finding.category, finding.file_path, finding.preview)
        },
        "locations": [{
            "physicalLocation": {
                "artifactLocation": {
                    "uri": finding.file_path,
                }
            }
        }],
    });
    if let Some(line_number) = finding.line_number {
        result["locations"][0]["physicalLocation"]["region"] = json!({ "startLine": line_number });
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensitive::{SensitiveSeverity, SensitiveTier};

    fn env_diff() -> String {
        r#"diff --git a/.env b/.env
new file mode 100644
index 0000000..1111111
--- /dev/null
+++ b/.env
@@ -0,0 +1 @@
+OPENAI_API_KEY=sk-proj-live-secret-1234567890abcdefghijklmnopqrstuvwxyz
"#
        .to_owned()
    }

    fn sample_report(enforcement: SensitiveEnforcement) -> SensitiveReport {
        SensitiveReport::from_findings_with_enforcement(
            vec![SensitiveFinding {
                category: "token",
                rule: "OPENAI_API_KEY",
                file_path: ".env".to_owned(),
                line_number: Some(1),
                preview: "OPENAI_API_KEY=sk-proj-live-secret".to_owned(),
                tier: SensitiveTier::ConfirmedSecret,
                severity: SensitiveSeverity::Block,
            }],
            enforcement,
        )
    }

    #[test]
    fn run_scan_respects_enforcement() {
        let diff = env_diff();
        let changed_files = changed_files_from_diff(&diff);

        let warn = run_scan(&diff, &changed_files, SensitiveEnforcement::Warn, &[]);
        assert_eq!(warn.scanned_files, 1);
        assert!(warn.report.has_findings());
        assert!(!warn.report.has_blocking_findings());

        let blocking = run_scan(&diff, &changed_files, SensitiveEnforcement::BlockHigh, &[]);
        assert!(blocking.report.has_blocking_findings());
        assert!(blocking.report.blocking_count > 0);
    }

    #[test]
    fn load_allowlist_file_supports_root_and_nested_tables() {
        let temp_root = std::env::temp_dir().join(format!(
            "occ-scan-allowlist-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_root).unwrap();

        let root_path = temp_root.join("root.toml");
        std::fs::write(
            &root_path,
            r#"
allowlist = [
  { rule = "OPENAI_API_KEY", path-regex = "\\.env$" }
]
"#,
        )
        .unwrap();
        let root_entries = load_allowlist_file(&root_path).unwrap();
        assert_eq!(root_entries.len(), 1);
        assert_eq!(root_entries[0].rule.as_deref(), Some("OPENAI_API_KEY"));

        let nested_path = temp_root.join("nested.toml");
        std::fs::write(
            &nested_path,
            r#"
[sensitive]
allowlist = [
  { path-regex = "\\.map$", value-regex = "localhost" }
]
"#,
        )
        .unwrap();
        let nested_entries = load_allowlist_file(&nested_path).unwrap();
        assert_eq!(nested_entries.len(), 1);
        assert_eq!(nested_entries[0].path_regex.as_deref(), Some("\\.map$"));

        let _ = std::fs::remove_dir_all(temp_root);
    }

    #[test]
    fn formatters_emit_machine_readable_outputs() {
        let report = sample_report(SensitiveEnforcement::BlockHigh);

        let json = format_json(&report);
        assert_eq!(json["blocking_count"], 1);
        assert_eq!(json["findings"][0]["file_path"], ".env");

        let sarif = format_sarif(&report);
        assert_eq!(sarif["version"], "2.1.0");
        assert_eq!(sarif["runs"][0]["results"][0]["level"], "error");
        assert_eq!(
            sarif["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["artifactLocation"]
                ["uri"],
            ".env"
        );

        let annotations = format_github_annotations(&report);
        assert!(annotations.contains("::error file=.env,line=1::"));
        assert!(annotations.contains("OPENAI_API_KEY=sk-proj-live-secret"));
    }
}
