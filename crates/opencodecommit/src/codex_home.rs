//! Minimal `CODEX_HOME` setup for faster `codex exec` invocations.
//!
//! The user's `~/.codex` directory accumulates state (MCP server registries,
//! history, sqlite caches, plugins, sessions, skills). On every `codex exec`
//! call, codex loads and parses this entire tree, which adds measurable
//! wall-clock overhead — paired benches showed ~47 % slower runs against a
//! populated user home vs. a freshly-scoped empty home.
//!
//! This module creates an occ-managed minimal codex home at
//! `$XDG_CACHE_HOME/opencodecommit/codex-home` (fallback
//! `$HOME/.cache/opencodecommit/codex-home`) containing only:
//!
//!   - `auth.json`, a symlink to the user's real `~/.codex/auth.json` so
//!     token refreshes from `codex login` are seen transparently.
//!   - `config.toml`, an empty managed file that suppresses codex's
//!     first-run "missing config" warning.
//!
//! Everything else (`installation_id`, `models_cache.json`, …) codex creates
//! on first use and caches across subsequent invocations.

use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

static CACHED_HOME: LazyLock<Mutex<Option<PathBuf>>> = LazyLock::new(|| Mutex::new(None));

/// Ensure the occ-managed minimal codex home exists with a live auth symlink.
///
/// Returns the absolute path on success. On any failure (no
/// `~/.codex/auth.json`, unwritable cache dir, symlink creation failure)
/// returns `None` so callers fall back to the user's default `~/.codex`.
///
/// Cached after the first successful call per process.
pub fn ensure_minimal_codex_home() -> Option<PathBuf> {
    if let Ok(guard) = CACHED_HOME.lock()
        && let Some(path) = guard.as_ref()
        && path.join("auth.json").exists()
    {
        return Some(path.clone());
    }

    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    let cache_dir = resolve_cache_dir(&home)?;
    let result = ensure_minimal_codex_home_at(&cache_dir, &home);

    if let Some(ref path) = result
        && let Ok(mut guard) = CACHED_HOME.lock()
    {
        *guard = Some(path.clone());
    }

    result
}

/// Compute `$XDG_CACHE_HOME/opencodecommit/codex-home` with fallback to
/// `$HOME/.cache/opencodecommit/codex-home`.
fn resolve_cache_dir(home: &Path) -> Option<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_CACHE_HOME") {
        let xdg = PathBuf::from(xdg);
        if xdg.is_absolute() {
            return Some(xdg.join("opencodecommit").join("codex-home"));
        }
    }
    Some(
        home.join(".cache")
            .join("opencodecommit")
            .join("codex-home"),
    )
}

/// Testable inner: set up a minimal codex home under `target_root` using
/// `home_dir` as the source of `.codex/auth.json`. Returns `None` on any
/// failure so callers fall back to the default codex path.
fn ensure_minimal_codex_home_at(target_root: &Path, home_dir: &Path) -> Option<PathBuf> {
    std::fs::create_dir_all(target_root).ok()?;

    let source_auth = home_dir.join(".codex").join("auth.json");
    if !source_auth.exists() {
        return None;
    }

    let link_path = target_root.join("auth.json");
    ensure_auth_symlink(&source_auth, &link_path)?;
    ensure_empty_config(&target_root.join("config.toml"))?;

    Some(target_root.to_path_buf())
}

/// Point `link_path` at `source_auth`, replacing any existing entry that
/// doesn't already resolve to the correct target.
fn ensure_auth_symlink(source_auth: &Path, link_path: &Path) -> Option<()> {
    if let Ok(existing_target) = std::fs::read_link(link_path)
        && existing_target == source_auth
        && link_path.exists()
    {
        return Some(());
    }

    match std::fs::remove_file(link_path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return None,
    }

    create_symlink(source_auth, link_path).ok()
}

#[cfg(unix)]
fn create_symlink(source: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(source, link)
}

#[cfg(windows)]
fn create_symlink(source: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(source, link)
}

#[cfg(not(any(unix, windows)))]
fn create_symlink(_source: &Path, _link: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "symlinks not supported on this platform",
    ))
}

fn ensure_empty_config(path: &Path) -> Option<()> {
    if path.exists() {
        return Some(());
    }
    std::fs::write(path, "# managed by opencodecommit\n").ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TestDirs {
        root: PathBuf,
    }

    impl TestDirs {
        fn new(label: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let counter = COUNTER.fetch_add(1, Ordering::SeqCst);
            let pid = std::process::id();
            let root = std::env::temp_dir().join(format!(
                "occ-codex-home-test-{label}-{pid}-{nanos}-{counter}"
            ));
            std::fs::create_dir_all(&root).expect("create test root");
            Self { root }
        }

        fn home(&self) -> PathBuf {
            self.root.join("home")
        }

        fn target(&self) -> PathBuf {
            self.root.join("target")
        }

        fn plant_auth(&self) -> PathBuf {
            let codex_dir = self.home().join(".codex");
            std::fs::create_dir_all(&codex_dir).expect("create .codex");
            let auth = codex_dir.join("auth.json");
            std::fs::write(&auth, br#"{"stub":"test-auth"}"#).expect("write auth");
            auth
        }
    }

    impl Drop for TestDirs {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn idempotent_creates_once() {
        let dirs = TestDirs::new("idempotent");
        dirs.plant_auth();

        let first = ensure_minimal_codex_home_at(&dirs.target(), &dirs.home())
            .expect("first call succeeds");
        let second = ensure_minimal_codex_home_at(&dirs.target(), &dirs.home())
            .expect("second call succeeds");
        assert_eq!(first, second);

        let link = dirs.target().join("auth.json");
        let metadata = std::fs::symlink_metadata(&link).expect("link exists");
        assert!(
            metadata.file_type().is_symlink(),
            "auth.json must be a symlink"
        );
        let resolved = std::fs::read_link(&link).expect("readlink");
        assert_eq!(resolved, dirs.home().join(".codex").join("auth.json"));

        assert!(dirs.target().join("config.toml").exists());
    }

    #[test]
    fn missing_auth_returns_none() {
        let dirs = TestDirs::new("missing");
        // Do NOT plant auth.
        let result = ensure_minimal_codex_home_at(&dirs.target(), &dirs.home());
        assert!(result.is_none(), "expected None when auth.json is missing");
    }

    #[test]
    fn repairs_stale_symlink() {
        let dirs = TestDirs::new("stale");
        dirs.plant_auth();
        std::fs::create_dir_all(dirs.target()).expect("target");

        // Plant a stale symlink pointing at a nonexistent path.
        let stale_target = dirs.root.join("nonexistent-auth.json");
        let link = dirs.target().join("auth.json");
        create_symlink(&stale_target, &link).expect("stale symlink");

        let result =
            ensure_minimal_codex_home_at(&dirs.target(), &dirs.home()).expect("repair succeeds");
        assert_eq!(result, dirs.target());

        let resolved = std::fs::read_link(&link).expect("readlink");
        assert_eq!(resolved, dirs.home().join(".codex").join("auth.json"));
    }

    #[test]
    fn resolve_cache_dir_prefers_xdg_when_absolute() {
        // Saved guard for XDG so tests don't pollute subsequent runs.
        let prev = std::env::var_os("XDG_CACHE_HOME");
        // SAFETY: tests in this module mutate process env; Rust 2024 requires
        // `unsafe` around `set_var`, and we accept the risk because these test
        // cases are fast and cooperate via the CACHED_HOME mutex upstream.
        unsafe {
            std::env::set_var("XDG_CACHE_HOME", "/explicit/xdg/cache");
        }
        let home = PathBuf::from("/home/testuser");
        assert_eq!(
            resolve_cache_dir(&home),
            Some(PathBuf::from(
                "/explicit/xdg/cache/opencodecommit/codex-home"
            ))
        );

        unsafe {
            match prev {
                Some(val) => std::env::set_var("XDG_CACHE_HOME", val),
                None => std::env::remove_var("XDG_CACHE_HOME"),
            }
        }
    }

    #[test]
    fn resolve_cache_dir_falls_back_to_home_cache() {
        let prev = std::env::var_os("XDG_CACHE_HOME");
        unsafe {
            std::env::remove_var("XDG_CACHE_HOME");
        }
        let home = PathBuf::from("/home/testuser");
        assert_eq!(
            resolve_cache_dir(&home),
            Some(PathBuf::from(
                "/home/testuser/.cache/opencodecommit/codex-home"
            ))
        );
        unsafe {
            if let Some(val) = prev {
                std::env::set_var("XDG_CACHE_HOME", val);
            }
        }
    }
}
