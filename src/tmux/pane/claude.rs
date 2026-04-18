//! Claude Code pane classification — infers status from visible pane content.

use crate::theme;
use ratatui::style::Color;

const SPINNERS: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

/// Bullet symbols Claude Code prefixes tool/status lines with while actively working.
const WORKING_INDICATORS: &[char] = &['✢', '✶', '✻', '✳', '·'];

/// What a Claude pane is currently doing.
#[derive(Debug, Clone, PartialEq)]
pub enum ClaudeStatus {
    /// Input box visible and Claude is asking a question (? found just above the text box).
    AwaitingInput,
    /// Input box visible, task just completed — Claude is not asking anything.
    Done,
    /// Tool permission prompt — needs user approval to proceed.
    AwaitingPermission,
    /// Extended reasoning in progress.
    Thinking,
    /// Generating, running tools, etc.
    Executing,
    /// State could not be determined from visible content.
    Unknown,
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
        // Input box visible: distinguish a question (? above the box) from task completion.
        if Self::is_input_box(content) {
            return if Self::is_asking_question(content) {
                ClaudeStatus::AwaitingInput
            } else {
                ClaudeStatus::Done
            };
        }
        ClaudeStatus::Unknown
    }

    fn is_working(content: &str) -> bool {
        // One of WORKING_INDICATORS starts a tool/status line in positions 4–7 from the bottom
        // AND the line contains "ing…" — distinguishes active work (e.g. "✻ Processing…")
        // from a completion summary that also uses WORKING_INDICATORS (e.g. "✻ Churned for 43s").
        content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .rev()
            .skip(3)
            .take(4)
            .any(|l| {
                l.chars()
                    .next()
                    .is_some_and(|c| WORKING_INDICATORS.contains(&c))
                    && l.contains("ing…")
            })
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

    fn is_asking_question(content: &str) -> bool {
        // Check only the content strictly above the input box — not the box itself or
        // anything the user may have typed inside it, which could contain '?'.
        Self::lines_above_input_box(content)
            .map(|lines| lines.iter().rev().take(4).any(|l| l.contains('?')))
            .unwrap_or(false)
    }

    fn is_input_box(content: &str) -> bool {
        Self::lines_above_input_box(content).is_some()
    }

    /// Returns the non-empty lines that appear above the input box, or `None` if no
    /// input box is found.
    ///
    /// The input box is bounded top and bottom by a separator line containing `"───"`.
    /// Uses a two-pass approach: find the bottom separator within the last 3 non-empty
    /// lines (it is always at the very end of the visible pane), then scan upward from
    /// there to find the top separator. This correctly handles multi-line input — the
    /// user may have typed or pasted many lines, pushing the top separator arbitrarily
    /// far from the bottom.
    fn lines_above_input_box<'a>(content: &'a str) -> Option<Vec<&'a str>> {
        let non_empty: Vec<&'a str> = content.lines().filter(|l| !l.trim().is_empty()).collect();

        // Pass 1: bottom separator must be within the last 3 non-empty lines.
        let bottom_idx = non_empty
            .iter()
            .enumerate()
            .rev()
            .take(3)
            .find(|(_, l)| l.contains("───"))
            .map(|(i, _)| i)?;

        // Pass 2: scan upward from just above the bottom separator for the top separator.
        let top_idx = non_empty[..bottom_idx]
            .iter()
            .enumerate()
            .rev()
            .find(|(_, l)| l.contains("───"))
            .map(|(i, _)| i)?;

        Some(non_empty[..top_idx].to_vec())
    }

    pub(super) fn display(&self) -> (&'static str, Color) {
        match self {
            ClaudeStatus::AwaitingInput => theme::ICON_AWAITING_INPUT,
            ClaudeStatus::Done => theme::ICON_DONE,
            ClaudeStatus::AwaitingPermission => theme::ICON_AWAITING_PERMISSION,
            ClaudeStatus::Thinking => theme::ICON_THINKING,
            ClaudeStatus::Executing => theme::ICON_EXECUTING,
            ClaudeStatus::Unknown => theme::ICON_UNKNOWN,
        }
    }
}
