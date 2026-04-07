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

use opencodecommit::config::CliBackend;

use crate::actions::{
    self, ActionError, BackendProgress, BranchPreview, CommitPreview, CommitRequest, HookOperation, PrContext,
    PrPreview, RepoSummary,
};

type TuiTerminal = Terminal<CrosstermBackend<io::Stdout>>;

// ── Output panel content ──

#[derive(Debug, Clone)]
enum OutputContent {
    CommitMessage { preview: CommitPreview },
    SensitiveWarning { report: SensitiveReport },
    BranchPreview { preview: BranchPreview },
    PrPreview { preview: PrPreview },
    HookMenu,
    HookConfirm { operation: HookOperation },
}

// ── Panel buttons (rendered inside the output panel) ──

/// Returns the labels for panel buttons given the current output content.
fn panel_buttons(output: &OutputContent) -> &'static [&'static str] {
    match output {
        OutputContent::CommitMessage { .. } => &["[c Commit]", "[s Shorten]", "[r Regenerate]"],
        OutputContent::SensitiveWarning { .. } => &["[a Allow & Continue]"],
        OutputContent::BranchPreview { .. } => &["[c Create Branch]", "[r Regenerate]"],
        OutputContent::PrPreview { .. } => &["[s Submit PR]", "[p Copy]", "[r Regenerate]"],
        OutputContent::HookMenu => &["[i Install Hook]", "[u Uninstall Hook]"],
        OutputContent::HookConfirm { .. } => &["[y Yes]", "[n No]"],
    }
}

// ── Bottom bar button definitions ──

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ButtonId {
    Commit,     // 1
    Branch,     // 2
    Pr,         // 3
    SafetyHook, // 4
    Quit,       // 0
}

impl ButtonId {
    const ALL: [ButtonId; 5] = [
        ButtonId::Commit,
        ButtonId::Branch,
        ButtonId::Pr,
        ButtonId::SafetyHook,
        ButtonId::Quit,
    ];

    fn number(self) -> char {
        match self {
            ButtonId::Commit => '1',
            ButtonId::Branch => '2',
            ButtonId::Pr => '3',
            ButtonId::SafetyHook => '4',
            ButtonId::Quit => '0',
        }
    }

    fn label(self) -> &'static str {
        match self {
            ButtonId::Commit => "Commit",
            ButtonId::Branch => "Branch",
            ButtonId::Pr => "PR",
            ButtonId::SafetyHook => "Safety Hook",
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
            ButtonId::SafetyHook => "Install or uninstall the prepare-commit-msg safety hook",
            ButtonId::Quit => "Exit the TUI",
        }
    }

    fn is_available(self, app: &App) -> bool {
        match self {
            ButtonId::Commit => !app.sensitive_blocked,
            _ => true,
        }
    }
}

// ── File sidebar types ──

#[derive(Debug, Clone)]
struct CommitGroup {
    subject: String,
    files: Vec<String>,
}

// ── Focus tracking ──
//
// Focus can be in the sidebar, output panel, or on the bottom bar.
// Tab cycles: sidebar → panel buttons → bar buttons → wrap.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FocusArea {
    Sidebar,
    Panel,
    Bar,
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

// ── Backend progress log ──

#[derive(Debug, Clone)]
enum BackendLogStatus {
    Trying,
    Failed(String),
}

#[derive(Debug, Clone)]
struct BackendLogEntry {
    backend: CliBackend,
    status: BackendLogStatus,
}

// ── Worker messages ──

enum WorkerMessage {
    BackendProgress(BackendProgress),
    CommitGenerated(actions::Result<CommitPreview>),
    CommitShortened(actions::Result<CommitPreview>),
    CommitApplied(actions::Result<actions::CommitResult>),
    BranchGenerated(actions::Result<BranchPreview>),
    BranchCreated(actions::Result<actions::BranchResult>),
    PrGenerated(actions::Result<PrPreview>, Option<PrContext>),
    HookRan(actions::Result<String>),
    PrSubmitted(Result<String, String>),
    UpdateAvailable(Option<String>),
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
    focus_area: FocusArea,
    focused_panel_btn: usize,
    focused_bar_btn: usize,
    pending: Option<PendingJob>,
    spinner_tick: usize,
    notice: Option<Notice>,
    hook_installed: bool,
    sensitive_blocked: bool,
    should_quit: bool,
    // File sidebar state
    file_groups: Vec<CommitGroup>,
    selected_file: usize,     // 0 = [All], then indexed into flattened list
    file_sidebar_scroll: usize,
    base_branch: String,
    commit_count: usize,
    update_notice: Option<String>,
    backend_log: Vec<BackendLogEntry>,
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
            focus_area: FocusArea::Bar,
            focused_panel_btn: 0,
            focused_bar_btn: 0,
            pending: None,
            spinner_tick: 0,
            notice: None,
            hook_installed,
            sensitive_blocked: false,
            should_quit: false,
            file_groups: vec![],
            selected_file: 0,
            file_sidebar_scroll: 0,
            base_branch: String::new(),
            commit_count: 0,
            update_notice: None,
            backend_log: vec![],
        }
    }

    fn panel_button_count(&self) -> usize {
        match &self.output {
            Some(content) => panel_buttons(content).len(),
            None => 0,
        }
    }

    fn has_sidebar(&self) -> bool {
        !self.file_groups.is_empty()
    }

    /// Total number of entries in the sidebar (1 for [All] + all files).
    fn sidebar_entry_count(&self) -> usize {
        if self.file_groups.is_empty() {
            return 0;
        }
        1 + self.file_groups.iter().map(|g| g.files.len()).sum::<usize>()
    }

    /// Get the file path for the currently selected sidebar entry.
    /// Returns None for [All] (index 0) or if sidebar is empty.
    fn selected_file_path(&self) -> Option<&str> {
        if self.selected_file == 0 || self.file_groups.is_empty() {
            return None;
        }
        let mut idx = self.selected_file - 1;
        for group in &self.file_groups {
            if idx < group.files.len() {
                return Some(&group.files[idx]);
            }
            idx -= group.files.len();
        }
        None
    }

    /// Populate file groups from a PrContext.
    fn populate_file_groups(&mut self, pr_ctx: &PrContext) {
        self.base_branch = pr_ctx.base_branch.clone();
        self.commit_count = pr_ctx.commit_count;

        if !pr_ctx.from_branch_diff {
            // Working tree diff mode: single group
            if !pr_ctx.changed_files.is_empty() {
                self.file_groups = vec![CommitGroup {
                    subject: "Working changes".to_owned(),
                    files: pr_ctx.changed_files.clone(),
                }];
            }
            return;
        }

        // Build commit-grouped file list from commit messages
        // Each commit message from get_commits_ahead has format: hash\nsubject\n\nbody\n
        let mut groups = Vec::new();
        for commit in &pr_ctx.commits {
            let mut lines = commit.lines();
            let _hash = lines.next().unwrap_or("");
            let subject = lines.next().unwrap_or("(no message)").to_owned();
            groups.push(CommitGroup {
                subject,
                files: vec![],
            });
        }

        // Assign files to groups (approximate: just distribute evenly since
        // we don't have per-commit file info from git log)
        // For a better approach, we'd need `git diff-tree` per commit.
        // For now, list all files under the last group.
        if groups.is_empty() {
            groups.push(CommitGroup {
                subject: "Changes".to_owned(),
                files: pr_ctx.changed_files.clone(),
            });
        } else if let Some(last) = groups.last_mut() {
            last.files = pr_ctx.changed_files.clone();
        }

        self.file_groups = groups;
        self.selected_file = 0;
    }

    /// Advance focus forward by one position, wrapping around.
    fn focus_next(&mut self) {
        let panel_count = self.panel_button_count();
        let has_sidebar = self.has_sidebar();
        match self.focus_area {
            FocusArea::Sidebar => {
                if panel_count > 0 {
                    self.focus_area = FocusArea::Panel;
                    self.focused_panel_btn = 0;
                } else {
                    self.focus_area = FocusArea::Bar;
                    self.focused_bar_btn = 0;
                }
            }
            FocusArea::Panel => {
                if self.focused_panel_btn + 1 < panel_count {
                    self.focused_panel_btn += 1;
                } else {
                    self.focus_area = FocusArea::Bar;
                    self.focused_bar_btn = 0;
                }
            }
            FocusArea::Bar => {
                if self.focused_bar_btn + 1 < ButtonId::ALL.len() {
                    self.focused_bar_btn += 1;
                } else if has_sidebar {
                    self.focus_area = FocusArea::Sidebar;
                } else if panel_count > 0 {
                    self.focus_area = FocusArea::Panel;
                    self.focused_panel_btn = 0;
                } else {
                    self.focused_bar_btn = 0;
                }
            }
        }
    }

    /// Move focus backward by one position, wrapping around.
    fn focus_prev(&mut self) {
        let panel_count = self.panel_button_count();
        let has_sidebar = self.has_sidebar();
        match self.focus_area {
            FocusArea::Sidebar => {
                self.focus_area = FocusArea::Bar;
                self.focused_bar_btn = ButtonId::ALL.len() - 1;
            }
            FocusArea::Panel => {
                if self.focused_panel_btn > 0 {
                    self.focused_panel_btn -= 1;
                } else if has_sidebar {
                    self.focus_area = FocusArea::Sidebar;
                } else {
                    self.focus_area = FocusArea::Bar;
                    self.focused_bar_btn = ButtonId::ALL.len() - 1;
                }
            }
            FocusArea::Bar => {
                if self.focused_bar_btn > 0 {
                    self.focused_bar_btn -= 1;
                } else if panel_count > 0 {
                    self.focus_area = FocusArea::Panel;
                    self.focused_panel_btn = panel_count - 1;
                } else if has_sidebar {
                    self.focus_area = FocusArea::Sidebar;
                } else {
                    self.focused_bar_btn = ButtonId::ALL.len() - 1;
                }
            }
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

    fn set_output(&mut self, content: OutputContent) {
        self.output = Some(content);
        self.output_scroll = 0;
        self.focus_area = FocusArea::Panel;
        self.focused_panel_btn = 0;
    }

    fn clear_output(&mut self) {
        self.output = None;
        self.output_scroll = 0;
        self.focus_area = FocusArea::Bar;
        self.focused_panel_btn = 0;
    }

    fn focused_description(&self) -> &'static str {
        match self.focus_area {
            FocusArea::Sidebar => "Navigate files. Up/Down to select, Tab to switch focus.",
            FocusArea::Panel => {
                if let Some(content) = &self.output {
                    let btns = panel_buttons(content);
                    if self.focused_panel_btn < btns.len() {
                        return panel_button_description(content, self.focused_panel_btn);
                    }
                }
                ""
            }
            FocusArea::Bar => {
                let btn = ButtonId::ALL[self.focused_bar_btn];
                btn.description(self)
            }
        }
    }
}

fn panel_button_description(content: &OutputContent, index: usize) -> &'static str {
    match content {
        OutputContent::CommitMessage { .. } => match index {
            0 => "Commit with the generated message",
            1 => "Shorten the commit message using AI",
            2 => "Regenerate the commit message",
            _ => "",
        },
        OutputContent::SensitiveWarning { .. } => "Allow sensitive content and continue generating",
        OutputContent::BranchPreview { .. } => match index {
            0 => "Create and checkout this branch",
            1 => "Generate a new branch name",
            _ => "",
        },
        OutputContent::PrPreview { .. } => match index {
            0 => "Submit PR via gh CLI",
            1 => "Copy PR title and body to clipboard",
            2 => "Regenerate the PR title and body",
            _ => "",
        },
        OutputContent::HookMenu => match index {
            0 => "Install the prepare-commit-msg hook to auto-generate commit messages",
            1 => "Remove the prepare-commit-msg hook",
            _ => "",
        },
        OutputContent::HookConfirm { operation } => match (operation, index) {
            (HookOperation::Install, 0) => "Confirm installing the hook",
            (HookOperation::Uninstall, 0) => "Confirm uninstalling the hook",
            (_, 1) => "Cancel and go back",
            _ => "",
        },
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
    let diff_text =
        opencodecommit::git::get_diff(config.diff_source, &repo.repo_root).unwrap_or_default();
    let hook_installed = detect_hook_installed(&repo);

    // Try loading PR context for file sidebar (branch diff fallback)
    let pr_ctx = actions::load_pr_context(&config, None).ok();

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
    let auto_update = config.auto_update;
    let mut app = App::new(config, repo, diff_text.clone(), hook_installed);

    // Populate file sidebar and use branch diff if available
    if let Some(ctx) = &pr_ctx {
        app.populate_file_groups(ctx);
        if ctx.from_branch_diff && diff_text.is_empty() {
            // Use the branch diff for the diff panel
            app.diff_text = ctx.diff.clone();
        }
    }

    if auto_update {
        let tx = tx.clone();
        std::thread::spawn(move || {
            let (needs_check, cached) = crate::update::should_check();
            if needs_check {
                let source = crate::update::detect_install_source();
                match crate::update::check_latest_version(source) {
                    Ok(latest) => {
                        crate::update::write_cache(&latest);
                        let current = env!("CARGO_PKG_VERSION");
                        if crate::update::is_newer(current, &latest) {
                            let _ = tx.send(WorkerMessage::UpdateAvailable(Some(latest)));
                        }
                    }
                    Err(_) => {}
                }
            } else if let Some(latest) = cached {
                let current = env!("CARGO_PKG_VERSION");
                if crate::update::is_newer(current, &latest) {
                    let _ = tx.send(WorkerMessage::UpdateAvailable(Some(latest)));
                }
            }
        });
    }

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

    if app.pending.is_some() {
        return;
    }

    match key.code {
        // Tab / Shift-Tab cycle through all focusable buttons
        KeyCode::Tab | KeyCode::Right => {
            app.focus_next();
            app.notice = None;
        }
        KeyCode::BackTab | KeyCode::Left => {
            app.focus_prev();
            app.notice = None;
        }

        // Up/Down: depends on focus area
        KeyCode::Up | KeyCode::Char('k') => {
            if app.focus_area == FocusArea::Sidebar {
                if app.selected_file > 0 {
                    app.selected_file -= 1;
                    // Keep sidebar scroll in view
                    if app.selected_file < app.file_sidebar_scroll {
                        app.file_sidebar_scroll = app.selected_file;
                    }
                }
            } else if app.output.is_some() {
                app.output_scroll = app.output_scroll.saturating_sub(1);
            } else {
                app.diff_scroll = app.diff_scroll.saturating_sub(1);
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.focus_area == FocusArea::Sidebar {
                let max = app.sidebar_entry_count().saturating_sub(1);
                if app.selected_file < max {
                    app.selected_file += 1;
                }
            } else if app.output.is_some() {
                app.output_scroll = app.output_scroll.saturating_add(1);
            } else {
                app.diff_scroll = app.diff_scroll.saturating_add(1);
            }
        }

        // Enter activates the focused item
        KeyCode::Enter => {
            activate_focused(app, tx);
        }

        // Esc clears output panel / notice
        KeyCode::Esc => {
            if app.output.is_some() {
                app.clear_output();
                app.sensitive_blocked = false;
            } else {
                app.notice = None;
            }
        }

        // Number keys for bottom bar shortcuts
        KeyCode::Char(ch @ '0'..='4') => {
            if let Some(btn) = ButtonId::ALL.iter().find(|b| b.number() == ch) {
                activate_bar_button(app, *btn, tx);
            }
        }

        // Letter shortcuts for panel buttons (work regardless of focus)
        KeyCode::Char(ch) => {
            handle_panel_shortcut(app, ch, tx);
        }

        _ => {}
    }
}

fn activate_focused(app: &mut App, tx: &Sender<WorkerMessage>) {
    match app.focus_area {
        FocusArea::Sidebar => {
            // Enter on sidebar just keeps the file selected (navigation is via up/down)
        }
        FocusArea::Panel => activate_panel_button(app, app.focused_panel_btn, tx),
        FocusArea::Bar => {
            let btn = ButtonId::ALL[app.focused_bar_btn];
            activate_bar_button(app, btn, tx);
        }
    }
}

fn activate_bar_button(app: &mut App, btn: ButtonId, tx: &Sender<WorkerMessage>) {
    if !btn.is_available(app) {
        app.set_error(btn.description(app));
        return;
    }

    match btn {
        ButtonId::Commit => spawn_generate_commit(app, tx, false),
        ButtonId::Branch => spawn_generate_branch(app, tx),
        ButtonId::Pr => spawn_generate_pr(app, tx),
        ButtonId::SafetyHook => {
            app.set_output(OutputContent::HookMenu);
        }
        ButtonId::Quit => app.should_quit = true,
    }
}

fn activate_panel_button(app: &mut App, index: usize, tx: &Sender<WorkerMessage>) {
    let Some(content) = &app.output else { return };
    match content {
        OutputContent::CommitMessage { .. } => match index {
            0 => spawn_apply_commit(app, tx),
            1 => spawn_shorten_commit(app, tx),
            2 => spawn_generate_commit(app, tx, false),
            _ => {}
        },
        OutputContent::SensitiveWarning { .. } => {
            if index == 0 {
                app.sensitive_blocked = false;
                app.clear_output();
                spawn_generate_commit(app, tx, true);
            }
        }
        OutputContent::BranchPreview { .. } => match index {
            0 => spawn_create_branch(app, tx),
            1 => spawn_generate_branch(app, tx),
            _ => {}
        },
        OutputContent::PrPreview { preview } => match index {
            0 => spawn_submit_pr(app, tx),
            1 => {
                let text = format!("{}\n\n{}", preview.title, preview.body);
                if copy_to_clipboard(&text) {
                    app.set_info("PR copied to clipboard.");
                } else {
                    app.set_error("Clipboard copy failed. Select text manually.");
                }
            }
            2 => spawn_generate_pr(app, tx),
            _ => {}
        },
        OutputContent::HookMenu => match index {
            0 => {
                app.set_output(OutputContent::HookConfirm {
                    operation: HookOperation::Install,
                });
            }
            1 => {
                app.set_output(OutputContent::HookConfirm {
                    operation: HookOperation::Uninstall,
                });
            }
            _ => {}
        },
        OutputContent::HookConfirm { operation } => {
            let op = *operation;
            match index {
                0 => spawn_run_hook(app, tx, op),
                1 => app.clear_output(),
                _ => {}
            }
        }
    }
}

/// Handle letter key shortcuts for panel buttons (work regardless of focus area).
fn handle_panel_shortcut(app: &mut App, ch: char, tx: &Sender<WorkerMessage>) {
    let Some(content) = &app.output else { return };
    match content {
        OutputContent::CommitMessage { .. } => match ch {
            'c' => spawn_apply_commit(app, tx),
            's' => spawn_shorten_commit(app, tx),
            'r' => spawn_generate_commit(app, tx, false),
            _ => {}
        },
        OutputContent::SensitiveWarning { .. } => {
            if ch == 'a' {
                app.sensitive_blocked = false;
                app.clear_output();
                spawn_generate_commit(app, tx, true);
            }
        }
        OutputContent::BranchPreview { .. } => match ch {
            'c' => spawn_create_branch(app, tx),
            'r' => spawn_generate_branch(app, tx),
            _ => {}
        },
        OutputContent::PrPreview { preview } => match ch {
            's' => spawn_submit_pr(app, tx),
            'p' => {
                let text = format!("{}\n\n{}", preview.title, preview.body);
                if copy_to_clipboard(&text) {
                    app.set_info("PR copied to clipboard.");
                } else {
                    app.set_error("Clipboard copy failed. Select text manually.");
                }
            }
            'r' => spawn_generate_pr(app, tx),
            _ => {}
        },
        OutputContent::HookMenu => match ch {
            'i' => {
                app.set_output(OutputContent::HookConfirm {
                    operation: HookOperation::Install,
                });
            }
            'u' => {
                app.set_output(OutputContent::HookConfirm {
                    operation: HookOperation::Uninstall,
                });
            }
            _ => {}
        },
        OutputContent::HookConfirm { operation } => {
            let op = *operation;
            match ch {
                'y' => spawn_run_hook(app, tx, op),
                'n' => app.clear_output(),
                _ => {}
            }
        }
    }
}

// ── Worker thread spawners ──

fn spawn_generate_commit(app: &mut App, tx: &Sender<WorkerMessage>, allow_sensitive: bool) {
    app.pending = Some(PendingJob::GeneratingCommit);
    app.backend_log.clear();
    let config = app.config.clone();
    let tx = tx.clone();
    std::thread::spawn(move || {
        let request = CommitRequest {
            refine: None,
            feedback: None,
            stdin_diff: None,
            allow_sensitive,
        };
        let progress_tx = tx.clone();
        let _ = tx.send(WorkerMessage::CommitGenerated(
            actions::generate_commit_preview_with_fallback(&config, &request, move |p| {
                let _ = progress_tx.send(WorkerMessage::BackendProgress(p));
            }),
        ));
    });
}

fn spawn_shorten_commit(app: &mut App, tx: &Sender<WorkerMessage>) {
    let Some(OutputContent::CommitMessage { preview }) = &app.output else {
        app.set_error("Generate a commit message first.");
        return;
    };

    app.pending = Some(PendingJob::ShorteningCommit);
    app.backend_log.clear();
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
        let progress_tx = tx.clone();
        let _ = tx.send(WorkerMessage::CommitShortened(
            actions::generate_commit_preview_with_fallback(&config, &request, move |p| {
                let _ = progress_tx.send(WorkerMessage::BackendProgress(p));
            }),
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
    app.backend_log.clear();
    let config = app.config.clone();
    let tx = tx.clone();
    std::thread::spawn(move || {
        let progress_tx = tx.clone();
        let _ = tx.send(WorkerMessage::BranchGenerated(
            actions::generate_branch_preview_with_fallback(
                &config,
                None,
                config.branch_mode,
                move |p| {
                    let _ = progress_tx.send(WorkerMessage::BackendProgress(p));
                },
            ),
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
    app.backend_log.clear();
    let config = app.config.clone();
    let tx = tx.clone();
    std::thread::spawn(move || {
        // Load PR context for sidebar population
        let pr_ctx = actions::load_pr_context(&config, None).ok();
        let progress_tx = tx.clone();
        let result = actions::generate_pr_preview_with_fallback(&config, move |p| {
            let _ = progress_tx.send(WorkerMessage::BackendProgress(p));
        });
        let _ = tx.send(WorkerMessage::PrGenerated(result, pr_ctx));
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
            && let Some(stdin) = child.stdin.as_mut()
            && stdin.write_all(text.as_bytes()).is_ok()
        {
            drop(child.stdin.take());
            if child.wait().is_ok_and(|s| s.success()) {
                return true;
            }
        }
    }
    false
}

fn spawn_run_hook(app: &mut App, tx: &Sender<WorkerMessage>, operation: HookOperation) {
    app.pending = Some(PendingJob::RunningHook);
    app.clear_output();
    let tx = tx.clone();
    std::thread::spawn(move || {
        let _ = tx.send(WorkerMessage::HookRan(actions::run_hook(operation)));
    });
}

// ── Worker message handling ──

fn format_success_notice(action: &str, provider: &str, failures: &[actions::BackendFailure]) -> String {
    if failures.is_empty() {
        format!("{action}.")
    } else {
        let failed: Vec<&str> = failures.iter().map(|f| f.backend.as_str()).collect();
        format!("{action} via {provider} ({} failed).", failed.join(", "))
    }
}

fn apply_worker_message(app: &mut App, message: WorkerMessage) {
    match message {
        WorkerMessage::BackendProgress(progress) => {
            match progress {
                BackendProgress::Trying(backend) => {
                    app.backend_log.push(BackendLogEntry {
                        backend,
                        status: BackendLogStatus::Trying,
                    });
                }
                BackendProgress::Failed { backend, error } => {
                    // Update the existing "Trying" entry to "Failed"
                    if let Some(entry) = app.backend_log.iter_mut().rev().find(|e| e.backend == backend) {
                        entry.status = BackendLogStatus::Failed(error);
                    }
                }
            }
            // Don't clear pending — the job is still running
        }
        WorkerMessage::CommitGenerated(result) => {
            app.pending = None;
            app.backend_log.clear();
            match result {
                Ok(preview) => {
                    let notice = format_success_notice("Generated commit message", &preview.provider, &preview.backend_failures);
                    app.set_info(notice);
                    app.set_output(OutputContent::CommitMessage { preview });
                }
                Err(ActionError::SensitiveContent(report)) => {
                    app.sensitive_blocked = true;
                    app.set_output(OutputContent::SensitiveWarning { report });
                }
                Err(err) => app.set_error(err.to_string()),
            }
        }
        WorkerMessage::CommitShortened(result) => {
            app.pending = None;
            app.backend_log.clear();
            match result {
                Ok(preview) => {
                    let notice = format_success_notice("Shortened commit message", &preview.provider, &preview.backend_failures);
                    app.set_info(notice);
                    app.set_output(OutputContent::CommitMessage { preview });
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
                    app.clear_output();
                    app.refresh_repo();
                    app.diff_text =
                        opencodecommit::git::get_diff(app.config.diff_source, &app.repo.repo_root)
                            .unwrap_or_default();
                    app.diff_scroll = 0;
                }
                Err(err) => app.set_error(err.to_string()),
            }
        }
        WorkerMessage::BranchGenerated(result) => {
            app.pending = None;
            app.backend_log.clear();
            match result {
                Ok(preview) => {
                    let failed: Vec<&str> = preview.backend_failures.iter().map(|f| f.backend.as_str()).collect();
                    let notice = if failed.is_empty() {
                        "Generated branch name.".to_owned()
                    } else {
                        format!("Generated branch name ({} failed).", failed.join(", "))
                    };
                    app.set_info(notice);
                    app.set_output(OutputContent::BranchPreview { preview });
                }
                Err(err) => app.set_error(err.to_string()),
            }
        }
        WorkerMessage::BranchCreated(result) => {
            app.pending = None;
            match result {
                Ok(result) => {
                    app.set_info(format!("Switched to new branch '{}'.", result.name));
                    app.clear_output();
                    app.refresh_repo();
                }
                Err(err) => app.set_error(err.to_string()),
            }
        }
        WorkerMessage::PrGenerated(result, pr_ctx) => {
            app.pending = None;
            app.backend_log.clear();
            match result {
                Ok(preview) => {
                    if let Some(ctx) = &pr_ctx {
                        app.populate_file_groups(ctx);
                        if ctx.from_branch_diff && app.diff_text.is_empty() {
                            app.diff_text = ctx.diff.clone();
                        }
                    }
                    let failed: Vec<&str> = preview.backend_failures.iter().map(|f| f.backend.as_str()).collect();
                    let notice = if failed.is_empty() {
                        "Generated PR preview.".to_owned()
                    } else {
                        format!("Generated PR preview ({} failed).", failed.join(", "))
                    };
                    app.set_info(notice);
                    app.set_output(OutputContent::PrPreview { preview });
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
        WorkerMessage::UpdateAvailable(version) => {
            if let Some(v) = version {
                app.update_notice = Some(format!("v{v} available \u{2014} run `occ update`"));
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
    } else if line.starts_with("diff ")
        || line.starts_with("index ")
        || line.starts_with("--- ")
        || line.starts_with("+++ ")
    {
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
        Constraint::Min(5),                // main content area
        Constraint::Length(output_height), // output panel (0 when empty)
        Constraint::Length(1),             // button bar
        Constraint::Length(1),             // description line
    ])
    .split(area);

    render_header(frame, chunks[0], app);

    // Three-panel layout when sidebar is available
    if app.has_sidebar() {
        let cols = Layout::horizontal([
            Constraint::Percentage(20), // file sidebar
            Constraint::Percentage(80), // diff viewer
        ])
        .split(chunks[1]);
        render_file_sidebar(frame, cols[0], app);
        render_diff(frame, cols[1], app);
    } else {
        render_diff(frame, chunks[1], app);
    }

    if output_height > 0 {
        render_output_panel(frame, chunks[2], app);
    }
    render_button_bar(frame, chunks[3], app);
    render_description(frame, chunks[4], app);

    if let Some(pending) = app.pending {
        render_pending_overlay(frame, area, pending, app.spinner_tick, &app.backend_log);
    }
}

fn output_panel_height(app: &App) -> u16 {
    match &app.output {
        None => 0,
        Some(OutputContent::CommitMessage { preview }) => {
            let lines = preview.message.lines().count() as u16;
            // border(2) + label + blank + message + blank + 2 metadata lines + blank + buttons
            (lines + 9).min(16)
        }
        Some(OutputContent::SensitiveWarning { report }) => {
            let lines = report.findings.len() as u16;
            (lines + 6).min(15)
        }
        Some(OutputContent::BranchPreview { .. }) => 7,
        Some(OutputContent::PrPreview { preview }) => {
            let lines = preview.title.lines().count() as u16 + preview.body.lines().count() as u16;
            (lines + 6).min(20)
        }
        Some(OutputContent::HookMenu) => 6,
        Some(OutputContent::HookConfirm { .. }) => 5,
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let backend_status = match (&app.repo.backend_path, &app.repo.backend_error) {
        (Some(_), _) => "ready".to_owned(),
        (None, Some(err)) => format!("missing ({err})"),
        (None, None) => "missing".to_owned(),
    };
    let mut spans = vec![
        Span::styled(
            "OpenCodeCommit",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            &app.repo.repo_name,
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw("  branch: "),
        Span::styled(&app.repo.branch, Style::default().fg(Color::Green)),
    ];

    // Show base branch comparison if available
    if !app.base_branch.is_empty() {
        spans.push(Span::raw("  "));
        spans.push(Span::styled("<->", Style::default().fg(Color::DarkGray)));
        spans.push(Span::raw("  "));
        spans.push(Span::styled(&app.base_branch, Style::default().fg(Color::Cyan)));
        spans.push(Span::styled(
            format!("  ({} commits ahead)", app.commit_count),
            Style::default().fg(Color::DarkGray),
        ));
    }

    spans.push(Span::raw(format!(
        "  backend: {} [{}]",
        app.repo.backend_label, backend_status
    )));
    if let Some(notice) = &app.update_notice {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            notice.as_str(),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_file_sidebar(frame: &mut Frame, area: Rect, app: &App) {
    let is_focused = app.focus_area == FocusArea::Sidebar;
    let border_style = if is_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = if app.commit_count > 0 {
        format!("Files ({})", app.commit_count)
    } else {
        "Files".to_owned()
    };

    let mut lines = Vec::new();
    let mut flat_idx = 0usize;

    // [All] entry
    let all_style = if app.selected_file == 0 && is_focused {
        Style::default().fg(Color::Black).bg(Color::Cyan)
    } else if app.selected_file == 0 {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    lines.push(Line::styled(" [All]", all_style));

    for group in &app.file_groups {
        // Commit subject as header
        lines.push(Line::styled(
            format!(" {}", &group.subject),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));

        for file in &group.files {
            flat_idx += 1;
            let file_style = if app.selected_file == flat_idx && is_focused {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else if app.selected_file == flat_idx {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            // Show just the filename, truncated to fit
            let display = if file.len() > (area.width as usize).saturating_sub(4) {
                format!("  …{}", &file[file.len().saturating_sub(area.width as usize - 5)..])
            } else {
                format!("  {file}")
            };
            lines.push(Line::styled(display, file_style));
        }
    }

    let scroll_offset = app.file_sidebar_scroll as u16;
    let widget = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(Line::from(title)),
        )
        .scroll((scroll_offset, 0));
    frame.render_widget(widget, area);
}

/// Filter diff text to show only the section for a specific file.
fn filter_diff_for_file(diff: &str, file_path: &str) -> String {
    let mut result = String::new();
    let mut in_target = false;

    for line in diff.lines() {
        if line.starts_with("diff --git ") {
            in_target = line.contains(&format!("b/{file_path}"));
        }
        if in_target {
            result.push_str(line);
            result.push('\n');
        }
    }
    result
}

fn render_diff(frame: &mut Frame, area: Rect, app: &App) {
    if app.diff_text.is_empty() {
        let msg = Paragraph::new("No changes detected.")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).title(Line::from("Diff")));
        frame.render_widget(msg, area);
        return;
    }

    // Filter diff to selected file if sidebar has a selection
    let display_diff = if let Some(file_path) = app.selected_file_path() {
        filter_diff_for_file(&app.diff_text, file_path)
    } else {
        app.diff_text.clone()
    };

    let lines: Vec<Line> = display_diff.lines().map(style_diff_line).collect();
    let diff = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(Line::from("Diff")))
        .scroll((app.diff_scroll, 0));
    frame.render_widget(diff, area);
}

fn render_output_panel(frame: &mut Frame, area: Rect, app: &App) {
    let Some(content) = &app.output else { return };
    match content {
        OutputContent::CommitMessage { preview } => {
            render_commit_output(frame, area, preview, app);
        }
        OutputContent::SensitiveWarning { report } => {
            render_sensitive_output(frame, area, report, app);
        }
        OutputContent::BranchPreview { preview } => {
            render_branch_output(frame, area, preview, app);
        }
        OutputContent::PrPreview { preview } => {
            render_pr_output(frame, area, preview, app);
        }
        OutputContent::HookMenu => {
            render_hook_menu(frame, area, app);
        }
        OutputContent::HookConfirm { operation } => {
            render_hook_confirm(frame, area, *operation, app);
        }
    }
}

/// Render a row of panel buttons with focus highlighting.
fn render_panel_button_line(app: &App) -> Line<'static> {
    let Some(content) = &app.output else {
        return Line::raw("");
    };
    let labels = panel_buttons(content);
    let mut spans: Vec<Span> = Vec::new();

    for (i, label) in labels.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        let focused = app.focus_area == FocusArea::Panel && app.focused_panel_btn == i;
        let style = if focused {
            // First button gets a primary color, rest are secondary
            if i == 0 {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default().fg(Color::Black).bg(Color::White)
            }
        } else if i == 0 {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        spans.push(Span::styled(label.to_string(), style));
    }

    Line::from(spans)
}

fn render_commit_output(frame: &mut Frame, area: Rect, preview: &CommitPreview, app: &App) {
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
    lines.push(Line::styled(
        format!(
            "branch: {}  source: {}  tracked files: {}",
            preview.branch,
            preview.diff_origin,
            preview.changed_files.len()
        ),
        Style::default().fg(Color::DarkGray),
    ));
    lines.push(Line::raw(""));
    lines.push(render_panel_button_line(app));

    let widget = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        )
        .scroll((app.output_scroll, 0));
    frame.render_widget(widget, area);
}

fn render_sensitive_output(frame: &mut Frame, area: Rect, report: &SensitiveReport, app: &App) {
    let mut lines = vec![Line::styled(
        "SENSITIVE CONTENT DETECTED",
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
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
    lines.push(render_panel_button_line(app));
    lines.push(Line::styled(
        "Generation blocked until resolved or allowed",
        Style::default().fg(Color::DarkGray),
    ));

    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red)),
    );
    frame.render_widget(widget, area);
}

fn render_branch_output(frame: &mut Frame, area: Rect, preview: &BranchPreview, app: &App) {
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
        render_panel_button_line(app),
    ];

    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(widget, area);
}

fn render_pr_output(frame: &mut Frame, area: Rect, preview: &PrPreview, app: &App) {
    // Use only top/bottom borders so terminal text selection doesn't pick up │ characters
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
    lines.push(render_panel_button_line(app));

    let widget = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::TOP | Borders::BOTTOM)
                .border_style(Style::default().fg(Color::Magenta)),
        )
        .wrap(Wrap { trim: false })
        .scroll((app.output_scroll, 0));
    frame.render_widget(widget, area);
}

fn render_hook_menu(frame: &mut Frame, area: Rect, app: &App) {
    let status = if app.hook_installed {
        "Hook is currently installed."
    } else {
        "Hook is not installed."
    };
    let lines = vec![
        Line::styled(
            "SAFETY HOOK",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        ),
        Line::styled(status, Style::default().fg(Color::DarkGray)),
        Line::raw(""),
        render_panel_button_line(app),
    ];

    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );
    frame.render_widget(widget, area);
}

fn render_hook_confirm(frame: &mut Frame, area: Rect, operation: HookOperation, app: &App) {
    let action = match operation {
        HookOperation::Install => "Install",
        HookOperation::Uninstall => "Uninstall",
    };

    let lines = vec![
        Line::styled(
            format!("{action} the prepare-commit-msg hook?"),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Line::raw(""),
        render_panel_button_line(app),
    ];

    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );
    frame.render_widget(widget, area);
}

fn render_button_bar(frame: &mut Frame, area: Rect, app: &App) {
    let mut spans: Vec<Span> = Vec::new();

    for (i, &btn) in ButtonId::ALL.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" "));
        }

        let focused = app.focus_area == FocusArea::Bar && i == app.focused_bar_btn;
        let available = btn.is_available(app);
        let text = format!("[{} {}]", btn.number(), btn.label());

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
        let desc = app.focused_description();
        frame.render_widget(
            Paragraph::new(Span::styled(desc, Style::default().fg(Color::Cyan))),
            area,
        );
    }
}

fn render_pending_overlay(
    frame: &mut Frame,
    area: Rect,
    pending: PendingJob,
    tick: usize,
    backend_log: &[BackendLogEntry],
) {
    // Dynamic height: 3 (border + title line + border) + 1 per log entry + 1 blank separator if log present
    let log_lines = backend_log.len();
    let height = if log_lines > 0 { 4 + log_lines as u16 } else { 5 };
    let overlay = centered_rect(48, height, area);
    let spinner = ["-", "\\", "|", "/"][tick % 4];

    let mut lines: Vec<Line<'_>> = vec![
        Line::from(format!("{spinner} {}", pending.label())),
    ];

    if !backend_log.is_empty() {
        lines.push(Line::from(""));
        for entry in backend_log {
            match &entry.status {
                BackendLogStatus::Trying => {
                    lines.push(Line::styled(
                        format!("  {} ...", entry.backend),
                        Style::default().fg(Color::Yellow),
                    ));
                }
                BackendLogStatus::Failed(err) => {
                    let short_err = if err.len() > 40 { &err[..40] } else { err };
                    lines.push(Line::styled(
                        format!("  {} failed: {short_err}", entry.backend),
                        Style::default().fg(Color::DarkGray),
                    ));
                }
            }
        }
    }

    frame.render_widget(Clear, overlay);
    let widget = Paragraph::new(lines)
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
        let config = Config {
            backend: CliBackend::Codex,
            ..Config::default()
        };
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

    fn test_commit_preview() -> CommitPreview {
        CommitPreview {
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
            backend_failures: vec![],
        }
    }

    #[test]
    fn initial_state_shows_header_diff_and_buttons() {
        let app = test_app();
        let text = render_text(&app, 100, 24);
        assert!(text.contains("OpenCodeCommit"), "missing header");
        assert!(text.contains("demo"), "missing repo name");
        assert!(text.contains("main"), "missing branch");
        assert!(text.contains("1 Commit"), "missing Commit button");
        assert!(text.contains("2 Branch"), "missing Branch button");
        assert!(text.contains("3 PR"), "missing PR button");
        assert!(text.contains("4 Safety Hook"), "missing Safety Hook button");
        assert!(text.contains("0 Quit"), "missing Quit button");
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
    fn commit_message_shows_output_panel_with_buttons() {
        let mut app = test_app();
        app.set_output(OutputContent::CommitMessage {
            preview: test_commit_preview(),
        });
        let text = render_text(&app, 100, 30);
        assert!(text.contains("COMMIT MESSAGE"), "missing commit panel");
        assert!(text.contains("feat: add TUI"), "missing commit message");
        assert!(text.contains("[c Commit]"), "missing Commit action");
        assert!(text.contains("[s Shorten]"), "missing Shorten action");
        assert!(text.contains("[r Regenerate]"), "missing Regenerate action");
    }

    #[test]
    fn sensitive_warning_shows_panel() {
        let mut app = test_app();
        app.set_output(OutputContent::SensitiveWarning {
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
        assert!(text.contains("Allow & Continue"), "missing allow button");
    }

    #[test]
    fn tab_cycles_through_panel_and_bar() {
        let mut app = test_app();
        app.set_output(OutputContent::CommitMessage {
            preview: test_commit_preview(),
        });
        let (tx, _rx) = mpsc::channel();

        // Initially focused on panel button 0
        assert_eq!(app.focus_area, FocusArea::Panel);
        assert_eq!(app.focused_panel_btn, 0);

        // Tab through panel buttons
        handle_key(&mut app, KeyEvent::from(KeyCode::Tab), &tx);
        assert_eq!(app.focus_area, FocusArea::Panel);
        assert_eq!(app.focused_panel_btn, 1);

        handle_key(&mut app, KeyEvent::from(KeyCode::Tab), &tx);
        assert_eq!(app.focus_area, FocusArea::Panel);
        assert_eq!(app.focused_panel_btn, 2);

        // Next tab goes to bar
        handle_key(&mut app, KeyEvent::from(KeyCode::Tab), &tx);
        assert_eq!(app.focus_area, FocusArea::Bar);
        assert_eq!(app.focused_bar_btn, 0);

        // Tab through bar
        handle_key(&mut app, KeyEvent::from(KeyCode::Tab), &tx);
        assert_eq!(app.focus_area, FocusArea::Bar);
        assert_eq!(app.focused_bar_btn, 1);
    }

    #[test]
    fn backtab_wraps_from_bar_to_panel() {
        let mut app = test_app();
        app.set_output(OutputContent::BranchPreview {
            preview: BranchPreview {
                name: "feat/test".to_owned(),
                backend_failures: vec![],
            },
        });
        let (tx, _rx) = mpsc::channel();

        // Start at panel 0, backtab should go to last bar button
        handle_key(&mut app, KeyEvent::from(KeyCode::BackTab), &tx);
        assert_eq!(app.focus_area, FocusArea::Bar);
        assert_eq!(app.focused_bar_btn, ButtonId::ALL.len() - 1);
    }

    #[test]
    fn bar_focus_cycles_without_panel() {
        let mut app = test_app();
        let (tx, _rx) = mpsc::channel();

        assert_eq!(app.focus_area, FocusArea::Bar);
        assert_eq!(app.focused_bar_btn, 0);

        handle_key(&mut app, KeyEvent::from(KeyCode::Tab), &tx);
        assert_eq!(app.focused_bar_btn, 1);

        // Tab to end and wrap
        for _ in 0..ButtonId::ALL.len() - 1 {
            handle_key(&mut app, KeyEvent::from(KeyCode::Tab), &tx);
        }
        assert_eq!(app.focused_bar_btn, 0); // wrapped
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
        app.set_output(OutputContent::CommitMessage {
            preview: test_commit_preview(),
        });
        let (tx, _rx) = mpsc::channel();

        assert_eq!(app.output_scroll, 0);
        handle_key(&mut app, KeyEvent::from(KeyCode::Down), &tx);
        assert_eq!(app.output_scroll, 1);
        assert_eq!(app.diff_scroll, 0, "diff should not scroll");
    }

    #[test]
    fn esc_clears_output_and_returns_to_bar() {
        let mut app = test_app();
        app.set_output(OutputContent::BranchPreview {
            preview: BranchPreview {
                name: "feat/test".to_owned(),
                backend_failures: vec![],
            },
        });
        let (tx, _rx) = mpsc::channel();

        assert_eq!(app.focus_area, FocusArea::Panel);
        handle_key(&mut app, KeyEvent::from(KeyCode::Esc), &tx);
        assert!(app.output.is_none());
        assert_eq!(app.focus_area, FocusArea::Bar);
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
    fn safety_hook_shows_menu() {
        let mut app = test_app();
        app.set_output(OutputContent::HookMenu);
        let text = render_text(&app, 100, 24);
        assert!(text.contains("SAFETY HOOK"), "missing hook menu title");
        assert!(text.contains("Install Hook"), "missing install button");
        assert!(text.contains("Uninstall Hook"), "missing uninstall button");
    }

    #[test]
    fn empty_diff_shows_no_changes() {
        let config = Config {
            backend: CliBackend::Codex,
            ..Config::default()
        };
        let app = App::new(config, test_repo(), String::new(), false);
        let text = render_text(&app, 100, 24);
        assert!(text.contains("No changes detected"));
    }

    #[test]
    fn description_line_shows_panel_button_description() {
        let mut app = test_app();
        app.set_output(OutputContent::CommitMessage {
            preview: test_commit_preview(),
        });
        let text = render_text(&app, 100, 30);
        assert!(
            text.contains("Commit with the generated message"),
            "should show panel button 0 description"
        );
    }

    #[test]
    fn sidebar_renders_when_file_groups_present() {
        let mut app = test_app();
        app.file_groups = vec![CommitGroup {
            subject: "feat: add login".to_owned(),
            files: vec!["src/login.rs".to_owned(), "src/auth.rs".to_owned()],
        }];
        app.base_branch = "main".to_owned();
        app.commit_count = 1;
        let text = render_text(&app, 120, 24);
        assert!(text.contains("Files"), "missing Files title");
        assert!(text.contains("[All]"), "missing All entry");
        assert!(text.contains("login.rs"), "missing file entry");
    }

    #[test]
    fn sidebar_does_not_render_when_empty() {
        let app = test_app();
        assert!(!app.has_sidebar());
        let text = render_text(&app, 120, 24);
        assert!(!text.contains("Files ("), "should not show sidebar");
    }

    #[test]
    fn header_shows_base_branch_when_set() {
        let mut app = test_app();
        app.base_branch = "main".to_owned();
        app.commit_count = 3;
        let text = render_text(&app, 120, 24);
        assert!(text.contains("<->"), "missing base branch separator");
        assert!(text.contains("3 commits ahead"), "missing commit count");
    }

    #[test]
    fn sidebar_navigation_changes_selected_file() {
        let mut app = test_app();
        app.file_groups = vec![CommitGroup {
            subject: "test".to_owned(),
            files: vec!["a.rs".to_owned(), "b.rs".to_owned()],
        }];
        app.focus_area = FocusArea::Sidebar;
        let (tx, _rx) = mpsc::channel();

        assert_eq!(app.selected_file, 0); // [All]
        handle_key(&mut app, KeyEvent::from(KeyCode::Down), &tx);
        assert_eq!(app.selected_file, 1);
        handle_key(&mut app, KeyEvent::from(KeyCode::Down), &tx);
        assert_eq!(app.selected_file, 2);
        handle_key(&mut app, KeyEvent::from(KeyCode::Up), &tx);
        assert_eq!(app.selected_file, 1);
    }

    #[test]
    fn focus_cycles_through_sidebar() {
        let mut app = test_app();
        app.file_groups = vec![CommitGroup {
            subject: "test".to_owned(),
            files: vec!["a.rs".to_owned()],
        }];
        app.set_output(OutputContent::PrPreview {
            preview: PrPreview {
                title: "PR title".to_owned(),
                body: "PR body".to_owned(),
            },
        });
        // With sidebar + panel + bar, tab should eventually reach sidebar
        let (tx, _rx) = mpsc::channel();

        // Start at panel (set_output puts focus there)
        assert_eq!(app.focus_area, FocusArea::Panel);

        // Tab through panel buttons (3 for PR: Submit, Copy, Regenerate)
        handle_key(&mut app, KeyEvent::from(KeyCode::Tab), &tx);
        handle_key(&mut app, KeyEvent::from(KeyCode::Tab), &tx);
        // Now at bar
        handle_key(&mut app, KeyEvent::from(KeyCode::Tab), &tx);
        assert_eq!(app.focus_area, FocusArea::Bar);

        // Tab through all 5 bar buttons
        for _ in 0..4 {
            handle_key(&mut app, KeyEvent::from(KeyCode::Tab), &tx);
        }
        // Should wrap to sidebar
        handle_key(&mut app, KeyEvent::from(KeyCode::Tab), &tx);
        assert_eq!(app.focus_area, FocusArea::Sidebar);
    }
}
