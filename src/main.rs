//! Entry point for the tmux pane monitor.
//!
//! `main` is intentionally thin: it wires together terminal setup, the
//! background poller, and the event loop. Each concern lives in its own
//! helper so the top-level flow is readable at a glance.

use clap::Parser as _;
use crossterm::{
    event::{Event, EventStream, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures::StreamExt;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::Stdout;
use std::sync::Arc;
use std::time::Duration;
use tmux_claude_watcher::return_key::{Args, Binding};
use tmux_claude_watcher::tmux::{
    pane::PaneInfo,
    pane_actions::{jump_to_pane, kill_pane, new_session, new_window, split_pane},
    pane_manager::PaneManager,
    ui::{App, AppAction},
};

type BoxError = Box<dyn std::error::Error + Send + Sync>;
type PaneSnapshot = Arc<Vec<PaneInfo>>;

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    require_tmux();
    setup_logging();

    let args = Args::parse();
    let binding = Binding::register(args.return_key);

    let mut terminal = setup_terminal()?;
    let (tx, mut rx) = tokio::sync::watch::channel::<PaneSnapshot>(Arc::new(vec![]));
    spawn_poller(tx);

    let mut app = App::new();

    // Surface the return-key binding outcome in the footer.
    match &binding {
        Ok(b) => {
            if let Some(key) = b.key {
                app.set_return_key(key);
            }
        }
        Err(e) => app.set_error(format!("return key: {e}")),
    }
    let binding = binding.unwrap_or(Binding { key: None });

    let mut events = EventStream::new();

    'event_loop: loop {
        tokio::select! {
            Ok(()) = rx.changed() => {
                app.update_panes(Arc::clone(&rx.borrow_and_update()));
            }
            Some(Ok(Event::Key(key))) = events.next() => {
                if key.kind == KeyEventKind::Press {
                    if let Some(action) = app.handle_key(key) {
                        if dispatch(&mut app, action) {
                            break 'event_loop;
                        }
                    }
                }
            }
        }
        terminal.draw(|f| app.render(f))?;
    }

    restore_terminal(&mut terminal)?;
    binding.deregister();
    Ok(())
}

// ---------------------------------------------------------------------------
// Action dispatch
// ---------------------------------------------------------------------------

/// Dispatches an [`AppAction`]. Returns `true` if the app should quit.
fn dispatch(app: &mut App, action: AppAction) -> bool {
    match action {
        AppAction::Quit => return true,
        AppAction::JumpToPane(id) => {
            if let Err(e) = jump_to_pane(&id) {
                on_err(app, e, "jump to pane")
            }
        }
        AppAction::NewSession { name } => {
            if let Err(e) = new_session(&name) {
                on_err(app, e, "new session")
            }
        }
        AppAction::NewWindow { session, name } => {
            if let Err(e) = new_window(&session, &name) {
                on_err(app, e, "new window")
            }
        }
        AppAction::SplitPane { session, window } => {
            if let Err(e) = split_pane(&session, &window) {
                on_err(app, e, "split pane")
            }
        }
        AppAction::DeletePane(id) => {
            if let Err(e) = kill_pane(&id) {
                on_err(app, e, "delete pane")
            }
        }
    }
    false
}

/// Logs `e` and surfaces it in the footer for the error TTL.
fn on_err(app: &mut App, e: impl std::fmt::Display, ctx: &'static str) {
    tracing::error!(error = %e, "{ctx}");
    app.set_error(e.to_string());
}

// ---------------------------------------------------------------------------
// Setup / teardown
// ---------------------------------------------------------------------------

fn spawn_poller(tx: tokio::sync::watch::Sender<PaneSnapshot>) {
    tokio::task::spawn_blocking(move || {
        let mut manager = PaneManager::new();
        loop {
            match manager.refresh() {
                Ok(()) => {
                    if tx.send(manager.panes()).is_err() {
                        break; // UI has exited, receiver dropped.
                    }
                }
                Err(e) => tracing::error!(error = %e, "refresh failed"),
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    });
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>, BoxError> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<(), BoxError> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

fn setup_logging() {
    let file_appender = tracing_appender::rolling::daily("/tmp", "tmux-claude-watcher.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt().with_writer(non_blocking).init();
    tracing::info!("starting pane monitor");
}

/// Exits with a clear message if not running inside a tmux session.
fn require_tmux() {
    if std::env::var("TMUX").is_err() {
        const RED: &str = "\x1b[38;2;243;139;168m";
        const TEAL: &str = "\x1b[38;2;148;226;213m";
        const RESET: &str = "\x1b[0m";
        eprintln!("{RED}error{RESET}: tc-watcher must be run inside a tmux session.");
        eprintln!("To create one: {TEAL}tmux new-session -s <your-session-name>{RESET}");
        std::process::exit(1);
    }
}
