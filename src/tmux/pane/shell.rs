//! Shell pane classification — kind (bash/zsh/fish/sh) and status (idle/awaiting/just-finished).

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

/// Exit outcome of the most recently completed subprocess.
#[derive(Debug, Clone, PartialEq)]
pub enum ProcessOutcome {
    Success,
    Failed,
}

impl ProcessOutcome {
    pub fn from_exit_status(code: i32) -> Self {
        if code == 0 {
            Self::Success
        } else {
            Self::Failed
        }
    }
}

/// What a shell pane is currently doing.
#[derive(Debug, Clone, PartialEq)]
pub enum ShellStatus {
    /// Shell prompt visible — user can freely type a command.
    Idle,
    /// Shell is foreground but no prompt visible — e.g. `read`, sudo password, select menu.
    AwaitingInput,
    /// A subprocess just finished; shown until the user focuses the pane.
    JustFinished {
        cmd: String,
        outcome: ProcessOutcome,
    },
}

impl ShellStatus {
    /// Infers idle vs awaiting from the pane's visible content.
    pub fn from_pane_content(content: &str) -> ShellStatus {
        if Self::has_prompt(content) {
            ShellStatus::Idle
        } else {
            ShellStatus::AwaitingInput
        }
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
            ShellStatus::Idle => theme::ICON_IDLE,
            ShellStatus::AwaitingInput => theme::ICON_AWAITING_INPUT,
            ShellStatus::JustFinished { outcome, .. } => match outcome {
                ProcessOutcome::Success => theme::ICON_DONE,
                ProcessOutcome::Failed => theme::ICON_ERROR,
            },
        }
    }
}
