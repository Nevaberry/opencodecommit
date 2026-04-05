use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::Error;

// --- Enums ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CliBackend {
    #[default]
    Opencode,
    Claude,
    Codex,
    Gemini,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum CommitMode {
    #[default]
    Adaptive,
    AdaptiveOneliner,
    Conventional,
    ConventionalOneliner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DiffSource {
    Staged,
    All,
    #[default]
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BranchMode {
    #[default]
    Conventional,
    Adaptive,
}

// --- Config structs ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageConfig {
    pub label: String,
    pub instruction: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CustomConfig {
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub type_rules: String,
    #[serde(default)]
    pub commit_message_rules: String,
    #[serde(default)]
    pub emojis: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RefineConfig {
    #[serde(default = "default_refine_feedback")]
    pub default_feedback: String,
}

impl Default for RefineConfig {
    fn default() -> Self {
        Self {
            default_feedback: default_refine_feedback(),
        }
    }
}

fn default_refine_feedback() -> String {
    "make it shorter".to_owned()
}

/// Main configuration. All fields have defaults matching the TypeScript extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(default = "default_backend")]
    pub backend: CliBackend,

    #[serde(default = "default_commit_mode")]
    pub commit_mode: CommitMode,

    #[serde(default = "default_commit_mode")]
    pub sparkle_mode: CommitMode,

    #[serde(default = "default_provider")]
    pub provider: String,

    #[serde(default = "default_model")]
    pub model: String,

    #[serde(default)]
    pub cli_path: String,

    #[serde(default)]
    pub claude_path: String,

    #[serde(default)]
    pub codex_path: String,

    #[serde(default = "default_claude_model")]
    pub claude_model: String,

    #[serde(default = "default_codex_model")]
    pub codex_model: String,

    #[serde(default)]
    pub codex_provider: String,

    #[serde(default)]
    pub gemini_path: String,

    #[serde(default)]
    pub gemini_model: String,

    #[serde(default)]
    pub branch_mode: BranchMode,

    #[serde(default = "default_diff_source")]
    pub diff_source: DiffSource,

    #[serde(default = "default_max_diff_length")]
    pub max_diff_length: usize,

    #[serde(default)]
    pub use_emojis: bool,

    #[serde(default = "default_true")]
    pub use_lower_case: bool,

    #[serde(default = "default_commit_template")]
    pub commit_template: String,

    #[serde(default = "default_languages")]
    pub languages: Vec<LanguageConfig>,

    #[serde(default = "default_active_language")]
    pub active_language: String,

    #[serde(default)]
    pub show_language_selector: bool,

    #[serde(default)]
    pub refine: RefineConfig,

    #[serde(default)]
    pub custom: CustomConfig,
}

// --- Default value functions ---

fn default_backend() -> CliBackend {
    CliBackend::Opencode
}

fn default_commit_mode() -> CommitMode {
    CommitMode::Adaptive
}

fn default_provider() -> String {
    "openai".to_owned()
}

fn default_model() -> String {
    "gpt-5.4-mini".to_owned()
}

fn default_claude_model() -> String {
    "claude-sonnet-4-6".to_owned()
}

fn default_codex_model() -> String {
    "gpt-5.4-mini".to_owned()
}

fn default_diff_source() -> DiffSource {
    DiffSource::Auto
}

fn default_max_diff_length() -> usize {
    10000
}

fn default_true() -> bool {
    true
}

fn default_commit_template() -> String {
    "{{type}}: {{message}}".to_owned()
}

fn default_languages() -> Vec<LanguageConfig> {
    vec![LanguageConfig {
        label: "English".to_owned(),
        instruction: "Write the commit message in English.".to_owned(),
    }]
}

fn default_active_language() -> String {
    "English".to_owned()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            backend: default_backend(),
            commit_mode: default_commit_mode(),
            sparkle_mode: default_commit_mode(),
            provider: default_provider(),
            model: default_model(),
            cli_path: String::new(),
            claude_path: String::new(),
            codex_path: String::new(),
            claude_model: default_claude_model(),
            codex_model: default_codex_model(),
            codex_provider: String::new(),
            gemini_path: String::new(),
            gemini_model: String::new(),
            branch_mode: BranchMode::default(),
            diff_source: default_diff_source(),
            max_diff_length: default_max_diff_length(),
            use_emojis: false,
            use_lower_case: true,
            commit_template: default_commit_template(),
            languages: default_languages(),
            active_language: default_active_language(),
            show_language_selector: false,
            refine: RefineConfig::default(),
            custom: CustomConfig::default(),
        }
    }
}

impl Config {
    /// Resolve the language instruction for the active language.
    pub fn active_language_instruction(&self) -> String {
        self.languages
            .iter()
            .find(|l| l.label == self.active_language)
            .map(|l| l.instruction.clone())
            .unwrap_or_else(|| "Write the commit message in English.".to_owned())
    }

    /// Return the CLI path for the current backend.
    pub fn backend_cli_path(&self) -> &str {
        match self.backend {
            CliBackend::Opencode => &self.cli_path,
            CliBackend::Claude => &self.claude_path,
            CliBackend::Codex => &self.codex_path,
            CliBackend::Gemini => &self.gemini_path,
        }
    }

    /// Return the model for the current backend.
    pub fn backend_model(&self) -> &str {
        match self.backend {
            CliBackend::Opencode => &self.model,
            CliBackend::Claude => &self.claude_model,
            CliBackend::Codex => &self.codex_model,
            CliBackend::Gemini => &self.gemini_model,
        }
    }

    /// Default config file path: `$XDG_CONFIG_HOME/opencodecommit/config.toml`
    /// or `$HOME/.config/opencodecommit/config.toml`.
    pub fn default_config_path() -> Option<PathBuf> {
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            let p = PathBuf::from(xdg).join("opencodecommit/config.toml");
            if p.exists() {
                return Some(p);
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            let p = PathBuf::from(home).join(".config/opencodecommit/config.toml");
            if p.exists() {
                return Some(p);
            }
        }
        #[cfg(target_os = "windows")]
        if let Ok(appdata) = std::env::var("APPDATA") {
            let p = PathBuf::from(appdata).join("opencodecommit\\config.toml");
            if p.exists() {
                return Some(p);
            }
        }
        None
    }

    /// Load config from a TOML file. Missing fields get defaults.
    pub fn load(path: &Path) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            Error::Config(format!("failed to read config file {}: {e}", path.display()))
        })?;
        toml::from_str(&content)
            .map_err(|e| Error::Config(format!("failed to parse config file: {e}")))
    }

    /// Load from the default config path, or return defaults if no file exists.
    pub fn load_or_default(explicit_path: Option<&Path>) -> crate::Result<Self> {
        if let Some(path) = explicit_path {
            return Self::load(path);
        }
        if let Some(path) = Self::default_config_path() {
            return Self::load(&path);
        }
        Ok(Self::default())
    }
}

/// Default emoji map matching the TypeScript extension.
pub const DEFAULT_EMOJIS: &[(&str, &str)] = &[
    ("feat", "\u{2728}"),    // ✨
    ("fix", "\u{1f41b}"),    // 🐛
    ("docs", "\u{1f4dd}"),   // 📝
    ("style", "\u{1f48e}"),  // 💎
    ("refactor", "\u{267b}\u{fe0f}"), // ♻️
    ("test", "\u{1f9ea}"),   // 🧪
    ("chore", "\u{1f4e6}"),  // 📦
    ("perf", "\u{26a1}"),    // ⚡
    ("security", "\u{1f512}"), // 🔒
    ("revert", "\u{23ea}"),  // ⏪
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn default_values_match_typescript() {
        let cfg = Config::default();
        assert_eq!(cfg.backend, CliBackend::Opencode);
        assert_eq!(cfg.commit_mode, CommitMode::Adaptive);
        assert_eq!(cfg.sparkle_mode, CommitMode::Adaptive);
        assert_eq!(cfg.provider, "openai");
        assert_eq!(cfg.model, "gpt-5.4-mini");
        assert_eq!(cfg.cli_path, "");
        assert_eq!(cfg.claude_path, "");
        assert_eq!(cfg.codex_path, "");
        assert_eq!(cfg.claude_model, "claude-sonnet-4-6");
        assert_eq!(cfg.codex_model, "gpt-5.4-mini");
        assert_eq!(cfg.codex_provider, "");
        assert_eq!(cfg.gemini_path, "");
        assert_eq!(cfg.gemini_model, "");
        assert_eq!(cfg.diff_source, DiffSource::Auto);
        assert_eq!(cfg.max_diff_length, 10000);
        assert!(!cfg.use_emojis);
        assert!(cfg.use_lower_case);
        assert_eq!(cfg.commit_template, "{{type}}: {{message}}");
        assert_eq!(cfg.languages.len(), 1);
        assert_eq!(cfg.languages[0].label, "English");
        assert_eq!(cfg.active_language, "English");
        assert_eq!(cfg.refine.default_feedback, "make it shorter");
        assert!(cfg.custom.prompt.is_empty());
        assert!(cfg.custom.emojis.is_empty());
    }

    #[test]
    fn active_language_instruction_lookup() {
        let mut cfg = Config::default();
        assert_eq!(
            cfg.active_language_instruction(),
            "Write the commit message in English."
        );

        cfg.languages.push(LanguageConfig {
            label: "Suomi".to_owned(),
            instruction: "Kirjoita commit-viesti suomeksi.".to_owned(),
        });
        cfg.active_language = "Suomi".to_owned();
        assert_eq!(
            cfg.active_language_instruction(),
            "Kirjoita commit-viesti suomeksi."
        );

        cfg.active_language = "Nonexistent".to_owned();
        assert_eq!(
            cfg.active_language_instruction(),
            "Write the commit message in English."
        );
    }

    #[test]
    fn backend_model_and_path() {
        let mut cfg = Config::default();
        assert_eq!(cfg.backend_model(), "gpt-5.4-mini");
        assert_eq!(cfg.backend_cli_path(), "");

        cfg.backend = CliBackend::Claude;
        cfg.claude_path = "/usr/bin/claude".to_owned();
        assert_eq!(cfg.backend_model(), "claude-sonnet-4-6");
        assert_eq!(cfg.backend_cli_path(), "/usr/bin/claude");

        cfg.backend = CliBackend::Codex;
        cfg.codex_path = "/usr/bin/codex".to_owned();
        assert_eq!(cfg.backend_model(), "gpt-5.4-mini");
        assert_eq!(cfg.backend_cli_path(), "/usr/bin/codex");

        cfg.backend = CliBackend::Gemini;
        cfg.gemini_path = "/usr/bin/gemini".to_owned();
        cfg.gemini_model = "gemini-2.5-flash".to_owned();
        assert_eq!(cfg.backend_model(), "gemini-2.5-flash");
        assert_eq!(cfg.backend_cli_path(), "/usr/bin/gemini");
    }

    #[test]
    fn load_from_toml() {
        let dir = std::env::temp_dir().join("occ-test-config");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("config.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"
backend = "claude"
commit-mode = "conventional"
provider = "anthropic"
model = "opus"
claude-model = "opus"
max-diff-length = 5000
use-emojis = true
use-lower-case = false

[refine]
default-feedback = "be more specific"

[custom]
prompt = "Generate: {{{{diff}}}}"
"#
        )
        .unwrap();
        drop(f);

        let cfg = Config::load(&path).unwrap();
        assert_eq!(cfg.backend, CliBackend::Claude);
        assert_eq!(cfg.commit_mode, CommitMode::Conventional);
        assert_eq!(cfg.provider, "anthropic");
        assert_eq!(cfg.model, "opus");
        assert_eq!(cfg.claude_model, "opus");
        assert_eq!(cfg.max_diff_length, 5000);
        assert!(cfg.use_emojis);
        assert!(!cfg.use_lower_case);
        assert_eq!(cfg.refine.default_feedback, "be more specific");
        assert!(!cfg.custom.prompt.is_empty());

        // Unset fields should get defaults
        assert_eq!(cfg.diff_source, DiffSource::Auto);
        assert_eq!(cfg.commit_template, "{{type}}: {{message}}");
        assert_eq!(cfg.codex_model, "gpt-5.4-mini");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_nonexistent_file_errors() {
        let result = Config::load(Path::new("/tmp/nonexistent-occ-config.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn load_or_default_with_no_file() {
        let cfg = Config::load_or_default(None).unwrap();
        assert_eq!(cfg.backend, CliBackend::Opencode);
        assert_eq!(cfg.model, "gpt-5.4-mini");
    }

    #[test]
    fn branch_mode_serde() {
        let cfg: Config = toml::from_str("branch-mode = \"adaptive\"").unwrap();
        assert_eq!(cfg.branch_mode, BranchMode::Adaptive);
        let cfg2: Config = toml::from_str("branch-mode = \"conventional\"").unwrap();
        assert_eq!(cfg2.branch_mode, BranchMode::Conventional);
    }

    #[test]
    fn branch_mode_default() {
        let cfg = Config::default();
        assert_eq!(cfg.branch_mode, BranchMode::Conventional);
    }

    #[test]
    fn serde_roundtrip() {
        let cfg = Config::default();
        let toml_str = toml::to_string(&cfg).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.backend, cfg.backend);
        assert_eq!(parsed.model, cfg.model);
        assert_eq!(parsed.commit_mode, cfg.commit_mode);
        assert_eq!(parsed.max_diff_length, cfg.max_diff_length);
    }
}
