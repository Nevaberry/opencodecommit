use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use crate::codex_home;
use crate::config::{CliBackend, Config};
use crate::{Error, Result};

static CACHED_PATHS: LazyLock<Mutex<HashMap<CliBackend, PathBuf>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

const TIMEOUT_SECS: u64 = 120;

/// Labels for error messages.
fn backend_label(backend: CliBackend) -> &'static str {
    match backend {
        CliBackend::Opencode => "OpenCode CLI",
        CliBackend::Claude => "Claude Code CLI",
        CliBackend::Codex => "Codex CLI",
        CliBackend::Gemini => "Gemini CLI",
    }
}

/// Binary name for a backend.
fn backend_binary(backend: CliBackend) -> &'static str {
    match backend {
        CliBackend::Opencode => "opencode",
        CliBackend::Claude => "claude",
        CliBackend::Codex => "codex",
        CliBackend::Gemini => "gemini",
    }
}

fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.is_file()
            && std::fs::metadata(path)
                .map(|m| m.permissions().mode() & 0o111 != 0)
                .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

fn run_which(binary: &str) -> Option<PathBuf> {
    let cmd = if cfg!(target_os = "windows") {
        "where"
    } else {
        "which"
    };
    let output = Command::new(cmd)
        .arg(binary)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;

    if output.status.success() {
        let result = String::from_utf8_lossy(&output.stdout);
        let first_line = result.lines().next()?.trim();
        if !first_line.is_empty() {
            return Some(PathBuf::from(first_line));
        }
    }
    None
}

fn run_shell_source_which(binary: &str) -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        return None;
    }
    let output = Command::new("bash")
        .args([
            "-c",
            &format!(
                "source ~/.zshrc 2>/dev/null || source ~/.bashrc 2>/dev/null || true; which {binary}"
            ),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;

    if output.status.success() {
        let result = String::from_utf8_lossy(&output.stdout);
        let last_line = result.lines().next_back()?.trim();
        if !last_line.is_empty() {
            return Some(PathBuf::from(last_line));
        }
    }
    None
}

fn common_paths(binary: &str) -> Vec<PathBuf> {
    let home = std::env::var("HOME").unwrap_or_default();

    if cfg!(target_os = "windows") {
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
        return vec![
            PathBuf::from(format!("{appdata}/npm/{binary}.cmd")),
            PathBuf::from(format!("{local}/npm/{binary}.cmd")),
        ];
    }

    vec![
        PathBuf::from(format!("/usr/local/bin/{binary}")),
        PathBuf::from(format!("/usr/bin/{binary}")),
        PathBuf::from(format!("{home}/.local/bin/{binary}")),
        PathBuf::from(format!("{home}/bin/{binary}")),
        PathBuf::from(format!("/opt/homebrew/bin/{binary}")),
    ]
}

#[cfg(target_os = "windows")]
fn run_wsl_which(binary: &str) -> Option<String> {
    let output = Command::new("wsl")
        .args(["which", binary])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;

    if output.status.success() {
        let result = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if !result.is_empty() {
            return Some(format!("wsl {binary}"));
        }
    }
    None
}

/// Detect the CLI binary path using a 6-step cascade.
pub fn detect_cli(backend: CliBackend, config_path: &str) -> Result<PathBuf> {
    let binary = backend_binary(backend);
    let label = backend_label(backend);

    // 1. User-configured path
    if !config_path.is_empty() {
        let p = PathBuf::from(config_path);
        if is_executable(&p) {
            return Ok(p);
        }
        return Err(Error::BackendNotFound(format!(
            "configured {label} path is not executable: {config_path}"
        )));
    }

    // 2. Cached path
    if let Ok(cache) = CACHED_PATHS.lock()
        && let Some(p) = cache.get(&backend)
        && is_executable(p)
    {
        return Ok(p.to_path_buf());
    }

    // 3. which / where
    if let Some(p) = run_which(binary)
        && is_executable(&p)
    {
        if let Ok(mut cache) = CACHED_PATHS.lock() {
            cache.insert(backend, p.clone());
        }
        return Ok(p);
    }

    // 4. Shell profile sourcing (Unix)
    if let Some(p) = run_shell_source_which(binary)
        && is_executable(&p)
    {
        if let Ok(mut cache) = CACHED_PATHS.lock() {
            cache.insert(backend, p.clone());
        }
        return Ok(p);
    }

    // 5. Common paths
    for p in common_paths(binary) {
        if is_executable(&p) {
            if let Ok(mut cache) = CACHED_PATHS.lock() {
                cache.insert(backend, p.clone());
            }
            return Ok(p);
        }
    }

    // 6. WSL fallback (Windows only)
    #[cfg(target_os = "windows")]
    if let Some(wsl_cmd) = run_wsl_which(binary) {
        let p = PathBuf::from(&wsl_cmd);
        if let Ok(mut cache) = CACHED_PATHS.lock() {
            cache.insert(backend, p.clone());
        }
        return Ok(p);
    }

    Err(Error::BackendNotFound(format!(
        "{label} CLI not found — install it or set the path in config"
    )))
}

/// Invocation describes how to call a backend.
pub struct Invocation {
    pub command: PathBuf,
    pub args: Vec<String>,
    pub stdin: Option<String>,
    /// Environment variables to set on the child process. Ordered for test
    /// determinism. Empty for backends that inherit the parent environment.
    pub env: Vec<(String, String)>,
}

/// Build the command invocation for a given backend.
pub fn build_invocation(cli_path: &Path, prompt: &str, config: &Config) -> Invocation {
    let backend = config.backend.cli_backend().unwrap_or(CliBackend::Opencode);
    build_invocation_for(cli_path, prompt, config, backend)
}

/// Build the command invocation for a specific backend (used in failover).
pub fn build_invocation_for(
    cli_path: &Path,
    prompt: &str,
    config: &Config,
    backend: CliBackend,
) -> Invocation {
    match backend {
        CliBackend::Opencode => {
            let model_spec = format!("{}/{}", config.provider, config.model);
            Invocation {
                command: cli_path.to_owned(),
                args: opencode_base_args(&model_spec, prompt),
                stdin: None,
                env: vec![],
            }
        }
        CliBackend::Claude => Invocation {
            command: cli_path.to_owned(),
            args: vec![
                "-p".to_owned(),
                "--model".to_owned(),
                config.claude_model.clone(),
                "--output-format".to_owned(),
                "text".to_owned(),
                "--max-turns".to_owned(),
                "1".to_owned(),
            ],
            stdin: Some(prompt.to_owned()),
            env: vec![],
        },
        CliBackend::Codex => {
            let mut args = codex_base_args(&config.codex_model);
            if !config.codex_provider.is_empty() {
                args.push("-c".to_owned());
                args.push(format!("model_provider=\"{}\"", config.codex_provider));
            }
            args.push("-".to_owned());
            Invocation {
                command: cli_path.to_owned(),
                args,
                stdin: Some(prompt.to_owned()),
                env: codex_env(),
            }
        }
        CliBackend::Gemini => {
            let mut args = vec!["-p".to_owned(), prompt.to_owned()];
            if !config.gemini_model.is_empty() {
                args.push("-m".to_owned());
                args.push(config.gemini_model.clone());
            }
            args.push("--output-format".to_owned());
            args.push("text".to_owned());
            Invocation {
                command: cli_path.to_owned(),
                args,
                stdin: None,
                env: vec![],
            }
        }
    }
}

/// Build the environment overrides for a `codex exec` invocation. Points
/// `CODEX_HOME` at an occ-managed minimal directory so codex doesn't parse
/// the user's accumulated state on every call. Returns empty on any failure;
/// callers then inherit the parent environment (i.e., the user's real
/// `~/.codex`), matching pre-1.6 behaviour.
fn codex_env() -> Vec<(String, String)> {
    match codex_home::ensure_minimal_codex_home() {
        Some(path) => vec![(
            "CODEX_HOME".to_owned(),
            path.to_string_lossy().into_owned(),
        )],
        None => vec![],
    }
}

/// Flags that are always safe and always cheap for `codex exec`, regardless
/// of whether the task is a commit or a PR synthesis:
///   --ephemeral                  no session files on disk
///   --skip-git-repo-check        avoid the repo-detection round trip
///   -s read-only                 sandbox
///   --dangerously-bypass-*       skip interactive approvals
///   --disable plugins            skip loading user-configured codex plugins
///   -c mcp_servers={}            belt-and-braces: never spawn MCP servers
///                                even if CODEX_HOME falls back to the user's
///                                real home with its own MCP registry
fn codex_common_args(model: &str) -> Vec<String> {
    vec![
        "exec".to_owned(),
        "--ephemeral".to_owned(),
        "--skip-git-repo-check".to_owned(),
        "-s".to_owned(),
        "read-only".to_owned(),
        "--dangerously-bypass-approvals-and-sandbox".to_owned(),
        "--disable".to_owned(),
        "plugins".to_owned(),
        "-c".to_owned(),
        "mcp_servers={}".to_owned(),
        "-m".to_owned(),
        model.to_owned(),
    ]
}

/// `codex exec` argv for the fast commit-generation path. Adds:
///   --disable apps                     — skip loading external "apps"
///                                        integrations we don't use for commit
///                                        generation (consistently ~0.5–1 s off
///                                        cold invocations in benchmarking)
///   -c model_reasoning_effort="none"   — commits don't need reasoning
///   -c web_search="disabled"           — remove the web_search tool
///                                        (required for reasoning < low, and
///                                        trims tool preamble tokens)
fn codex_base_args(model: &str) -> Vec<String> {
    let mut args = codex_common_args(model);
    args.push("--disable".to_owned());
    args.push("apps".to_owned());
    args.push("-c".to_owned());
    args.push("model_reasoning_effort=\"none\"".to_owned());
    args.push("-c".to_owned());
    args.push("web_search=\"disabled\"".to_owned());
    args
}

/// `opencode run` argv shared by commit and PR paths. The prompt is the last
/// positional argument because opencode reads it from argv, not stdin.
fn opencode_common_args(model_spec: &str, prompt: &str) -> Vec<String> {
    vec![
        "run".to_owned(),
        "-m".to_owned(),
        model_spec.to_owned(),
        prompt.to_owned(),
    ]
}

/// `opencode run` argv for the fast commit-generation path. Adds
/// `--variant minimal` (provider-specific minimal reasoning effort), which is
/// the one flag that measurably reduces wall time for short, templated tasks
/// like commit-message generation. Not applied to the PR path, where
/// synthesis benefits from the provider's default reasoning budget.
fn opencode_base_args(model_spec: &str, prompt: &str) -> Vec<String> {
    let mut args = opencode_common_args(model_spec, prompt);
    // Insert "--variant minimal" between "run" and "-m" so the prompt stays
    // as the final positional argument.
    args.insert(1, "minimal".to_owned());
    args.insert(1, "--variant".to_owned());
    args
}

/// Build the command invocation with explicit model and provider overrides.
/// Used by the two-stage PR pipeline to invoke different models for each stage.
pub fn build_invocation_with_model(
    cli_path: &Path,
    prompt: &str,
    config: &Config,
    model: &str,
    provider: Option<&str>,
) -> Invocation {
    let backend = config.backend.cli_backend().unwrap_or(CliBackend::Opencode);
    build_invocation_with_model_for(cli_path, prompt, config, backend, model, provider)
}

pub fn build_invocation_with_model_for(
    cli_path: &Path,
    prompt: &str,
    config: &Config,
    backend: CliBackend,
    model: &str,
    provider: Option<&str>,
) -> Invocation {
    match backend {
        CliBackend::Opencode => {
            let prov = provider.unwrap_or(&config.provider);
            let model_spec = format!("{prov}/{model}");
            // PR stages intentionally omit --variant minimal; summary/final
            // synthesis benefits from the full reasoning budget assigned by
            // the user's provider defaults.
            Invocation {
                command: cli_path.to_owned(),
                args: opencode_common_args(&model_spec, prompt),
                stdin: None,
                env: vec![],
            }
        }
        CliBackend::Claude => Invocation {
            command: cli_path.to_owned(),
            args: vec![
                "-p".to_owned(),
                "--model".to_owned(),
                model.to_owned(),
                "--output-format".to_owned(),
                "text".to_owned(),
                "--max-turns".to_owned(),
                "1".to_owned(),
            ],
            stdin: Some(prompt.to_owned()),
            env: vec![],
        },
        CliBackend::Codex => {
            // PR stages (summary + final) keep whatever reasoning_effort and
            // web_search the user configured in ~/.codex/config.toml, so PR
            // quality is preserved. We still apply the cheap flags
            // (--ephemeral, --skip-git-repo-check, --disable plugins) because
            // they cost nothing and never affect output quality.
            let mut args = codex_common_args(model);
            let prov = provider.unwrap_or(if !config.codex_provider.is_empty() {
                &config.codex_provider
            } else {
                ""
            });
            if !prov.is_empty() {
                args.push("-c".to_owned());
                args.push(format!("model_provider=\"{prov}\""));
            }
            args.push("-".to_owned());
            Invocation {
                command: cli_path.to_owned(),
                args,
                stdin: Some(prompt.to_owned()),
                env: codex_env(),
            }
        }
        CliBackend::Gemini => {
            let mut args = vec!["-p".to_owned(), prompt.to_owned()];
            if !model.is_empty() {
                args.push("-m".to_owned());
                args.push(model.to_owned());
            }
            args.push("--output-format".to_owned());
            args.push("text".to_owned());
            Invocation {
                command: cli_path.to_owned(),
                args,
                stdin: None,
                env: vec![],
            }
        }
    }
}

/// Strip ANSI escape codes from text.
pub fn strip_ansi(text: &str) -> String {
    static RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap());
    RE.replace_all(text, "").to_string()
}

/// Execute a CLI invocation and return stdout, with a configurable timeout.
pub fn exec_cli_with_timeout(invocation: &Invocation, timeout_secs: u64) -> Result<String> {
    let mut cmd = Command::new(&invocation.command);
    cmd.args(&invocation.args);
    cmd.envs(
        invocation
            .env
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str())),
    );
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    if invocation.stdin.is_some() {
        cmd.stdin(Stdio::piped());
    } else {
        cmd.stdin(Stdio::null());
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| Error::BackendExecution(format!("failed to start CLI: {e}")))?;

    // Write stdin if needed
    if let Some(ref input) = invocation.stdin
        && let Some(mut stdin) = child.stdin.take()
    {
        let _ = stdin.write_all(input.as_bytes());
        // stdin is dropped here, closing the pipe
    }

    // Wait with timeout
    let timeout = Duration::from_secs(timeout_secs);
    let start = std::time::Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = child
                    .stdout
                    .take()
                    .map(|s| std::io::read_to_string(s).unwrap_or_default())
                    .unwrap_or_default();
                let stderr = child
                    .stderr
                    .take()
                    .map(|s| std::io::read_to_string(s).unwrap_or_default())
                    .unwrap_or_default();

                if status.success() {
                    return Ok(strip_ansi(stdout.trim()));
                } else {
                    return Err(Error::BackendExecution(format!(
                        "CLI exited with code {}: {}",
                        status.code().unwrap_or(-1),
                        strip_ansi(stderr.trim())
                    )));
                }
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(Error::BackendTimeout(timeout_secs));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => {
                return Err(Error::BackendExecution(format!(
                    "failed to wait on CLI: {e}"
                )));
            }
        }
    }
}

/// Execute a CLI invocation and return stdout (default 120s timeout).
pub fn exec_cli(invocation: &Invocation) -> Result<String> {
    exec_cli_with_timeout(invocation, TIMEOUT_SECS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Backend;

    /// Codex env is either empty (ensure_minimal_codex_home fell back because
    /// no ~/.codex/auth.json is available, e.g., on CI) or a single entry
    /// `("CODEX_HOME", <non-empty path>)` — and nothing else must leak.
    fn assert_codex_env_shape(env: &[(String, String)]) {
        match env {
            [] => {}
            [(k, v)] => {
                assert_eq!(k, "CODEX_HOME");
                assert!(!v.is_empty(), "CODEX_HOME path must not be empty");
            }
            other => panic!("unexpected codex env: {other:?}"),
        }
    }

    #[test]
    fn build_invocation_opencode() {
        let config = Config {
            backend: Backend::Opencode,
            provider: "openai".to_owned(),
            model: "gpt-5.4-mini".to_owned(),
            ..Config::default()
        };
        let inv = build_invocation(Path::new("/usr/bin/opencode"), "hello", &config);
        // Fast-path flags: --variant minimal on the commit path.
        assert_eq!(inv.args[0], "run");
        assert!(inv.args.contains(&"--variant".to_owned()));
        let variant_idx = inv
            .args
            .iter()
            .position(|a| a == "--variant")
            .expect("--variant present");
        assert_eq!(inv.args.get(variant_idx + 1).map(String::as_str), Some("minimal"));
        assert!(inv.args.contains(&"-m".to_owned()));
        assert!(inv.args.contains(&"openai/gpt-5.4-mini".to_owned()));
        // Prompt must be the final positional arg (opencode reads it from argv).
        assert_eq!(inv.args.last().map(String::as_str), Some("hello"));
        assert!(inv.stdin.is_none());
        assert!(inv.env.is_empty(), "opencode must not leak CODEX_HOME");
    }

    #[test]
    fn build_invocation_claude() {
        let config = Config {
            backend: Backend::Claude,
            claude_model: "claude-sonnet-4-6".to_owned(),
            ..Config::default()
        };
        let inv = build_invocation(Path::new("/usr/bin/claude"), "hello", &config);
        assert_eq!(inv.args[0], "-p");
        assert_eq!(inv.args[1], "--model");
        assert_eq!(inv.args[2], "claude-sonnet-4-6");
        assert_eq!(inv.args[3], "--output-format");
        assert_eq!(inv.args[4], "text");
        assert_eq!(inv.args[5], "--max-turns");
        assert_eq!(inv.args[6], "1");
        assert_eq!(inv.stdin.as_deref(), Some("hello"));
        assert!(inv.env.is_empty(), "claude must not leak CODEX_HOME");
    }

    #[test]
    fn build_invocation_codex() {
        let config = Config {
            backend: Backend::Codex,
            codex_model: "gpt-5.4-mini".to_owned(),
            ..Config::default()
        };
        let inv = build_invocation(Path::new("/usr/bin/codex"), "hello", &config);
        // Fast-path flags: no reasoning, no web_search tool, no plugins, no apps.
        assert_eq!(inv.args[0], "exec");
        assert!(inv.args.contains(&"--ephemeral".to_owned()));
        assert!(inv.args.contains(&"--skip-git-repo-check".to_owned()));
        assert!(
            inv.args
                .contains(&"--dangerously-bypass-approvals-and-sandbox".to_owned())
        );
        assert!(
            inv.args
                .contains(&"model_reasoning_effort=\"none\"".to_owned())
        );
        assert!(inv.args.contains(&"web_search=\"disabled\"".to_owned()));
        // Belt-and-braces: mcp_servers override is present even if CODEX_HOME
        // setup fell back to the user's real home.
        assert!(inv.args.contains(&"mcp_servers={}".to_owned()));
        // Each --disable has its feature name as a separate positional argument.
        let disables: Vec<&str> = inv
            .args
            .iter()
            .enumerate()
            .filter_map(|(i, a)| {
                if a == "--disable" {
                    inv.args.get(i + 1).map(String::as_str)
                } else {
                    None
                }
            })
            .collect();
        assert!(disables.contains(&"plugins"));
        assert!(disables.contains(&"apps"));
        assert!(inv.args.contains(&"-m".to_owned()));
        assert!(inv.args.contains(&"gpt-5.4-mini".to_owned()));
        assert_eq!(inv.args.last().map(String::as_str), Some("-"));
        assert_eq!(inv.stdin.as_deref(), Some("hello"));
        assert_codex_env_shape(&inv.env);
    }

    #[test]
    fn build_invocation_codex_with_provider() {
        let config = Config {
            backend: Backend::Codex,
            codex_model: "gpt-5.4-mini".to_owned(),
            codex_provider: "openrouter".to_owned(),
            ..Config::default()
        };
        let inv = build_invocation(Path::new("/usr/bin/codex"), "hello", &config);
        assert!(inv.args.contains(&"-c".to_owned()));
        assert!(
            inv.args
                .contains(&"model_provider=\"openrouter\"".to_owned())
        );
        assert!(inv.args.contains(&"mcp_servers={}".to_owned()));
        assert_codex_env_shape(&inv.env);
    }

    #[test]
    fn build_invocation_gemini() {
        let config = Config {
            backend: Backend::Gemini,
            gemini_model: "gemini-2.5-flash".to_owned(),
            ..Config::default()
        };
        let inv = build_invocation(Path::new("/usr/bin/gemini"), "hello", &config);
        assert_eq!(inv.args[0], "-p");
        assert_eq!(inv.args[1], "hello");
        assert_eq!(inv.args[2], "-m");
        assert_eq!(inv.args[3], "gemini-2.5-flash");
        assert_eq!(inv.args[4], "--output-format");
        assert_eq!(inv.args[5], "text");
        assert_eq!(inv.stdin, None);
        assert!(inv.env.is_empty(), "gemini must not leak CODEX_HOME");
    }

    #[test]
    fn build_invocation_gemini_no_model() {
        let config = Config {
            backend: Backend::Gemini,
            gemini_model: String::new(),
            ..Config::default()
        };
        let inv = build_invocation(Path::new("/usr/bin/gemini"), "hello", &config);
        assert_eq!(inv.args[0], "-p");
        assert_eq!(inv.args[1], "hello");
        assert_eq!(inv.args[2], "--output-format");
        assert_eq!(inv.args[3], "text");
        assert_eq!(inv.stdin, None);
        assert!(inv.env.is_empty(), "gemini must not leak CODEX_HOME");
    }

    #[test]
    fn build_invocation_with_model_opencode() {
        let config = Config {
            backend: Backend::Opencode,
            provider: "openai".to_owned(),
            ..Config::default()
        };
        let inv = build_invocation_with_model(
            Path::new("/usr/bin/opencode"),
            "hello",
            &config,
            "gpt-5.4",
            Some("anthropic"),
        );
        assert_eq!(inv.args[0], "run");
        assert!(inv.args.contains(&"-m".to_owned()));
        assert!(inv.args.contains(&"anthropic/gpt-5.4".to_owned()));
        assert_eq!(inv.args.last().map(String::as_str), Some("hello"));
        assert!(inv.stdin.is_none());
        // The PR stage must NOT pass --variant minimal — PR synthesis quality
        // relies on the provider's default reasoning budget.
        assert!(!inv.args.contains(&"--variant".to_owned()));
        assert!(inv.env.is_empty(), "opencode must not leak CODEX_HOME");
    }

    #[test]
    fn build_invocation_with_model_claude() {
        let config = Config {
            backend: Backend::Claude,
            ..Config::default()
        };
        let inv = build_invocation_with_model(
            Path::new("/usr/bin/claude"),
            "hello",
            &config,
            "claude-opus-4-6",
            None,
        );
        assert_eq!(inv.args[2], "claude-opus-4-6");
        assert_eq!(inv.stdin.as_deref(), Some("hello"));
        assert!(inv.env.is_empty(), "claude must not leak CODEX_HOME");
    }

    #[test]
    fn build_invocation_with_model_codex_provider() {
        let config = Config {
            backend: Backend::Codex,
            ..Config::default()
        };
        let inv = build_invocation_with_model(
            Path::new("/usr/bin/codex"),
            "hello",
            &config,
            "gpt-5.4",
            Some("openrouter"),
        );
        assert!(inv.args.contains(&"-m".to_owned()));
        assert!(inv.args.contains(&"gpt-5.4".to_owned()));
        assert!(
            inv.args
                .contains(&"model_provider=\"openrouter\"".to_owned())
        );
        // The PR stage must NOT force reasoning=none — PR quality matters and
        // the user's ~/.codex/config.toml setting should win here.
        assert!(
            !inv.args
                .contains(&"model_reasoning_effort=\"none\"".to_owned())
        );
        // But the cheap-always flags should still be present.
        assert!(inv.args.contains(&"--skip-git-repo-check".to_owned()));
        assert!(inv.args.contains(&"--disable".to_owned()));
        assert!(inv.args.contains(&"plugins".to_owned()));
        // MCP override must also apply on the PR path.
        assert!(inv.args.contains(&"mcp_servers={}".to_owned()));
        assert_codex_env_shape(&inv.env);
    }

    #[test]
    fn build_invocation_with_model_gemini() {
        let config = Config {
            backend: Backend::Gemini,
            ..Config::default()
        };
        let inv = build_invocation_with_model(
            Path::new("/usr/bin/gemini"),
            "hello",
            &config,
            "gemini-3-flash-preview",
            None,
        );
        assert_eq!(inv.args[0], "-p");
        assert_eq!(inv.args[1], "hello");
        assert_eq!(inv.args[2], "-m");
        assert_eq!(inv.args[3], "gemini-3-flash-preview");
        assert_eq!(inv.stdin, None);
        assert!(inv.env.is_empty(), "gemini must not leak CODEX_HOME");
    }

    #[test]
    fn strip_ansi_codes() {
        assert_eq!(
            strip_ansi("\x1b[32mfeat: add login\x1b[0m"),
            "feat: add login"
        );
        assert_eq!(strip_ansi("no codes here"), "no codes here");
        assert_eq!(strip_ansi("\x1b[1;31mred bold\x1b[0m"), "red bold");
    }

    #[test]
    fn detect_nonexistent_path_errors() {
        let result = detect_cli(CliBackend::Opencode, "/nonexistent/path/opencode");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not executable"));
    }

    #[test]
    fn detect_empty_path_tries_system() {
        // This test just verifies the detection doesn't panic.
        // It may or may not find a binary depending on the system.
        let _ = detect_cli(CliBackend::Opencode, "");
    }
}
