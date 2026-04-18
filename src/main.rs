//! Entry point for the tmux pane monitor.
//!
//! Spawns a background thread that polls [`PaneManager::refresh`] every 2 seconds,
//! and runs an event-driven [`ratatui`] loop on the main thread that redraws only
//! when new pane data arrives or the user presses a key.

use crossterm::event::KeyEventKind;
use crossterm::{
    event::{Event, EventStream},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures::StreamExt;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::sync::Arc;
use std::time::Duration;
use tmux_claude_watcher::tmux::{
    pane_manager::PaneManager,
    ui::{App, AppAction},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Log to a rolling daily file — we must not write to stdout while ratatui owns it.
    let file_appender = tracing_appender::rolling::daily("/tmp", "tmux-claude-watcher.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt().with_writer(non_blocking).init();

    tracing::info!("starting pane monitor");

    // Put the terminal into raw mode and switch to the alternate screen buffer
    // so we don't clobber the user's shell history.
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;

    // `watch` channel: the sender wraps the snapshot in an Arc (no deep copy),
    // and the receiver clones the Arc (O(1) refcount bump) rather than the data.
    let (tx, mut rx) = tokio::sync::watch::channel::<Arc<Vec<_>>>(Arc::new(vec![]));

    // Polling task — runs on a dedicated blocking thread because
    // `std::process::Command` is synchronous and must not block the async executor.
    tokio::task::spawn_blocking(move || {
        let mut manager = PaneManager::new();
        loop {
            match manager.refresh() {
                Ok(()) => {
                    // panes() returns an Arc clone — O(1), no data copied.
                    if tx.send(manager.panes()).is_err() {
                        break; // UI has exited, receiver dropped.
                    }
                }
                Err(e) => tracing::error!(error = %e, "refresh failed"),
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    });

    let mut app = App::new();
    // EventStream wakes the loop only when a real key event occurs — no polling.
    let mut events = EventStream::new();

    loop {
        tokio::select! {
            // New pane snapshot published by the poller.
            Ok(()) = rx.changed() => {
                app.update_panes(Arc::clone(&rx.borrow_and_update()));
            }
            // User pressed a key.
            Some(Ok(Event::Key(key))) = events.next() => {
                // Filter out key-release events (only process key-press/repeat).
                if key.kind == KeyEventKind::Press
                    && let Some(action) = app.handle_key(key)
                {
                    match action {
                        AppAction::Quit => break,
                    }
                }
            }
        }

        terminal.draw(|f| app.render(f))?;
    }

    tracing::info!("shutting down");

    // Restore the terminal to its original state before exiting.
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
