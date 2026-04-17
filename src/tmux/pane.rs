use ratatui::style::Color;
use std::time::SystemTime;
use strum::AsRefStr;

#[cfg(test)]
#[path = "pane_tests.rs"]
mod tests;

/// All information about one pane needed to render a row.
#[derive(Debug, Clone)]
pub struct PaneInfo {
    pub id: PaneId,
    pub pane_active: bool,   // whether this is the currently focused pane
    pub pane_in_mode: bool,  // whether in copy_mode
    pub current_cmd: String, // foreground process name reported by tmux
    pub state: PaneState,
    pub last_updated: SystemTime,
    pub last_focused_at: Option<SystemTime>, // when pane_active last flipped false→true; None if never observed
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
    Other(String), // unrecognized process — name stored for display
}

impl PaneState {
    /// Constructs a `PaneState` from a tmux process name and the pane's visible content.
    pub fn from_process(cmd: &str, content: &str) -> PaneState {
        if let Some(kind) = ShellKind::from_process_name(cmd) {
            return PaneState::Shell(kind, ShellStatus::from_pane_content(content));
        }
        if cmd == "claude" {
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
            PaneState::Other(name) => (format!("? {}", name), Color::Gray),
        }
    }
}

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
    AwaitingInput, // prompt visible — waiting for a command
    Processing,    // command is running
    Idle,          // no content / inactive
    Error,         // error output visible
}

fn last_nonempty_line(content: &str) -> &str {
    content
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
}

impl ShellStatus {
    /// Infers the shell's current status from the pane's visible content.
    /// Checks are evaluated in priority order.
    pub fn from_pane_content(content: &str) -> ShellStatus {
        if Self::has_error(content) {
            return ShellStatus::Error;
        }
        if Self::has_prompt(content) {
            return ShellStatus::AwaitingInput;
        }
        if content.trim().is_empty() {
            return ShellStatus::Idle;
        }
        ShellStatus::Processing
    }

    fn has_error(content: &str) -> bool {
        let last = last_nonempty_line(content);
        last.contains("command not found")
            || last.contains("No such file")
            || last.starts_with("bash:")
            || last.starts_with("zsh:")
    }

    fn has_prompt(content: &str) -> bool {
        let last = last_nonempty_line(content).trim_end();
        last.ends_with('$')
            || last.ends_with('%')
            || last.ends_with('>')
            || last.ends_with('~')
            || last.ends_with('#')
    }

    fn display(&self) -> (&'static str, Color) {
        match self {
            ShellStatus::AwaitingInput => (">_", Color::Gray),
            ShellStatus::Processing => ("◉", Color::LightYellow),
            ShellStatus::Idle => ("○", Color::DarkGray),
            ShellStatus::Error => ("✗", Color::LightRed),
        }
    }
}

/// What a Claude pane is currently doing.
#[derive(Debug, Clone, PartialEq)]
pub enum ClaudeStatus {
    AwaitingInput, // input box (╭─) visible — waiting for a message
    Generating,    // streaming text output
    Thinking,      // extended reasoning phase (spinner visible)
    Executing,     // tool use in progress (⏺ marker visible)
    Idle,          // open but no activity
    Error,         // error output visible
}

impl ClaudeStatus {
    /// Infers Claude's current status from the pane's visible content.
    /// Checks are evaluated in priority order.
    pub fn from_pane_content(content: &str) -> ClaudeStatus {
        if Self::has_error(content) {
            return ClaudeStatus::Error;
        }
        if Self::is_thinking(content) {
            return ClaudeStatus::Thinking;
        }
        if Self::is_executing(content) {
            return ClaudeStatus::Executing;
        }
        if Self::is_awaiting_input(content) {
            return ClaudeStatus::AwaitingInput;
        }
        if content.trim().is_empty() {
            return ClaudeStatus::Idle;
        }
        ClaudeStatus::Generating
    }

    fn has_error(content: &str) -> bool {
        content.contains("Error:") || content.contains("✗ ")
    }

    fn is_thinking(content: &str) -> bool {
        const SPINNERS: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        let last = last_nonempty_line(content);
        last.chars().next().is_some_and(|c| SPINNERS.contains(&c))
            || content.to_lowercase().contains("thinking")
    }

    fn is_executing(content: &str) -> bool {
        content.contains('⏺')
    }

    fn is_awaiting_input(content: &str) -> bool {
        // Claude's input box uses box-drawing characters in the last few visible lines
        let last_few = content.lines().rev().take(5).collect::<Vec<_>>().join("\n");
        last_few.contains('╭') || last_few.contains('│') || last_few.contains('❯')
    }

    fn display(&self) -> (&'static str, Color) {
        match self {
            ClaudeStatus::AwaitingInput => (">_", Color::Gray),
            ClaudeStatus::Generating => ("◉", Color::LightYellow),
            ClaudeStatus::Thinking => ("◌", Color::LightCyan),
            ClaudeStatus::Executing => ("⚙", Color::LightYellow),
            ClaudeStatus::Idle => ("○", Color::DarkGray),
            ClaudeStatus::Error => ("✗", Color::LightRed),
        }
    }
}
