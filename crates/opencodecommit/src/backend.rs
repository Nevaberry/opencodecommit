use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

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
        CliBackend::Opencode => Invocation {
            command: cli_path.to_owned(),
            args: vec![
                "run".to_owned(),
                "-m".to_owned(),
                format!("{}/{}", config.provider, config.model),
                prompt.to_owned(),
            ],
            stdin: None,
        },
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
            }
        }
    }
}

/// Flags that are always safe and always cheap for `codex exec`, regardless
/// of whether the task is a commit or a PR synthesis:
///   --ephemeral                  no session files on disk
///   --skip-git-repo-check        avoid the repo-detection round trip
///   -s read-only                 sandbox
///   --dangerously-bypass-*       skip interactive approvals
///   --disable plugins            skip loading user-configured codex plugins
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
        "-m".to_owned(),
        model.to_owned(),
    ]
}

/// `codex exec` argv for the fast commit-generation path. Adds:
///   -c model_reasoning_effort="none"   — commits don't need reasoning
///   -c web_search="disabled"           — remove the web_search tool
///                                        (required for reasoning < low, and
///                                        trims tool preamble tokens)
fn codex_base_args(model: &str) -> Vec<String> {
    let mut args = codex_common_args(model);
    // Insert reasoning+web_search overrides before -m so argv ordering stays
    // predictable for tests, without depending on insertion index.
    args.push("-c".to_owned());
    args.push("model_reasoning_effort=\"none\"".to_owned());
    args.push("-c".to_owned());
    args.push("web_search=\"disabled\"".to_owned());
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
            Invocation {
                command: cli_path.to_owned(),
                args: vec![
                    "run".to_owned(),
                    "-m".to_owned(),
                    format!("{prov}/{model}"),
                    prompt.to_owned(),
                ],
                stdin: None,
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

    #[test]
    fn build_invocation_opencode() {
        let config = Config {
            backend: Backend::Opencode,
            provider: "openai".to_owned(),
            model: "gpt-5.4-mini".to_owned(),
            ..Config::default()
        };
        let inv = build_invocation(Path::new("/usr/bin/opencode"), "hello", &config);
        assert_eq!(inv.args[0], "run");
        assert_eq!(inv.args[1], "-m");
        assert_eq!(inv.args[2], "openai/gpt-5.4-mini");
        assert_eq!(inv.args[3], "hello");
        assert!(inv.stdin.is_none());
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
    }

    #[test]
    fn build_invocation_codex() {
        let config = Config {
            backend: Backend::Codex,
            codex_model: "gpt-5.4-mini".to_owned(),
            ..Config::default()
        };
        let inv = build_invocation(Path::new("/usr/bin/codex"), "hello", &config);
        // Fast-path flags: no reasoning, no web_search tool, no plugins.
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
        assert!(inv.args.contains(&"--disable".to_owned()));
        assert!(inv.args.contains(&"plugins".to_owned()));
        assert!(inv.args.contains(&"-m".to_owned()));
        assert!(inv.args.contains(&"gpt-5.4-mini".to_owned()));
        assert_eq!(inv.args.last().map(String::as_str), Some("-"));
        assert_eq!(inv.stdin.as_deref(), Some("hello"));
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
        assert_eq!(inv.args[2], "anthropic/gpt-5.4");
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
