//! Terminal UI for the tmux pane monitor.
//!
//! Renders a live table of all active tmux panes using [`ratatui`], with
//! color-coded state, focus timing, and keyboard navigation.
//!
//! Input handling is intentionally separated from behavior: [`App::handle_key`]
//! maps keypresses to [`AppAction`] values, which `main` dispatches. Adding a
//! new action (e.g. jumping to a pane) only requires a new [`AppAction`] variant
//! and a new match arm — no other files need to change.

use crate::tmux::pane::PaneInfo;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// High-level actions produced by input handling and dispatched by `main`.
///
/// Keeping actions separate from key bindings means new behaviors can be added
/// without touching the event loop — just add a variant and handle it in `main`.
pub enum AppAction {
    Quit,
    // Future examples:
    // JumpToPane(crate::tmux::pane::PaneId),
    // KillPane(crate::tmux::pane::PaneId),
}

/// Root application state for the TUI.
pub struct App {
    /// Latest snapshot of active panes, shared via [`Arc`] to avoid deep copies.
    pub panes: Arc<Vec<PaneInfo>>,
    selected: usize,
}

impl App {
    pub fn new() -> Self {
        App {
            panes: Arc::new(vec![]),
            selected: 0,
        }
    }

    /// Replaces the pane snapshot with a new one received from the poller.
    /// Resets the selection if it would be out of bounds after the update.
    pub fn update_panes(&mut self, panes: Arc<Vec<PaneInfo>>) {
        self.panes = panes;
        // Clamp selection so it stays valid after panes are added or removed.
        if self.selected >= self.panes.len() {
            self.selected = self.panes.len().saturating_sub(1);
        }
    }

    /// Translates a raw key event into an [`AppAction`], updating local navigation
    /// state as a side effect. Returns `None` for keys that have no binding.
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<AppAction> {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => Some(AppAction::Quit),
            KeyCode::Down | KeyCode::Char('j') => {
                self.next();
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.prev();
                None
            }
            _ => None,
        }
    }

    /// Moves the selection down one row, wrapping at the bottom.
    fn next(&mut self) {
        if !self.panes.is_empty() {
            self.selected = (self.selected + 1) % self.panes.len();
        }
    }

    /// Moves the selection up one row, clamping at the top.
    fn prev(&mut self) {
        if !self.panes.is_empty() {
            self.selected = self.selected.saturating_sub(1);
        }
    }

    /// Renders the full UI into `frame`: a pane table and a key-binding footer.
    pub fn render(&self, frame: &mut Frame) {
        // Split the screen vertically: table takes all available space,
        // footer is a fixed single line at the bottom.
        let [table_area, footer_area] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(1)])
                .areas(frame.area());

        // Bold header row — column names match the data rendered below.
        let header = Row::new(["ID", "Session:Win", "State", "Last Focus", "Status For"])
            .style(Style::default().add_modifier(Modifier::BOLD));

        // Build one row per pane. The selected row gets a dark background;
        // the State cell is colored according to the pane's current status.
        let rows: Vec<Row> = self
            .panes
            .iter()
            .enumerate()
            .map(|(i, pane)| {
                let (label, color) = pane.state.display();
                let style = if i == self.selected {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                };
                Row::new(vec![
                    Cell::from(pane.id.pane_id.to_string()),
                    Cell::from(format!("{}:{}", pane.id.session_name, pane.id.window_name)),
                    Cell::from(label).style(Style::default().fg(color)),
                    Cell::from(format_ago(pane.last_focused_at)),
                    Cell::from(format_ago(pane.status_changed_at)),
                ])
                .style(style)
            })
            .collect();

        // Assemble the table: fixed-width columns for IDs and timing,
        // Min(0) for Session:Win so it stretches to fill remaining space.
        let table = Table::new(
            rows,
            [
                Constraint::Length(5),
                Constraint::Min(0),
                Constraint::Length(18),
                Constraint::Length(12),
                Constraint::Length(12),
            ],
        )
        .header(header)
        .block(Block::default().title("Tmux Pane Monitor").borders(Borders::ALL));

        frame.render_widget(table, table_area);

        // Key-binding hint rendered as a dim single line below the table.
        let footer = Paragraph::new(Span::raw("q quit  ↑↓/jk navigate"))
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(footer, footer_area);
    }
}

/// Formats a [`SystemTime`] as a human-readable "X ago" string.
///
/// Returns `"never"` for [`UNIX_EPOCH`] (the unset sentinel),
/// `"Xs ago"` for durations under a minute, and `"Xm ago"` otherwise.
fn format_ago(t: SystemTime) -> String {
    if t == UNIX_EPOCH {
        return "never".to_string();
    }
    match t.elapsed() {
        Ok(d) if d.as_secs() < 60 => format!("{}s ago", d.as_secs()),
        Ok(d) => format!("{}m ago", d.as_secs() / 60),
        Err(_) => "—".to_string(),
    }
}
