use crate::theme;
use ratatui::style::Color;
use strum::AsRefStr;

/// The specific shell variant.
#[derive(Debug, Clone, PartialEq, AsRefStr)]
#[strum(serialize_all = "lowercase")]
pub enum ShellKind {
    Bash,
    Zsh,
    Fish,
    Sh, // covers sh, dash, and other POSIX-compatible shells
}

impl ShellKind {
    /// Maps a process name to a `ShellKind`, or `None` if not a recognized shell.
    pub fn from_process_name(cmd: &str) -> Option<ShellKind> {
        match cmd {
            "bash" => Some(ShellKind::Bash),
            "zsh" => Some(ShellKind::Zsh),
            "fish" => Some(ShellKind::Fish),
            "sh" | "dash" => Some(ShellKind::Sh),
            _ => None,
        }
    }
}

/// What a shell pane is currently doing.
#[derive(Debug, Clone, PartialEq)]
pub enum ShellStatus {
    /// Shell prompt visible (%, $, #) — user can freely type a command.
    Idle,
    /// A running process is requesting input; format varies widely, so this is the fallback.
    AwaitingInput,
    /// Error output visible on the last line.
    Error,
}

impl ShellStatus {
    /// Infers the shell's current status from the pane's visible content.
    /// Checks are evaluated in priority order.
    pub fn from_pane_content(content: &str) -> ShellStatus {
        if Self::has_error(content) {
            return ShellStatus::Error;
        }
        if Self::has_prompt(content) {
            return ShellStatus::Idle;
        }
        ShellStatus::AwaitingInput
    }

    fn has_error(content: &str) -> bool {
        let last = super::last_nonempty_line(content);
        last.contains("command not found")
            || last.contains("No such file")
            || last.starts_with("bash:")
            || last.starts_with("zsh:")
    }

    fn has_prompt(content: &str) -> bool {
        let last = super::last_nonempty_line(content).trim_end();
        last.ends_with('$')
            || last.ends_with('%')
            || last.ends_with('>')
            || last.ends_with('~')
            || last.ends_with('#')
    }

    pub(super) fn display(&self) -> (&'static str, Color) {
        match self {
            ShellStatus::Idle => ("○", theme::GREEN),
            ShellStatus::AwaitingInput => ("❯", theme::RED),
            ShellStatus::Error => ("✗", theme::RED),
        }
    }
}
