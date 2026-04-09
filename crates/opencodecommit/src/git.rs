use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::DiffSource;
use crate::{Error, Result};

/// Run a git command in the given repo directory, return trimmed stdout.
fn git(repo: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .map_err(|e| Error::Git(format!("failed to run git: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Git(format!(
            "git {} failed: {}",
            args.join(" "),
            stderr.trim()
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn git_global(args: &[&str]) -> Result<(String, i32)> {
    let output = Command::new("git")
        .args(args)
        .output()
        .map_err(|e| Error::Git(format!("failed to run git: {e}")))?;

    let status = output.status.code().unwrap_or(1);
    if !output.status.success() && status != 1 && status != 5 {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Git(format!(
            "git {} failed: {}",
            args.join(" "),
            stderr.trim()
        )));
    }

    Ok((
        String::from_utf8_lossy(&output.stdout).trim().to_owned(),
        status,
    ))
}

fn git_with_allowed_statuses(repo: &Path, args: &[&str], allowed: &[i32]) -> Result<(String, i32)> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .map_err(|e| Error::Git(format!("failed to run git: {e}")))?;

    let status = output.status.code().unwrap_or(1);
    if !output.status.success() && !allowed.contains(&status) {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Git(format!(
            "git {} failed: {}",
            args.join(" "),
            stderr.trim()
        )));
    }

    Ok((
        String::from_utf8_lossy(&output.stdout).trim().to_owned(),
        status,
    ))
}

fn git_lines(repo: &Path, args: &[&str]) -> Result<Vec<String>> {
    let output = git(repo, args)?;
    if output.is_empty() {
        return Ok(vec![]);
    }

    Ok(output
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| line.to_owned())
        .collect())
}

/// Find the repository root from the current directory.
pub fn get_repo_root() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| Error::Git(format!("failed to run git: {e}")))?;

    if !output.status.success() {
        return Err(Error::Git("not a git repository".to_owned()));
    }

    Ok(PathBuf::from(
        String::from_utf8_lossy(&output.stdout).trim(),
    ))
}

/// Find the resolved `.git` directory for the given repository.
pub fn get_git_dir(repo: &Path) -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--absolute-git-dir"])
        .current_dir(repo)
        .output()
        .map_err(|e| Error::Git(format!("failed to run git: {e}")))?;

    if !output.status.success() {
        return Err(Error::Git("not a git repository".to_owned()));
    }

    Ok(PathBuf::from(
        String::from_utf8_lossy(&output.stdout).trim(),
    ))
}

/// Get the globally configured hooks path, with `~` expanded when Git supports it.
pub fn get_global_hooks_path() -> Result<Option<PathBuf>> {
    let (output, status) = git_global(&[
        "config",
        "--global",
        "--type=path",
        "--get",
        "core.hooksPath",
    ])?;
    if status == 1 || output.is_empty() {
        return Ok(None);
    }
    Ok(Some(PathBuf::from(output)))
}

/// Configure the global hooks path.
pub fn set_global_hooks_path(path: &Path) -> Result<()> {
    let value = path.to_string_lossy().to_string();
    let _ = git_global(&["config", "--global", "core.hooksPath", &value])?;
    Ok(())
}

/// Remove the global hooks path, if one is configured.
pub fn unset_global_hooks_path() -> Result<()> {
    let _ = git_global(&["config", "--global", "--unset", "core.hooksPath"])?;
    Ok(())
}

/// Get the diff based on the configured source.
/// - `Staged`: staged changes only (`git diff --cached`)
/// - `All`: all working tree changes (`git diff HEAD`)
/// - `Auto`: staged first, fallback to all
pub fn get_diff(source: DiffSource, repo: &Path) -> Result<String> {
    match source {
        DiffSource::Staged => {
            let diff = git(repo, &["diff", "--cached"])?;
            if diff.is_empty() {
                return Err(Error::NoChanges);
            }
            Ok(diff)
        }
        DiffSource::All => {
            let diff = git(repo, &["diff", "HEAD"])?;
            if diff.is_empty() {
                return Err(Error::NoChanges);
            }
            Ok(diff)
        }
        DiffSource::Auto => {
            let staged = git(repo, &["diff", "--cached"])?;
            if !staged.is_empty() {
                return Ok(staged);
            }
            let all = git(repo, &["diff", "HEAD"])?;
            if !all.is_empty() {
                return Ok(all);
            }
            Err(Error::NoChanges)
        }
    }
}

/// Get the N most recent commit summaries (oneline format).
pub fn get_recent_commits(repo: &Path, count: usize) -> Result<Vec<String>> {
    let n = count.to_string();
    let output = git(repo, &["log", "--oneline", "-n", &n])?;
    if output.is_empty() {
        return Ok(vec![]);
    }
    Ok(output.lines().map(|l| l.to_owned()).collect())
}

/// Get the current branch name.
pub fn get_branch_name(repo: &Path) -> Result<String> {
    git(repo, &["rev-parse", "--abbrev-ref", "HEAD"])
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    pub path: String,
    pub staged: bool,
    pub unstaged: bool,
    pub untracked: bool,
}

/// Stage all changes.
pub fn stage_all(repo: &Path) -> Result<()> {
    git(repo, &["add", "-A"])?;
    Ok(())
}

/// Stage a single path.
pub fn stage_path(repo: &Path, path: &str) -> Result<()> {
    git(repo, &["add", "-A", "--", path])?;
    Ok(())
}

/// Unstage a single path.
pub fn unstage_path(repo: &Path, path: &str) -> Result<()> {
    git(repo, &["restore", "--staged", "--", path])?;
    Ok(())
}

/// Commit staged changes with the given message.
pub fn git_commit(repo: &Path, message: &str) -> Result<String> {
    git(repo, &["commit", "-m", message])
}

/// Create and checkout a new branch.
pub fn create_and_checkout_branch(repo: &Path, name: &str) -> Result<()> {
    git(repo, &["checkout", "-b", name])?;
    Ok(())
}

/// Get the N most recent branch names sorted by committer date.
pub fn get_recent_branch_names(repo: &Path, count: usize) -> Result<Vec<String>> {
    let output = git(
        repo,
        &[
            "branch",
            "--sort=-committerdate",
            "--format=%(refname:short)",
        ],
    )?;
    if output.is_empty() {
        return Ok(vec![]);
    }
    Ok(output.lines().take(count).map(|l| l.to_owned()).collect())
}

/// Get the list of changed file paths.
pub fn get_changed_files(source: DiffSource, repo: &Path) -> Result<Vec<String>> {
    match source {
        DiffSource::Staged => git_lines(repo, &["diff", "--cached", "--name-only"]),
        DiffSource::All => git_lines(repo, &["diff", "HEAD", "--name-only"]),
        DiffSource::Auto => {
            let staged = git_lines(repo, &["diff", "--cached", "--name-only"])?;
            if staged.is_empty() {
                git_lines(repo, &["diff", "HEAD", "--name-only"])
            } else {
                Ok(staged)
            }
        }
    }
}

/// Get the list of unstaged file paths.
pub fn get_unstaged_files(repo: &Path) -> Result<Vec<String>> {
    git_lines(repo, &["diff", "--name-only"])
}

/// Run a git command allowing exit code 1 (not found / empty result).
fn git_allow_not_found(repo: &Path, args: &[&str]) -> Result<(String, i32)> {
    git_with_allowed_statuses(repo, args, &[1, 128])
}

/// Get staged, unstaged, and untracked file state in one merged view.
pub fn get_file_changes(repo: &Path) -> Result<Vec<FileChange>> {
    let staged = git_lines(
        repo,
        &["diff", "--cached", "--name-only", "--diff-filter=ACDMRTUXB"],
    )?;
    let unstaged = git_lines(repo, &["diff", "--name-only", "--diff-filter=ACDMRTUXB"])?;
    let untracked = git_lines(repo, &["ls-files", "--others", "--exclude-standard"])?;

    let mut changes = BTreeMap::<String, FileChange>::new();

    for path in staged {
        let entry = changes.entry(path.clone()).or_insert_with(|| FileChange {
            path,
            staged: false,
            unstaged: false,
            untracked: false,
        });
        entry.staged = true;
    }

    for path in unstaged {
        let entry = changes.entry(path.clone()).or_insert_with(|| FileChange {
            path,
            staged: false,
            unstaged: false,
            untracked: false,
        });
        entry.unstaged = true;
    }

    for path in untracked {
        let entry = changes.entry(path.clone()).or_insert_with(|| FileChange {
            path,
            staged: false,
            unstaged: false,
            untracked: false,
        });
        entry.unstaged = true;
        entry.untracked = true;
    }

    Ok(changes.into_values().collect())
}

/// Get a display diff containing tracked changes plus patches for untracked files.
pub fn get_combined_diff(repo: &Path) -> Result<String> {
    let mut diff = git(repo, &["diff", "HEAD"])?;
    let null_path = if cfg!(windows) { "NUL" } else { "/dev/null" };

    for change in get_file_changes(repo)?
        .into_iter()
        .filter(|change| change.untracked)
    {
        let (patch, _) = git_with_allowed_statuses(
            repo,
            &["diff", "--no-index", "--", null_path, &change.path],
            &[1],
        )?;
        if !patch.is_empty() {
            if !diff.is_empty() {
                diff.push_str("\n\n");
            }
            diff.push_str(&patch);
        }
    }

    if diff.is_empty() {
        Err(Error::NoChanges)
    } else {
        Ok(diff)
    }
}

/// Detect the base branch for PR-style diffs.
///
/// Priority: explicit flag > upstream tracking branch > main > master.
pub fn detect_base_branch(repo: &Path, explicit_base: Option<&str>) -> Result<String> {
    if let Some(base) = explicit_base {
        return Ok(base.to_owned());
    }

    // Try upstream tracking branch
    if let Ok((upstream, code)) =
        git_allow_not_found(repo, &["rev-parse", "--abbrev-ref", "@{upstream}"])
        && code == 0
        && !upstream.is_empty()
    {
        // e.g. "origin/main" -> "main"
        if let Some(branch) = upstream.rsplit_once('/') {
            return Ok(branch.1.to_owned());
        }
    }

    // Try main
    if let Ok((_, code)) = git_allow_not_found(repo, &["rev-parse", "--verify", "main"])
        && code == 0
    {
        return Ok("main".to_owned());
    }

    // Try master
    if let Ok((_, code)) = git_allow_not_found(repo, &["rev-parse", "--verify", "master"])
        && code == 0
    {
        return Ok("master".to_owned());
    }

    Err(Error::Git(
        "could not detect base branch — use --base".to_owned(),
    ))
}

/// Get the unified diff between the base branch and HEAD (three-dot diff).
pub fn get_branch_diff(repo: &Path, base: &str) -> Result<String> {
    let arg = format!("{base}...HEAD");
    let diff = git(repo, &["diff", &arg])?;
    if diff.is_empty() {
        return Err(Error::NoChanges);
    }
    Ok(diff)
}

/// Get the commit messages between base and HEAD.
pub fn get_commits_ahead(repo: &Path, base: &str) -> Result<Vec<String>> {
    let range = format!("{base}..HEAD");
    let output = git(repo, &["log", &range, "--format=%H%n%s%n%n%b%n---"])?;
    Ok(output
        .split("---\n")
        .chain(std::iter::once(
            output.rsplit_once("---").map_or("", |(_, r)| r),
        ))
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect())
}

/// Get the list of files changed between the base branch and HEAD.
pub fn get_branch_changed_files(repo: &Path, base: &str) -> Result<Vec<String>> {
    let arg = format!("{base}...HEAD");
    let output = git(repo, &["diff", &arg, "--name-only"])?;
    if output.is_empty() {
        return Ok(vec![]);
    }
    Ok(output
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_owned())
        .collect())
}

/// Count the number of commits HEAD is ahead of the base branch.
pub fn count_commits_ahead(repo: &Path, base: &str) -> Result<usize> {
    let range = format!("{base}..HEAD");
    let output = git(repo, &["rev-list", "--count", &range])?;
    Ok(output.parse::<usize>().unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;

    /// Create a temporary git repo with an initial commit.
    fn setup_repo(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("occ-git-test-{}-{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let run = |args: &[&str]| {
            Command::new("git")
                .args(args)
                .current_dir(&dir)
                .env("GIT_AUTHOR_NAME", "Test")
                .env("GIT_AUTHOR_EMAIL", "test@test.com")
                .env("GIT_COMMITTER_NAME", "Test")
                .env("GIT_COMMITTER_EMAIL", "test@test.com")
                .output()
                .unwrap()
        };

        run(&["init"]);
        run(&["config", "user.email", "test@test.com"]);
        run(&["config", "user.name", "Test"]);
        fs::write(dir.join("README.md"), "# Hello").unwrap();
        run(&["add", "README.md"]);
        run(&["commit", "-m", "initial commit"]);

        dir
    }

    fn cleanup(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn get_diff_staged() {
        let dir = setup_repo("diff-staged");
        fs::write(dir.join("file.txt"), "hello").unwrap();
        Command::new("git")
            .args(["add", "file.txt"])
            .current_dir(&dir)
            .output()
            .unwrap();

        let diff = get_diff(DiffSource::Staged, &dir).unwrap();
        assert!(diff.contains("file.txt"));
        assert!(diff.contains("+hello"));
        cleanup(&dir);
    }

    #[test]
    fn get_diff_all_unstaged() {
        let dir = setup_repo("diff-all");
        fs::write(dir.join("file.txt"), "hello").unwrap();
        Command::new("git")
            .args(["add", "file.txt"])
            .current_dir(&dir)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "add file"])
            .current_dir(&dir)
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@test.com")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@test.com")
            .output()
            .unwrap();
        fs::write(dir.join("file.txt"), "changed").unwrap();

        let diff = get_diff(DiffSource::All, &dir).unwrap();
        assert!(diff.contains("file.txt"));
        assert!(diff.contains("+changed"));
        cleanup(&dir);
    }

    #[test]
    fn get_diff_auto_prefers_staged() {
        let dir = setup_repo("diff-auto");
        fs::write(dir.join("staged.txt"), "staged").unwrap();
        Command::new("git")
            .args(["add", "staged.txt"])
            .current_dir(&dir)
            .output()
            .unwrap();
        fs::write(dir.join("unstaged.txt"), "unstaged").unwrap();

        let diff = get_diff(DiffSource::Auto, &dir).unwrap();
        assert!(diff.contains("staged.txt"));
        // Unstaged file should not appear since staged exists
        assert!(!diff.contains("unstaged.txt"));
        cleanup(&dir);
    }

    #[test]
    fn get_diff_no_changes() {
        let dir = setup_repo("diff-none");
        let result = get_diff(DiffSource::Auto, &dir);
        assert!(matches!(result, Err(Error::NoChanges)));
        cleanup(&dir);
    }

    #[test]
    fn recent_commits() {
        let dir = setup_repo("commits");
        let commits = get_recent_commits(&dir, 10).unwrap();
        assert_eq!(commits.len(), 1);
        assert!(commits[0].contains("initial commit"));
        cleanup(&dir);
    }

    #[test]
    fn branch_name() {
        let dir = setup_repo("branch");
        let branch = get_branch_name(&dir).unwrap();
        // Default branch could be main or master depending on git config
        assert!(!branch.is_empty());
        cleanup(&dir);
    }

    #[test]
    fn git_dir_resolves_for_repo() {
        let dir = setup_repo("git-dir");
        let git_dir = get_git_dir(&dir).unwrap();
        assert!(git_dir.ends_with(".git"));
        cleanup(&dir);
    }

    #[test]
    fn changed_files_staged() {
        let dir = setup_repo("changed-files");
        fs::write(dir.join("a.txt"), "a").unwrap();
        fs::write(dir.join("b.txt"), "b").unwrap();
        Command::new("git")
            .args(["add", "a.txt", "b.txt"])
            .current_dir(&dir)
            .output()
            .unwrap();

        let files = get_changed_files(DiffSource::Staged, &dir).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.contains(&"a.txt".to_owned()));
        assert!(files.contains(&"b.txt".to_owned()));
        cleanup(&dir);
    }

    #[test]
    fn unstaged_files_returns_only_worktree_changes() {
        let dir = setup_repo("unstaged-files");
        fs::write(dir.join("tracked.txt"), "initial").unwrap();
        Command::new("git")
            .args(["add", "tracked.txt"])
            .current_dir(&dir)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "add tracked file"])
            .current_dir(&dir)
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@test.com")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@test.com")
            .output()
            .unwrap();
        fs::write(dir.join("tracked.txt"), "changed").unwrap();

        let files = get_unstaged_files(&dir).unwrap();
        assert_eq!(files, vec!["tracked.txt".to_owned()]);
        cleanup(&dir);
    }

    #[test]
    fn stage_all_stages_files() {
        let dir = setup_repo("stage-all");
        fs::write(dir.join("new.txt"), "new content").unwrap();
        stage_all(&dir).unwrap();
        let status = git(&dir, &["diff", "--cached", "--name-only"]).unwrap();
        assert!(status.contains("new.txt"));
        cleanup(&dir);
    }

    #[test]
    fn stage_path_stages_only_selected_file() {
        let dir = setup_repo("stage-path");
        fs::write(dir.join("a.txt"), "a").unwrap();
        fs::write(dir.join("b.txt"), "b").unwrap();

        stage_path(&dir, "a.txt").unwrap();

        let status = git(&dir, &["diff", "--cached", "--name-only"]).unwrap();
        assert_eq!(status, "a.txt");
        cleanup(&dir);
    }

    #[test]
    fn unstage_path_removes_file_from_index() {
        let dir = setup_repo("unstage-path");
        fs::write(dir.join("a.txt"), "a").unwrap();
        stage_all(&dir).unwrap();

        unstage_path(&dir, "a.txt").unwrap();

        let staged = git(&dir, &["diff", "--cached", "--name-only"]).unwrap();
        let changes = get_file_changes(&dir).unwrap();
        let change = changes
            .iter()
            .find(|change| change.path == "a.txt")
            .unwrap();
        assert!(staged.is_empty());
        assert!(!change.staged);
        assert!(change.unstaged);
        assert!(change.untracked);
        cleanup(&dir);
    }

    #[test]
    fn file_changes_include_staged_unstaged_and_untracked() {
        let dir = setup_repo("file-changes");
        fs::write(dir.join("tracked.txt"), "tracked").unwrap();
        stage_path(&dir, "tracked.txt").unwrap();
        Command::new("git")
            .args(["commit", "-m", "add tracked file"])
            .current_dir(&dir)
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@test.com")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@test.com")
            .output()
            .unwrap();
        fs::write(dir.join("tracked.txt"), "changed").unwrap();

        fs::write(dir.join("staged.txt"), "staged").unwrap();
        stage_path(&dir, "staged.txt").unwrap();
        fs::write(dir.join("new.txt"), "new").unwrap();

        let changes = get_file_changes(&dir).unwrap();
        assert!(changes.contains(&FileChange {
            path: "staged.txt".to_owned(),
            staged: true,
            unstaged: false,
            untracked: false,
        }));
        assert!(changes.contains(&FileChange {
            path: "tracked.txt".to_owned(),
            staged: false,
            unstaged: true,
            untracked: false,
        }));
        assert!(changes.contains(&FileChange {
            path: "new.txt".to_owned(),
            staged: false,
            unstaged: true,
            untracked: true,
        }));
        cleanup(&dir);
    }

    #[test]
    fn combined_diff_includes_untracked_files() {
        let dir = setup_repo("combined-diff");
        fs::write(dir.join("new.txt"), "new").unwrap();

        let diff = get_combined_diff(&dir).unwrap();
        assert!(diff.contains("diff --git a/new.txt b/new.txt"));
        assert!(diff.contains("+new"));
        cleanup(&dir);
    }

    #[test]
    fn git_commit_succeeds() {
        let dir = setup_repo("commit-ok");
        fs::write(dir.join("file.txt"), "content").unwrap();
        stage_all(&dir).unwrap();
        let output = git_commit(&dir, "test: add file").unwrap();
        assert!(output.contains("test: add file"));
        cleanup(&dir);
    }

    #[test]
    fn git_commit_fails_with_nothing_staged() {
        let dir = setup_repo("commit-empty");
        let result = git_commit(&dir, "empty");
        assert!(result.is_err());
        cleanup(&dir);
    }

    #[test]
    fn create_and_checkout_branch_works() {
        let dir = setup_repo("branch-create");
        create_and_checkout_branch(&dir, "feat/test-branch").unwrap();
        let branch = get_branch_name(&dir).unwrap();
        assert_eq!(branch, "feat/test-branch");
        cleanup(&dir);
    }

    #[test]
    fn create_and_checkout_branch_fails_on_duplicate() {
        let dir = setup_repo("branch-dup");
        create_and_checkout_branch(&dir, "feat/dup").unwrap();
        // Switch back and try again
        Command::new("git")
            .args(["checkout", "-"])
            .current_dir(&dir)
            .output()
            .unwrap();
        let result = create_and_checkout_branch(&dir, "feat/dup");
        assert!(result.is_err());
        cleanup(&dir);
    }

    #[test]
    fn get_recent_branch_names_returns_sorted() {
        let dir = setup_repo("branch-names");
        create_and_checkout_branch(&dir, "feat/first").unwrap();
        fs::write(dir.join("a.txt"), "a").unwrap();
        Command::new("git")
            .args(["add", "-A"])
            .current_dir(&dir)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "a"])
            .current_dir(&dir)
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@test.com")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@test.com")
            .env("GIT_COMMITTER_DATE", "2025-01-01T00:00:00+00:00")
            .output()
            .unwrap();
        create_and_checkout_branch(&dir, "fix/second").unwrap();
        fs::write(dir.join("b.txt"), "b").unwrap();
        Command::new("git")
            .args(["add", "-A"])
            .current_dir(&dir)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "b"])
            .current_dir(&dir)
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@test.com")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@test.com")
            .env("GIT_COMMITTER_DATE", "2025-06-01T00:00:00+00:00")
            .output()
            .unwrap();
        let branches = get_recent_branch_names(&dir, 10).unwrap();
        // Should include master/main, feat/first, fix/second (at least 3)
        assert!(branches.len() >= 3);
        // fix/second has the latest committer date, so it should come before feat/first
        let pos_second = branches.iter().position(|b| b == "fix/second").unwrap();
        let pos_first = branches.iter().position(|b| b == "feat/first").unwrap();
        assert!(
            pos_second < pos_first,
            "fix/second should come before feat/first, got: {branches:?}"
        );
        // Count limiting works
        let limited = get_recent_branch_names(&dir, 2).unwrap();
        assert_eq!(limited.len(), 2);
        cleanup(&dir);
    }

    /// Create a temporary git repo with an explicit "main" branch.
    fn setup_repo_with_main(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("occ-git-test-{}-{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let run = |args: &[&str]| {
            Command::new("git")
                .args(args)
                .current_dir(&dir)
                .env("GIT_AUTHOR_NAME", "Test")
                .env("GIT_AUTHOR_EMAIL", "test@test.com")
                .env("GIT_COMMITTER_NAME", "Test")
                .env("GIT_COMMITTER_EMAIL", "test@test.com")
                .output()
                .unwrap()
        };

        run(&["init", "-b", "main"]);
        run(&["config", "user.email", "test@test.com"]);
        run(&["config", "user.name", "Test"]);
        fs::write(dir.join("README.md"), "# Hello").unwrap();
        run(&["add", "README.md"]);
        run(&["commit", "-m", "initial commit"]);

        dir
    }

    fn setup_feature_branch(dir: &Path) {
        let run = |args: &[&str]| {
            Command::new("git")
                .args(args)
                .current_dir(dir)
                .env("GIT_AUTHOR_NAME", "Test")
                .env("GIT_AUTHOR_EMAIL", "test@test.com")
                .env("GIT_COMMITTER_NAME", "Test")
                .env("GIT_COMMITTER_EMAIL", "test@test.com")
                .output()
                .unwrap()
        };

        run(&["checkout", "-b", "feature/test"]);
        fs::write(dir.join("feature.txt"), "feature content").unwrap();
        run(&["add", "feature.txt"]);
        run(&["commit", "-m", "feat: add feature file"]);
        fs::write(dir.join("another.txt"), "another file").unwrap();
        run(&["add", "another.txt"]);
        run(&["commit", "-m", "feat: add another file"]);
    }

    #[test]
    fn detect_base_branch_explicit() {
        let dir = setup_repo_with_main("detect-explicit");
        let base = detect_base_branch(&dir, Some("develop")).unwrap();
        assert_eq!(base, "develop");
        cleanup(&dir);
    }

    #[test]
    fn detect_base_branch_fallback_main() {
        let dir = setup_repo_with_main("detect-main");
        setup_feature_branch(&dir);
        let base = detect_base_branch(&dir, None).unwrap();
        assert_eq!(base, "main");
        cleanup(&dir);
    }

    #[test]
    fn count_commits_ahead_basic() {
        let dir = setup_repo_with_main("count-ahead");
        setup_feature_branch(&dir);
        let count = count_commits_ahead(&dir, "main").unwrap();
        assert_eq!(count, 2);
        cleanup(&dir);
    }

    #[test]
    fn get_branch_diff_shows_changes() {
        let dir = setup_repo_with_main("branch-diff");
        setup_feature_branch(&dir);
        let diff = get_branch_diff(&dir, "main").unwrap();
        assert!(diff.contains("feature content"));
        assert!(diff.contains("another file"));
        cleanup(&dir);
    }

    #[test]
    fn get_commits_ahead_returns_messages() {
        let dir = setup_repo_with_main("commits-ahead");
        setup_feature_branch(&dir);
        let commits = get_commits_ahead(&dir, "main").unwrap();
        assert!(!commits.is_empty());
        let joined = commits.join("\n");
        assert!(joined.contains("feat: add feature file"));
        assert!(joined.contains("feat: add another file"));
        cleanup(&dir);
    }

    #[test]
    fn get_branch_changed_files_lists_files() {
        let dir = setup_repo_with_main("branch-files");
        setup_feature_branch(&dir);
        let files = get_branch_changed_files(&dir, "main").unwrap();
        assert!(files.contains(&"feature.txt".to_owned()));
        assert!(files.contains(&"another.txt".to_owned()));
        assert_eq!(files.len(), 2);
        cleanup(&dir);
    }
}
