use std::io::{self, IsTerminal};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use opencodecommit::config::Config;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{Frame, Terminal};

use crate::actions::{
    self, ActionError, BranchPreview, ChangelogPreview, CommitPreview, CommitRequest,
    HookOperation, PrPreview, RepoSummary,
};

type TuiTerminal = Terminal<CrosstermBackend<io::Stdout>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HomeItem {
    Commit,
    Branch,
    Pr,
    Changelog,
    Hook,
    Quit,
}

impl HomeItem {
    const ALL: [HomeItem; 6] = [
        HomeItem::Commit,
        HomeItem::Branch,
        HomeItem::Pr,
        HomeItem::Changelog,
        HomeItem::Hook,
        HomeItem::Quit,
    ];

    fn title(self) -> &'static str {
        match self {
            HomeItem::Commit => "Commit",
            HomeItem::Branch => "Branch",
            HomeItem::Pr => "PR",
            HomeItem::Changelog => "Changelog",
            HomeItem::Hook => "Hook",
            HomeItem::Quit => "Quit",
        }
    }

    fn summary(self) -> &'static str {
        match self {
            HomeItem::Commit => "Generate, shorten, and commit one message.",
            HomeItem::Branch => "Preview and create one branch name.",
            HomeItem::Pr => "Preview a PR title and body.",
            HomeItem::Changelog => "Preview a changelog entry.",
            HomeItem::Hook => "Install or uninstall the prepare-commit-msg hook.",
            HomeItem::Quit => "Leave the TUI.",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    Home,
    Commit,
    Branch,
    Pr,
    Changelog,
    Hook,
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingJob {
    GeneratingCommit,
    ShorteningCommit,
    ApplyingCommit,
    GeneratingBranch,
    CreatingBranch,
    GeneratingPr,
    GeneratingChangelog,
    RunningHook,
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
            PendingJob::GeneratingChangelog => "Generating changelog entry",
            PendingJob::RunningHook => "Updating git hook",
        }
    }
}

#[derive(Debug, Default)]
struct CommitView {
    preview: Option<CommitPreview>,
    scroll: u16,
}

#[derive(Debug, Default)]
struct BranchView {
    preview: Option<BranchPreview>,
}

#[derive(Debug, Default)]
struct PrView {
    preview: Option<PrPreview>,
    scroll: u16,
}

#[derive(Debug, Default)]
struct ChangelogView {
    preview: Option<ChangelogPreview>,
    scroll: u16,
}

#[derive(Debug)]
struct App {
    config: Config,
    repo: RepoSummary,
    screen: Screen,
    home_index: usize,
    commit: CommitView,
    branch: BranchView,
    pr: PrView,
    changelog: ChangelogView,
    hook_index: usize,
    pending: Option<PendingJob>,
    spinner_tick: usize,
    notice: Option<Notice>,
    should_quit: bool,
}

impl App {
    fn new(config: Config, repo: RepoSummary) -> Self {
        Self {
            config,
            repo,
            screen: Screen::Home,
            home_index: 0,
            commit: CommitView::default(),
            branch: BranchView::default(),
            pr: PrView::default(),
            changelog: ChangelogView::default(),
            hook_index: 0,
            pending: None,
            spinner_tick: 0,
            notice: None,
            should_quit: false,
        }
    }

    fn selected_home_item(&self) -> HomeItem {
        HomeItem::ALL[self.home_index]
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
}

enum WorkerMessage {
    CommitGenerated(actions::Result<CommitPreview>),
    CommitShortened(actions::Result<CommitPreview>),
    CommitApplied(actions::Result<actions::CommitResult>),
    BranchGenerated(actions::Result<BranchPreview>),
    BranchCreated(actions::Result<actions::BranchResult>),
    PrGenerated(actions::Result<PrPreview>),
    ChangelogGenerated(actions::Result<ChangelogPreview>),
    HookRan(actions::Result<String>),
}

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

pub fn run(config: Config) -> Result<(), ActionError> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(ActionError::NonTty(
            "`occ tui` requires an interactive terminal. Use the existing text subcommands when piping or scripting.".to_owned(),
        ));
    }

    let repo = actions::load_repo_summary(&config)?;
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
    let mut app = App::new(config, repo);

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

fn handle_key(app: &mut App, key: KeyEvent, tx: &Sender<WorkerMessage>) {
    if matches!(key.code, KeyCode::Char('q'))
        || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
    {
        app.should_quit = true;
        return;
    }

    if app.pending.is_some() {
        return;
    }

    match app.screen {
        Screen::Home => handle_home_key(app, key),
        Screen::Commit => handle_commit_key(app, key, tx),
        Screen::Branch => handle_branch_key(app, key, tx),
        Screen::Pr => handle_pr_key(app, key, tx),
        Screen::Changelog => handle_changelog_key(app, key, tx),
        Screen::Hook => handle_hook_key(app, key, tx),
    }
}

fn handle_home_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Down | KeyCode::Char('j') => {
            app.home_index = (app.home_index + 1) % HomeItem::ALL.len();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.home_index == 0 {
                app.home_index = HomeItem::ALL.len() - 1;
            } else {
                app.home_index -= 1;
            }
        }
        KeyCode::Enter => match app.selected_home_item() {
            HomeItem::Commit => app.screen = Screen::Commit,
            HomeItem::Branch => app.screen = Screen::Branch,
            HomeItem::Pr => app.screen = Screen::Pr,
            HomeItem::Changelog => app.screen = Screen::Changelog,
            HomeItem::Hook => app.screen = Screen::Hook,
            HomeItem::Quit => app.should_quit = true,
        },
        _ => {}
    }
}

fn handle_commit_key(app: &mut App, key: KeyEvent, tx: &Sender<WorkerMessage>) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('b') => app.screen = Screen::Home,
        KeyCode::Char('g') => spawn_generate_commit(app, tx),
        KeyCode::Char('s') => spawn_shorten_commit(app, tx),
        KeyCode::Char('c') => spawn_apply_commit(app, tx),
        KeyCode::Down | KeyCode::Char('j') => {
            app.commit.scroll = app.commit.scroll.saturating_add(1);
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.commit.scroll = app.commit.scroll.saturating_sub(1);
        }
        _ => {}
    }
}

fn handle_branch_key(app: &mut App, key: KeyEvent, tx: &Sender<WorkerMessage>) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('b') => app.screen = Screen::Home,
        KeyCode::Char('g') => spawn_generate_branch(app, tx),
        KeyCode::Char('c') | KeyCode::Enter => spawn_create_branch(app, tx),
        _ => {}
    }
}

fn handle_pr_key(app: &mut App, key: KeyEvent, tx: &Sender<WorkerMessage>) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('b') => app.screen = Screen::Home,
        KeyCode::Char('g') => spawn_generate_pr(app, tx),
        KeyCode::Down | KeyCode::Char('j') => {
            app.pr.scroll = app.pr.scroll.saturating_add(1);
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.pr.scroll = app.pr.scroll.saturating_sub(1);
        }
        _ => {}
    }
}

fn handle_changelog_key(app: &mut App, key: KeyEvent, tx: &Sender<WorkerMessage>) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('b') => app.screen = Screen::Home,
        KeyCode::Char('g') => spawn_generate_changelog(app, tx),
        KeyCode::Down | KeyCode::Char('j') => {
            app.changelog.scroll = app.changelog.scroll.saturating_add(1);
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.changelog.scroll = app.changelog.scroll.saturating_sub(1);
        }
        _ => {}
    }
}

fn handle_hook_key(app: &mut App, key: KeyEvent, tx: &Sender<WorkerMessage>) {
    match key.code {
        KeyCode::Esc | KeyCode::Char('b') => app.screen = Screen::Home,
        KeyCode::Down | KeyCode::Char('j') => {
            app.hook_index = (app.hook_index + 1) % 2;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.hook_index = if app.hook_index == 0 { 1 } else { 0 };
        }
        KeyCode::Enter | KeyCode::Char('r') => spawn_run_hook(app, tx),
        _ => {}
    }
}

fn spawn_generate_commit(app: &mut App, tx: &Sender<WorkerMessage>) {
    app.pending = Some(PendingJob::GeneratingCommit);
    let config = app.config.clone();
    let tx = tx.clone();
    std::thread::spawn(move || {
        let request = CommitRequest {
            refine: None,
            feedback: None,
            stdin_diff: None,
            allow_sensitive: false,
        };
        let _ = tx.send(WorkerMessage::CommitGenerated(
            actions::generate_commit_preview(&config, &request),
        ));
    });
}

fn spawn_shorten_commit(app: &mut App, tx: &Sender<WorkerMessage>) {
    let Some(preview) = app.commit.preview.as_ref() else {
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
    let Some(preview) = app.commit.preview.as_ref() else {
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
    let Some(preview) = app.branch.preview.as_ref() else {
        app.set_error("Generate a branch name first.");
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

fn spawn_generate_changelog(app: &mut App, tx: &Sender<WorkerMessage>) {
    app.pending = Some(PendingJob::GeneratingChangelog);
    let config = app.config.clone();
    let tx = tx.clone();
    std::thread::spawn(move || {
        let _ = tx.send(WorkerMessage::ChangelogGenerated(
            actions::generate_changelog_preview(&config),
        ));
    });
}

fn spawn_run_hook(app: &mut App, tx: &Sender<WorkerMessage>) {
    app.pending = Some(PendingJob::RunningHook);
    let action = if app.hook_index == 0 {
        HookOperation::Install
    } else {
        HookOperation::Uninstall
    };
    let tx = tx.clone();
    std::thread::spawn(move || {
        let _ = tx.send(WorkerMessage::HookRan(actions::run_hook(action)));
    });
}

fn apply_worker_message(app: &mut App, message: WorkerMessage) {
    match message {
        WorkerMessage::CommitGenerated(result) => {
            app.pending = None;
            match result {
                Ok(preview) => {
                    app.commit.preview = Some(preview);
                    app.commit.scroll = 0;
                    app.set_info("Generated commit message.");
                }
                Err(err) => app.set_error(err.to_string()),
            }
        }
        WorkerMessage::CommitShortened(result) => {
            app.pending = None;
            match result {
                Ok(preview) => {
                    app.commit.preview = Some(preview);
                    app.commit.scroll = 0;
                    app.set_info("Shortened commit message.");
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
                    app.refresh_repo();
                }
                Err(err) => app.set_error(err.to_string()),
            }
        }
        WorkerMessage::BranchGenerated(result) => {
            app.pending = None;
            match result {
                Ok(preview) => {
                    app.branch.preview = Some(preview);
                    app.set_info("Generated branch name.");
                }
                Err(err) => app.set_error(err.to_string()),
            }
        }
        WorkerMessage::BranchCreated(result) => {
            app.pending = None;
            match result {
                Ok(result) => {
                    app.set_info(format!("Switched to new branch '{}'.", result.name));
                    app.refresh_repo();
                }
                Err(err) => app.set_error(err.to_string()),
            }
        }
        WorkerMessage::PrGenerated(result) => {
            app.pending = None;
            match result {
                Ok(preview) => {
                    app.pr.preview = Some(preview);
                    app.pr.scroll = 0;
                    app.set_info("Generated PR preview.");
                }
                Err(err) => app.set_error(err.to_string()),
            }
        }
        WorkerMessage::ChangelogGenerated(result) => {
            app.pending = None;
            match result {
                Ok(preview) => {
                    app.changelog.preview = Some(preview);
                    app.changelog.scroll = 0;
                    app.set_info("Generated changelog entry.");
                }
                Err(err) => app.set_error(err.to_string()),
            }
        }
        WorkerMessage::HookRan(result) => {
            app.pending = None;
            match result {
                Ok(message) => {
                    app.set_info(message);
                    app.refresh_repo();
                }
                Err(err) => app.set_error(err.to_string()),
            }
        }
    }
}

fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(area);

    render_header(frame, layout[0], app);
    match app.screen {
        Screen::Home => render_home(frame, layout[1], app),
        Screen::Commit => render_commit(frame, layout[1], app),
        Screen::Branch => render_branch(frame, layout[1], app),
        Screen::Pr => render_pr(frame, layout[1], app),
        Screen::Changelog => render_changelog(frame, layout[1], app),
        Screen::Hook => render_hook(frame, layout[1], app),
    }
    render_footer(frame, layout[2], app);

    if let Some(pending) = app.pending {
        render_pending_overlay(frame, area, pending, app.spinner_tick);
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let title = Line::from(vec![
        Span::styled(
            "OpenCodeCommit",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::raw(format!("repo: {}", app.repo.repo_name)),
        Span::raw("  "),
        Span::raw(format!("branch: {}", app.repo.branch)),
    ]);

    let header =
        Paragraph::new(title).block(Block::default().borders(Borders::ALL).title("occ tui"));
    frame.render_widget(header, area);
}

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let help = match app.screen {
        Screen::Home => "Arrows/jk move  Enter open  q quit",
        Screen::Commit => "g generate  s shorten  c commit  Esc/b back  q quit",
        Screen::Branch => "g generate  c create  Esc/b back  q quit",
        Screen::Pr => "g generate  Arrows/jk scroll  Esc/b back  q quit",
        Screen::Changelog => "g generate  Arrows/jk scroll  Esc/b back  q quit",
        Screen::Hook => "Arrows/jk move  Enter run  Esc/b back  q quit",
    };
    let notice = app.notice.as_ref().map(|notice| {
        let style = match notice.kind {
            NoticeKind::Info => Style::default().fg(Color::Green),
            NoticeKind::Error => Style::default().fg(Color::Red),
        };
        Span::styled(notice.message.clone(), style)
    });

    let mut line = vec![Span::raw(help)];
    if let Some(notice) = notice {
        line.push(Span::raw("  "));
        line.push(notice);
    }

    let footer = Paragraph::new(Line::from(line)).block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, area);
}

fn render_home(frame: &mut Frame, area: Rect, app: &App) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    let items: Vec<ListItem> = HomeItem::ALL
        .iter()
        .map(|item| ListItem::new(item.title()))
        .collect();
    let mut list_state = ListState::default();
    list_state.select(Some(app.home_index));
    let list = List::new(items)
        .block(Block::default().title("Launcher").borders(Borders::ALL))
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Yellow))
        .highlight_symbol(">> ");
    frame.render_stateful_widget(list, columns[0], &mut list_state);

    let backend_line = match (&app.repo.backend_path, &app.repo.backend_error) {
        (Some(path), _) => format!("{}: ready ({})", app.repo.backend_label, path.display()),
        (None, Some(err)) => format!("{}: {}", app.repo.backend_label, err),
        (None, None) => format!("{}: unavailable", app.repo.backend_label),
    };

    let details = vec![
        Line::from(Span::styled(
            app.selected_home_item().title(),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(app.selected_home_item().summary()),
        Line::default(),
        Line::from(format!("Repo root: {}", app.repo.repo_root.display())),
        Line::from(format!("Active language: {}", app.repo.active_language)),
        Line::from(backend_line),
        Line::default(),
        Line::from(format!("Staged files: {}", app.repo.staged_files)),
        Line::from(format!("Unstaged files: {}", app.repo.unstaged_files)),
        Line::default(),
        Line::from("Minimal mode: no staging UI, no editor, no git dashboard."),
    ];
    let details = Paragraph::new(details)
        .block(Block::default().title("Repository").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(details, columns[1]);
}

fn render_commit(frame: &mut Frame, area: Rect, app: &App) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(8),
        ])
        .split(area);

    let summary = match app.commit.preview.as_ref() {
        Some(preview) => format!(
            "Branch: {} | Diff: {} | Files: {} | Backend: {}",
            preview.branch, preview.diff_origin, preview.files_analyzed, preview.provider
        ),
        None => format!(
            "Branch: {} | Diff source: {} | Backend: {}",
            app.repo.branch,
            diff_source_label(app.config.diff_source),
            app.repo.backend_label
        ),
    };
    let summary =
        Paragraph::new(summary).block(Block::default().title("Summary").borders(Borders::ALL));
    frame.render_widget(summary, sections[0]);

    let message = if let Some(preview) = app.commit.preview.as_ref() {
        preview.message.clone()
    } else {
        "Press g to generate a commit message.\nPress s after that to run the built-in shorten refine step.".to_owned()
    };
    let message = Paragraph::new(Text::from(message))
        .block(Block::default().title("Message").borders(Borders::ALL))
        .wrap(Wrap { trim: false })
        .scroll((app.commit.scroll, 0));
    frame.render_widget(message, sections[1]);

    let files = if let Some(preview) = app.commit.preview.as_ref() {
        let mut lines: Vec<Line> = preview
            .changed_files
            .iter()
            .take(6)
            .map(|file| Line::from(format!("- {file}")))
            .collect();
        if preview.changed_files.len() > 6 {
            lines.push(Line::from(format!(
                "... and {} more",
                preview.changed_files.len() - 6
            )));
        }
        lines
    } else {
        vec![
            Line::from("No preview yet."),
            Line::from("The TUI mirrors current CLI behavior when you commit."),
            Line::from("If nothing is staged, occ stages all changes right before commit."),
        ]
    };
    let files = Paragraph::new(files)
        .block(
            Block::default()
                .title("Changed Files")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(files, sections[2]);
}

fn diff_source_label(source: opencodecommit::config::DiffSource) -> &'static str {
    match source {
        opencodecommit::config::DiffSource::Staged => "staged",
        opencodecommit::config::DiffSource::All => "all",
        opencodecommit::config::DiffSource::Auto => "auto",
    }
}

fn render_branch(frame: &mut Frame, area: Rect, app: &App) {
    let text = if let Some(preview) = app.branch.preview.as_ref() {
        format!(
            "{}\n\nPress c to create and checkout this branch.",
            preview.name
        )
    } else {
        "Press g to generate a branch name from the current diff.".to_owned()
    };
    let widget = Paragraph::new(Text::from(text))
        .block(Block::default().title("Branch").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(widget, area);
}

fn render_pr(frame: &mut Frame, area: Rect, app: &App) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(10)])
        .split(area);

    let title = app
        .pr
        .preview
        .as_ref()
        .map(|preview| preview.title.clone())
        .unwrap_or_else(|| "Press g to generate a PR title and body.".to_owned());
    let title = Paragraph::new(title)
        .block(Block::default().title("Title").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(title, sections[0]);

    let body = app
        .pr
        .preview
        .as_ref()
        .map(|preview| preview.body.clone())
        .unwrap_or_else(|| "Generated PR body will appear here.".to_owned());
    let body = Paragraph::new(Text::from(body))
        .block(Block::default().title("Body").borders(Borders::ALL))
        .wrap(Wrap { trim: false })
        .scroll((app.pr.scroll, 0));
    frame.render_widget(body, sections[1]);
}

fn render_changelog(frame: &mut Frame, area: Rect, app: &App) {
    let text = app
        .changelog
        .preview
        .as_ref()
        .map(|preview| preview.entry.clone())
        .unwrap_or_else(|| "Press g to generate a changelog entry.".to_owned());
    let widget = Paragraph::new(Text::from(text))
        .block(Block::default().title("Changelog").borders(Borders::ALL))
        .wrap(Wrap { trim: false })
        .scroll((app.changelog.scroll, 0));
    frame.render_widget(widget, area);
}

fn render_hook(frame: &mut Frame, area: Rect, app: &App) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(6)])
        .split(area);

    let items = vec![
        ListItem::new("Install hook"),
        ListItem::new("Uninstall hook"),
    ];
    let mut state = ListState::default();
    state.select(Some(app.hook_index));
    let list = List::new(items)
        .block(Block::default().title("Hook Action").borders(Borders::ALL))
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Yellow))
        .highlight_symbol(">> ");
    frame.render_stateful_widget(list, sections[0], &mut state);

    let text = if app.hook_index == 0 {
        "Install the prepare-commit-msg hook generated by OpenCodeCommit."
    } else {
        "Remove the prepare-commit-msg hook if it was installed by OpenCodeCommit."
    };
    let description = Paragraph::new(text)
        .block(Block::default().title("Details").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(description, sections[1]);
}

fn render_pending_overlay(frame: &mut Frame, area: Rect, pending: PendingJob, tick: usize) {
    let overlay = centered_rect(48, 18, area);
    let spinner = ["-", "\\", "|", "/"][tick % 4];
    let message = format!("{spinner} {}", pending.label());

    frame.render_widget(Clear, overlay);
    let widget = Paragraph::new(message)
        .block(Block::default().title("Working").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(widget, overlay);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencodecommit::config::CliBackend;
    use ratatui::backend::TestBackend;

    fn repo_summary() -> RepoSummary {
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

    fn app() -> App {
        let mut config = Config::default();
        config.backend = CliBackend::Codex;
        App::new(config, repo_summary())
    }

    fn render_lines(app: &App, width: u16, height: u16) -> Vec<String> {
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
        lines
    }

    #[test]
    fn renders_home_screen_details() {
        let app = app();
        let text = render_lines(&app, 80, 24).join("\n");
        assert!(text.contains("Launcher"));
        assert!(text.contains("Commit"));
        assert!(text.contains("Codex CLI: ready"));
    }

    #[test]
    fn renders_commit_screen_placeholder() {
        let mut app = app();
        app.screen = Screen::Commit;
        let text = render_lines(&app, 80, 24).join("\n");
        assert!(text.contains("Press g to generate a commit message."));
        assert!(text.contains("Changed Files"));
    }

    #[test]
    fn renders_pending_overlay() {
        let mut app = app();
        app.pending = Some(PendingJob::GeneratingCommit);
        let text = render_lines(&app, 80, 24).join("\n");
        assert!(text.contains("Working"));
        assert!(text.contains("Generating commit message"));
    }

    #[test]
    fn renders_generated_commit_message() {
        let mut app = app();
        app.screen = Screen::Commit;
        app.commit.preview = Some(CommitPreview {
            message: "feat: add tui launcher".to_owned(),
            parsed: opencodecommit::response::ParsedCommit {
                type_name: "feat".to_owned(),
                message: "add tui launcher".to_owned(),
                description: None,
            },
            provider: "codex".to_owned(),
            files_analyzed: 3,
            duration_ms: 42,
            changed_files: vec!["src/main.rs".to_owned(), "src/tui.rs".to_owned()],
            branch: "feat-ratatui".to_owned(),
            diff_origin: actions::DiffOrigin::Staged,
        });
        let text = render_lines(&app, 80, 24).join("\n");
        assert!(text.contains("feat: add tui launcher"));
        assert!(text.contains("src/main.rs"));
        assert!(text.contains("staged"));
    }

    #[test]
    fn renders_error_notice() {
        let mut app = app();
        app.notice = Some(Notice {
            kind: NoticeKind::Error,
            message: "backend missing".to_owned(),
        });
        let text = render_lines(&app, 80, 24).join("\n");
        assert!(text.contains("backend"));
        assert!(text.contains("missing"));
    }
}
