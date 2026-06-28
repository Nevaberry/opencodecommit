use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{Datelike, SecondsFormat, Utc};
use opencodecommit::config::DiffSource;
use opencodecommit::git;
use opencodecommit::sensitive::{
    SensitiveEnforcement, scan_diff_for_sensitive_content_with_options,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const EVIDENCE_CONFIG: &str = "evidence.toml";
const ASSIST_NEXT: &str = "assist-next.toml";
const ASSIST_TTL_SECS: u64 = 15 * 60;
const DEFENCE_WARNING: &str = "OpenCodeCommit defence evidence profile is all-in cleartext evidence collection. It may record public IP, local network details, host identifiers, hardware/security state, exact tool versions, and other operational metadata. Use only in private, access-controlled repositories or encrypted artifact stores.";

#[derive(Debug)]
pub enum EvidenceError {
    Occ(opencodecommit::Error),
    Io(std::io::Error),
    Invalid(String),
}

impl fmt::Display for EvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvidenceError::Occ(err) => write!(f, "{err}"),
            EvidenceError::Io(err) => write!(f, "{err}"),
            EvidenceError::Invalid(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for EvidenceError {}

impl From<opencodecommit::Error> for EvidenceError {
    fn from(err: opencodecommit::Error) -> Self {
        Self::Occ(err)
    }
}

impl From<std::io::Error> for EvidenceError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

pub type Result<T> = std::result::Result<T, EvidenceError>;

#[derive(Debug, Clone)]
struct EvidencePaths {
    repo_root: PathBuf,
    git_dir: PathBuf,
    occ_dir: PathBuf,
    config_path: PathBuf,
    assist_next_path: PathBuf,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EvidenceProfile {
    #[default]
    Samd,
    Defence,
}

impl EvidenceProfile {
    pub fn label(self) -> &'static str {
        match self {
            EvidenceProfile::Samd => "samd",
            EvidenceProfile::Defence => "defence",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EvidenceStorage {
    #[default]
    Local,
    Repo,
    Artifact,
}

impl EvidenceStorage {
    pub fn label(self) -> &'static str {
        match self {
            EvidenceStorage::Local => "local",
            EvidenceStorage::Repo => "repo",
            EvidenceStorage::Artifact => "artifact",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceMode {
    #[default]
    Compact,
    Sidecar,
    HighAssurance,
}

impl EvidenceMode {
    fn label(self) -> &'static str {
        match self {
            EvidenceMode::Compact => "compact",
            EvidenceMode::Sidecar => "sidecar",
            EvidenceMode::HighAssurance => "high-assurance",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EvidenceRedaction {
    #[default]
    Strict,
    Cleartext,
}

impl EvidenceRedaction {
    fn label(self) -> &'static str {
        match self {
            EvidenceRedaction::Strict => "strict",
            EvidenceRedaction::Cleartext => "cleartext",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct EvidenceFieldConfig {
    pub developer_id: bool,
    pub hostname: String,
    pub os: bool,
    pub os_kernel: bool,
    pub tool_versions: bool,
    pub browser_versions: bool,
    pub agent_versions: bool,
    pub network_profile: String,
    pub public_ip: bool,
    pub raw_ip_addr: bool,
    pub mac_addresses: String,
    pub hardware_serial: bool,
    pub security_state: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AssistedByConfig {
    pub enabled: bool,
    pub prompt: String,
    pub dedupe: bool,
    pub harnesses: Vec<String>,
    pub models: Vec<String>,
    pub quick: Vec<AssistedByQuickOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AssistedByQuickOption {
    pub label: String,
    pub agent: String,
    pub model: String,
    #[serde(default)]
    pub version_command: String,
    #[serde(default)]
    pub version_pattern: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelCatalog {
    assisted_by: AssistedByCatalog,
}

#[derive(Debug, Clone, Deserialize)]
struct AssistedByCatalog {
    harnesses: Vec<String>,
    models: Vec<String>,
    quick: Vec<AssistedByCatalogQuickOption>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssistedByCatalogQuickOption {
    label: String,
    agent: String,
    model: String,
    #[serde(default)]
    version_command: String,
    #[serde(default)]
    version_pattern: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct EvidenceConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub profile: EvidenceProfile,
    #[serde(default)]
    pub mode: EvidenceMode,
    #[serde(default)]
    pub storage: EvidenceStorage,
    #[serde(default)]
    pub redaction: EvidenceRedaction,
    #[serde(default)]
    pub require_confirmation: bool,
    #[serde(default)]
    pub fields: EvidenceFieldConfig,
    #[serde(default)]
    pub assisted_by: AssistedByConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct AssistNextToken {
    kind: String,
    index_tree: String,
    expires_at_unix: u64,
    trailers: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AssistedByInput {
    pub agent: String,
    pub model: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct EvidenceSnapshot {
    pub generated_at: String,
    pub profile: EvidenceProfile,
    pub redaction: EvidenceRedaction,
    pub repository: RepositorySnapshot,
    pub environment: EnvironmentSnapshot,
    pub tools: BTreeMap<String, ToolSnapshot>,
    pub network: BTreeMap<String, String>,
    pub security: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct RepositorySnapshot {
    pub root: String,
    pub branch: String,
    pub head: String,
    pub index_tree: String,
    pub staged_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct EnvironmentSnapshot {
    pub developer: String,
    pub hostname: String,
    pub os: String,
    pub arch: String,
    pub kernel: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ToolSnapshot {
    pub command: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
struct EvidenceSidecar {
    schema_version: u32,
    generated_at: String,
    profile: EvidenceProfile,
    mode: EvidenceMode,
    storage: EvidenceStorage,
    redaction: EvidenceRedaction,
    commit_message_subject: String,
    repository: RepositorySnapshot,
    environment: EnvironmentSnapshot,
    tools: BTreeMap<String, ToolSnapshot>,
    network: BTreeMap<String, String>,
    security: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct EvidenceWrite {
    pub pointer: String,
}

impl Default for EvidenceConfig {
    fn default() -> Self {
        Self::for_profile(EvidenceProfile::Samd, EvidenceStorage::Repo)
    }
}

impl Default for EvidenceFieldConfig {
    fn default() -> Self {
        EvidenceConfig::for_profile(EvidenceProfile::Samd, EvidenceStorage::Repo).fields
    }
}

impl Default for AssistedByConfig {
    fn default() -> Self {
        default_assisted_by_config()
    }
}

impl EvidenceConfig {
    pub fn for_profile(profile: EvidenceProfile, storage: EvidenceStorage) -> Self {
        let defence = profile == EvidenceProfile::Defence;
        Self {
            enabled: true,
            profile,
            mode: if defence {
                EvidenceMode::HighAssurance
            } else {
                EvidenceMode::Compact
            },
            storage,
            redaction: if defence {
                EvidenceRedaction::Cleartext
            } else {
                EvidenceRedaction::Strict
            },
            require_confirmation: defence,
            fields: EvidenceFieldConfig {
                developer_id: true,
                hostname: if defence { "raw" } else { "label" }.to_owned(),
                os: true,
                os_kernel: true,
                tool_versions: true,
                browser_versions: true,
                agent_versions: true,
                network_profile: if defence { "raw" } else { "labelled" }.to_owned(),
                public_ip: defence,
                raw_ip_addr: defence,
                mac_addresses: if defence { "raw" } else { "hash" }.to_owned(),
                hardware_serial: defence,
                security_state: defence,
            },
            assisted_by: default_assisted_by_config(),
        }
    }
}

pub fn install(
    profile: EvidenceProfile,
    storage: Option<EvidenceStorage>,
    allow_cleartext_repo_evidence: bool,
) -> Result<String> {
    let paths = resolve_paths()?;
    let storage = storage.unwrap_or(match profile {
        EvidenceProfile::Samd => EvidenceStorage::Repo,
        EvidenceProfile::Defence => EvidenceStorage::Artifact,
    });

    if profile == EvidenceProfile::Defence
        && storage == EvidenceStorage::Repo
        && !allow_cleartext_repo_evidence
    {
        return Err(EvidenceError::Invalid(
            "defence repo storage can commit clear machine and network evidence; pass --allow-cleartext-repo-evidence to confirm".to_owned(),
        ));
    }

    let config = EvidenceConfig::for_profile(profile, storage);
    save_config_to(&paths.config_path, &config)?;

    let mut lines = vec![format!(
        "installed OpenCodeCommit evidence profile '{}' at {}",
        profile.label(),
        paths.config_path.display()
    )];
    lines.push(format!("storage: {}", storage.label()));
    if profile == EvidenceProfile::Defence {
        lines.push(String::new());
        lines.push(DEFENCE_WARNING.to_owned());
    }
    Ok(lines.join("\n"))
}

pub fn uninstall() -> Result<String> {
    let paths = resolve_paths()?;
    match std::fs::remove_file(&paths.config_path) {
        Ok(()) => Ok("uninstalled OpenCodeCommit evidence mode".to_owned()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            Ok("OpenCodeCommit evidence mode was not installed".to_owned())
        }
        Err(err) => Err(err.into()),
    }
}

pub fn status() -> Result<String> {
    let paths = resolve_paths()?;
    let Some(config) = load_config_from(&paths.config_path)? else {
        return Ok(format!(
            "OpenCodeCommit evidence: not installed\nconfig: {}",
            paths.config_path.display()
        ));
    };

    let mut lines = vec![
        format!(
            "OpenCodeCommit evidence: {}",
            if config.enabled {
                "installed"
            } else {
                "installed (disabled)"
            }
        ),
        format!("config: {}", paths.config_path.display()),
        format!("profile: {}", config.profile.label()),
        format!("mode: {}", config.mode.label()),
        format!("storage: {}", config.storage.label()),
        format!("redaction: {}", config.redaction.label()),
        format!("sidecar root: {}", sidecar_root(&paths, &config).display()),
        format!(
            "collects public IP: {}",
            if config.fields.public_ip { "yes" } else { "no" }
        ),
        format!(
            "collects raw network state: {}",
            if config.fields.raw_ip_addr {
                "yes"
            } else {
                "no"
            }
        ),
        format!(
            "Assisted-by quick options: {}",
            config
                .assisted_by
                .quick
                .iter()
                .map(|option| option.label.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    ];
    if config.profile == EvidenceProfile::Defence {
        lines.push(String::new());
        lines.push(DEFENCE_WARNING.to_owned());
    }
    Ok(lines.join("\n"))
}

pub fn snapshot(profile_override: Option<EvidenceProfile>) -> Result<String> {
    let paths = resolve_paths()?;
    let config = match load_config_from(&paths.config_path)? {
        Some(mut config) => {
            if let Some(profile) = profile_override {
                let storage = config.storage;
                config = EvidenceConfig::for_profile(profile, storage);
            }
            config
        }
        None => EvidenceConfig::for_profile(
            profile_override.unwrap_or(EvidenceProfile::Samd),
            EvidenceStorage::Local,
        ),
    };
    let snapshot = collect_snapshot(&paths.repo_root, &config)?;
    Ok(format_snapshot(&snapshot, &config))
}

pub fn append_to_commit_message(repo_root: &Path, message: &str) -> Result<String> {
    let paths = resolve_paths_for_repo(repo_root)?;
    let mut message = append_pending_assisted_by(repo_root, message)?;
    let Some(config) = load_config_from(&paths.config_path)? else {
        return Ok(message);
    };
    if !config.enabled || has_trailer(&message, "OCC-Evidence:") {
        return Ok(message);
    }

    let write = write_sidecar(&paths, &config, &message)?;
    message = append_trailer(&message, &format!("OCC-Evidence: {}", write.pointer), true);
    Ok(message)
}

pub fn add_assisted_by(input: AssistedByInput) -> Result<String> {
    let paths = resolve_paths()?;
    let trailer = assisted_by_trailer(&input);
    let index_tree = git::write_index_tree(&paths.repo_root)?;
    let mut token = load_assist_token(&paths.assist_next_path)?.unwrap_or(AssistNextToken {
        kind: "assisted-by".to_owned(),
        index_tree: index_tree.clone(),
        expires_at_unix: now_unix_secs() + ASSIST_TTL_SECS,
        trailers: vec![],
    });
    if token.index_tree != index_tree || now_unix_secs() > token.expires_at_unix {
        token.index_tree = index_tree;
        token.expires_at_unix = now_unix_secs() + ASSIST_TTL_SECS;
        token.trailers.clear();
    }
    if !token.trailers.iter().any(|existing| existing == &trailer) {
        token.trailers.push(trailer.clone());
    }
    save_assist_token(&paths.assist_next_path, &token)?;
    Ok(format!("queued {trailer} for the next matching commit"))
}

pub fn add_assisted_by_quick(label: &str) -> Result<String> {
    let paths = resolve_paths()?;
    let config = load_config_from(&paths.config_path)?.unwrap_or_default();
    let Some(option) = config
        .assisted_by
        .quick
        .iter()
        .find(|option| quick_option_label_matches(&option.label, label))
    else {
        return Err(EvidenceError::Invalid(format!(
            "unknown Assisted-by quick option '{label}'"
        )));
    };
    let version = detect_version_for_option(option).ok();
    add_assisted_by(AssistedByInput {
        agent: option.agent.clone(),
        model: option.model.clone(),
        version,
    })
}

fn quick_option_label_matches(option_label: &str, requested: &str) -> bool {
    option_label == requested
        || normalized_quick_option_label(option_label) == normalized_quick_option_label(requested)
}

fn normalized_quick_option_label(label: &str) -> &str {
    match label {
        "Codex GPT" => "GPT",
        "Claude Code Opus" => "Opus",
        "Claude Code Fable" => "Fable",
        _ => label,
    }
}

pub fn assist_status() -> Result<String> {
    let paths = resolve_paths()?;
    let config = load_config_from(&paths.config_path)?.unwrap_or_default();
    let token = load_assist_token(&paths.assist_next_path)?;
    let mut lines = vec![
        "OpenCodeCommit Assisted-by:".to_owned(),
        format!(
            "quick options: {}",
            config
                .assisted_by
                .quick
                .iter()
                .map(|option| option.label.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    ];
    if let Some(token) = token {
        lines.push(format!("pending trailers: {}", token.trailers.len()));
        lines.extend(token.trailers);
    } else {
        lines.push("pending trailers: 0".to_owned());
    }
    Ok(lines.join("\n"))
}

pub fn assist_detect() -> Result<String> {
    let config = load_config_from(&resolve_paths()?.config_path)?.unwrap_or_default();
    let mut lines = vec!["Detected AI harness versions:".to_owned()];
    for option in &config.assisted_by.quick {
        let version =
            detect_version_for_option(option).unwrap_or_else(|_| "not-installed".to_owned());
        lines.push(format!("- {}: {}", option.agent, version));
    }
    Ok(lines.join("\n"))
}

fn write_sidecar(
    paths: &EvidencePaths,
    config: &EvidenceConfig,
    message: &str,
) -> Result<EvidenceWrite> {
    let now = Utc::now();
    let snapshot = collect_snapshot(&paths.repo_root, config)?;
    let sidecar = EvidenceSidecar {
        schema_version: 1,
        generated_at: snapshot.generated_at.clone(),
        profile: config.profile,
        mode: config.mode,
        storage: config.storage,
        redaction: config.redaction,
        commit_message_subject: message.lines().next().unwrap_or("").to_owned(),
        repository: snapshot.repository,
        environment: snapshot.environment,
        tools: snapshot.tools,
        network: snapshot.network,
        security: snapshot.security,
    };
    let content = toml::to_string_pretty(&sidecar).map_err(|err| {
        EvidenceError::Invalid(format!("failed to serialize evidence sidecar: {err}"))
    })?;

    let rel = format!(
        "{:04}/{:02}/{}-{}.toml",
        now.year(),
        now.month(),
        now.format("%Y%m%dT%H%M%SZ"),
        short_hash(&sidecar.repository.index_tree)
    );
    let root = sidecar_root(paths, config);
    let path = root.join(&rel);
    scan_sidecar_content(&content, &path)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, content)?;

    let pointer = match config.storage {
        EvidenceStorage::Local => {
            format!("local:{}", display_local_pointer(paths, &path))
        }
        EvidenceStorage::Repo => {
            let repo_rel = path.strip_prefix(&paths.repo_root).unwrap_or(&path);
            stage_repo_sidecar(&paths.repo_root, repo_rel)?;
            format!("repo:{}", repo_rel.display())
        }
        EvidenceStorage::Artifact => {
            let digest = file_sha256(&path)?;
            format!("artifact:sha256:{digest}")
        }
    };

    Ok(EvidenceWrite { pointer })
}

fn collect_snapshot(repo_root: &Path, config: &EvidenceConfig) -> Result<EvidenceSnapshot> {
    let generated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let branch = command_stdout_in(repo_root, "git", &["rev-parse", "--abbrev-ref", "HEAD"])
        .unwrap_or_else(|| "unknown".to_owned());
    let head = command_stdout_in(repo_root, "git", &["rev-parse", "--verify", "HEAD"])
        .unwrap_or_else(|| "unborn".to_owned());
    let index_tree = git::write_index_tree(repo_root).unwrap_or_else(|_| "unwritten".to_owned());
    let staged_files = git::get_changed_files(DiffSource::Staged, repo_root).unwrap_or_default();

    let developer = if config.fields.developer_id {
        std::env::var("OCC_DEVELOPER_ID")
            .or_else(|_| std::env::var("USER"))
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "unknown".to_owned())
    } else {
        "disabled".to_owned()
    };
    let raw_hostname = command_stdout("hostname", &[]).unwrap_or_else(|| "unknown".to_owned());
    let hostname = match config.fields.hostname.as_str() {
        "raw" => raw_hostname,
        "none" | "disabled" => "disabled".to_owned(),
        _ => configured_label("OCC_WORKSTATION_ID").unwrap_or_else(|| "label:unknown".to_owned()),
    };
    let kernel = if config.fields.os_kernel {
        command_stdout("uname", &["-sr"]).unwrap_or_else(|| "unknown".to_owned())
    } else {
        "disabled".to_owned()
    };

    let mut tools = BTreeMap::new();
    if config.fields.tool_versions {
        collect_tool_versions(&mut tools, TOOL_COMMANDS);
    }
    if config.fields.browser_versions {
        collect_tool_versions(&mut tools, BROWSER_COMMANDS);
    }
    if config.fields.agent_versions {
        collect_tool_versions(&mut tools, AGENT_COMMANDS);
    }

    let mut network = BTreeMap::new();
    if config.fields.network_profile == "raw" {
        network.insert(
            "profile".to_owned(),
            configured_label("OCC_NETWORK_PROFILE").unwrap_or_else(|| "raw".to_owned()),
        );
    } else {
        network.insert(
            "profile".to_owned(),
            configured_label("OCC_NETWORK_PROFILE").unwrap_or_else(|| "unknown".to_owned()),
        );
    }
    if config.fields.public_ip {
        network.insert(
            "public-ip".to_owned(),
            command_stdout(
                "curl",
                &["--max-time", "2", "-fsS", "https://api.ipify.org"],
            )
            .unwrap_or_else(|| "unavailable".to_owned()),
        );
    } else {
        network.insert("public-ip".to_owned(), "disabled".to_owned());
    }
    if config.fields.raw_ip_addr {
        network.insert(
            "ip-addr".to_owned(),
            command_stdout("ip", &["-brief", "addr"]).unwrap_or_else(|| "unavailable".to_owned()),
        );
        network.insert(
            "route".to_owned(),
            command_stdout("ip", &["route"]).unwrap_or_else(|| "unavailable".to_owned()),
        );
        network.insert(
            "dns".to_owned(),
            command_stdout("resolvectl", &["dns"]).unwrap_or_else(|| {
                std::fs::read_to_string("/etc/resolv.conf")
                    .map(|content| {
                        content
                            .lines()
                            .filter(|line| line.trim_start().starts_with("nameserver"))
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .unwrap_or_else(|_| "unavailable".to_owned())
            }),
        );
    } else {
        network.insert("ip-addr".to_owned(), "disabled".to_owned());
        network.insert(
            "mac-addresses".to_owned(),
            config.fields.mac_addresses.clone(),
        );
    }

    let mut security = BTreeMap::new();
    if config.fields.security_state {
        security.insert(
            "disk-encryption".to_owned(),
            command_stdout("lsblk", &["-o", "NAME,TYPE,FSTYPE,MOUNTPOINTS"])
                .unwrap_or_else(|| "unavailable".to_owned()),
        );
        security.insert(
            "secure-boot".to_owned(),
            command_stdout("mokutil", &["--sb-state"]).unwrap_or_else(|| "unavailable".to_owned()),
        );
        security.insert("tpm".to_owned(), path_exists_label("/sys/class/tpm/tpm0"));
        security.insert(
            "smart-card".to_owned(),
            command_stdout("ykman", &["list"]).unwrap_or_else(|| "unavailable".to_owned()),
        );
    } else {
        security.insert("security-state".to_owned(), "disabled".to_owned());
    }
    if config.fields.hardware_serial {
        security.insert(
            "machine-id".to_owned(),
            std::fs::read_to_string("/etc/machine-id")
                .map(|value| value.trim().to_owned())
                .unwrap_or_else(|_| "unavailable".to_owned()),
        );
        security.insert(
            "hardware-uuid".to_owned(),
            std::fs::read_to_string("/sys/class/dmi/id/product_uuid")
                .map(|value| value.trim().to_owned())
                .unwrap_or_else(|_| "unavailable".to_owned()),
        );
    } else {
        security.insert("hardware-serial".to_owned(), "disabled".to_owned());
    }

    Ok(EvidenceSnapshot {
        generated_at,
        profile: config.profile,
        redaction: config.redaction,
        repository: RepositorySnapshot {
            root: repo_root.display().to_string(),
            branch,
            head,
            index_tree,
            staged_files,
        },
        environment: EnvironmentSnapshot {
            developer,
            hostname,
            os: if config.fields.os {
                std::env::consts::OS.to_owned()
            } else {
                "disabled".to_owned()
            },
            arch: std::env::consts::ARCH.to_owned(),
            kernel,
        },
        tools,
        network,
        security,
    })
}

fn format_snapshot(snapshot: &EvidenceSnapshot, config: &EvidenceConfig) -> String {
    let mut lines = vec![
        "OpenCodeCommit evidence snapshot".to_owned(),
        format!("generated-at: {}", snapshot.generated_at),
        format!("profile: {}", snapshot.profile.label()),
        format!("redaction: {}", snapshot.redaction.label()),
        format!("storage: {}", config.storage.label()),
        format!("repo: {}", snapshot.repository.root),
        format!("branch: {}", snapshot.repository.branch),
        format!("head: {}", snapshot.repository.head),
        format!("index-tree: {}", snapshot.repository.index_tree),
        format!(
            "staged-files: {}",
            snapshot.repository.staged_files.join(", ")
        ),
        format!("developer: {}", snapshot.environment.developer),
        format!("hostname: {}", snapshot.environment.hostname),
        format!(
            "os: {} {} {}",
            snapshot.environment.os, snapshot.environment.kernel, snapshot.environment.arch
        ),
        "tools:".to_owned(),
    ];
    for (name, tool) in &snapshot.tools {
        lines.push(format!("  {name}: {}", tool.version));
    }
    lines.push("network:".to_owned());
    for (name, value) in &snapshot.network {
        lines.push(format!("  {name}: {}", value.replace('\n', "\\n")));
    }
    lines.push("security:".to_owned());
    for (name, value) in &snapshot.security {
        lines.push(format!("  {name}: {}", value.replace('\n', "\\n")));
    }
    if config.profile == EvidenceProfile::Defence {
        lines.push(String::new());
        lines.push(DEFENCE_WARNING.to_owned());
    }
    lines.join("\n")
}

fn append_pending_assisted_by(repo_root: &Path, message: &str) -> Result<String> {
    let paths = resolve_paths_for_repo(repo_root)?;
    let Some(token) = load_assist_token(&paths.assist_next_path)? else {
        return Ok(message.to_owned());
    };
    if token.kind != "assisted-by" || now_unix_secs() > token.expires_at_unix {
        let _ = std::fs::remove_file(&paths.assist_next_path);
        return Ok(message.to_owned());
    }
    let index_tree = git::write_index_tree(repo_root)?;
    if token.index_tree != index_tree {
        return Ok(message.to_owned());
    }
    let _ = std::fs::remove_file(&paths.assist_next_path);
    Ok(append_trailers(message, &token.trailers))
}

fn append_trailers(message: &str, trailers: &[String]) -> String {
    trailers
        .iter()
        .fold(message.to_owned(), |current, trailer| {
            append_trailer(&current, trailer, true)
        })
}

fn append_trailer(message: &str, trailer: &str, blank_before_first: bool) -> String {
    if message.lines().any(|line| line.trim() == trailer) {
        return message.to_owned();
    }
    let mut result = message.trim_end().to_owned();
    if result.is_empty() {
        return format!("{trailer}\n");
    }
    if blank_before_first && !ends_with_trailer(&result) {
        result.push_str("\n\n");
    } else {
        result.push('\n');
    }
    result.push_str(trailer);
    result.push('\n');
    result
}

fn ends_with_trailer(message: &str) -> bool {
    const TRAILER_PREFIXES: [&str; 3] = ["Assisted-by:", "OCC-Evidence:", "Co-authored-by:"];
    message
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .map(|line| {
            let trimmed = line.trim_start();
            TRAILER_PREFIXES
                .iter()
                .any(|prefix| trimmed.starts_with(prefix))
        })
        .unwrap_or(false)
}

fn has_trailer(message: &str, prefix: &str) -> bool {
    message
        .lines()
        .any(|line| line.trim_start().starts_with(prefix))
}

fn assisted_by_trailer(input: &AssistedByInput) -> String {
    let agent = input.agent.trim();
    let model = input.model.trim();
    match input
        .version
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(version) => format!("Assisted-by: {agent} {version}:{model}"),
        None => format!("Assisted-by: {agent}:{model}"),
    }
}

fn scan_sidecar_content(content: &str, path: &Path) -> Result<()> {
    let actual_display = path.display().to_string();
    let display = "occ-evidence-sidecar.toml";
    let mut diff = format!(
        "diff --git a/{display} b/{display}\n--- /dev/null\n+++ b/{display}\n@@ -0,0 +1,{} @@\n",
        content.lines().count()
    );
    for line in content.lines() {
        diff.push('+');
        diff.push_str(line);
        diff.push('\n');
    }
    let report = scan_diff_for_sensitive_content_with_options(
        &diff,
        &[display.to_owned()],
        SensitiveEnforcement::BlockHigh,
        &[],
    );
    if report.has_blocking_findings() {
        return Err(EvidenceError::Invalid(format!(
            "evidence sidecar {} blocked by sensitive-content scanner:\n{}",
            actual_display,
            report.format_git_hook_message()
        )));
    }
    Ok(())
}

fn resolve_paths() -> Result<EvidencePaths> {
    let repo_root = git::get_repo_root()?;
    resolve_paths_for_repo(&repo_root)
}

fn resolve_paths_for_repo(repo_root: &Path) -> Result<EvidencePaths> {
    let git_dir = git::get_git_dir(repo_root)?;
    let occ_dir = git_dir.join("occ");
    Ok(EvidencePaths {
        repo_root: repo_root.to_path_buf(),
        config_path: occ_dir.join(EVIDENCE_CONFIG),
        assist_next_path: occ_dir.join(ASSIST_NEXT),
        occ_dir,
        git_dir,
    })
}

fn load_config_from(path: &Path) -> Result<Option<EvidenceConfig>> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.into()),
    };
    toml::from_str(&content)
        .map(Some)
        .map_err(|err| EvidenceError::Invalid(format!("failed to parse {}: {err}", path.display())))
}

fn save_config_to(path: &Path, config: &EvidenceConfig) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(config).map_err(|err| {
        EvidenceError::Invalid(format!("failed to serialize evidence config: {err}"))
    })?;
    std::fs::write(path, content)?;
    Ok(())
}

fn load_assist_token(path: &Path) -> Result<Option<AssistNextToken>> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.into()),
    };
    toml::from_str(&content)
        .map(Some)
        .map_err(|err| EvidenceError::Invalid(format!("failed to parse {}: {err}", path.display())))
}

fn save_assist_token(path: &Path, token: &AssistNextToken) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = toml::to_string(token).map_err(|err| {
        EvidenceError::Invalid(format!("failed to serialize assist token: {err}"))
    })?;
    std::fs::write(path, content)?;
    Ok(())
}

fn sidecar_root(paths: &EvidencePaths, config: &EvidenceConfig) -> PathBuf {
    match config.storage {
        EvidenceStorage::Local => paths.occ_dir.join("evidence"),
        EvidenceStorage::Repo => paths.repo_root.join(".occ").join("evidence"),
        EvidenceStorage::Artifact => paths.occ_dir.join("evidence").join("artifacts"),
    }
}

fn display_local_pointer(paths: &EvidencePaths, path: &Path) -> String {
    let default_git = paths.repo_root.join(".git");
    if same_path(&paths.git_dir, &default_git)
        && let Ok(rel) = path.strip_prefix(&paths.repo_root)
    {
        return rel.display().to_string();
    }
    path.display().to_string()
}

fn stage_repo_sidecar(repo_root: &Path, repo_rel: &Path) -> Result<()> {
    let status = Command::new("git")
        .args(["add", "--"])
        .arg(repo_rel)
        .current_dir(repo_root)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(EvidenceError::Invalid(format!(
            "failed to stage evidence sidecar {}",
            repo_rel.display()
        )))
    }
}

fn default_assisted_by_config() -> AssistedByConfig {
    let catalog = model_catalog().assisted_by;
    AssistedByConfig {
        enabled: true,
        prompt: "ask".to_owned(),
        dedupe: true,
        harnesses: catalog.harnesses,
        models: catalog.models,
        quick: catalog
            .quick
            .into_iter()
            .map(|option| AssistedByQuickOption {
                label: option.label,
                agent: option.agent,
                model: option.model,
                version_command: option.version_command,
                version_pattern: option
                    .version_pattern
                    .replace("(?<version>", "(?P<version>"),
            })
            .collect(),
    }
}

fn model_catalog() -> ModelCatalog {
    serde_json::from_str(include_str!("../model-catalog.json"))
        .expect("model-catalog.json must be valid")
}

fn detect_version_for_option(option: &AssistedByQuickOption) -> std::result::Result<String, ()> {
    if option.version_command.trim().is_empty() {
        return Err(());
    }
    let mut parts = option.version_command.split_whitespace();
    let command = parts.next().ok_or(())?;
    let args = parts.collect::<Vec<_>>();
    let output = command_stdout_maybe_host(command, &args).ok_or(())?;
    if option.version_pattern.trim().is_empty() {
        return Ok(output);
    }
    let regex = regex::Regex::new(&option.version_pattern).map_err(|_| ())?;
    regex
        .captures(&output)
        .and_then(|captures| {
            captures
                .name("version")
                .map(|value| value.as_str().to_owned())
        })
        .ok_or(())
}

const TOOL_COMMANDS: &[(&str, &str, &[&str])] = &[
    ("git", "git", &["--version"]),
    ("occ", "occ", &["--version"]),
    ("rustc", "rustc", &["--version"]),
    ("cargo", "cargo", &["--version"]),
    ("node", "node", &["--version"]),
    ("bun", "bun", &["--version"]),
    ("pnpm", "pnpm", &["--version"]),
    ("python3", "python3", &["--version"]),
    ("docker", "docker", &["--version"]),
    ("docker-compose", "docker", &["compose", "version"]),
    ("java", "java", &["--version"]),
    ("gradle", "gradle", &["--version"]),
];

const BROWSER_COMMANDS: &[(&str, &str, &[&str])] = &[
    ("google-chrome", "google-chrome", &["--version"]),
    ("chromium", "chromium", &["--version"]),
    ("edge", "microsoft-edge", &["--version"]),
    ("firefox", "firefox", &["--version"]),
];

const AGENT_COMMANDS: &[(&str, &str, &[&str])] = &[
    ("codex", "codex", &["-V"]),
    ("claude", "claude", &["-v"]),
    ("opencode", "opencode", &["--version"]),
    ("gemini", "gemini", &["--version"]),
    ("antigravity", "antigravity", &["--version"]),
];

fn collect_tool_versions(
    tools: &mut BTreeMap<String, ToolSnapshot>,
    commands: &[(&str, &str, &[&str])],
) {
    for (name, command, args) in commands {
        let version = command_stdout(command, args).unwrap_or_else(|| "not-installed".to_owned());
        tools.insert(
            (*name).to_owned(),
            ToolSnapshot {
                command: std::iter::once(*command)
                    .chain(args.iter().copied())
                    .collect::<Vec<_>>()
                    .join(" "),
                version,
            },
        );
    }
}

fn command_stdout(command: &str, args: &[&str]) -> Option<String> {
    command_stdout_in(Path::new("."), command, args)
}

fn command_stdout_in(cwd: &Path, command: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(command)
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if !stdout.is_empty() {
        return Some(stdout);
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    (!stderr.is_empty()).then_some(stderr)
}

fn is_flatpak() -> bool {
    Path::new("/.flatpak-info").exists()
}

/// Run a version command, escaping to the host under a Flatpak sandbox so host
/// CLIs (e.g. `claude`/`codex`) resolve via the user's shell PATH. Mirrors the
/// VS Code extension's flatpak-spawn handling (evidence.ts) so the Assisted-by
/// trailer keeps the harness version inside sandboxed editors.
fn command_stdout_maybe_host(command: &str, args: &[&str]) -> Option<String> {
    if is_flatpak() {
        let script = format!(
            "source ~/.zshrc 2>/dev/null || source ~/.bashrc 2>/dev/null || true; {} {}",
            command,
            args.join(" ")
        );
        command_stdout("flatpak-spawn", &["--host", "bash", "-c", script.as_str()])
    } else {
        command_stdout(command, args)
    }
}

fn configured_label(var: &str) -> Option<String> {
    std::env::var(var)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn path_exists_label(path: &str) -> String {
    if Path::new(path).exists() {
        "present".to_owned()
    } else {
        "absent".to_owned()
    }
}

fn file_sha256(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(hex(&hasher.finalize()))
}

fn short_hash(value: &str) -> String {
    value.chars().take(7).collect()
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn same_path(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn assisted_by_trailer_uses_requested_format() {
        let trailer = assisted_by_trailer(&AssistedByInput {
            agent: "Codex".to_owned(),
            model: "GPT-5.5".to_owned(),
            version: Some("0.133.0".to_owned()),
        });
        assert_eq!(trailer, "Assisted-by: Codex 0.133.0:GPT-5.5");
    }

    #[test]
    fn default_assisted_by_config_uses_catalog_slugs_and_codex_first() {
        let config = default_assisted_by_config();

        assert_eq!(config.harnesses[0], "Codex");
        assert_eq!(config.harnesses[1], "Claude Code");
        assert!(config.harnesses.contains(&"GitHub Copilot".to_owned()));
        assert!(config.models.contains(&"claude-opus-4.8".to_owned()));
        assert!(config.models.contains(&"gpt-5.5-pro".to_owned()));
        assert!(config.models.contains(&"openai/gpt-5.5-pro".to_owned()));
        assert!(config.models.contains(&"gpt-5.6-sol".to_owned()));
        assert!(config.models.contains(&"gpt-5.6-terra".to_owned()));
        assert!(config.models.contains(&"gpt-5.6-luna".to_owned()));
        assert!(config.models.contains(&"composer-2.5".to_owned()));
        assert!(config.models.contains(&"big-pickle".to_owned()));
        assert!(!config.models.contains(&"opus-4.8".to_owned()));
        assert!(
            !config
                .models
                .contains(&"anthropic/claude-opus-4.8".to_owned())
        );
        assert_eq!(
            config
                .quick
                .iter()
                .map(|option| option.label.as_str())
                .collect::<Vec<_>>(),
            vec!["GPT", "Sol", "Terra", "Opus", "Fable"]
        );
        assert_eq!(
            config
                .quick
                .iter()
                .find(|option| option.label == "Sol")
                .map(|option| option.model.as_str()),
            Some("gpt-5.6-sol")
        );
        assert_eq!(
            config
                .quick
                .iter()
                .find(|option| option.label == "Terra")
                .map(|option| option.model.as_str()),
            Some("gpt-5.6-terra")
        );
        assert_eq!(
            config
                .quick
                .iter()
                .find(|option| option.label == "Opus")
                .map(|option| option.model.as_str()),
            Some("claude-opus-4.8")
        );
        assert_eq!(
            config
                .quick
                .iter()
                .find(|option| option.label == "Fable")
                .map(|option| option.model.as_str()),
            Some("claude-fable-5.0")
        );
    }

    #[test]
    fn quick_option_label_matching_accepts_legacy_labels() {
        assert!(quick_option_label_matches("GPT", "Codex GPT"));
        assert!(quick_option_label_matches("Claude Code Opus", "Opus"));
        assert!(quick_option_label_matches("Fable", "Claude Code Fable"));
        assert!(!quick_option_label_matches("Fable", "Opus"));
    }

    #[test]
    fn append_trailer_keeps_message_body_compact() {
        let message = "feat(auth): rotate sessions\n\nBody.";
        let next = append_trailer(
            message,
            "OCC-Evidence: local:.git/occ/evidence/x.toml",
            true,
        );
        assert!(next.contains("\n\nOCC-Evidence:"));
        let again = append_trailer(&next, "OCC-Evidence: local:.git/occ/evidence/x.toml", true);
        assert_eq!(next, again);
    }

    #[test]
    fn assisted_by_trailers_get_one_blank_line_before_block() {
        let out = append_trailers(
            "feat: x\n\nBody.",
            &[
                "Assisted-by: Codex 0.133.0:gpt-5.5".to_owned(),
                "Assisted-by: Claude Code 2.1.0:claude-opus-4.8".to_owned(),
            ],
        );
        assert_eq!(
            out,
            "feat: x\n\nBody.\n\nAssisted-by: Codex 0.133.0:gpt-5.5\nAssisted-by: Claude Code 2.1.0:claude-opus-4.8\n"
        );
    }

    #[test]
    fn assisted_by_blank_line_after_oneliner_subject() {
        let out = append_trailers(
            "fix: rename foo",
            &["Assisted-by: Codex 0.133.0:gpt-5.5".to_owned()],
        );
        assert_eq!(
            out,
            "fix: rename foo\n\nAssisted-by: Codex 0.133.0:gpt-5.5\n"
        );
    }

    #[test]
    fn install_blocks_defence_repo_without_acknowledgement() {
        let repo = setup_repo("defence-ack");
        with_repo(&repo, || {
            let err = install(EvidenceProfile::Defence, Some(EvidenceStorage::Repo), false)
                .unwrap_err()
                .to_string();
            assert!(err.contains("--allow-cleartext-repo-evidence"));
        });
        let _ = fs::remove_dir_all(repo);
    }

    #[test]
    fn install_status_uninstall_roundtrip() {
        let repo = setup_repo("roundtrip");
        with_repo(&repo, || {
            install(EvidenceProfile::Samd, Some(EvidenceStorage::Local), false).unwrap();
            let status = status().unwrap();
            assert!(status.contains("OpenCodeCommit evidence: installed"));
            assert!(status.contains("profile: samd"));
            assert!(status.contains("storage: local"));
            assert!(uninstall().unwrap().contains("uninstalled"));
        });
        let _ = fs::remove_dir_all(repo);
    }

    #[test]
    fn append_to_commit_message_writes_local_sidecar_pointer() {
        let repo = setup_repo("sidecar");
        with_repo(&repo, || {
            install(EvidenceProfile::Samd, Some(EvidenceStorage::Local), false).unwrap();
            fs::write("file.txt", "content").unwrap();
            Command::new("git")
                .args(["add", "file.txt"])
                .output()
                .unwrap();

            let message = append_to_commit_message(&repo, "feat: add file").unwrap();
            assert!(message.contains("OCC-Evidence: local:"));
            assert!(message.contains(".git/occ/evidence/"));
        });
        let _ = fs::remove_dir_all(repo);
    }

    fn setup_repo(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("occ-evidence-repo-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Command::new("git")
            .arg("init")
            .current_dir(&dir)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(&dir)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(&dir)
            .output()
            .unwrap();
        fs::write(dir.join("README.md"), "# Hello").unwrap();
        Command::new("git")
            .args(["add", "README.md"])
            .current_dir(&dir)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial commit"])
            .current_dir(&dir)
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@test.com")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@test.com")
            .output()
            .unwrap();
        dir
    }

    fn with_repo<T>(repo: &Path, f: impl FnOnce() -> T) -> T {
        let _lock = opencodecommit::TEST_CWD_LOCK.lock().unwrap();
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(repo).unwrap();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        std::env::set_current_dir(original).unwrap();
        match result {
            Ok(value) => value,
            Err(payload) => std::panic::resume_unwind(payload),
        }
    }
}
