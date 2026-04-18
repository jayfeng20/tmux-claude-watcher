//! Terminal UI for the tmux pane monitor.
//!
//! Renders a live table of all active tmux panes using [`ratatui`], with
//! color-coded state, focus timing, and keyboard navigation.
//!
//! Input handling is intentionally separated from behavior: [`App::handle_key`]
//! maps keypresses to [`AppAction`] values, which `main` dispatches. Adding a
//! new action (e.g. jumping to a pane) only requires a new [`AppAction`] variant
//! and a new match arm — no other files need to change.

use crate::theme;
use crate::tmux::pane::PaneInfo;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table},
};
use std::sync::Arc;
use std::time::SystemTime;

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
    show_help: bool,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        App {
            panes: Arc::new(vec![]),
            selected: 0,
            show_help: false,
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
            KeyCode::Char('?') | KeyCode::Esc if self.show_help => {
                self.show_help = false;
                None
            }
            KeyCode::Char('?') => {
                self.show_help = true;
                None
            }
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
            Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(frame.area());

        let header = Row::new(["ID", "Type", "State", "Active", "Last Updated"])
            .style(Style::default().add_modifier(Modifier::BOLD));

        let rows: Vec<Row> = self
            .panes
            .iter()
            .enumerate()
            .map(|(i, pane)| {
                let style = if i == self.selected {
                    Style::default().bg(theme::SURFACE1)
                } else {
                    Style::default()
                };
                // A pane is truly focused only when all three tmux flags align:
                // session_attached (a terminal is viewing this session),
                // window_active (this is the front window in that session), and
                // pane_active (this is the selected pane within that window).
                let is_active = pane.session_attached && pane.window_active && pane.pane_active;
                let (active_label, active_color) = if is_active {
                    ("yes", theme::GREEN)
                } else {
                    ("no", theme::OVERLAY0)
                };
                Row::new(vec![
                    Cell::from(format!(
                        "{}:{}.{}",
                        pane.id.session_name, pane.id.window_index, pane.id.pane_id
                    )),
                    Cell::from(pane.state.type_cell()),
                    Cell::from(pane.state.state_cell()),
                    Cell::from(active_label).style(Style::default().fg(active_color)),
                    Cell::from(format_ago(pane.status_changed_at)),
                ])
                .style(style)
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(20), // session:window.pane — e.g. "main:0.3"
                Constraint::Length(10), // process name — e.g. "claude", "zsh"
                Constraint::Length(6),  // state icon — e.g. ">_", "◉", "◌"
                Constraint::Length(8),  // active yes/no
                Constraint::Length(13), // last updated — e.g. "42s ago"
            ],
        )
        .header(header)
        .block(
            Block::default()
                .title("Tmux Pane Monitor")
                .borders(Borders::ALL),
        );

        frame.render_widget(table, table_area);

        let footer = Paragraph::new(Span::raw("q quit  ↑↓/jk navigate  ? help"))
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(footer, footer_area);

        if self.show_help {
            render_help(frame);
        }
    }
}

fn render_help(frame: &mut Frame) {
    let area = centered_rect(44, 20, frame.area());
    frame.render_widget(Clear, area);

    let lines = vec![
        Line::from(Span::styled(
            "State icons",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("◌ ", Style::default().fg(theme::PEACH)),
            Span::raw(" Thinking        (claude)"),
        ]),
        Line::from(vec![
            Span::styled("◑ ", Style::default().fg(theme::YELLOW)),
            Span::raw(" Executing       (claude)"),
        ]),
        Line::from(vec![
            Span::styled("❯ ", Style::default().fg(theme::RED)),
            Span::raw(" Awaiting input"),
        ]),
        Line::from(vec![
            Span::styled("! ", Style::default().fg(theme::RED)),
            Span::raw(" Awaiting permission (claude)"),
        ]),
        Line::from(vec![
            Span::styled("✓ ", Style::default().fg(theme::GREEN)),
            Span::raw(" Done             (claude)"),
        ]),
        Line::from(vec![
            Span::styled("○ ", Style::default().fg(theme::GREEN)),
            Span::raw(" Idle            (shell)"),
        ]),
        Line::from(vec![
            Span::styled("✗ ", Style::default().fg(theme::RED)),
            Span::raw(" Error           (shell)"),
        ]),
        Line::from(vec![
            Span::styled("? ", Style::default().fg(theme::OVERLAY0)),
            Span::raw(" Unknown"),
        ]),
        Line::raw(""),
        Line::from(Span::styled(
            "Columns",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::raw("ID      session:window.pane"),
        Line::raw("Type    process (zsh, claude, …)"),
        Line::raw("State   activity icon"),
        Line::raw("Active  focused pane in window"),
        Line::raw("Last Updated  time since state changed"),
        Line::raw(""),
        Line::from(Span::styled(
            "? or Esc to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let help = Paragraph::new(lines).block(
        Block::default()
            .title("Help")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black)),
    );
    frame.render_widget(help, area);
}

/// Returns a centered rect of the given width and height within `area`.
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}

fn format_ago(t: Option<SystemTime>) -> String {
    let t = match t {
        None => return "never".to_string(),
        Some(t) => t,
    };
    match t.elapsed() {
        Ok(d) if d.as_secs() < 60 => format!("{}s ago", d.as_secs()),
        Ok(d) => format!("{}m ago", d.as_secs() / 60),
        Err(_) => "—".to_string(),
    }
}
