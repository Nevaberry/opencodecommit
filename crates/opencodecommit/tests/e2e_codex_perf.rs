#![cfg(not(target_os = "windows"))]

use std::env;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const BASELINE_ENV: &str = "OCC_CODEX_PERF_BASELINE_PATH";
const CANDIDATE_ENV: &str = "OCC_CODEX_PERF_CANDIDATE_PATH";
const OCC_ENV: &str = "OCC_CODEX_PERF_OCC_PATH";
const RUNS_ENV: &str = "OCC_CODEX_PERF_RUNS";
const MIN_SPEEDUP_ENV: &str = "OCC_CODEX_PERF_MIN_TOTAL_SPEEDUP";
const DEFAULT_RUNS: usize = 7;
const DEFAULT_MIN_SPEEDUP: f64 = 1.05;
const RESPONSE: &str = "feat: add subtract helper\n\n## Added\n- Add subtract helper";

#[test]
fn codex_candidate_beats_baseline_for_occ_artifact_flows() -> Result<(), Box<dyn std::error::Error>>
{
    let Some(baseline_codex) = env_path(BASELINE_ENV) else {
        eprintln!("skipping Codex perf e2e: set {BASELINE_ENV} to a baseline codex binary");
        return Ok(());
    };
    let Some(candidate_codex) = env_path(CANDIDATE_ENV) else {
        eprintln!("skipping Codex perf e2e: set {CANDIDATE_ENV} to a candidate codex binary");
        return Ok(());
    };
    ensure_file(&baseline_codex)?;
    ensure_file(&candidate_codex)?;

    let occ = env_path(OCC_ENV).unwrap_or_else(occ_bin);
    ensure_file(&occ)?;

    let runs = env_usize(RUNS_ENV, DEFAULT_RUNS).max(3);
    let min_speedup = env_f64(MIN_SPEEDUP_ENV, DEFAULT_MIN_SPEEDUP);
    let root = TempRoot::new("occ-codex-perf")?;
    let schema_fixture = root.path.join("schema.sse");
    let plain_fixture = root.path.join("plain.sse");
    let branch_fixture = root.path.join("branch.sse");
    write_sse_fixture(
        &schema_fixture,
        &serde_json::json!({ "response": RESPONSE }).to_string(),
    )?;
    write_sse_fixture(&plain_fixture, RESPONSE)?;
    write_sse_fixture(&branch_fixture, "feat/add-subtract-helper")?;

    let staged_repo = prepare_staged_repo(&root.path.join("staged-repo"))?;
    let branch_repo = prepare_branch_repo(&root.path.join("branch-repo"))?;
    let staged_diff = git_stdout(&staged_repo, &["diff", "--cached"])?;

    let baseline_config = root.path.join("baseline.toml");
    let candidate_config = root.path.join("candidate.toml");
    write_config(&baseline_config, &baseline_codex)?;
    write_config(&candidate_config, &candidate_codex)?;

    let cases = [
        PerfCase {
            name: "commit",
            repo: staged_repo.as_path(),
            fixture: schema_fixture.as_path(),
            args: &[
                "commit",
                "--stdin",
                "--dry-run",
                "--text",
                "--backend",
                "codex",
                "--mode",
                "conventional",
            ],
            stdin: Some(staged_diff.as_str()),
        },
        PerfCase {
            name: "changelog",
            repo: staged_repo.as_path(),
            fixture: schema_fixture.as_path(),
            args: &["changelog", "--text", "--backend", "codex"],
            stdin: None,
        },
        PerfCase {
            name: "branch",
            repo: staged_repo.as_path(),
            fixture: branch_fixture.as_path(),
            args: &[
                "branch",
                "--dry-run",
                "--text",
                "--backend",
                "codex",
                "--mode",
                "conventional",
            ],
            stdin: None,
        },
        PerfCase {
            name: "pr",
            repo: branch_repo.as_path(),
            fixture: plain_fixture.as_path(),
            args: &["pr", "--text", "--backend", "codex"],
            stdin: None,
        },
    ];

    let mut baseline_total = 0.0;
    let mut candidate_total = 0.0;
    for case in cases {
        let baseline = measure_case(&occ, &baseline_config, &case, runs)?;
        let candidate = measure_case(&occ, &candidate_config, &case, runs)?;
        let baseline_median = median(baseline);
        let candidate_median = median(candidate);
        let speedup = baseline_median.as_secs_f64() / candidate_median.as_secs_f64();
        baseline_total += baseline_median.as_secs_f64();
        candidate_total += candidate_median.as_secs_f64();
        eprintln!(
            "occ Codex perf {name}: baseline_median={baseline_ms}ms candidate_median={candidate_ms}ms speedup={speedup:.2}x",
            name = case.name,
            baseline_ms = millis(baseline_median),
            candidate_ms = millis(candidate_median),
        );
    }
    let total_speedup = baseline_total / candidate_total;
    eprintln!(
        "occ Codex perf total: baseline_total={}ms candidate_total={}ms speedup={total_speedup:.2}x",
        (baseline_total * 1000.0).round() as u128,
        (candidate_total * 1000.0).round() as u128,
    );
    assert!(
        total_speedup >= min_speedup,
        "expected aggregate candidate e2e time to be at least {min_speedup:.2}x faster than baseline; \
         baseline total={}ms candidate total={}ms speedup={total_speedup:.2}x",
        (baseline_total * 1000.0).round() as u128,
        (candidate_total * 1000.0).round() as u128,
    );

    Ok(())
}

struct PerfCase<'a> {
    name: &'a str,
    repo: &'a Path,
    fixture: &'a Path,
    args: &'a [&'a str],
    stdin: Option<&'a str>,
}

fn measure_case(
    occ: &Path,
    config: &Path,
    case: &PerfCase<'_>,
    runs: usize,
) -> Result<Vec<Duration>, Box<dyn std::error::Error>> {
    let mut samples = Vec::with_capacity(runs);
    for idx in 0..runs {
        samples.push(run_occ_once(occ, config, case, idx)?);
    }
    Ok(samples)
}

fn run_occ_once(
    occ: &Path,
    config: &Path,
    case: &PerfCase<'_>,
    idx: usize,
) -> Result<Duration, Box<dyn std::error::Error>> {
    let mut command = Command::new(occ);
    command
        .current_dir(case.repo)
        .args(case.args)
        .arg("--config")
        .arg(config)
        .env("CODEX_RS_SSE_FIXTURE", case.fixture)
        .env("CODEX_API_KEY", "dummy")
        .env("NO_COLOR", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if case.stdin.is_some() {
        command.stdin(Stdio::piped());
    }

    let started = Instant::now();
    let mut child = command.spawn()?;
    if let Some(stdin) = case.stdin {
        child
            .stdin
            .as_mut()
            .expect("stdin pipe")
            .write_all(stdin.as_bytes())?;
    }
    let output = child.wait_with_output()?;
    let elapsed = started.elapsed();

    if !output.status.success() {
        return Err(format!(
            "{} run {idx} failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
            case.name,
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        )
        .into());
    }

    Ok(elapsed)
}

fn write_sse_fixture(path: &Path, text: &str) -> std::io::Result<()> {
    let event = serde_json::json!({
        "type": "response.output_item.done",
        "item": {
            "type": "message",
            "role": "assistant",
            "content": [{ "type": "output_text", "text": text }]
        }
    });
    fs::write(
        path,
        format!(
            "event: response.created\n\
             data: {{\"type\":\"response.created\",\"response\":{{\"id\":\"resp1\"}}}}\n\n\
             event: response.output_item.done\n\
             data: {event}\n\n\
             event: response.completed\n\
             data: {{\"type\":\"response.completed\",\"response\":{{\"id\":\"resp1\",\"output\":[]}}}}\n\n",
        ),
    )
}

fn write_config(path: &Path, codex_path: &Path) -> std::io::Result<()> {
    fs::write(
        path,
        format!(
            "backend = \"codex\"\n\
             backend-order = [\"codex\"]\n\
             diff-source = \"auto\"\n\
             pr-base-branch = \"main\"\n\
             codex-path = \"{}\"\n\
             codex-model = \"gpt-5.4-mini\"\n\
             codex-pr-model = \"gpt-5.4\"\n\
             codex-cheap-model = \"gpt-5.4-mini\"\n",
            toml_escape(&codex_path.to_string_lossy()),
        ),
    )
}

fn prepare_staged_repo(path: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    seed_repo(path)?;
    fs::write(
        path.join("src/app.ts"),
        "export function add(left: number, right: number): number {\n  return left + right\n}\n\nexport function subtract(left: number, right: number): number {\n  return left - right\n}\n",
    )?;
    fs::write(path.join("docs/notes.md"), "- add subtract helper\n")?;
    run_git(path, &["add", "src/app.ts", "docs/notes.md"])?;
    Ok(path.to_path_buf())
}

fn prepare_branch_repo(path: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    seed_repo(path)?;
    fs::write(
        path.join("src/app.ts"),
        "export function add(left: number, right: number): number {\n  return left + right\n}\n\nexport function subtract(left: number, right: number): number {\n  return left - right\n}\n",
    )?;
    fs::write(path.join("docs/notes.md"), "- add subtract helper\n")?;
    run_git(path, &["add", "src/app.ts", "docs/notes.md"])?;
    run_git(path, &["commit", "-q", "-m", "feat: add subtract helper"])?;
    Ok(path.to_path_buf())
}

fn seed_repo(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(path.join("src"))?;
    fs::create_dir_all(path.join("docs"))?;
    run_git(path, &["init", "-q"])?;
    run_git(path, &["config", "user.name", "OpenCodeCommit Perf"])?;
    run_git(path, &["config", "user.email", "perf@example.com"])?;
    fs::write(
        path.join("src/app.ts"),
        "export function add(left: number, right: number): number {\n  return left + right\n}\n",
    )?;
    fs::write(path.join("README.md"), "# OpenCodeCommit Perf Fixture\n")?;
    run_git(path, &["add", "README.md", "src/app.ts"])?;
    run_git(path, &["commit", "-q", "-m", "chore: seed perf fixture"])?;
    run_git(path, &["branch", "-M", "main"])?;
    run_git(path, &["checkout", "-q", "-b", "feature/codex-perf"])?;
    Ok(())
}

fn run_git(repo: &Path, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let status = Command::new("git")
        .current_dir(repo)
        .args(args)
        .env("GIT_AUTHOR_NAME", "OpenCodeCommit Perf")
        .env("GIT_AUTHOR_EMAIL", "perf@example.com")
        .env("GIT_COMMITTER_NAME", "OpenCodeCommit Perf")
        .env("GIT_COMMITTER_EMAIL", "perf@example.com")
        .status()?;
    if !status.success() {
        return Err(format!("git {args:?} failed with status {status}").into());
    }
    Ok(())
}

fn git_stdout(repo: &Path, args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git").current_dir(repo).args(args).output()?;
    if !output.status.success() {
        return Err(format!(
            "git {args:?} failed with status {:?}\nstderr:\n{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr),
        )
        .into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(prefix: &str) -> std::io::Result<Self> {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = env::temp_dir().join(format!("{prefix}-{}-{ts}", std::process::id()));
        fs::create_dir_all(&path)?;
        Ok(Self { path })
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn median(mut samples: Vec<Duration>) -> Duration {
    samples.sort_unstable();
    samples[samples.len() / 2]
}

fn occ_bin() -> PathBuf {
    env::var_os("CARGO_BIN_EXE_occ")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_root().join("target/debug/occ"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn ensure_file(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !path.is_file() {
        return Err(format!("expected file to exist: {}", path.display()).into());
    }
    Ok(())
}

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn env_usize(name: &str, fallback: usize) -> usize {
    env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(fallback)
}

fn env_f64(name: &str, fallback: f64) -> f64 {
    env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(fallback)
}

fn toml_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn millis(duration: Duration) -> u128 {
    duration.as_millis()
}
