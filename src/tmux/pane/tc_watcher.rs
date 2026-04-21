//! State classification for tc-watcher panes — active (polling)
//! or paused (`tmux` copy mode).

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
