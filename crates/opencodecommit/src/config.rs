use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::Error;
use crate::sensitive::{SensitiveAllowlistEntry, SensitiveEnforcement};

const CONFIG_ENV: &str = "OPENCODECOMMIT_CONFIG";

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

impl fmt::Display for CliBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliBackend::Opencode => write!(f, "opencode"),
            CliBackend::Claude => write!(f, "claude"),
            CliBackend::Codex => write!(f, "codex"),
            CliBackend::Gemini => write!(f, "gemini"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Backend {
    #[default]
    Opencode,
    Claude,
    Codex,
    Gemini,
    OpenaiApi,
    AnthropicApi,
    GeminiApi,
    OpenrouterApi,
    OpencodeApi,
    OllamaApi,
    LmStudioApi,
    CustomApi,
}

impl Backend {
    pub const ALL: [Backend; 12] = [
        Backend::Opencode,
        Backend::Claude,
        Backend::Codex,
        Backend::Gemini,
        Backend::OpenaiApi,
        Backend::AnthropicApi,
        Backend::GeminiApi,
        Backend::OpenrouterApi,
        Backend::OpencodeApi,
        Backend::OllamaApi,
        Backend::LmStudioApi,
        Backend::CustomApi,
    ];

    pub fn is_cli(self) -> bool {
        self.cli_backend().is_some()
    }

    pub fn is_api(self) -> bool {
        !self.is_cli()
    }

    pub fn cli_backend(self) -> Option<CliBackend> {
        match self {
            Backend::Opencode => Some(CliBackend::Opencode),
            Backend::Claude => Some(CliBackend::Claude),
            Backend::Codex => Some(CliBackend::Codex),
            Backend::Gemini => Some(CliBackend::Gemini),
            Backend::OpenaiApi
            | Backend::AnthropicApi
            | Backend::GeminiApi
            | Backend::OpenrouterApi
            | Backend::OpencodeApi
            | Backend::OllamaApi
            | Backend::LmStudioApi
            | Backend::CustomApi => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Backend::Opencode => "OpenCode CLI",
            Backend::Claude => "Claude Code CLI",
            Backend::Codex => "Codex CLI",
            Backend::Gemini => "Gemini CLI",
            Backend::OpenaiApi => "OpenAI API",
            Backend::AnthropicApi => "Anthropic API",
            Backend::GeminiApi => "Gemini API",
            Backend::OpenrouterApi => "OpenRouter API",
            Backend::OpencodeApi => "OpenCode Zen API",
            Backend::OllamaApi => "Ollama API",
            Backend::LmStudioApi => "LM Studio API",
            Backend::CustomApi => "Custom API",
        }
    }
}

impl fmt::Display for Backend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Backend::Opencode => write!(f, "opencode"),
            Backend::Claude => write!(f, "claude"),
            Backend::Codex => write!(f, "codex"),
            Backend::Gemini => write!(f, "gemini"),
            Backend::OpenaiApi => write!(f, "openai-api"),
            Backend::AnthropicApi => write!(f, "anthropic-api"),
            Backend::GeminiApi => write!(f, "gemini-api"),
            Backend::OpenrouterApi => write!(f, "openrouter-api"),
            Backend::OpencodeApi => write!(f, "opencode-api"),
            Backend::OllamaApi => write!(f, "ollama-api"),
            Backend::LmStudioApi => write!(f, "lm-studio-api"),
            Backend::CustomApi => write!(f, "custom-api"),
        }
    }
}

impl From<CliBackend> for Backend {
    fn from(value: CliBackend) -> Self {
        match value {
            CliBackend::Opencode => Backend::Opencode,
            CliBackend::Claude => Backend::Claude,
            CliBackend::Codex => Backend::Codex,
            CliBackend::Gemini => Backend::Gemini,
        }
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensitiveProfile {
    Human,
    StrictAgent,
}

// --- Config structs ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct LanguageConfig {
    pub label: String,
    pub instruction: String,
    #[serde(default)]
    pub base_module: Option<String>,
    #[serde(default)]
    pub adaptive_format: Option<String>,
    #[serde(default)]
    pub conventional_format: Option<String>,
    #[serde(default)]
    pub multiline_length: Option<String>,
    #[serde(default)]
    pub oneliner_length: Option<String>,
    #[serde(default)]
    pub sensitive_content_note: Option<String>,
}

/// Resolved prompt modules for the active language.
/// Missing fields fall back to the first language in the array.
#[derive(Debug, Clone)]
pub struct PromptModules {
    pub base_module: String,
    pub adaptive_format: String,
    pub conventional_format: String,
    pub multiline_length: String,
    pub oneliner_length: String,
    pub sensitive_content_note: String,
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
pub struct ApiProviderConfig {
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub key_env: String,
    #[serde(default)]
    pub pr_model: String,
    #[serde(default)]
    pub cheap_model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ApiConfig {
    #[serde(default = "default_openai_api_config")]
    pub openai: ApiProviderConfig,
    #[serde(default = "default_anthropic_api_config")]
    pub anthropic: ApiProviderConfig,
    #[serde(default = "default_gemini_api_config")]
    pub gemini: ApiProviderConfig,
    #[serde(default = "default_openrouter_api_config")]
    pub openrouter: ApiProviderConfig,
    #[serde(default = "default_opencode_api_config")]
    pub opencode: ApiProviderConfig,
    #[serde(default = "default_ollama_api_config")]
    pub ollama: ApiProviderConfig,
    #[serde(default = "default_lm_studio_api_config")]
    pub lm_studio: ApiProviderConfig,
    #[serde(default = "default_custom_api_config")]
    pub custom: ApiProviderConfig,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            openai: default_openai_api_config(),
            anthropic: default_anthropic_api_config(),
            gemini: default_gemini_api_config(),
            openrouter: default_openrouter_api_config(),
            opencode: default_opencode_api_config(),
            ollama: default_ollama_api_config(),
            lm_studio: default_lm_studio_api_config(),
            custom: default_custom_api_config(),
        }
    }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SensitiveConfig {
    #[serde(default)]
    pub enforcement: SensitiveEnforcement,
    #[serde(default)]
    pub allowlist: Vec<SensitiveAllowlistEntry>,
}

impl Default for SensitiveConfig {
    fn default() -> Self {
        Self {
            enforcement: SensitiveEnforcement::Warn,
            allowlist: vec![],
        }
    }
}

fn default_refine_feedback() -> String {
    "make it shorter".to_owned()
}

fn fallback_str<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.is_empty() { fallback } else { value }
}

/// Main configuration. All fields have defaults matching the TypeScript extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(default = "default_backend")]
    pub backend: Backend,

    #[serde(default = "default_backend_order")]
    pub backend_order: Vec<Backend>,

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

    #[serde(default = "default_gemini_model")]
    pub gemini_model: String,

    // --- PR pipeline models ---
    #[serde(default = "default_opencode_pr_provider")]
    pub opencode_pr_provider: String,

    #[serde(default = "default_opencode_pr_model")]
    pub opencode_pr_model: String,

    #[serde(default = "default_opencode_cheap_provider")]
    pub opencode_cheap_provider: String,

    #[serde(default = "default_opencode_cheap_model")]
    pub opencode_cheap_model: String,

    #[serde(default = "default_claude_pr_model")]
    pub claude_pr_model: String,

    #[serde(default = "default_claude_cheap_model")]
    pub claude_cheap_model: String,

    #[serde(default = "default_codex_pr_model")]
    pub codex_pr_model: String,

    #[serde(default = "default_codex_cheap_model")]
    pub codex_cheap_model: String,

    #[serde(default)]
    pub codex_pr_provider: String,

    #[serde(default)]
    pub codex_cheap_provider: String,

    #[serde(default = "default_gemini_pr_model")]
    pub gemini_pr_model: String,

    #[serde(default = "default_gemini_cheap_model")]
    pub gemini_cheap_model: String,

    #[serde(default)]
    pub pr_base_branch: String,

    #[serde(default)]
    pub branch_mode: BranchMode,

    #[serde(default = "default_diff_source")]
    pub diff_source: DiffSource,

    #[serde(default = "default_max_diff_length")]
    pub max_diff_length: usize,

    #[serde(default = "default_commit_branch_timeout_seconds")]
    pub commit_branch_timeout_seconds: u64,

    #[serde(default = "default_pr_timeout_seconds")]
    pub pr_timeout_seconds: u64,

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

    #[serde(default = "default_true")]
    pub auto_update: bool,

    #[serde(default)]
    pub refine: RefineConfig,

    #[serde(default)]
    pub custom: CustomConfig,

    #[serde(default)]
    pub sensitive: SensitiveConfig,

    #[serde(default)]
    pub api: ApiConfig,
}

// --- Default value functions ---

fn default_backend() -> Backend {
    Backend::Codex
}

fn default_backend_order() -> Vec<Backend> {
    vec![
        Backend::Codex,
        Backend::Opencode,
        Backend::Claude,
        Backend::Gemini,
    ]
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

fn default_gemini_model() -> String {
    "gemini-2.5-flash".to_owned()
}

fn default_diff_source() -> DiffSource {
    DiffSource::Auto
}

fn default_max_diff_length() -> usize {
    10000
}

fn default_commit_branch_timeout_seconds() -> u64 {
    70
}

fn default_pr_timeout_seconds() -> u64 {
    180
}

fn default_true() -> bool {
    true
}

fn default_commit_template() -> String {
    "{{type}}({{scope}}): {{message}}".to_owned()
}

fn default_languages() -> Vec<LanguageConfig> {
    crate::languages::default_languages()
}

fn default_active_language() -> String {
    "English".to_owned()
}

fn default_opencode_pr_provider() -> String {
    "openai".to_owned()
}

fn default_opencode_pr_model() -> String {
    "gpt-5.4".to_owned()
}

fn default_opencode_cheap_provider() -> String {
    "openai".to_owned()
}

fn default_opencode_cheap_model() -> String {
    "gpt-5.4-mini".to_owned()
}

fn default_claude_pr_model() -> String {
    "claude-opus-4-6".to_owned()
}

fn default_claude_cheap_model() -> String {
    "claude-haiku-4-5".to_owned()
}

fn default_codex_pr_model() -> String {
    "gpt-5.4".to_owned()
}

fn default_codex_cheap_model() -> String {
    "gpt-5.4-mini".to_owned()
}

fn default_gemini_pr_model() -> String {
    "gemini-3-flash-preview".to_owned()
}

fn default_gemini_cheap_model() -> String {
    "gemini-3.1-flash-lite-preview".to_owned()
}

fn default_openai_api_config() -> ApiProviderConfig {
    ApiProviderConfig {
        model: "gpt-5.4-mini".to_owned(),
        endpoint: "https://api.openai.com/v1/chat/completions".to_owned(),
        key_env: "OPENAI_API_KEY".to_owned(),
        pr_model: "gpt-5.4".to_owned(),
        cheap_model: "gpt-5.4-mini".to_owned(),
    }
}

fn default_anthropic_api_config() -> ApiProviderConfig {
    ApiProviderConfig {
        model: "claude-sonnet-4-6".to_owned(),
        endpoint: "https://api.anthropic.com/v1/messages".to_owned(),
        key_env: "ANTHROPIC_API_KEY".to_owned(),
        pr_model: "claude-opus-4-6".to_owned(),
        cheap_model: "claude-haiku-4-5".to_owned(),
    }
}

fn default_gemini_api_config() -> ApiProviderConfig {
    ApiProviderConfig {
        model: "gemini-2.5-flash".to_owned(),
        endpoint: "https://generativelanguage.googleapis.com/v1beta".to_owned(),
        key_env: "GEMINI_API_KEY".to_owned(),
        pr_model: "gemini-3-flash-preview".to_owned(),
        cheap_model: "gemini-3.1-flash-lite-preview".to_owned(),
    }
}

fn default_openrouter_api_config() -> ApiProviderConfig {
    ApiProviderConfig {
        model: "anthropic/claude-sonnet-4".to_owned(),
        endpoint: "https://openrouter.ai/api/v1/chat/completions".to_owned(),
        key_env: "OPENROUTER_API_KEY".to_owned(),
        pr_model: "openai/gpt-5.4".to_owned(),
        cheap_model: "openai/gpt-5.4-mini".to_owned(),
    }
}

fn default_opencode_api_config() -> ApiProviderConfig {
    ApiProviderConfig {
        model: "gpt-5.4-mini".to_owned(),
        endpoint: "https://opencode.ai/zen/v1/chat/completions".to_owned(),
        key_env: "OPENCODE_API_KEY".to_owned(),
        pr_model: "gpt-5.4".to_owned(),
        cheap_model: "gpt-5.4-mini".to_owned(),
    }
}

fn default_ollama_api_config() -> ApiProviderConfig {
    ApiProviderConfig {
        model: String::new(),
        endpoint: "http://localhost:11434".to_owned(),
        key_env: String::new(),
        pr_model: String::new(),
        cheap_model: String::new(),
    }
}

fn default_lm_studio_api_config() -> ApiProviderConfig {
    ApiProviderConfig {
        model: String::new(),
        endpoint: "http://localhost:1234".to_owned(),
        key_env: String::new(),
        pr_model: String::new(),
        cheap_model: String::new(),
    }
}

fn default_custom_api_config() -> ApiProviderConfig {
    ApiProviderConfig {
        model: String::new(),
        endpoint: String::new(),
        key_env: String::new(),
        pr_model: String::new(),
        cheap_model: String::new(),
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            backend: default_backend(),
            backend_order: default_backend_order(),
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
            gemini_model: default_gemini_model(),
            opencode_pr_provider: default_opencode_pr_provider(),
            opencode_pr_model: default_opencode_pr_model(),
            opencode_cheap_provider: default_opencode_cheap_provider(),
            opencode_cheap_model: default_opencode_cheap_model(),
            claude_pr_model: default_claude_pr_model(),
            claude_cheap_model: default_claude_cheap_model(),
            codex_pr_model: default_codex_pr_model(),
            codex_cheap_model: default_codex_cheap_model(),
            codex_pr_provider: String::new(),
            codex_cheap_provider: String::new(),
            gemini_pr_model: default_gemini_pr_model(),
            gemini_cheap_model: default_gemini_cheap_model(),
            pr_base_branch: String::new(),
            branch_mode: BranchMode::default(),
            diff_source: default_diff_source(),
            max_diff_length: default_max_diff_length(),
            commit_branch_timeout_seconds: default_commit_branch_timeout_seconds(),
            pr_timeout_seconds: default_pr_timeout_seconds(),
            use_emojis: false,
            use_lower_case: true,
            commit_template: default_commit_template(),
            languages: default_languages(),
            active_language: default_active_language(),
            show_language_selector: true,
            auto_update: true,
            refine: RefineConfig::default(),
            custom: CustomConfig::default(),
            sensitive: SensitiveConfig::default(),
            api: ApiConfig::default(),
        }
    }
}

impl Config {
    fn default_config_dir_from_env(
        is_windows: bool,
        xdg: Option<PathBuf>,
        home: Option<PathBuf>,
        appdata: Option<PathBuf>,
    ) -> Option<PathBuf> {
        if is_windows {
            if let Some(appdata) = appdata {
                return Some(appdata.join("opencodecommit"));
            }
            return home.map(|home| home.join("AppData/Roaming/opencodecommit"));
        }

        if let Some(xdg) = xdg {
            return Some(xdg.join("opencodecommit"));
        }

        home.map(|home| home.join(".config/opencodecommit"))
    }

    /// Default config directory for the current host.
    pub fn default_config_dir() -> Option<PathBuf> {
        Self::default_config_dir_from_env(
            cfg!(target_os = "windows"),
            std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
            std::env::var_os("HOME").map(PathBuf::from),
            std::env::var_os("APPDATA").map(PathBuf::from),
        )
    }

    /// Canonical config file path from env override or the host default path.
    pub fn resolved_config_path() -> Option<PathBuf> {
        std::env::var_os(CONFIG_ENV)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .or_else(|| Self::default_config_dir().map(|dir| dir.join("config.toml")))
    }

    /// Resolve the language instruction for the active language.
    pub fn active_language_instruction(&self) -> String {
        self.languages
            .iter()
            .find(|l| l.label == self.active_language)
            .map(|l| l.instruction.clone())
            .unwrap_or_else(|| "Write the commit message in English.".to_owned())
    }

    /// Resolve prompt modules for the active language.
    /// Missing fields on the active language fall back to the first language.
    pub fn active_prompt_modules(&self) -> PromptModules {
        let active = self
            .languages
            .iter()
            .find(|l| l.label == self.active_language);
        let fallback = self.languages.first();

        let resolve = |getter: fn(&LanguageConfig) -> Option<&str>| -> String {
            active
                .and_then(getter)
                .or_else(|| fallback.and_then(getter))
                .unwrap_or("")
                .to_owned()
        };

        PromptModules {
            base_module: resolve(|l| l.base_module.as_deref()),
            adaptive_format: resolve(|l| l.adaptive_format.as_deref()),
            conventional_format: resolve(|l| l.conventional_format.as_deref()),
            multiline_length: resolve(|l| l.multiline_length.as_deref()),
            oneliner_length: resolve(|l| l.oneliner_length.as_deref()),
            sensitive_content_note: resolve(|l| l.sensitive_content_note.as_deref()),
        }
    }

    /// Return the CLI path for the current backend.
    pub fn backend_cli_path(&self) -> &str {
        self.backend
            .cli_backend()
            .map(|backend| self.cli_path_for(backend))
            .unwrap_or("")
    }

    /// Return the CLI path for a specific backend.
    pub fn cli_path_for(&self, backend: CliBackend) -> &str {
        match backend {
            CliBackend::Opencode => &self.cli_path,
            CliBackend::Claude => &self.claude_path,
            CliBackend::Codex => &self.codex_path,
            CliBackend::Gemini => &self.gemini_path,
        }
    }

    /// Return the effective backend order for failover.
    pub fn effective_backend_order(&self) -> &[Backend] {
        &self.backend_order
    }

    /// Return the model for the current backend.
    pub fn backend_model(&self) -> &str {
        self.backend_model_for(self.backend)
    }

    /// Return the PR model for the current backend.
    pub fn backend_pr_model(&self) -> &str {
        self.backend_pr_model_for(self.backend)
    }

    /// Return the cheap model for the current backend.
    pub fn backend_cheap_model(&self) -> &str {
        self.backend_cheap_model_for(self.backend)
    }

    /// Return the PR provider for the current backend (OpenCode/Codex only).
    pub fn backend_pr_provider(&self) -> &str {
        self.backend_pr_provider_for(self.backend)
    }

    /// Return the cheap provider for the current backend (OpenCode/Codex only).
    pub fn backend_cheap_provider(&self) -> &str {
        self.backend_cheap_provider_for(self.backend)
    }

    pub fn backend_model_for(&self, backend: Backend) -> &str {
        match backend {
            Backend::Opencode => &self.model,
            Backend::Claude => &self.claude_model,
            Backend::Codex => &self.codex_model,
            Backend::Gemini => &self.gemini_model,
            Backend::OpenaiApi => &self.api.openai.model,
            Backend::AnthropicApi => &self.api.anthropic.model,
            Backend::GeminiApi => &self.api.gemini.model,
            Backend::OpenrouterApi => &self.api.openrouter.model,
            Backend::OpencodeApi => &self.api.opencode.model,
            Backend::OllamaApi => &self.api.ollama.model,
            Backend::LmStudioApi => &self.api.lm_studio.model,
            Backend::CustomApi => &self.api.custom.model,
        }
    }

    pub fn backend_pr_model_for(&self, backend: Backend) -> &str {
        match backend {
            Backend::Opencode => &self.opencode_pr_model,
            Backend::Claude => &self.claude_pr_model,
            Backend::Codex => &self.codex_pr_model,
            Backend::Gemini => &self.gemini_pr_model,
            Backend::OpenaiApi => fallback_str(&self.api.openai.pr_model, &self.api.openai.model),
            Backend::AnthropicApi => {
                fallback_str(&self.api.anthropic.pr_model, &self.api.anthropic.model)
            }
            Backend::GeminiApi => fallback_str(&self.api.gemini.pr_model, &self.api.gemini.model),
            Backend::OpenrouterApi => {
                fallback_str(&self.api.openrouter.pr_model, &self.api.openrouter.model)
            }
            Backend::OpencodeApi => {
                fallback_str(&self.api.opencode.pr_model, &self.api.opencode.model)
            }
            Backend::OllamaApi => fallback_str(&self.api.ollama.pr_model, &self.api.ollama.model),
            Backend::LmStudioApi => {
                fallback_str(&self.api.lm_studio.pr_model, &self.api.lm_studio.model)
            }
            Backend::CustomApi => fallback_str(&self.api.custom.pr_model, &self.api.custom.model),
        }
    }

    pub fn backend_cheap_model_for(&self, backend: Backend) -> &str {
        match backend {
            Backend::Opencode => &self.opencode_cheap_model,
            Backend::Claude => &self.claude_cheap_model,
            Backend::Codex => &self.codex_cheap_model,
            Backend::Gemini => &self.gemini_cheap_model,
            Backend::OpenaiApi => {
                fallback_str(&self.api.openai.cheap_model, &self.api.openai.model)
            }
            Backend::AnthropicApi => {
                fallback_str(&self.api.anthropic.cheap_model, &self.api.anthropic.model)
            }
            Backend::GeminiApi => {
                fallback_str(&self.api.gemini.cheap_model, &self.api.gemini.model)
            }
            Backend::OpenrouterApi => {
                fallback_str(&self.api.openrouter.cheap_model, &self.api.openrouter.model)
            }
            Backend::OpencodeApi => {
                fallback_str(&self.api.opencode.cheap_model, &self.api.opencode.model)
            }
            Backend::OllamaApi => {
                fallback_str(&self.api.ollama.cheap_model, &self.api.ollama.model)
            }
            Backend::LmStudioApi => {
                fallback_str(&self.api.lm_studio.cheap_model, &self.api.lm_studio.model)
            }
            Backend::CustomApi => {
                fallback_str(&self.api.custom.cheap_model, &self.api.custom.model)
            }
        }
    }

    pub fn backend_pr_provider_for(&self, backend: Backend) -> &str {
        match backend {
            Backend::Opencode => &self.opencode_pr_provider,
            Backend::Codex => &self.codex_pr_provider,
            _ => "",
        }
    }

    pub fn backend_cheap_provider_for(&self, backend: Backend) -> &str {
        match backend {
            Backend::Opencode => &self.opencode_cheap_provider,
            Backend::Codex => &self.codex_cheap_provider,
            _ => "",
        }
    }

    pub fn api_config_for(&self, backend: Backend) -> Option<&ApiProviderConfig> {
        match backend {
            Backend::OpenaiApi => Some(&self.api.openai),
            Backend::AnthropicApi => Some(&self.api.anthropic),
            Backend::GeminiApi => Some(&self.api.gemini),
            Backend::OpenrouterApi => Some(&self.api.openrouter),
            Backend::OpencodeApi => Some(&self.api.opencode),
            Backend::OllamaApi => Some(&self.api.ollama),
            Backend::LmStudioApi => Some(&self.api.lm_studio),
            Backend::CustomApi => Some(&self.api.custom),
            Backend::Opencode | Backend::Claude | Backend::Codex | Backend::Gemini => None,
        }
    }

    pub fn api_endpoint_for(&self, backend: Backend) -> &str {
        self.api_config_for(backend)
            .map(|provider| provider.endpoint.as_str())
            .unwrap_or("")
    }

    pub fn api_key_env_for(&self, backend: Backend) -> &str {
        self.api_config_for(backend)
            .map(|provider| provider.key_env.as_str())
            .unwrap_or("")
    }

    /// Existing canonical config file path, if present.
    pub fn default_config_path() -> Option<PathBuf> {
        let path = Self::resolved_config_path()?;
        path.exists().then_some(path)
    }

    fn materialize_default_config_path() -> crate::Result<Option<PathBuf>> {
        let Some(path) = Self::resolved_config_path() else {
            return Ok(None);
        };

        if path.exists() {
            return Ok(Some(path));
        }

        let config = Self::default();
        config.save_to_path(&path)?;
        Ok(Some(path))
    }

    /// Load config from a TOML file. Missing fields get defaults.
    pub fn load(path: &Path) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            Error::Config(format!(
                "failed to read config file {}: {e}",
                path.display()
            ))
        })?;
        let config: Self = toml::from_str(&content)
            .map_err(|e| Error::Config(format!("failed to parse config file: {e}")))?;
        config.validate()?;
        Ok(config)
    }

    /// Load from the explicit or canonical config path, or materialize defaults if absent.
    pub fn load_or_default(explicit_path: Option<&Path>) -> crate::Result<Self> {
        if let Some(path) = explicit_path {
            return Self::load(path);
        }

        if let Some(path) = Self::resolved_config_path() {
            if path.exists() {
                return Self::load(&path);
            }
        }

        if Self::materialize_default_config_path()?.is_some() {
            return Ok(Self::default());
        }

        Ok(Self::default())
    }

    pub fn save_to_path(&self, path: &Path) -> crate::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        self.validate()?;
        let content = toml::to_string_pretty(self)
            .map_err(|err| Error::Config(format!("failed to serialize config file: {err}")))?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn save_default(&self) -> crate::Result<PathBuf> {
        let path = Self::resolved_config_path()
            .ok_or_else(|| Error::Config("failed to resolve config path".to_owned()))?;
        self.save_to_path(&path)?;
        Ok(path)
    }

    pub fn validate(&self) -> crate::Result<()> {
        for (index, entry) in self.sensitive.allowlist.iter().enumerate() {
            if entry.path_regex.is_none() && entry.rule.is_none() && entry.value_regex.is_none() {
                return Err(Error::Config(format!(
                    "sensitive.allowlist[{index}] must define path-regex, rule, or value-regex"
                )));
            }

            if let Some(pattern) = entry.path_regex.as_deref() {
                regex::Regex::new(pattern).map_err(|err| {
                    Error::Config(format!(
                        "invalid sensitive.allowlist[{index}].path-regex: {err}"
                    ))
                })?;
            }

            if let Some(pattern) = entry.value_regex.as_deref() {
                regex::Regex::new(pattern).map_err(|err| {
                    Error::Config(format!(
                        "invalid sensitive.allowlist[{index}].value-regex: {err}"
                    ))
                })?;
            }
        }

        Ok(())
    }

    pub fn apply_sensitive_profile(&mut self, profile: SensitiveProfile) {
        self.sensitive.enforcement = match profile {
            SensitiveProfile::Human => SensitiveEnforcement::Warn,
            SensitiveProfile::StrictAgent => SensitiveEnforcement::StrictAll,
        };
    }
}

/// Default emoji map matching the TypeScript extension.
pub const DEFAULT_EMOJIS: &[(&str, &str)] = &[
    ("feat", "\u{2728}"),             // ✨
    ("fix", "\u{1f41b}"),             // 🐛
    ("docs", "\u{1f4dd}"),            // 📝
    ("style", "\u{1f48e}"),           // 💎
    ("refactor", "\u{267b}\u{fe0f}"), // ♻️
    ("test", "\u{1f9ea}"),            // 🧪
    ("chore", "\u{1f4e6}"),           // 📦
    ("perf", "\u{26a1}"),             // ⚡
    ("security", "\u{1f512}"),        // 🔒
    ("revert", "\u{23ea}"),           // ⏪
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{LazyLock, Mutex};

    static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
    use std::io::Write;

    #[test]
    fn default_values_match_typescript() {
        let cfg = Config::default();
        assert_eq!(cfg.backend, Backend::Codex);
        assert_eq!(
            cfg.backend_order,
            vec![
                Backend::Codex,
                Backend::Opencode,
                Backend::Claude,
                Backend::Gemini
            ]
        );
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
        assert_eq!(cfg.gemini_model, "gemini-2.5-flash");
        assert_eq!(cfg.diff_source, DiffSource::Auto);
        assert_eq!(cfg.max_diff_length, 10000);
        assert_eq!(cfg.commit_branch_timeout_seconds, 70);
        assert_eq!(cfg.pr_timeout_seconds, 180);
        assert!(!cfg.use_emojis);
        assert!(cfg.use_lower_case);
        assert_eq!(cfg.commit_template, "{{type}}({{scope}}): {{message}}");
        assert_eq!(cfg.languages.len(), 12);
        assert_eq!(cfg.languages[0].label, "English");
        assert_eq!(cfg.languages[1].label, "Finnish");
        assert_eq!(cfg.languages[2].label, "Japanese");
        assert_eq!(cfg.languages[3].label, "Chinese");
        assert_eq!(cfg.languages[4].label, "Spanish");
        assert_eq!(cfg.languages[5].label, "Portuguese");
        assert_eq!(cfg.languages[6].label, "French");
        assert_eq!(cfg.languages[7].label, "Korean");
        assert_eq!(cfg.languages[8].label, "Russian");
        assert_eq!(cfg.languages[9].label, "Vietnamese");
        assert_eq!(cfg.languages[10].label, "German");
        assert_eq!(cfg.languages[11].label, "Custom (example)");
        assert_eq!(cfg.active_language, "English");
        assert!(cfg.show_language_selector);
        assert_eq!(cfg.refine.default_feedback, "make it shorter");
        assert!(cfg.custom.prompt.is_empty());
        assert!(cfg.custom.emojis.is_empty());
        assert_eq!(cfg.api.openai.key_env, "OPENAI_API_KEY");
        assert_eq!(
            cfg.api.openai.endpoint,
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(cfg.api.anthropic.key_env, "ANTHROPIC_API_KEY");
        assert_eq!(cfg.api.ollama.endpoint, "http://localhost:11434");
        assert!(cfg.api.ollama.key_env.is_empty());
        assert_eq!(cfg.api.lm_studio.endpoint, "http://localhost:1234");
    }

    #[test]
    fn active_language_instruction_lookup() {
        let mut cfg = Config::default();
        assert_eq!(
            cfg.active_language_instruction(),
            "Write the commit message in English."
        );

        cfg.active_language = "Finnish".to_owned();
        assert!(cfg.active_language_instruction().contains("suomeksi"));

        cfg.active_language = "Japanese".to_owned();
        assert!(cfg.active_language_instruction().contains("日本語"));

        cfg.active_language = "Chinese".to_owned();
        assert!(cfg.active_language_instruction().contains("中文"));

        cfg.active_language = "Spanish".to_owned();
        assert!(cfg.active_language_instruction().contains("español"));

        cfg.active_language = "Korean".to_owned();
        assert!(cfg.active_language_instruction().contains("한국어"));

        cfg.active_language = "Nonexistent".to_owned();
        assert_eq!(
            cfg.active_language_instruction(),
            "Write the commit message in English."
        );
    }

    #[test]
    fn active_prompt_modules_english() {
        let cfg = Config::default();
        let mods = cfg.active_prompt_modules();
        assert!(
            mods.base_module
                .contains("expert at writing git commit messages")
        );
        assert!(mods.adaptive_format.contains("{recentCommits}"));
        assert!(
            mods.conventional_format
                .contains("conventional commit format")
        );
        assert!(mods.multiline_length.contains("72 characters"));
        assert!(mods.oneliner_length.contains("exactly one line"));
        assert!(mods.sensitive_content_note.contains("sensitive content"));
    }

    #[test]
    fn active_prompt_modules_finnish() {
        let cfg = Config {
            active_language: "Finnish".to_owned(),
            ..Config::default()
        };
        let mods = cfg.active_prompt_modules();
        assert!(mods.base_module.contains("Olet asiantuntija"));
        assert!(mods.adaptive_format.contains("Noudata alla"));
        assert!(
            mods.conventional_format
                .contains("conventional commit -muotoa")
        );
    }

    #[test]
    fn active_prompt_modules_japanese() {
        let cfg = Config {
            active_language: "Japanese".to_owned(),
            ..Config::default()
        };
        let mods = cfg.active_prompt_modules();
        assert!(mods.base_module.contains("コミットメッセージ"));
        assert!(mods.adaptive_format.contains("最近のコミット"));
        assert!(
            mods.conventional_format
                .contains("conventional commit 形式")
        );
    }

    #[test]
    fn active_prompt_modules_chinese() {
        let cfg = Config {
            active_language: "Chinese".to_owned(),
            ..Config::default()
        };
        let mods = cfg.active_prompt_modules();
        assert!(mods.base_module.contains("提交信息"));
        assert!(mods.adaptive_format.contains("最近的提交"));
        assert!(
            mods.conventional_format
                .contains("conventional commit 格式")
        );
    }

    #[test]
    fn active_prompt_modules_additional_languages() {
        for (label, needle) in [
            ("Spanish", "mensaje de commit"),
            ("Portuguese", "mensagem de commit"),
            ("French", "message de commit"),
            ("Korean", "커밋 메시지"),
            ("Russian", "сообщением коммита"),
            ("Vietnamese", "commit message"),
            ("German", "Commit-Nachricht"),
        ] {
            let cfg = Config {
                active_language: label.to_owned(),
                ..Config::default()
            };
            let mods = cfg.active_prompt_modules();
            assert!(mods.base_module.contains(needle), "{label} base module");
            assert!(mods.adaptive_format.contains("{recentCommits}"));
            assert!(!cfg.active_language_instruction().is_empty());
        }
    }

    #[test]
    fn active_prompt_modules_custom_falls_back_to_english() {
        let cfg = Config {
            active_language: "Custom (example)".to_owned(),
            ..Config::default()
        };
        let mods = cfg.active_prompt_modules();
        // Custom has no modules → falls back to first language (English)
        assert!(
            mods.base_module
                .contains("expert at writing git commit messages")
        );
    }

    #[test]
    fn backend_model_and_path() {
        let mut cfg = Config::default();
        assert_eq!(cfg.backend_model(), "gpt-5.4-mini");
        assert_eq!(cfg.backend_cli_path(), "");

        cfg.backend = Backend::Claude;
        cfg.claude_path = "/usr/bin/claude".to_owned();
        assert_eq!(cfg.backend_model(), "claude-sonnet-4-6");
        assert_eq!(cfg.backend_cli_path(), "/usr/bin/claude");

        cfg.backend = Backend::Codex;
        cfg.codex_path = "/usr/bin/codex".to_owned();
        assert_eq!(cfg.backend_model(), "gpt-5.4-mini");
        assert_eq!(cfg.backend_cli_path(), "/usr/bin/codex");

        cfg.backend = Backend::Gemini;
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
commit-branch-timeout-seconds = 95
pr-timeout-seconds = 240
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
        assert_eq!(cfg.backend, Backend::Claude);
        assert_eq!(cfg.commit_mode, CommitMode::Conventional);
        assert_eq!(cfg.provider, "anthropic");
        assert_eq!(cfg.model, "opus");
        assert_eq!(cfg.claude_model, "opus");
        assert_eq!(cfg.max_diff_length, 5000);
        assert_eq!(cfg.commit_branch_timeout_seconds, 95);
        assert_eq!(cfg.pr_timeout_seconds, 240);
        assert!(cfg.use_emojis);
        assert!(!cfg.use_lower_case);
        assert_eq!(cfg.refine.default_feedback, "be more specific");
        assert!(!cfg.custom.prompt.is_empty());

        // Unset fields should get defaults
        assert_eq!(cfg.diff_source, DiffSource::Auto);
        assert_eq!(cfg.commit_template, "{{type}}({{scope}}): {{message}}");
        assert_eq!(cfg.codex_model, "gpt-5.4-mini");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn api_tables_deserialize_and_drive_backend_helpers() {
        let cfg: Config = toml::from_str(
            r#"
backend = "openai-api"
backend-order = ["openai-api", "claude", "ollama-api"]

[api.openai]
model = "gpt-test"
endpoint = "https://example.test/v1/chat/completions"
key-env = "TEST_OPENAI_KEY"
pr-model = "gpt-test-pr"
cheap-model = "gpt-test-cheap"

[api.ollama]
model = "qwen2.5:latest"
endpoint = "http://127.0.0.1:11434"
"#,
        )
        .unwrap();

        assert_eq!(cfg.backend, Backend::OpenaiApi);
        assert_eq!(
            cfg.backend_order,
            vec![Backend::OpenaiApi, Backend::Claude, Backend::OllamaApi]
        );
        assert_eq!(cfg.backend_model(), "gpt-test");
        assert_eq!(cfg.backend_pr_model(), "gpt-test-pr");
        assert_eq!(cfg.backend_cheap_model(), "gpt-test-cheap");
        assert_eq!(
            cfg.api_endpoint_for(Backend::OpenaiApi),
            "https://example.test/v1/chat/completions"
        );
        assert_eq!(cfg.api_key_env_for(Backend::OpenaiApi), "TEST_OPENAI_KEY");
        assert_eq!(cfg.backend_model_for(Backend::OllamaApi), "qwen2.5:latest");
        assert_eq!(
            cfg.api_endpoint_for(Backend::OllamaApi),
            "http://127.0.0.1:11434"
        );
    }

    #[test]
    fn load_nonexistent_file_errors() {
        let result = Config::load(Path::new("/tmp/nonexistent-occ-config.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn default_config_dir_prefers_appdata_on_windows_inputs() {
        let home = PathBuf::from(r"C:/Users/tester");
        let appdata = PathBuf::from(r"C:/Users/tester/AppData/Roaming");
        let dir = Config::default_config_dir_from_env(
            true,
            Some(PathBuf::from("/ignored-xdg")),
            Some(home),
            Some(appdata.clone()),
        )
        .unwrap();

        assert_eq!(dir, appdata.join("opencodecommit"));
    }

    #[test]
    fn default_config_dir_uses_windows_home_fallback_without_appdata() {
        let home = PathBuf::from(r"C:/Users/tester");
        let dir =
            Config::default_config_dir_from_env(true, None, Some(home.clone()), None).unwrap();

        assert_eq!(dir, home.join("AppData/Roaming/opencodecommit"));
    }

    #[test]
    fn env_config_path_overrides_default_location() {
        let _env_guard = ENV_LOCK.lock().unwrap();
        let temp_root = std::env::temp_dir().join(format!(
            "occ-env-config-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let xdg_root = temp_root.join("xdg");
        let env_path = temp_root.join("shared").join("config.toml");
        let default_path = xdg_root.join("opencodecommit").join("config.toml");
        let previous_xdg = std::env::var_os("XDG_CONFIG_HOME");
        let previous_env = std::env::var_os(CONFIG_ENV);

        let _ = std::fs::remove_dir_all(&temp_root);

        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", &xdg_root);
            std::env::set_var(CONFIG_ENV, &env_path);
        }

        let cfg = Config::load_or_default(None).unwrap();

        assert_eq!(cfg.backend, Backend::Codex);
        assert!(env_path.exists());
        assert!(!default_path.exists());

        match previous_xdg {
            Some(value) => unsafe {
                std::env::set_var("XDG_CONFIG_HOME", value);
            },
            None => unsafe {
                std::env::remove_var("XDG_CONFIG_HOME");
            },
        }
        match previous_env {
            Some(value) => unsafe {
                std::env::set_var(CONFIG_ENV, value);
            },
            None => unsafe {
                std::env::remove_var(CONFIG_ENV);
            },
        }
        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn load_or_default_with_no_file() {
        let _env_guard = ENV_LOCK.lock().unwrap();
        let temp_root = std::env::temp_dir().join(format!(
            "occ-load-or-default-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let config_root = temp_root.join("xdg");
        let config_path = config_root.join("opencodecommit").join("config.toml");
        let previous_xdg = std::env::var_os("XDG_CONFIG_HOME");
        let previous_env = std::env::var_os(CONFIG_ENV);

        let _ = std::fs::remove_dir_all(&temp_root);

        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", &config_root);
            std::env::remove_var(CONFIG_ENV);
        }

        let cfg = Config::load_or_default(None).unwrap();

        let serialized = std::fs::read_to_string(&config_path).unwrap();
        assert_eq!(cfg.backend, Backend::Codex);
        assert_eq!(cfg.model, "gpt-5.4-mini");
        assert!(config_path.exists());
        assert!(serialized.contains("backend-order"));
        assert!(serialized.contains("commit-branch-timeout-seconds"));
        assert!(serialized.contains("pr-timeout-seconds"));
        assert!(serialized.contains("sensitive"));
        assert!(serialized.contains("[[languages]]"));
        assert!(serialized.contains("base-module"));
        assert!(serialized.contains("sensitive-content-note"));
        assert!(serialized.contains("[api.openai]"));
        assert!(serialized.contains("key-env = \"OPENAI_API_KEY\""));
        assert!(serialized.contains("[api.ollama]"));

        match previous_xdg {
            Some(value) => unsafe {
                std::env::set_var("XDG_CONFIG_HOME", value);
            },
            None => unsafe {
                std::env::remove_var("XDG_CONFIG_HOME");
            },
        }
        match previous_env {
            Some(value) => unsafe {
                std::env::set_var(CONFIG_ENV, value);
            },
            None => unsafe {
                std::env::remove_var(CONFIG_ENV);
            },
        }
        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn backend_pr_and_cheap_models() {
        let mut cfg = Config::default();
        // Default backend is Codex, so PR/cheap pair comes from codex fields.
        assert_eq!(cfg.backend_pr_model(), "gpt-5.4");
        assert_eq!(cfg.backend_cheap_model(), "gpt-5.4-mini");
        assert_eq!(cfg.backend_pr_provider(), "");
        assert_eq!(cfg.backend_cheap_provider(), "");

        cfg.backend = Backend::Opencode;
        assert_eq!(cfg.backend_pr_model(), "gpt-5.4");
        assert_eq!(cfg.backend_cheap_model(), "gpt-5.4-mini");
        assert_eq!(cfg.backend_pr_provider(), "openai");
        assert_eq!(cfg.backend_cheap_provider(), "openai");

        cfg.backend = Backend::Claude;
        assert_eq!(cfg.backend_pr_model(), "claude-opus-4-6");
        assert_eq!(cfg.backend_cheap_model(), "claude-haiku-4-5");
        assert_eq!(cfg.backend_pr_provider(), "");
        assert_eq!(cfg.backend_cheap_provider(), "");

        cfg.backend = Backend::Codex;
        assert_eq!(cfg.backend_pr_model(), "gpt-5.4");
        assert_eq!(cfg.backend_cheap_model(), "gpt-5.4-mini");

        cfg.backend = Backend::Gemini;
        assert_eq!(cfg.backend_pr_model(), "gemini-3-flash-preview");
        assert_eq!(cfg.backend_cheap_model(), "gemini-3.1-flash-lite-preview");
    }

    #[test]
    fn pr_base_branch_defaults_empty() {
        let cfg = Config::default();
        assert_eq!(cfg.pr_base_branch, "");
    }

    #[test]
    fn pr_model_fields_deserialize() {
        let cfg: Config = toml::from_str(
            r#"
claude-pr-model = "claude-sonnet-4-6"
claude-cheap-model = "claude-haiku-4-5"
pr-base-branch = "develop"
"#,
        )
        .unwrap();
        assert_eq!(cfg.claude_pr_model, "claude-sonnet-4-6");
        assert_eq!(cfg.claude_cheap_model, "claude-haiku-4-5");
        assert_eq!(cfg.pr_base_branch, "develop");
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
