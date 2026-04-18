//! Terminal UI for the tmux pane monitor.
//!
//! [`App`] owns all UI state. [`App::render`] orchestrates the three sub-widgets
//! (table, footer, help overlay), each implemented in their own module.
//! [`App::handle_key`] maps keypresses to [`AppAction`] values, which `main` dispatches.

mod constants;
mod footer;
mod help;
mod table;

use crate::tmux::pane::{PaneId, PaneInfo};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
};
use std::sync::Arc;
use std::time::Instant;

/// High-level actions produced by input handling and dispatched by `main`.
pub enum AppAction {
    Quit,
    JumpToPane(PaneId),
}

/// Root application state for the TUI.
pub struct App {
    /// Latest snapshot of active panes, shared via [`Arc`] to avoid deep copies.
    pub panes: Arc<Vec<PaneInfo>>,
    selected: usize,
    show_help: bool,
    /// An error to surface in the footer, together with when it was set.
    /// Displayed for a fixed TTL then replaced by the normal key-hint line.
    error: Option<(String, Instant)>,
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
            error: None,
        }
    }

    /// Replaces the pane snapshot. Clamps the selection if it would go out of bounds.
    pub fn update_panes(&mut self, panes: Arc<Vec<PaneInfo>>) {
        self.panes = panes;
        if self.selected >= self.panes.len() {
            self.selected = self.panes.len().saturating_sub(1);
        }
    }

    /// Surfaces an error message in the footer for a short TTL.
    pub fn set_error(&mut self, msg: String) {
        self.error = Some((msg, Instant::now()));
    }

    /// Translates a raw key event into an [`AppAction`], updating local navigation
    /// state as a side effect. Returns `None` for keys with no binding.
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
            KeyCode::Enter => self
                .panes
                .get(self.selected)
                .map(|p| AppAction::JumpToPane(p.id.clone())),
            _ => None,
        }
    }

    fn next(&mut self) {
        if !self.panes.is_empty() {
            self.selected = (self.selected + 1) % self.panes.len();
        }
    }

    fn prev(&mut self) {
        if !self.panes.is_empty() {
            self.selected = self.selected.saturating_sub(1);
        }
    }

    /// Renders the full UI: pane table, footer (or error banner), and optional help overlay.
    pub fn render(&self, frame: &mut Frame) {
        let [table_area, footer_area] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(frame.area());

        table::render(frame, table_area, &self.panes, self.selected);
        footer::render(frame, footer_area, self.error.as_ref());

        if self.show_help {
            help::render(frame);
        }
    }
}
