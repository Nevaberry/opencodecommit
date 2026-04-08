use std::fmt;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use opencodecommit::config::Config;
use serde::{Deserialize, Serialize};

// ── Install source ──

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallSource {
    Npm,
    Cargo,
    Unknown,
}

impl fmt::Display for InstallSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Npm => write!(f, "npm"),
            Self::Cargo => write!(f, "cargo"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

pub fn detect_install_source() -> InstallSource {
    let exe = match std::env::current_exe().and_then(|p| std::fs::canonicalize(p)) {
        Ok(p) => p,
        Err(_) => return InstallSource::Unknown,
    };
    let path_str = exe.to_string_lossy();

    // npm: binary lives inside node_modules or the opencodecommit npm package
    if path_str.contains("node_modules") || path_str.contains("npm/opencodecommit/platforms/") {
        return InstallSource::Npm;
    }

    // cargo: binary lives in $CARGO_HOME/bin/ (default ~/.cargo/bin/)
    let cargo_bin = std::env::var("CARGO_HOME")
        .map(|h| PathBuf::from(h).join("bin"))
        .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".cargo").join("bin")));
    if let Ok(bin_dir) = cargo_bin {
        if let Ok(canon) = std::fs::canonicalize(&bin_dir) {
            if exe.starts_with(&canon) {
                return InstallSource::Cargo;
            }
        }
        // fallback: compare without canonicalize in case of symlink differences
        if exe.starts_with(&bin_dir) {
            return InstallSource::Cargo;
        }
    }

    InstallSource::Unknown
}

// ── Version checking ──

pub fn check_latest_version(source: InstallSource) -> Result<String, String> {
    match source {
        InstallSource::Npm => check_npm_latest(),
        InstallSource::Cargo => check_cargo_latest(),
        InstallSource::Unknown => check_npm_latest().or_else(|_| check_cargo_latest()),
    }
}

fn check_npm_latest() -> Result<String, String> {
    let output = Command::new("npm")
        .args(["view", "opencodecommit", "version"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("failed to run npm: {e}"))?
        .wait_with_output()
        .map_err(|e| format!("npm failed: {e}"))?;

    if !output.status.success() {
        return Err("npm view failed".to_owned());
    }
    let version = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if version.is_empty() {
        return Err("empty version from npm".to_owned());
    }
    Ok(version)
}

fn check_cargo_latest() -> Result<String, String> {
    let output = Command::new("cargo")
        .args(["search", "opencodecommit", "--limit", "1"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("failed to run cargo: {e}"))?
        .wait_with_output()
        .map_err(|e| format!("cargo search failed: {e}"))?;

    if !output.status.success() {
        return Err("cargo search failed".to_owned());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    // Output format: opencodecommit = "1.2.3"    # description
    for line in text.lines() {
        if line.starts_with("opencodecommit") {
            if let Some(start) = line.find('"') {
                if let Some(end) = line[start + 1..].find('"') {
                    return Ok(line[start + 1..start + 1 + end].to_owned());
                }
            }
        }
    }
    Err("could not parse cargo search output".to_owned())
}

// ── Version comparison ──

pub fn is_newer(current: &str, latest: &str) -> bool {
    let parse = |v: &str| -> Option<(u32, u32, u32)> {
        let mut parts = v.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next()?.parse().ok()?;
        Some((major, minor, patch))
    };
    match (parse(current), parse(latest)) {
        (Some(c), Some(l)) => l > c,
        _ => false,
    }
}

// ── Update check cache ──

const CHECK_INTERVAL_SECS: u64 = 86400; // 24 hours

#[derive(Serialize, Deserialize)]
struct UpdateCache {
    last_check_epoch: u64,
    latest_version: String,
}

fn cache_path() -> Option<PathBuf> {
    Config::default_config_dir().map(|d| d.join("update-check.json"))
}

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn read_cache() -> Option<UpdateCache> {
    let path = cache_path()?;
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn write_cache(latest_version: &str) {
    let Some(path) = cache_path() else { return };
    let cache = UpdateCache {
        last_check_epoch: now_epoch(),
        latest_version: latest_version.to_owned(),
    };
    if let Ok(json) = serde_json::to_string(&cache) {
        let _ = std::fs::write(path, json);
    }
}

/// Returns `(should_do_network_check, cached_latest_version)`.
pub fn should_check() -> (bool, Option<String>) {
    match read_cache() {
        Some(cache) => {
            let elapsed = now_epoch().saturating_sub(cache.last_check_epoch);
            if elapsed >= CHECK_INTERVAL_SECS {
                (true, Some(cache.latest_version))
            } else {
                (false, Some(cache.latest_version))
            }
        }
        None => (true, None),
    }
}

// ── Update execution ──

pub fn run_update(source: InstallSource) -> Result<(), String> {
    match source {
        InstallSource::Npm => {
            eprintln!("Running: npm install -g opencodecommit");
            let status = Command::new("npm")
                .args(["install", "-g", "opencodecommit"])
                .status()
                .map_err(|e| format!("failed to run npm: {e}"))?;
            if status.success() {
                Ok(())
            } else {
                Err(format!("npm exited with {status}"))
            }
        }
        InstallSource::Cargo => {
            eprintln!("Running: cargo install opencodecommit");
            let status = Command::new("cargo")
                .args(["install", "opencodecommit"])
                .status()
                .map_err(|e| format!("failed to run cargo: {e}"))?;
            if status.success() {
                Ok(())
            } else {
                Err(format!("cargo exited with {status}"))
            }
        }
        InstallSource::Unknown => Err("Could not detect installation source.\n\
                 Update manually with one of:\n  \
                 npm install -g opencodecommit\n  \
                 cargo install opencodecommit"
            .to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_newer() {
        assert!(is_newer("1.1.4", "1.2.0"));
        assert!(is_newer("1.1.4", "1.1.5"));
        assert!(is_newer("1.1.4", "2.0.0"));
        assert!(!is_newer("1.2.0", "1.2.0"));
        assert!(!is_newer("2.0.0", "1.9.9"));
        assert!(!is_newer("1.1.5", "1.1.4"));
    }

    #[test]
    fn test_is_newer_invalid() {
        assert!(!is_newer("bad", "1.0.0"));
        assert!(!is_newer("1.0.0", "bad"));
        assert!(!is_newer("", ""));
    }

    #[test]
    fn test_detect_source_dev_build() {
        // When running from cargo test, the binary is in target/
        assert_eq!(detect_install_source(), InstallSource::Unknown);
    }

    #[test]
    fn test_cache_roundtrip() {
        let dir = std::env::temp_dir().join("occ-test-cache");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("update-check.json");

        let cache = UpdateCache {
            last_check_epoch: 1712505600,
            latest_version: "1.2.0".to_owned(),
        };
        let json = serde_json::to_string(&cache).unwrap();
        std::fs::write(&path, &json).unwrap();

        let loaded: UpdateCache =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(loaded.latest_version, "1.2.0");
        assert_eq!(loaded.last_check_epoch, 1712505600);

        let _ = std::fs::remove_dir_all(dir);
    }
}
