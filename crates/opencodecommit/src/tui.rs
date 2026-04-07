use std::io::{self, IsTerminal, Write as _};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use opencodecommit::config::Config;
use opencodecommit::sensitive::SensitiveReport;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::{Frame, Terminal};

use crate::actions::{
    self, ActionError, BranchPreview, CommitPreview, CommitRequest, HookOperation, PrPreview,
    RepoSummary,
};

type TuiTerminal = Terminal<CrosstermBackend<io::Stdout>>;

// ── Output panel content ──

#[derive(Debug, Clone)]
enum OutputContent {
    CommitMessage { preview: CommitPreview },
    SensitiveWarning { report: SensitiveReport },
    BranchPreview { preview: BranchPreview },
    PrPreview { preview: PrPreview },
    HookConfirm { operation: HookOperation },
}

// ── Button definitions ──

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ButtonId {
    Commit,        // 1
    Branch,        // 2
    Pr,            // 3
    InstallHook,   // 4
    UninstallHook, // 5
    Quit,          // 0
}

impl ButtonId {
    const ALL: [ButtonId; 6] = [
        ButtonId::Commit,
        ButtonId::Branch,
        ButtonId::Pr,
        ButtonId::InstallHook,
        ButtonId::UninstallHook,
        ButtonId::Quit,
    ];

    fn number(self) -> char {
        match self {
            ButtonId::Commit => '1',
            ButtonId::Branch => '2',
            ButtonId::Pr => '3',
            ButtonId::InstallHook => '4',
            ButtonId::UninstallHook => '5',
            ButtonId::Quit => '0',
        }
    }

    fn label(self) -> &'static str {
        match self {
            ButtonId::Commit => "Commit",
            ButtonId::Branch => "Branch",
            ButtonId::Pr => "PR",
            ButtonId::InstallHook => "Install Hook",
            ButtonId::UninstallHook => "Uninstall Hook",
            ButtonId::Quit => "Quit",
        }
    }

    fn description(self, app: &App) -> &'static str {
        match self {
            ButtonId::Commit => {
                if app.sensitive_blocked {
                    "Sensitive content found. Allow to continue or remove secrets."
                } else {
                    "Generate a commit message from the current diff using AI"
                }
            }
            ButtonId::Branch => "Generate a branch name from the current diff",
            ButtonId::Pr => "Generate a PR title and body from the current diff",
            ButtonId::InstallHook => {
                if app.hook_installed {
                    "Hook is already installed"
                } else {
                    "Install the prepare-commit-msg hook to auto-generate commit messages"
                }
            }
            ButtonId::UninstallHook => {
                if app.hook_installed {
                    "Remove the prepare-commit-msg hook"
                } else {
                    "Hook is not installed"
                }
            }
            ButtonId::Quit => "Exit the TUI",
        }
    }

    /// Whether this button should show its label (expanded) or just the number (collapsed).
    fn is_expanded(self, _app: &App) -> bool {
        match self {
            ButtonId::Commit | ButtonId::Branch | ButtonId::Pr | ButtonId::Quit => true,
            ButtonId::InstallHook | ButtonId::UninstallHook => false, // always collapsed
        }
    }

    /// Whether this button can be activated.
    fn is_available(self, app: &App) -> bool {
        match self {
            ButtonId::Commit => !app.sensitive_blocked,
            ButtonId::Branch | ButtonId::Pr | ButtonId::Quit => true,
            ButtonId::InstallHook => !app.hook_installed,
            ButtonId::UninstallHook => app.hook_installed,
        }
    }
}

// ── Pending jobs ──

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingJob {
    GeneratingCommit,
    ShorteningCommit,
    ApplyingCommit,
    GeneratingBranch,
    CreatingBranch,
    GeneratingPr,
    RunningHook,
    SubmittingPr,
}

impl PendingJob {
    fn label(self) -> &'static str {
        match self {
            PendingJob::GeneratingCommit => "Generating commit message",
            PendingJob::ShorteningCommit => "Shortening commit message",
            PendingJob::ApplyingCommit => "Committing changes",
            PendingJob::GeneratingBranch => "Generating branch name",
            PendingJob::CreatingBranch => "Creating branch",
            PendingJob::GeneratingPr => "Generating PR preview",
            PendingJob::RunningHook => "Updating git hook",
            PendingJob::SubmittingPr => "Submitting pull request",
        }
    }
}

// ── Notice ──

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NoticeKind {
    Info,
    Error,
}

#[derive(Clone, Debug)]
struct Notice {
    kind: NoticeKind,
    message: String,
}

// ── Worker messages ──

enum WorkerMessage {
    CommitGenerated(actions::Result<CommitPreview>),
    CommitShortened(actions::Result<CommitPreview>),
    CommitApplied(actions::Result<actions::CommitResult>),
    BranchGenerated(actions::Result<BranchPreview>),
    BranchCreated(actions::Result<actions::BranchResult>),
    PrGenerated(actions::Result<PrPreview>),
    HookRan(actions::Result<String>),
    PrSubmitted(Result<String, String>),
}

// ── App state ──

#[derive(Debug)]
struct App {
    config: Config,
    repo: RepoSummary,
    diff_text: String,
    diff_scroll: u16,
    output: Option<OutputContent>,
    output_scroll: u16,
    focused_button: usize,
    pending: Option<PendingJob>,
    spinner_tick: usize,
    notice: Option<Notice>,
    hook_installed: bool,
    sensitive_blocked: bool,
    should_quit: bool,
}

impl App {
    fn new(config: Config, repo: RepoSummary, diff_text: String, hook_installed: bool) -> Self {
        Self {
            config,
            repo,
            diff_text,
            diff_scroll: 0,
            output: None,
            output_scroll: 0,
            focused_button: 0,
            pending: None,
            spinner_tick: 0,
            notice: None,
            hook_installed,
            sensitive_blocked: false,
            should_quit: false,
        }
    }

    fn set_info(&mut self, message: impl Into<String>) {
        self.notice = Some(Notice {
            kind: NoticeKind::Info,
            message: message.into(),
        });
    }

    fn set_error(&mut self, message: impl Into<String>) {
        self.notice = Some(Notice {
            kind: NoticeKind::Error,
            message: message.into(),
        });
    }

    fn refresh_repo(&mut self) {
        if let Ok(summary) = actions::load_repo_summary(&self.config) {
            self.repo = summary;
        }
    }

    fn focused_button_id(&self) -> ButtonId {
        ButtonId::ALL[self.focused_button]
    }
}

// ── Terminal guard ──

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = restore_terminal();
    }
}

fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}

fn install_panic_cleanup_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = restore_terminal();
        previous(panic_info);
    }));
}

fn detect_hook_installed(repo: &RepoSummary) -> bool {
    let git_dir = repo.repo_root.join(".git");
    let hook_path = git_dir.join("hooks").join("prepare-commit-msg");
    if let Ok(content) = std::fs::read_to_string(&hook_path) {
        content.contains("opencodecommit")
    } else {
        false
    }
}

// ── Entry point ──

pub fn run(config: Config) -> Result<(), ActionError> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(ActionError::NonTty(
            "`occ tui` requires an interactive terminal.".to_owned(),
        ));
    }

    let repo = actions::load_repo_summary(&config)?;
    let diff_text = match opencodecommit::git::get_diff(config.diff_source, &repo.repo_root) {
        Ok(diff) => diff,
        Err(_) => String::new(),
    };
    let hook_installed = detect_hook_installed(&repo);

    install_panic_cleanup_hook();

    let _guard = TerminalGuard::enter().map_err(|err| {
        ActionError::InvalidInput(format!("failed to initialize terminal UI: {err}"))
    })?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend).map_err(|err| {
        ActionError::InvalidInput(format!("failed to create terminal backend: {err}"))
    })?;
    terminal
        .clear()
        .map_err(|err| ActionError::InvalidInput(format!("failed to clear terminal: {err}")))?;

    let (tx, rx) = mpsc::channel();
    let mut app = App::new(config, repo, diff_text, hook_installed);

    let result = event_loop(&mut terminal, &mut app, &tx, &rx);
    let _ = terminal.show_cursor();
    result.map_err(|err| ActionError::InvalidInput(format!("terminal UI error: {err}")))
}

fn event_loop(
    terminal: &mut TuiTerminal,
    app: &mut App,
    tx: &Sender<WorkerMessage>,
    rx: &Receiver<WorkerMessage>,
) -> io::Result<()> {
    loop {
        while let Ok(message) = rx.try_recv() {
            apply_worker_message(app, message);
        }

        terminal.draw(|frame| render(frame, app))?;
        if app.should_quit {
            break;
        }

        if event::poll(Duration::from_millis(80))?
            && let Event::Key(key) = event::read()?
        {
            handle_key(app, key, tx);
        }

        app.spinner_tick = app.spinner_tick.wrapping_add(1);
    }

    Ok(())
}

// ── Key handling ──

fn handle_key(app: &mut App, key: KeyEvent, tx: &Sender<WorkerMessage>) {
    // Global quit
    if key.code == KeyCode::Char('q')
        || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
    {
        app.should_quit = true;
        return;
    }

    // Block input while a job is running
    if app.pending.is_some() {
        return;
    }

    // Handle output-panel-specific keys first
    if handle_output_panel_key(app, key, tx) {
        return;
    }

    match key.code {
        // Number key direct activation
        KeyCode::Char(ch @ '0'..='5') => {
            if let Some(btn) = ButtonId::ALL.iter().find(|b| b.number() == ch) {
                activate_button(app, *btn, tx);
            }
        }

        // Tab / Shift-Tab to cycle focus
        KeyCode::Tab => {
            app.focused_button = (app.focused_button + 1) % ButtonId::ALL.len();
            app.notice = None;
        }
        KeyCode::BackTab => {
            if app.focused_button == 0 {
                app.focused_button = ButtonId::ALL.len() - 1;
            } else {
                app.focused_button -= 1;
            }
            app.notice = None;
        }

        // Left/Right to move focus
        KeyCode::Left => {
            if app.focused_button == 0 {
                app.focused_button = ButtonId::ALL.len() - 1;
            } else {
                app.focused_button -= 1;
            }
            app.notice = None;
        }
        KeyCode::Right => {
            app.focused_button = (app.focused_button + 1) % ButtonId::ALL.len();
            app.notice = None;
        }

        // Up/Down: scroll output panel if content exists, otherwise scroll diff
        KeyCode::Up | KeyCode::Char('k') => {
            if app.output.is_some() {
                app.output_scroll = app.output_scroll.saturating_sub(1);
            } else {
                app.diff_scroll = app.diff_scroll.saturating_sub(1);
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.output.is_some() {
                app.output_scroll = app.output_scroll.saturating_add(1);
            } else {
                app.diff_scroll = app.diff_scroll.saturating_add(1);
            }
        }

        // Enter activates focused button
        KeyCode::Enter => {
            let btn = app.focused_button_id();
            activate_button(app, btn, tx);
        }

        // Esc clears output panel / notice
        KeyCode::Esc => {
            if app.output.is_some() {
                app.output = None;
                app.output_scroll = 0;
                app.sensitive_blocked = false;
            } else {
                app.notice = None;
            }
        }

        _ => {}
    }
}

/// Handle keys specific to output panel interactive elements.
/// Returns true if the key was consumed.
fn handle_output_panel_key(app: &mut App, key: KeyEvent, tx: &Sender<WorkerMessage>) -> bool {
    match &app.output {
        Some(OutputContent::CommitMessage { .. }) => {
            if key.code == KeyCode::Char('c') || key.code == KeyCode::Enter {
                spawn_apply_commit(app, tx);
                return true;
            }
            if key.code == KeyCode::Char('s') {
                spawn_shorten_commit(app, tx);
                return true;
            }
            if key.code == KeyCode::Char('r') {
                spawn_generate_commit(app, tx, false);
                return true;
            }
        }
        Some(OutputContent::SensitiveWarning { .. }) => {
            if key.code == KeyCode::Char('a') || key.code == KeyCode::Enter {
                app.sensitive_blocked = false;
                app.output = None;
                app.output_scroll = 0;
                spawn_generate_commit(app, tx, true);
                return true;
            }
        }
        Some(OutputContent::BranchPreview { .. }) => {
            if key.code == KeyCode::Char('c') || key.code == KeyCode::Enter {
                spawn_create_branch(app, tx);
                return true;
            }
            if key.code == KeyCode::Char('r') {
                spawn_generate_branch(app, tx);
                return true;
            }
        }
        Some(OutputContent::PrPreview { .. }) => {
            if key.code == KeyCode::Char('s') || key.code == KeyCode::Enter {
                spawn_submit_pr(app, tx);
                return true;
            }
            if key.code == KeyCode::Char('r') {
                spawn_generate_pr(app, tx);
                return true;
            }
        }
        Some(OutputContent::HookConfirm { operation }) => {
            let op = *operation;
            if key.code == KeyCode::Char('y') || key.code == KeyCode::Enter {
                spawn_run_hook(app, tx, op);
                return true;
            }
            if key.code == KeyCode::Char('n') || key.code == KeyCode::Esc {
                app.output = None;
                app.output_scroll = 0;
                return true;
            }
        }
        _ => {}
    }
    false
}

fn activate_button(app: &mut App, btn: ButtonId, tx: &Sender<WorkerMessage>) {
    if !btn.is_available(app) {
        app.set_error(btn.description(app));
        return;
    }

    match btn {
        ButtonId::Commit => spawn_generate_commit(app, tx, false),
        ButtonId::Branch => spawn_generate_branch(app, tx),
        ButtonId::Pr => spawn_generate_pr(app, tx),
        ButtonId::InstallHook => {
            app.output = Some(OutputContent::HookConfirm {
                operation: HookOperation::Install,
            });
            app.output_scroll = 0;
        }
        ButtonId::UninstallHook => {
            app.output = Some(OutputContent::HookConfirm {
                operation: HookOperation::Uninstall,
            });
            app.output_scroll = 0;
        }
        ButtonId::Quit => app.should_quit = true,
    }
}

// ── Worker thread spawners ──

fn spawn_generate_commit(app: &mut App, tx: &Sender<WorkerMessage>, allow_sensitive: bool) {
    app.pending = Some(PendingJob::GeneratingCommit);
    let config = app.config.clone();
    let tx = tx.clone();
    std::thread::spawn(move || {
        let request = CommitRequest {
            refine: None,
            feedback: None,
            stdin_diff: None,
            allow_sensitive,
        };
        let _ = tx.send(WorkerMessage::CommitGenerated(
            actions::generate_commit_preview(&config, &request),
        ));
    });
}

fn spawn_shorten_commit(app: &mut App, tx: &Sender<WorkerMessage>) {
    let Some(OutputContent::CommitMessage { preview }) = &app.output else {
        app.set_error("Generate a commit message first.");
        return;
    };

    app.pending = Some(PendingJob::ShorteningCommit);
    let config = app.config.clone();
    let message = preview.message.clone();
    let tx = tx.clone();
    std::thread::spawn(move || {
        let request = CommitRequest {
            refine: Some(message),
            feedback: None,
            stdin_diff: None,
            allow_sensitive: false,
        };
        let _ = tx.send(WorkerMessage::CommitShortened(
            actions::generate_commit_preview(&config, &request),
        ));
    });
}

fn spawn_apply_commit(app: &mut App, tx: &Sender<WorkerMessage>) {
    let Some(OutputContent::CommitMessage { preview }) = &app.output else {
        app.set_error("Generate a commit message first.");
        return;
    };

    app.pending = Some(PendingJob::ApplyingCommit);
    let message = preview.message.clone();
    let tx = tx.clone();
    std::thread::spawn(move || {
        let _ = tx.send(WorkerMessage::CommitApplied(actions::commit_message(
            &message, false,
        )));
    });
}

fn spawn_generate_branch(app: &mut App, tx: &Sender<WorkerMessage>) {
    app.pending = Some(PendingJob::GeneratingBranch);
    let config = app.config.clone();
    let tx = tx.clone();
    std::thread::spawn(move || {
        let _ = tx.send(WorkerMessage::BranchGenerated(
            actions::generate_branch_preview(&config, None, config.branch_mode),
        ));
    });
}

fn spawn_create_branch(app: &mut App, tx: &Sender<WorkerMessage>) {
    let Some(OutputContent::BranchPreview { preview }) = &app.output else {
        return;
    };

    app.pending = Some(PendingJob::CreatingBranch);
    let name = preview.name.clone();
    let tx = tx.clone();
    std::thread::spawn(move || {
        let _ = tx.send(WorkerMessage::BranchCreated(actions::create_branch(&name)));
    });
}

fn spawn_generate_pr(app: &mut App, tx: &Sender<WorkerMessage>) {
    app.pending = Some(PendingJob::GeneratingPr);
    let config = app.config.clone();
    let tx = tx.clone();
    std::thread::spawn(move || {
        let _ = tx.send(WorkerMessage::PrGenerated(actions::generate_pr_preview(
            &config,
        )));
    });
}

fn spawn_submit_pr(app: &mut App, tx: &Sender<WorkerMessage>) {
    let Some(OutputContent::PrPreview { preview }) = &app.output else {
        return;
    };

    app.pending = Some(PendingJob::SubmittingPr);
    let title = preview.title.clone();
    let body = preview.body.clone();
    let tx = tx.clone();
    std::thread::spawn(move || {
        let result = std::process::Command::new("gh")
            .args(["pr", "create", "--title", &title, "--body", &body])
            .output();

        let msg = match result {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                Ok(stdout)
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
                Err(format!("gh pr create failed: {stderr}"))
            }
            Err(_) => {
                // gh not available — try clipboard
                if copy_to_clipboard(&format!("{title}\n\n{body}")) {
                    Err("gh not found. PR content copied to clipboard.".to_owned())
                } else {
                    Err(
                        "gh not found and clipboard copy failed. Select text to copy manually."
                            .to_owned(),
                    )
                }
            }
        };
        let _ = tx.send(WorkerMessage::PrSubmitted(msg));
    });
}

fn copy_to_clipboard(text: &str) -> bool {
    for cmd in &[
        "xclip -selection clipboard",
        "xsel --clipboard --input",
        "wl-copy",
        "pbcopy",
    ] {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if let Ok(mut child) = std::process::Command::new(parts[0])
            .args(&parts[1..])
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            if let Some(stdin) = child.stdin.as_mut() {
                if stdin.write_all(text.as_bytes()).is_ok() {
                    drop(child.stdin.take());
                    if child.wait().is_ok_and(|s| s.success()) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn spawn_run_hook(app: &mut App, tx: &Sender<WorkerMessage>, operation: HookOperation) {
    app.pending = Some(PendingJob::RunningHook);
    app.output = None;
    app.output_scroll = 0;
    let tx = tx.clone();
    std::thread::spawn(move || {
        let _ = tx.send(WorkerMessage::HookRan(actions::run_hook(operation)));
    });
}

// ── Worker message handling ──

fn apply_worker_message(app: &mut App, message: WorkerMessage) {
    match message {
        WorkerMessage::CommitGenerated(result) => {
            app.pending = None;
            match result {
                Ok(preview) => {
                    app.set_info("Generated commit message.");
                    app.output = Some(OutputContent::CommitMessage { preview });
                    app.output_scroll = 0;
                    app.focused_button = 0; // focus Commit button
                }
                Err(ActionError::SensitiveContent(report)) => {
                    app.sensitive_blocked = true;
                    app.output = Some(OutputContent::SensitiveWarning { report });
                    app.output_scroll = 0;
                }
                Err(err) => app.set_error(err.to_string()),
            }
        }
        WorkerMessage::CommitShortened(result) => {
            app.pending = None;
            match result {
                Ok(preview) => {
                    app.set_info("Shortened commit message.");
                    app.output = Some(OutputContent::CommitMessage { preview });
                    app.output_scroll = 0;
                    app.focused_button = 2;
                }
                Err(err) => app.set_error(err.to_string()),
            }
        }
        WorkerMessage::CommitApplied(result) => {
            app.pending = None;
            match result {
                Ok(result) => {
                    let summary = result
                        .git_output
                        .lines()
                        .next()
                        .unwrap_or(&result.git_output);
                    if result.staged_all {
                        app.set_info(format!("Committed: {summary} (staged all changes first)"));
                    } else {
                        app.set_info(format!("Committed: {summary}"));
                    }
                    app.output = None;
                    app.output_scroll = 0;
                    app.refresh_repo();
                    // Refresh diff
                    app.diff_text = opencodecommit::git::get_diff(
                        app.config.diff_source,
                        &app.repo.repo_root,
                    )
                    .unwrap_or_default();
                    app.diff_scroll = 0;
                }
                Err(err) => app.set_error(err.to_string()),
            }
        }
        WorkerMessage::BranchGenerated(result) => {
            app.pending = None;
            match result {
                Ok(preview) => {
                    app.set_info("Generated branch name.");
                    app.output = Some(OutputContent::BranchPreview { preview });
                    app.output_scroll = 0;
                }
                Err(err) => app.set_error(err.to_string()),
            }
        }
        WorkerMessage::BranchCreated(result) => {
            app.pending = None;
            match result {
                Ok(result) => {
                    app.set_info(format!("Switched to new branch '{}'.", result.name));
                    app.output = None;
                    app.output_scroll = 0;
                    app.refresh_repo();
                }
                Err(err) => app.set_error(err.to_string()),
            }
        }
        WorkerMessage::PrGenerated(result) => {
            app.pending = None;
            match result {
                Ok(preview) => {
                    app.set_info("Generated PR preview.");
                    app.output = Some(OutputContent::PrPreview { preview });
                    app.output_scroll = 0;
                }
                Err(err) => app.set_error(err.to_string()),
            }
        }
        WorkerMessage::HookRan(result) => {
            app.pending = None;
            match result {
                Ok(message) => {
                    app.set_info(message);
                    app.hook_installed = detect_hook_installed(&app.repo);
                }
                Err(err) => app.set_error(err.to_string()),
            }
        }
        WorkerMessage::PrSubmitted(result) => {
            app.pending = None;
            match result {
                Ok(url) => app.set_info(format!("PR created: {url}")),
                Err(msg) => app.set_error(msg),
            }
        }
    }
}

// ── Rendering ──

fn style_diff_line(line: &str) -> Line<'_> {
    if line.starts_with('+') && !line.starts_with("+++") {
        Line::styled(line, Style::default().fg(Color::Green))
    } else if line.starts_with('-') && !line.starts_with("---") {
        Line::styled(line, Style::default().fg(Color::Red))
    } else if line.starts_with("@@") {
        Line::styled(line, Style::default().fg(Color::Cyan))
    } else if line.starts_with("diff ") || line.starts_with("index ") {
        Line::styled(line, Style::default().fg(Color::Yellow))
    } else if line.starts_with("--- ") || line.starts_with("+++ ") {
        Line::styled(line, Style::default().fg(Color::Yellow))
    } else {
        Line::styled(line, Style::default().fg(Color::DarkGray))
    }
}

fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let output_height = output_panel_height(app);

    let chunks = Layout::vertical([
        Constraint::Length(1),             // header
        Constraint::Min(5),               // diff viewer
        Constraint::Length(output_height), // output panel (0 when empty)
        Constraint::Length(1),             // button bar
        Constraint::Length(1),             // description line
    ])
    .split(area);

    render_header(frame, chunks[0], app);
    render_diff(frame, chunks[1], app);
    if output_height > 0 {
        render_output_panel(frame, chunks[2], app);
    }
    render_button_bar(frame, chunks[3], app);
    render_description(frame, chunks[4], app);

    if let Some(pending) = app.pending {
        render_pending_overlay(frame, area, pending, app.spinner_tick);
    }
}

fn output_panel_height(app: &App) -> u16 {
    match &app.output {
        None => 0,
        Some(OutputContent::CommitMessage { preview }) => {
            let lines = preview.message.lines().count() as u16;
            // border(2) + label + blank + message + blank + metadata + blank + buttons
            (lines + 8).min(16)
        }
        Some(OutputContent::SensitiveWarning { report }) => {
            let lines = report.findings.len() as u16;
            (lines + 5).min(15) // border + header + findings + button
        }
        Some(OutputContent::BranchPreview { .. }) => 7,
        Some(OutputContent::PrPreview { preview }) => {
            let lines =
                preview.title.lines().count() as u16 + preview.body.lines().count() as u16;
            (lines + 6).min(20) // border + label + title + body + button
        }
        Some(OutputContent::HookConfirm { .. }) => 5,
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let spans = vec![
        Span::styled(
            "OpenCodeCommit",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            app.repo.repo_root.display().to_string(),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw("  branch: "),
        Span::styled(&app.repo.branch, Style::default().fg(Color::Green)),
        Span::raw(format!(
            "  staged: {}  unstaged: {}",
            app.repo.staged_files, app.repo.unstaged_files
        )),
    ];
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_diff(frame: &mut Frame, area: Rect, app: &App) {
    if app.diff_text.is_empty() {
        let msg = Paragraph::new("No changes detected.")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).title("Diff"));
        frame.render_widget(msg, area);
        return;
    }

    let lines: Vec<Line> = app.diff_text.lines().map(style_diff_line).collect();
    let diff = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Diff"))
        .scroll((app.diff_scroll, 0));
    frame.render_widget(diff, area);
}

fn render_output_panel(frame: &mut Frame, area: Rect, app: &App) {
    match &app.output {
        None => {}
        Some(OutputContent::CommitMessage { preview }) => {
            render_commit_output(frame, area, preview, app.output_scroll);
        }
        Some(OutputContent::SensitiveWarning { report }) => {
            render_sensitive_output(frame, area, report);
        }
        Some(OutputContent::BranchPreview { preview }) => {
            render_branch_output(frame, area, preview);
        }
        Some(OutputContent::PrPreview { preview }) => {
            render_pr_output(frame, area, preview, app.output_scroll);
        }
        Some(OutputContent::HookConfirm { operation }) => {
            render_hook_confirm(frame, area, *operation);
        }
    }
}

fn render_commit_output(frame: &mut Frame, area: Rect, preview: &CommitPreview, scroll: u16) {
    let mut lines = vec![
        Line::styled(
            "COMMIT MESSAGE PREVIEW",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
        Line::raw(""),
    ];
    for line in preview.message.lines() {
        lines.push(Line::raw(line.to_owned()));
    }
    lines.push(Line::raw(""));
    lines.push(Line::styled(
        format!(
            "provider: {}  files: {}  {:.1}s",
            preview.provider,
            preview.files_analyzed,
            preview.duration_ms as f64 / 1000.0
        ),
        Style::default().fg(Color::DarkGray),
    ));
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled(
            "[c Commit]",
            Style::default().fg(Color::Black).bg(Color::Green),
        ),
        Span::raw("  "),
        Span::styled("[s Shorten]", Style::default().fg(Color::White)),
        Span::raw("  "),
        Span::styled("[r Regenerate]", Style::default().fg(Color::White)),
    ]));

    let widget = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        )
        .scroll((scroll, 0));
    frame.render_widget(widget, area);
}

fn render_sensitive_output(frame: &mut Frame, area: Rect, report: &SensitiveReport) {
    let mut lines = vec![Line::styled(
        "SENSITIVE CONTENT DETECTED",
        Style::default()
            .fg(Color::Red)
            .add_modifier(Modifier::BOLD),
    )];

    for finding in &report.findings {
        let location = match finding.line_number {
            Some(line) => format!("{}:{}", finding.file_path, line),
            None => finding.file_path.clone(),
        };
        lines.push(Line::from(vec![
            Span::styled(location, Style::default().fg(Color::Yellow)),
            Span::raw(" · "),
            Span::styled(&finding.preview, Style::default().fg(Color::DarkGray)),
        ]));
    }

    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled(
            "[a Allow & Continue]",
            Style::default().fg(Color::Black).bg(Color::Red),
        ),
        Span::raw("  Generation blocked until resolved or allowed"),
    ]));

    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red)),
    );
    frame.render_widget(widget, area);
}

fn render_branch_output(frame: &mut Frame, area: Rect, preview: &BranchPreview) {
    let lines = vec![
        Line::styled(
            "BRANCH NAME PREVIEW",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
        Line::raw(""),
        Line::styled(
            &preview.name,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Line::raw(""),
        Line::from(vec![
            Span::styled(
                "[c Create Branch]",
                Style::default().fg(Color::Black).bg(Color::Cyan),
            ),
            Span::raw("  "),
            Span::styled("[r Regenerate]", Style::default().fg(Color::White)),
        ]),
    ];

    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(widget, area);
}

fn render_pr_output(frame: &mut Frame, area: Rect, preview: &PrPreview, scroll: u16) {
    let mut lines = vec![Line::styled(
        "PR PREVIEW — select text to copy, or press Submit PR",
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )];

    lines.push(Line::styled(
        format!("## {}", preview.title),
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
    ));
    lines.push(Line::raw(""));

    for line in preview.body.lines() {
        lines.push(Line::raw(line.to_owned()));
    }

    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled(
            "[s Submit PR]",
            Style::default().fg(Color::Black).bg(Color::Magenta),
        ),
        Span::raw("  "),
        Span::styled("[r Regenerate]", Style::default().fg(Color::White)),
    ]));

    let widget = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta)),
        )
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    frame.render_widget(widget, area);
}

fn render_hook_confirm(frame: &mut Frame, area: Rect, operation: HookOperation) {
    let (action, color) = match operation {
        HookOperation::Install => ("Install", Color::Cyan),
        HookOperation::Uninstall => ("Uninstall", Color::Yellow),
    };

    let lines = vec![
        Line::styled(
            format!("{action} the prepare-commit-msg hook?"),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Line::raw(""),
        Line::from(vec![
            Span::styled(
                "[y Yes]",
                Style::default().fg(Color::Black).bg(color),
            ),
            Span::raw("  "),
            Span::styled("[n No]", Style::default().fg(Color::White)),
        ]),
    ];

    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(color)),
    );
    frame.render_widget(widget, area);
}

fn render_button_bar(frame: &mut Frame, area: Rect, app: &App) {
    let mut spans: Vec<Span> = Vec::new();

    for (i, &btn) in ButtonId::ALL.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" "));
        }

        let focused = i == app.focused_button;
        let available = btn.is_available(app);
        let show_label = btn.is_expanded(app) || focused;

        let text = if show_label {
            format!("[{} {}]", btn.number(), btn.label())
        } else {
            format!("[{}]", btn.number())
        };

        let style = if focused && available {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else if focused && !available {
            Style::default().fg(Color::Black).bg(Color::DarkGray)
        } else if !available {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::White)
        };

        spans.push(Span::styled(text, style));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_description(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(notice) = &app.notice {
        let color = match notice.kind {
            NoticeKind::Info => Color::Green,
            NoticeKind::Error => Color::Red,
        };
        frame.render_widget(
            Paragraph::new(Span::styled(&notice.message, Style::default().fg(color))),
            area,
        );
    } else {
        let desc = app.focused_button_id().description(app);
        frame.render_widget(
            Paragraph::new(Span::styled(desc, Style::default().fg(Color::Cyan))),
            area,
        );
    }
}

fn render_pending_overlay(frame: &mut Frame, area: Rect, pending: PendingJob, tick: usize) {
    let overlay = centered_rect(48, 5, area);
    let spinner = ["-", "\\", "|", "/"][tick % 4];
    let message = format!("{spinner} {}", pending.label());

    frame.render_widget(Clear, overlay);
    let widget = Paragraph::new(message)
        .block(Block::default().title("Working").borders(Borders::ALL));
    frame.render_widget(widget, overlay);
}

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(height),
        Constraint::Fill(1),
    ])
    .split(area);
    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(vertical[1])[1]
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;
    use opencodecommit::config::CliBackend;
    use ratatui::backend::TestBackend;

    fn test_repo() -> RepoSummary {
        RepoSummary {
            repo_name: "demo".to_owned(),
            repo_root: "/tmp/demo".into(),
            branch: "main".to_owned(),
            staged_files: 2,
            unstaged_files: 1,
            active_language: "English".to_owned(),
            backend_label: "Codex CLI",
            backend_path: Some("/usr/bin/codex".into()),
            backend_error: None,
        }
    }

    fn test_diff() -> String {
        "diff --git a/src/main.rs b/src/main.rs\n\
         --- a/src/main.rs\n\
         +++ b/src/main.rs\n\
         @@ -1,3 +1,4 @@\n\
          fn main() {\n\
         -    println!(\"old\");\n\
         +    println!(\"new\");\n\
         +    extra();\n\
          }\n"
            .to_owned()
    }

    fn test_app() -> App {
        let mut config = Config::default();
        config.backend = CliBackend::Codex;
        App::new(config, test_repo(), test_diff(), false)
    }

    fn render_text(app: &App, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, app)).unwrap();
        let buffer = terminal.backend().buffer();
        let mut lines = Vec::new();
        for y in 0..height {
            let mut line = String::new();
            for x in 0..width {
                line.push_str(buffer[(x, y)].symbol());
            }
            lines.push(line);
        }
        lines.join("\n")
    }

    #[test]
    fn initial_state_shows_header_diff_and_buttons() {
        let app = test_app();
        let text = render_text(&app, 100, 24);
        assert!(text.contains("OpenCodeCommit"), "missing header");
        assert!(text.contains("/tmp/demo"), "missing repo path");
        assert!(text.contains("main"), "missing branch");
        assert!(text.contains("1 Commit"), "missing Commit button");
        assert!(text.contains("2 Branch"), "missing Branch button");
        assert!(text.contains("3 PR"), "missing PR button");
    }

    #[test]
    fn no_output_panel_initially() {
        let app = test_app();
        let text = render_text(&app, 100, 24);
        assert!(
            !text.contains("COMMIT MESSAGE"),
            "should not show commit panel"
        );
        assert!(
            !text.contains("SENSITIVE"),
            "should not show sensitive panel"
        );
    }

    #[test]
    fn commit_message_shows_output_panel() {
        let mut app = test_app();
        app.output = Some(OutputContent::CommitMessage {
            preview: CommitPreview {
                message: "feat: add TUI".to_owned(),
                parsed: opencodecommit::response::ParsedCommit {
                    type_name: "feat".to_owned(),
                    message: "add TUI".to_owned(),
                    description: None,
                },
                provider: "codex".to_owned(),
                files_analyzed: 2,
                duration_ms: 500,
                changed_files: vec!["src/main.rs".to_owned()],
                branch: "main".to_owned(),
                diff_origin: crate::actions::DiffOrigin::Staged,
            },
        });
        let text = render_text(&app, 100, 24);
        assert!(text.contains("COMMIT MESSAGE"), "missing commit panel");
        assert!(text.contains("feat: add TUI"), "missing commit message");
        // Panel action buttons rendered inside the output panel
        assert!(text.contains("[c Commit]"), "missing Commit action in panel\n{text}");
        assert!(text.contains("[s Shorten]"), "missing Shorten action in panel\n{text}");
        assert!(text.contains("[r Regenerate]"), "missing Regenerate action in panel\n{text}");
    }

    #[test]
    fn sensitive_warning_shows_panel() {
        let mut app = test_app();
        app.output = Some(OutputContent::SensitiveWarning {
            report: SensitiveReport::from_findings(vec![
                opencodecommit::sensitive::SensitiveFinding {
                    category: "token",
                    rule: "API_KEY",
                    file_path: ".env".to_owned(),
                    line_number: Some(1),
                    preview: "API_KEY=sk-██████".to_owned(),
                },
            ]),
        });
        app.sensitive_blocked = true;
        let text = render_text(&app, 100, 24);
        assert!(text.contains("SENSITIVE"), "missing sensitive header");
        assert!(text.contains(".env:1"), "missing finding location");
        assert!(
            text.contains("Allow & Continue"),
            "missing allow button"
        );
    }

    #[test]
    fn button_focus_cycles() {
        let mut app = test_app();
        let (tx, _rx) = mpsc::channel();

        assert_eq!(app.focused_button, 0);
        handle_key(&mut app, KeyEvent::from(KeyCode::Tab), &tx);
        assert_eq!(app.focused_button, 1);
        handle_key(&mut app, KeyEvent::from(KeyCode::Tab), &tx);
        assert_eq!(app.focused_button, 2);

        handle_key(&mut app, KeyEvent::from(KeyCode::BackTab), &tx);
        assert_eq!(app.focused_button, 1);
    }

    #[test]
    fn diff_scrolls_when_no_output() {
        let mut app = test_app();
        let (tx, _rx) = mpsc::channel();

        assert_eq!(app.diff_scroll, 0);
        handle_key(&mut app, KeyEvent::from(KeyCode::Down), &tx);
        assert_eq!(app.diff_scroll, 1);
        handle_key(&mut app, KeyEvent::from(KeyCode::Up), &tx);
        assert_eq!(app.diff_scroll, 0);
    }

    #[test]
    fn output_scrolls_when_panel_visible() {
        let mut app = test_app();
        app.output = Some(OutputContent::CommitMessage {
            preview: CommitPreview {
                message: "feat: test".to_owned(),
                parsed: opencodecommit::response::ParsedCommit {
                    type_name: "feat".to_owned(),
                    message: "test".to_owned(),
                    description: None,
                },
                provider: "test".to_owned(),
                files_analyzed: 1,
                duration_ms: 100,
                changed_files: vec![],
                branch: "main".to_owned(),
                diff_origin: crate::actions::DiffOrigin::Staged,
            },
        });
        let (tx, _rx) = mpsc::channel();

        assert_eq!(app.output_scroll, 0);
        handle_key(&mut app, KeyEvent::from(KeyCode::Down), &tx);
        assert_eq!(app.output_scroll, 1);
        assert_eq!(app.diff_scroll, 0, "diff should not scroll");
    }

    #[test]
    fn esc_clears_output() {
        let mut app = test_app();
        app.output = Some(OutputContent::BranchPreview {
            preview: BranchPreview {
                name: "feat/test".to_owned(),
            },
        });
        let (tx, _rx) = mpsc::channel();

        handle_key(&mut app, KeyEvent::from(KeyCode::Esc), &tx);
        assert!(app.output.is_none());
    }

    #[test]
    fn pending_overlay_renders() {
        let mut app = test_app();
        app.pending = Some(PendingJob::GeneratingCommit);
        let text = render_text(&app, 100, 24);
        assert!(text.contains("Working"), "missing overlay title");
        assert!(
            text.contains("Generating commit message"),
            "missing overlay text"
        );
    }

    #[test]
    fn hook_buttons_always_collapsed() {
        let app = test_app();
        let text = render_text(&app, 100, 24);
        assert!(text.contains("[4]"), "Install Hook should be collapsed");
        assert!(text.contains("[5]"), "Uninstall Hook should be collapsed");
    }

    #[test]
    fn empty_diff_shows_no_changes() {
        let mut config = Config::default();
        config.backend = CliBackend::Codex;
        let app = App::new(config, test_repo(), String::new(), false);
        let text = render_text(&app, 100, 24);
        assert!(text.contains("No changes detected"));
    }
}
