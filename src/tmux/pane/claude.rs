use crate::theme;
use ratatui::style::Color;

const SPINNERS: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

/// What a Claude pane is currently doing.
#[derive(Debug, Clone, PartialEq)]
pub enum ClaudeStatus {
    AwaitingInput,      // input box visible — waiting for next message
    AwaitingPermission, // tool permission prompt — needs user approval to proceed
    Thinking,           // extended reasoning in progress ("esc to interrupt" + thinking indicators)
    Executing, // tool active ("esc to interrupt" present — generating, running tools, etc.)
    Unknown,   // state could not be determined from visible content
}

impl ClaudeStatus {
    /// Infers Claude's current status from the pane's visible content.
    /// Checks are evaluated in priority order.
    pub fn from_pane_content(content: &str) -> ClaudeStatus {
        // "esc to interrupt" is shown as the last line whenever Claude is actively working.
        // It appears in both thinking and tool-execution states.
        if Self::is_working(content) {
            return if Self::has_thinking_indicators(content) {
                ClaudeStatus::Thinking
            } else {
                ClaudeStatus::Executing
            };
        }
        // Tool permission prompt: Claude needs user approval before proceeding.
        // "Esc to cancel · Tab to amend" is shown at the bottom of every permission prompt.
        if Self::is_permission_prompt(content) {
            return ClaudeStatus::AwaitingPermission;
        }
        // Normal chat input box: ────/❯ or ╭─ visible in the last few lines.
        if Self::is_input_box(content) {
            return ClaudeStatus::AwaitingInput;
        }
        ClaudeStatus::Unknown
    }

    fn is_working(content: &str) -> bool {
        super::last_nonempty_line(content)
            .to_lowercase()
            .contains("esc to interrupt")
    }

    fn has_thinking_indicators(content: &str) -> bool {
        // "thinking)" appears in Claude's progress lines (e.g. "· Sock-hopping… thinking)").
        // Spinner chars can appear anywhere above "esc to interrupt", so scan all lines.
        content.contains("thinking)")
            || content
                .lines()
                .any(|l| l.chars().next().is_some_and(|c| SPINNERS.contains(&c)))
    }

    fn is_permission_prompt(content: &str) -> bool {
        // "Esc to cancel · Tab to amend" is always the last visible line of a permission prompt.
        let last = super::last_nonempty_line(content);
        last.contains("Esc to cancel") || last.contains("Tab to amend")
    }

    fn is_input_box(content: &str) -> bool {
        // The input box is bounded at the bottom by a ─ separator line.
        // capture-pane pads to terminal height with blank lines, so skip those first,
        // then check whether either of the last 2 non-empty lines is the separator.
        let last_two: Vec<&str> = content
            .lines()
            .rev()
            .filter(|l| !l.trim().is_empty())
            .take(2)
            .collect();
        last_two.iter().any(|l| l.contains('─'))
    }

    pub(super) fn display(&self) -> (&'static str, Color) {
        match self {
            ClaudeStatus::AwaitingInput => (">_", theme::GREEN),
            ClaudeStatus::AwaitingPermission => ("!", theme::SAPPHIRE),
            ClaudeStatus::Thinking => ("◌", theme::YELLOW),
            ClaudeStatus::Executing => ("▶", theme::TEAL),
            ClaudeStatus::Unknown => ("?", theme::OVERLAY0),
        }
    }
}
