//! Pane data types, state classification, and table cell rendering.
//!
//! [`PaneState`] is the central enum — each variant encodes both the process
//! type and its current activity. [`PaneState::from_process`] constructs one
//! from a tmux process name and the pane's visible content. The `type_cell` and
//! `state_cell` methods produce styled [`ratatui`] spans for the table columns.

use crate::theme;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use std::fmt;
use std::time::SystemTime;

mod claude;
mod shell;
mod tc_watcher;

pub use claude::ClaudeStatus;
pub use shell::{ProcessOutcome, ShellKind, ShellStatus};
pub use tc_watcher::TcWatcherStatus;

#[cfg(test)]
mod tests;

/// All information about one pane needed to render a row.
#[derive(Debug, Clone)]
pub struct PaneInfo {
    pub id: PaneId,
    /// This pane is the selected pane within its window (receives input if the window is visible).
    pub pane_active: bool,
    /// This pane's window is the active window in its session (front tab).
    pub window_active: bool,
    /// At least one terminal client is attached to this pane's session (someone is looking at it).
    ///
    /// All three flags must be true for a pane to be truly focused — receiving keyboard input
    /// right now. A pane can have `pane_active && window_active` while `session_attached` is
    /// false when the session exists in the background with no terminal attached to it.
    pub session_attached: bool,
    pub pane_in_mode: bool,    // whether in copy_mode
    pub current_cmd: String,   // foreground process name reported by tmux
    pub last_exit_status: i32, // #{pane_last_exit_status} from tmux
    pub state: PaneState,
    pub last_updated: SystemTime,
    pub last_focused_at: Option<SystemTime>, // when pane became focused; None if never
    pub status_changed_at: Option<SystemTime>, // when PaneState last changed; None until first merge
}

impl PaneInfo {
    /// Returns the most recent of `status_changed_at` and `last_focused_at`,
    /// falling back to `UNIX_EPOCH` if both are `None`.
    pub fn most_recent_activity(&self) -> SystemTime {
        match (self.status_changed_at, self.last_focused_at) {
            (Some(a), Some(b)) => a.max(b),
            (Some(a), None) => a,
            (None, Some(b)) => b,
            (None, None) => SystemTime::UNIX_EPOCH,
        }
    }
}

// ---------------------------------------------------------------------------
// Tmux name / target types
// ---------------------------------------------------------------------------

/// The name of a tmux session.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionName(String);

/// The name of a tmux window.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WindowName(String);

macro_rules! impl_name {
    ($T:ident) => {
        impl $T {
            pub fn new(s: impl Into<String>) -> Self {
                Self(s.into())
            }
        }
        impl AsRef<str> for $T {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }
        impl fmt::Display for $T {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
        impl From<String> for $T {
            fn from(s: String) -> Self {
                Self(s)
            }
        }
        impl From<&str> for $T {
            fn from(s: &str) -> Self {
                Self(s.to_owned())
            }
        }
    };
}

impl_name!(SessionName);
impl_name!(WindowName);

// ---------------------------------------------------------------------------

/// Uniquely identifies a tmux pane.
#[derive(Debug, Clone)]
pub struct PaneId {
    pub session_name: String,
    pub window_index: u32,
    pub window_name: String,
    pub pane_id: u32,
}

impl PaneId {
    /// Returns the tmux target string used to address this pane in tmux commands.
    /// Uses the `%N` pane ID format, which is unique across all sessions and unambiguous.
    pub fn target(&self) -> String {
        format!("%{}", self.pane_id)
    }
}

impl fmt::Display for PaneId {
    /// Human-readable form: `session:window.%pane_id`
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}.%{}",
            self.session_name, self.window_name, self.pane_id
        )
    }
}

/// The full state of a pane — encodes both what is running and what it is doing.
/// Invalid combinations (e.g. Shell + Thinking) are unrepresentable.
#[derive(Debug, Clone, PartialEq)]
pub enum PaneState {
    Shell(ShellKind, ShellStatus),
    Claude(ClaudeStatus),
    TcWatcher(TcWatcherStatus),
    Other(String), // unrecognized process — name stored for display
}

/// A pattern used to identify Claude Code processes from `pane_current_command`.
///
/// Claude Code installs its binary under its own version number (e.g. `"2.1.113"`).
/// Verify with: `tmux list-panes -a -F '#{pane_current_command}'`
/// To support a different naming scheme, add a new variant and entry in `CLAUDE_PATTERNS`.
enum ClaudeProcessPattern {
    /// Matches a specific literal process name.
    ExactName(&'static str),
    /// Matches a semver-style name (`usize.usize.usize`), e.g. `"2.1.113"`.
    Semver,
}

impl ClaudeProcessPattern {
    fn matches(&self, cmd: &str) -> bool {
        match self {
            Self::ExactName(name) => cmd == *name,
            Self::Semver => {
                let parts: Vec<&str> = cmd.split('.').collect();
                parts.len() == 3 && parts.iter().all(|p| p.parse::<u32>().is_ok())
            }
        }
    }
}

const CLAUDE_PATTERNS: &[ClaudeProcessPattern] = &[
    ClaudeProcessPattern::ExactName("claude"),
    ClaudeProcessPattern::Semver,
];

fn is_claude_process(cmd: &str) -> bool {
    CLAUDE_PATTERNS.iter().any(|p| p.matches(cmd))
}

impl PaneState {
    /// Constructs a `PaneState` from a tmux process name, visible content, and copy-mode flag.
    pub fn from_process(cmd: &str, content: &str, pane_in_mode: bool) -> PaneState {
        if cmd == "tc-watcher" {
            return PaneState::TcWatcher(if pane_in_mode {
                TcWatcherStatus::Paused
            } else {
                TcWatcherStatus::Active
            });
        }
        if let Some(kind) = ShellKind::from_process_name(cmd) {
            return PaneState::Shell(kind, ShellStatus::from_pane_content(content));
        }
        if is_claude_process(cmd) {
            return PaneState::Claude(ClaudeStatus::from_pane_content(content));
        }
        PaneState::Other(cmd.to_string())
    }

    /// Urgency tier for sorting: lower = higher in the list.
    /// Tier is the first sort key; within a tier, panes are sorted by recency.
    /// TcWatcher is always tier 0; Claude states needing user action are tier 1; everything else is tier 2.
    pub fn urgency_tier(&self) -> u8 {
        match self {
            PaneState::TcWatcher(_) => 0,
            PaneState::Claude(ClaudeStatus::AwaitingInput)
            | PaneState::Claude(ClaudeStatus::AwaitingPermission)
            | PaneState::Shell(_, ShellStatus::JustFinished { .. }) => 1,
            _ => 2,
        }
    }

    /// Returns a styled [`Line`] for the Type column (the process name).
    pub fn type_cell(&self) -> Line<'_> {
        match self {
            PaneState::Shell(kind, _) => Line::from(Span::styled(
                kind.as_ref(),
                Style::default().fg(theme::SHELL_LABEL),
            )),
            PaneState::Claude(_) => Line::from(Span::styled(
                "claude",
                Style::default().fg(theme::CLAUDE_LABEL),
            )),
            PaneState::TcWatcher(_) => Line::from(Span::styled(
                "tc-watcher",
                Style::default().fg(theme::TC_WATCHER_LABEL),
            )),
            PaneState::Other(name) => {
                Line::from(Span::styled(name.as_str(), Style::default().fg(theme::DIM)))
            }
        }
    }

    /// Returns a styled [`Line`] for the State column (icon, or icon + cmd for `JustFinished`).
    pub fn state_cell(&self) -> Line<'_> {
        if let PaneState::Shell(_, ShellStatus::JustFinished { cmd, outcome }) = self {
            let (icon, color) = match outcome {
                ProcessOutcome::Success => theme::ICON_DONE,
                ProcessOutcome::Failed => theme::ICON_ERROR,
            };
            return Line::from(Span::styled(
                format!("{icon} {cmd}"),
                Style::default().fg(color),
            ));
        }

        let (icon, color) = match self {
            PaneState::Shell(_, status) => status.display(),
            PaneState::Claude(status) => status.display(),
            PaneState::TcWatcher(status) => status.display(),
            PaneState::Other(_) => theme::ICON_UNKNOWN,
        };
        Line::from(Span::styled(icon, Style::default().fg(color)))
    }
}

/// Returns the last non-empty line of `content`.
fn last_nonempty_line(content: &str) -> &str {
    content
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
}
