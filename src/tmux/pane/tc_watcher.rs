//! State classification for tc-watcher panes.
//!
//! The monitor recognises itself by process name (`tc-watcher`) and renders
//! with a distinctive label colour. The `Stopped` state is inferred by
//! [`crate::tmux::pane_manager::PaneManager`] via state history: when a pane
//! transitions from `TcWatcher(_)` to a shell process, the manager overrides
//! the new state back to `TcWatcher(Stopped)` so the pane stays visually
//! identifiable until tc-watcher is relaunched.

use crate::theme;
use ratatui::style::Color;

/// What the tc-watcher monitor pane is currently doing.
#[derive(Debug, Clone, PartialEq)]
pub enum TcWatcherStatus {
    /// Monitor is running and actively polling pane states.
    Active,
    /// Pane is in tmux copy/scroll mode — monitor output is frozen while the
    /// user scrolls. Detected via the `pane_in_mode` tmux flag.
    Paused,
}

impl TcWatcherStatus {
    pub(super) fn display(&self) -> (&'static str, Color) {
        match self {
            TcWatcherStatus::Active => theme::ICON_IDLE,
            TcWatcherStatus::Paused => theme::ICON_PAUSED,
        }
    }
}
