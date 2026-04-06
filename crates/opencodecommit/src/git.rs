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

/// Stage all changes.
pub fn stage_all(repo: &Path) -> Result<()> {
    git(repo, &["add", "-A"])?;
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
    let output = match source {
        DiffSource::Staged => git(repo, &["diff", "--cached", "--name-only"])?,
        DiffSource::All => git(repo, &["diff", "HEAD", "--name-only"])?,
        DiffSource::Auto => {
            let staged = git(repo, &["diff", "--cached", "--name-only"])?;
            if !staged.is_empty() {
                staged
            } else {
                git(repo, &["diff", "HEAD", "--name-only"])?
            }
        }
    };
    if output.is_empty() {
        return Ok(vec![]);
    }
    Ok(output.lines().map(|l| l.to_owned()).collect())
}

/// Get the list of unstaged file paths.
pub fn get_unstaged_files(repo: &Path) -> Result<Vec<String>> {
    let output = git(repo, &["diff", "--name-only"])?;
    if output.is_empty() {
        return Ok(vec![]);
    }
    Ok(output.lines().map(|l| l.to_owned()).collect())
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
}
