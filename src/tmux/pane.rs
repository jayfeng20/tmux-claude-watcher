use crate::theme;
use ratatui::style::Color;
use std::time::SystemTime;

mod claude;
mod shell;

pub use claude::ClaudeStatus;
pub use shell::{ShellKind, ShellStatus};

#[cfg(test)]
mod tests;

/// All information about one pane needed to render a row.
#[derive(Debug, Clone)]
pub struct PaneInfo {
    pub id: PaneId,
    pub pane_active: bool,   // whether this is the active pane within its window
    pub window_active: bool, // whether this pane's window is the active window in its session
    pub pane_in_mode: bool,  // whether in copy_mode
    pub current_cmd: String, // foreground process name reported by tmux
    pub state: PaneState,
    pub last_updated: SystemTime,
    pub last_focused_at: Option<SystemTime>, // when pane became focused (window_active && pane_active); None if never
    pub status_changed_at: Option<SystemTime>, // when PaneState last changed; None until first merge
}

/// Uniquely identifies a tmux pane.
#[derive(Debug, Clone)]
pub struct PaneId {
    pub session_name: String,
    pub window_index: u32,
    pub window_name: String,
    pub pane_id: u32,
}

impl PaneId {
    /// Returns the tmux target string used to address this pane in tmux commands,
    /// e.g. `tmux capture-pane -t "main:editor.3"`.
    pub fn target(&self) -> String {
        format!(
            "{}:{}.{}",
            self.session_name, self.window_index, self.pane_id
        )
    }
}

/// The full state of a pane — encodes both what is running and what it is doing.
/// Invalid combinations (e.g. Shell + Thinking) are unrepresentable.
#[derive(Debug, Clone, PartialEq)]
pub enum PaneState {
    Shell(ShellKind, ShellStatus),
    Claude(ClaudeStatus),
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
    /// Constructs a `PaneState` from a tmux process name and the pane's visible content.
    pub fn from_process(cmd: &str, content: &str) -> PaneState {
        if let Some(kind) = ShellKind::from_process_name(cmd) {
            return PaneState::Shell(kind, ShellStatus::from_pane_content(content));
        }
        if is_claude_process(cmd) {
            return PaneState::Claude(ClaudeStatus::from_pane_content(content));
        }
        PaneState::Other(cmd.to_string())
    }

    /// Returns (display label, color) for the State column.
    /// Label is composed as "{icon} {name}", e.g. ">_ bash", "◌ claude".
    pub fn display(&self) -> (String, Color) {
        match self {
            PaneState::Shell(kind, status) => {
                let (icon, color) = status.display();
                (format!("{} {}", icon, kind.as_ref()), color)
            }
            PaneState::Claude(status) => {
                let (icon, color) = status.display();
                (format!("{} claude", icon), color)
            }
            PaneState::Other(name) => (format!("? {}", name), theme::SUBTEXT0),
        }
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
