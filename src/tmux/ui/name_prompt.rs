//! Single-field name prompt used after the user selects "+ New …" in the picker.

use crate::tmux::pane::SessionName;
use crossterm::event::KeyCode;

/// What is being created — carries the context needed to dispatch the right
/// [`AppAction`] when the user confirms, and to reconstruct the correct picker
/// level when the user cancels.
pub(super) enum PendingCreate {
    Session,
    Window { session: SessionName },
}

impl PendingCreate {
    pub(super) fn prompt(&self) -> &'static str {
        match self {
            PendingCreate::Session => "Session name: ",
            PendingCreate::Window { .. } => "Window name: ",
        }
    }
}

pub(super) struct NamePromptState {
    pub(super) pending: PendingCreate,
    pub(super) buf: String,
}

pub(super) enum NamePromptResult {
    Continue,
    /// Name entered — caller reads `state.buf` and `state.pending`.
    Done,
    /// Esc pressed — caller rebuilds the picker level from `state.pending`.
    Cancel,
}

impl NamePromptState {
    pub(super) fn new(pending: PendingCreate) -> Self {
        NamePromptState {
            pending,
            buf: String::new(),
        }
    }

    pub(super) fn handle_key(&mut self, code: KeyCode) -> NamePromptResult {
        match code {
            KeyCode::Char(c) => {
                self.buf.push(c);
                NamePromptResult::Continue
            }
            KeyCode::Backspace => {
                self.buf.pop();
                NamePromptResult::Continue
            }
            KeyCode::Enter if !self.buf.is_empty() => NamePromptResult::Done,
            KeyCode::Esc => NamePromptResult::Cancel,
            _ => NamePromptResult::Continue,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyCode;

    fn prompt(pending: PendingCreate) -> NamePromptState {
        NamePromptState::new(pending)
    }

    #[test]
    fn characters_accumulate_in_buf() {
        let mut s = prompt(PendingCreate::Session);
        s.handle_key(KeyCode::Char('f'));
        s.handle_key(KeyCode::Char('o'));
        s.handle_key(KeyCode::Char('o'));
        assert_eq!(s.buf, "foo");
    }

    #[test]
    fn backspace_removes_last_character() {
        let mut s = prompt(PendingCreate::Session);
        s.handle_key(KeyCode::Char('a'));
        s.handle_key(KeyCode::Char('b'));
        s.handle_key(KeyCode::Backspace);
        assert_eq!(s.buf, "a");
    }

    #[test]
    fn backspace_on_empty_buf_is_a_noop() {
        let mut s = prompt(PendingCreate::Session);
        let result = s.handle_key(KeyCode::Backspace);
        assert!(matches!(result, NamePromptResult::Continue));
        assert!(s.buf.is_empty());
    }

    #[test]
    fn enter_on_non_empty_buf_returns_done() {
        let mut s = prompt(PendingCreate::Session);
        s.handle_key(KeyCode::Char('x'));
        assert!(matches!(
            s.handle_key(KeyCode::Enter),
            NamePromptResult::Done
        ));
    }

    #[test]
    fn enter_on_empty_buf_does_not_submit() {
        let mut s = prompt(PendingCreate::Session);
        // Should NOT return Done — empty name is not valid.
        assert!(matches!(
            s.handle_key(KeyCode::Enter),
            NamePromptResult::Continue
        ));
    }

    #[test]
    fn esc_returns_cancel() {
        let mut s = prompt(PendingCreate::Session);
        s.handle_key(KeyCode::Char('a'));
        assert!(matches!(
            s.handle_key(KeyCode::Esc),
            NamePromptResult::Cancel
        ));
    }
}
