use crate::theme;
use ratatui::style::Color;

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
        let last = super::last_nonempty_line(content);
        last.chars().next().is_some_and(|c| SPINNERS.contains(&c))
            || content.to_lowercase().contains("thinking")
    }

    fn is_executing(content: &str) -> bool {
        content.contains('⏺')
    }

    fn is_awaiting_input(content: &str) -> bool {
        // Claude's input box uses box-drawing characters in the last few visible lines.
        // The ❯ character appears in option-selection prompts (e.g. "do you want to proceed?").
        let last_few = content.lines().rev().take(5).collect::<Vec<_>>().join("\n");
        last_few.contains('╭') || last_few.contains('│') || last_few.contains('❯')
    }

    pub(super) fn display(&self) -> (&'static str, Color) {
        match self {
            ClaudeStatus::AwaitingInput => (">_", theme::OVERLAY2),
            ClaudeStatus::Generating => ("◉", theme::PEACH),
            ClaudeStatus::Thinking => ("◌", theme::SKY),
            ClaudeStatus::Executing => ("⚙", theme::GREEN),
            ClaudeStatus::Idle => ("○", theme::OVERLAY0),
            ClaudeStatus::Error => ("✗", theme::RED),
        }
    }
}
